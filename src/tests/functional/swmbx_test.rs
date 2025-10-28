// Licensed under the Apache-2.0 license

use crate::common::NoOpLogger;
use crate::i2c::ast1060_i2c::Ast1060I2c;
use crate::i2c::common::I2cConfigBuilder;
use crate::i2c::i2c_controller::{HardwareInterface, I2cController};
use crate::pinctrl;
use crate::uart::UartController;
use embedded_io::Write;

use crate::i2c::pfr::swmbx;
use crate::i2c::pfr::swmbx::SwmbxCtrl;
use crate::i2c::target::swmbx_target::SwmbxI2CTarget;
use core::mem::MaybeUninit;
use cortex_m::peripheral::NVIC;

static mut SWMBX_TARGET: MaybeUninit<SwmbxI2CTarget> = MaybeUninit::uninit();

pub const UFM_WRITE_FIFO: u8 = 0xd;
pub const UFM_READ_FIFO: u8 = 0xe;
pub const SWMBX_WRITE_FIFO_SIZE: u8 = 64;
pub const SWMBX_READ_FIFO_SIZE: u8 = 128;

pub const BMC_UPDATE_INTENT: u8 = 0x13;

#[no_mangle]
static mut UART_PTR: Option<&'static mut UartController<'static>> = None;

#[cfg(feature = "i2c_target")]
static mut I2C_PTR: Option<
    &'static mut I2cController<
        Ast1060I2c<ast1060_pac::I2c, SwmbxI2CTarget, NoOpLogger>,
        NoOpLogger,
    >,
> = None;
#[cfg(feature = "i2c_target")]
pub fn test_swmbx(uart: &mut UartController<'_>) {
    unsafe {
        UART_PTR = Some(core::mem::transmute::<
            &mut UartController<'_>,
            &'static mut UartController<'static>,
        >(uart));
    }
    writeln!(uart, "\r\n####### SWMBX test #######\r\n").unwrap();
    writeln!(uart, "\r\n## SWMBX: Starting up...\r\n").ok();

    let swmbx = SwmbxCtrl::init(swmbx::SWMBX_BUF_SIZE);
    let _ = swmbx.enable_behavior(
        swmbx::SWMBX_PROTECT | swmbx::SWMBX_NOTIFY | swmbx::SWMBX_FIFO,
        true,
    );
    let _ = swmbx.update_fifo(
        0,
        UFM_WRITE_FIFO,
        SWMBX_WRITE_FIFO_SIZE,
        swmbx::SWMBX_FIFO_NOTIFY_STOP,
        true,
    );
    let _ = swmbx.update_fifo(
        1,
        UFM_READ_FIFO,
        SWMBX_READ_FIFO_SIZE,
        swmbx::SWMBX_FIFO_NOTIFY_STOP,
        true,
    );
    let _ = swmbx.update_notify(0, BMC_UPDATE_INTENT, true);
    let access_control: [[u32; 8]; 2] = [
        // BMC
        [
            0xfff704ff, 0xffffffff, 0xffffffff, 0xfffffff2, 0xffffffff, 0xffffffff, 0x00000000,
            0x00000000,
        ],
        // PCH
        [
            0xfff884ff, 0xffffffff, 0xffffffff, 0xfffffff5, 0x00000000, 0x00000000, 0xffffffff,
            0xffffffff,
        ],
    ];
    let _ = swmbx.apply_protect(0, &access_control[0], 0);
    let _ = swmbx.apply_protect(1, &access_control[1], 0);
    SwmbxCtrl::store_ctrl_ptr(swmbx);

    let target = unsafe {
        SWMBX_TARGET.write(SwmbxI2CTarget::new(0, 0x38).expect("invalid address"));
        &mut *SWMBX_TARGET.as_mut_ptr()
    };

    let mut i2c0 = I2cController {
        hardware: Ast1060I2c::new(NoOpLogger {}),
        config: I2cConfigBuilder::new().build(),
        logger: NoOpLogger {},
    };

    target
        .attach(&mut i2c0)
        .expect("i2c target register failed");
    unsafe {
        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C0);
        I2C_PTR = Some(core::mem::transmute::<
            &mut I2cController<
                Ast1060I2c<ast1060_pac::I2c, SwmbxI2CTarget, NoOpLogger>,
                NoOpLogger,
            >,
            &'static mut I2cController<
                Ast1060I2c<ast1060_pac::I2c, SwmbxI2CTarget, NoOpLogger>,
                NoOpLogger,
            >,
        >(&mut i2c0));
        NVIC::unmask(ast1060_pac::Interrupt::i2c);
    }
    loop {
        // Execute the following command at the BMC prompt to verify SWMBX functionality:
        // - Verify the SWMBX address:
        //   aspeed-pfr-tool -w 0x13 0x8
        //   aspeed-pfr-tool -r 0x13
        //   [Expected output: 0x8]
        //
        // - Verify the protected SWMBX address:
        //   aspeed-pfr-tool -w 0x12 0x8
        //   aspeed-pfr-tool -r 0x12
        //   [Expected output: 0x0]
        //
        // - Verify the notify functionality:
        //   aspeed-pfr-tool -w 0x13 0x8
        //   On Ast1060 console, you should see [NOTIFY] SWMBX: notify triggered on port 0 addr 0x13
        //
        //  - Verify the FIFO functionality:
        //   aspeed-pfr-tool -w 0xd 0x1f
        //   aspeed-pfr-tool -w 0xd 0x2f
        //   aspeed-pfr-tool -w 0xd 0x3f
        //   aspeed-pfr-tool -r 0xd
        //   aspeed-pfr-tool -r 0xd
        //   aspeed-pfr-tool -r 0xd
        //   aspeed-pfr-tool -r 0xd
        //   [Expected output: 0x1f, 0x2f, 0x3f, 0x0]
        poll_notify();
    }
}

fn poll_notify() {
    unsafe {
        if let Some(uart) = UART_PTR.as_mut() {
            let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();

            for port in 0..swmbx::SWMBX_DEV_COUNT {
                for addr in 0..swmbx::SWMBX_NODE_COUNT {
                    let node = &mut ctrl.node[port][addr];
                    if node.notify_flag {
                        if let Err(e) = writeln!(
                            uart,
                            "[NOTIFY] SWMBX: notify triggered on port {} addr {:#x}\r\n",
                            port, addr
                        ) {
                            let _ = writeln!(uart, "write error: {:?}\r\n", e);
                        }

                        node.notify_flag = false;
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn i2c() {
    unsafe {
        if let Some(uart) = UART_PTR.as_mut() {
            let _ = uart.write_all(b"[ISR] I2C\r\n");
        }
        if let Some(i2c) = I2C_PTR.as_mut() {
            i2c.hardware.handle_interrupt();
        }
    }
}
