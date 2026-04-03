// Licensed under the Apache-2.0 license

use crate::common::{DummyDelay, UartLogger};
use crate::i2cmonitor::I2cMonitor;
use crate::pinctrl;
use crate::uart_core::UartController;
use ast1060_pac::{I2cFilterThr, I2cfilter, Peripherals, Scu};
use embedded_hal::delay::DelayNs;
use embedded_io::Write;

/*  example filter bitmap

let data_flt: [AstI2cFBitmap; 6] = [
    // block all (index 0)
    AstI2cFBitmap { element: [0; 8] },
    // accept all (index 1)
    AstI2cFBitmap {
        element: [0xffff_ffff; 8],
    },
    // block every 16 bytes (index 2)
    AstI2cFBitmap {
        element: [0xffff_0000; 8],
    },
    // block first 16 bytes (index 3)
    AstI2cFBitmap {
        element: [
            0xffff_0000,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
        ],
    },
    // block first 128 bytes (index 4)
    AstI2cFBitmap {
        element: [
            0x0,
            0x0,
            0x0,
            0x0,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
        ],
    },
    // block last 128 bytes (index 5)
    AstI2cFBitmap {
        element: [
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0xffff_ffff,
            0x0,
            0x0,
            0x0,
            0x0,
        ],
    },
];*/

pub fn test_i2cmonitor(uart: &mut UartController<'_>) {
    let peripherals = unsafe { Peripherals::steal() };
    let mut delay = DummyDelay {};
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut dbg_uart = UartController::new(uart_regs);
    let mut i2c_monitor = I2cMonitor::new(
        peripherals.i2cfilter,
        peripherals.i2c_filter_thr,
        peripherals.i2c_filter_thr1,
        peripherals.i2c_filter_thr2,
        peripherals.i2c_filter_thr3,
        UartLogger::new(&mut dbg_uart),
    );

    writeln!(uart, "\r\n####### I2C filter test #######\r").unwrap();

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2CF0);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2CF1);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2CF2);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2CF3);

    // enable it if i2c is not enabled
    enable_i2c_filter_register_write_access();

    for i in 0..4 {
        i2c_monitor.ast_i2c_filter_init(i);
        writeln!(uart, "i2c filter {i:} init done\r").unwrap();
        i2c_monitor
            .ast_i2c_filter_en(true, false, false, false, i)
            .unwrap();
        writeln!(uart, "bypass i2c filter {i:}\r").unwrap();
    }
    i2c_monitor.dump_regs();

    // test i2c filter 2 (0-based)
    let i2c_monitor_idx = 2;
    loop {
        write!(
            uart,
            "i2c filter {i2c_monitor_idx:} will be set to block all after 60s"
        )
        .unwrap();
        for _ in 0..60 {
            write!(uart, ".").unwrap();
            delay.delay_ms(100);
        }
        writeln!(uart, "\r\n").unwrap();

        i2c_monitor
            .ast_i2c_filter_en(true, true, false, false, i2c_monitor_idx)
            .unwrap();
        writeln!(uart, "i2c filter {i2c_monitor_idx:} block all\r").unwrap();

        write!(
            uart,
            "i2c filter {i2c_monitor_idx:} will be set to allow all after 60s"
        )
        .unwrap();
        for _ in 0..60 {
            write!(uart, ".").unwrap();
            delay.delay_ms(100);
        }
        writeln!(uart, "\r\n").unwrap();

        i2c_monitor
            .ast_i2c_filter_en(true, false, true, true, i2c_monitor_idx)
            .unwrap();
        writeln!(uart, "i2c filter {i2c_monitor_idx:} allow all\r").unwrap();
    }
}

// this is necessary if only test with i2c filter
fn enable_i2c_filter_register_write_access() {
    let scu = unsafe { &*Scu::ptr() };
    scu.scu050().write(|w| w.rst_i2csmbus_ctrl().set_bit());
    let mut delay = DummyDelay {};
    delay.delay_ns(1_000_000); // 1ms delay
    scu.scu054().write(|w| unsafe { w.bits(0x4) });
    delay.delay_ns(1_000_000); // 1ms delay
}

pub fn test_i2cmonitor_register_write(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n####### I2C filter reg write test #########\r").unwrap();

    enable_i2c_filter_register_write_access();

    // i2cfilter register
    let p0 = unsafe { &*I2cfilter::ptr() };
    p0.i2cfilter008()
        .write(|w| unsafe { w.topirqen().bits(0xf) });
    writeln!(
        uart,
        "i2cfilter008 {:#x}\r",
        p0.i2cfilter008().read().topirqen().bits()
    )
    .unwrap();

    //i2cfilter thr register
    let p1 = unsafe { &*I2cFilterThr::PTR };
    p1.i2cfilterthr08()
        .write(|w| unsafe { w.bits(0x1234_abcd) });
    writeln!(
        uart,
        "i2cfilterthr08 {:#x}\r",
        p1.i2cfilterthr08().read().bits()
    )
    .unwrap();

    p1.i2cfilterthr18().write(|w| unsafe { w.bits(0x1) });
    writeln!(
        uart,
        "i2cfilterthr18 {:#x}\r",
        p1.i2cfilterthr18().read().bits()
    )
    .unwrap();

    p1.i2cfilterthr14().write(|w| unsafe { w.bits(0x1) });
    writeln!(
        uart,
        "i2cfilterthr14 {:#x}\r",
        p1.i2cfilterthr14().read().bits()
    )
    .unwrap();

    writeln!(
        uart,
        "i2cfilterthr20 {:#x}\r",
        p1.i2cfilterthr20().read().bits()
    )
    .unwrap();

    writeln!(uart, "\r\n##########################################\r").unwrap();
}
