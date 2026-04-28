// Licensed under the Apache-2.0 license

//! I2C Master (Requester) Application with DMA
//!
//! This application demonstrates I2C master mode using DMA to send and receive
//! a 64-byte data chunk to/from an I2C slave device.
//!
//! # Hardware Setup
//! - Uses I2C2 controller
//! - Target slave address: 0x50
//! - Sends 64 bytes of test data
//! - Receives 64 bytes back from slave

#![no_std]
#![no_main]

use aspeed_ddk::common::DmaBuffer;
use aspeed_ddk::i2c_core::{
    self, Ast1060I2c, ClockConfig, Controller, I2cConfig, I2cController, I2cSpeed, I2cXferMode,
};
use aspeed_ddk::pinctrl;
use aspeed_ddk::uart_core::{UartConfig, UartController};
use ast1060_pac::{self, Peripherals};
use core::ptr::{read_volatile, write_volatile};
use cortex_m_rt::{entry, pre_init};
use embedded_io::Write;
use panic_halt as _;

// I2C Configuration
const I2C_MASTER_CTRL_ID: u8 = 2;
const SLAVE_ADDRESS: u8 = 0x50;
const DATA_SIZE: usize = 64;

// DMA buffer in non-cached RAM section
#[link_section = ".ram_nc"]
static mut DMA_BUF: DmaBuffer<4096> = DmaBuffer::new();

#[pre_init]
unsafe fn pre_init() {
    let jtag_pinmux_offset: u32 = 0x7e6e_2000 + 0x41c;
    let mut reg: u32;
    reg = read_volatile(jtag_pinmux_offset as *const u32);
    reg |= 0x1f << 25;
    write_volatile(jtag_pinmux_offset as *mut u32, reg);

    // Disable Cache
    let cache_ctrl_offset: u32 = 0x7e6e_2a58;
    write_volatile(cache_ctrl_offset as *mut u32, 0);

    // Configure Cache Area and Invalidation
    let cache_area_offset: u32 = 0x7e6e_2a50;
    let cache_val = 0x000f_ffff;
    write_volatile(cache_area_offset as *mut u32, cache_val);

    let cache_inval_offset: u32 = 0x7e6e_2a54;
    let cache_inval_val = 0x81e0_0000;
    write_volatile(cache_inval_offset as *mut u32, cache_inval_val);

    // Enable Cache
    write_volatile(cache_ctrl_offset as *mut u32, 1);
}

#[entry]
fn main() -> ! {
    // Get UART register block and create controller
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut uart = UartController::new(uart_regs);
    uart.init(&UartConfig::default()).unwrap();

    // Early startup message
    let _ = writeln!(&mut uart, "\r\n\r\n***** PROGRAM STARTED *****\r");
    let _ = writeln!(&mut uart, "\r\n========================================\r");
    let _ = writeln!(&mut uart, "I2C Master DMA Application\r");
    let _ = writeln!(&mut uart, "Version: 1.0\r");
    let _ = writeln!(&mut uart, "========================================\r\n");

    let peripherals = unsafe { Peripherals::steal() };

    let _ = writeln!(&mut uart, "[MASTER] Initializing I2C global registers...\r");

    // Initialize I2C global registers (must be called once before using I2C)
    i2c_core::init_i2c_global();

    let _ = writeln!(&mut uart, "[MASTER] Step 1: Applying pin control for I2C2...\r");

    // Apply pin control for I2C2
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C2);
    let _ = writeln!(&mut uart, "[MASTER] Step 1: Pin control applied\r");

    let _ = writeln!(&mut uart, "[MASTER] Step 2: Getting I2C2 registers...\r");

    unsafe {
        // Get I2C2 registers
        let i2c_regs = &peripherals.i2c2;
        let buff_regs = &peripherals.i2cbuff2;
        let _ = writeln!(&mut uart, "[MASTER] Step 2: Registers obtained\r");

        let _ = writeln!(&mut uart, "[MASTER] Step 3: Creating controller ID...\r");
        let Some(controller_id) = Controller::new(I2C_MASTER_CTRL_ID) else {
            let _ = writeln!(&mut uart, "[FAIL] Invalid controller ID\r");
            panic!("Invalid controller ID");
        };
        let _ = writeln!(&mut uart, "[MASTER] Step 3: Controller ID created\r");

        let _ = writeln!(&mut uart, "[MASTER] Step 4: Creating I2C controller wrapper...\r");
        let controller = I2cController {
            controller: controller_id,
            registers: i2c_regs,
            buff_registers: buff_regs,
        };
        let _ = writeln!(&mut uart, "[MASTER] Step 4: Controller wrapper created\r");

        // Configure I2C in DMA mode
        let _ = writeln!(&mut uart, "[MASTER] Step 5: Configuring I2C in DMA mode...\r");
        let config = I2cConfig {
            speed: I2cSpeed::Standard,
            xfer_mode: I2cXferMode::DmaMode,
            multi_master: true,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        };
        let _ = writeln!(&mut uart, "[MASTER] Step 5: Config created\r");

        // Get DMA buffer
        let _ = writeln!(&mut uart, "[MASTER] Step 6: Getting DMA buffer...\r");
        let dma_buf: &mut [u8] = &mut DMA_BUF.buf;
        let _ = writeln!(&mut uart, "[MASTER] Step 6: DMA buffer at {:p}, size {}\r", dma_buf.as_ptr(), dma_buf.len());

        let _ = writeln!(&mut uart, "[MASTER] Step 7: Calling new_with_dma()...\r");
        let mut i2c = match Ast1060I2c::new_with_dma(&controller, config, dma_buf) {
            Ok(i) => {
                let _ = writeln!(&mut uart, "[MASTER] Step 7: I2C1 DMA initialized successfully ✓\r");
                i
            }
            Err(e) => {
                let _ = writeln!(&mut uart, "[FAIL] Step 7: I2C1 DMA init error: {e:?}\r");
                panic!("I2C init failed");
            }
        };

        // Check bus state
        let _ = writeln!(&mut uart, "[MASTER] Step 7b: Checking I2C bus state...\r");
        let is_busy = i2c.is_bus_busy();
        let _ = writeln!(&mut uart, "[MASTER] Step 7b: Bus busy = {}\r", is_busy);
        if is_busy {
            let _ = writeln!(&mut uart, "[WARN] I2C bus is busy after init! This may indicate a problem.\r");
        }

        // Prepare 64-byte test data
        let _ = writeln!(&mut uart, "[MASTER] Step 8: Preparing test data...\r");
        let mut write_data = [0u8; DATA_SIZE];
        for (idx, byte) in write_data.iter_mut().enumerate() {
            *byte = (idx as u8).wrapping_add(0x10);
        }
        let _ = writeln!(&mut uart, "[MASTER] Step 8: Test data prepared\r");

        let _ = writeln!(
            &mut uart,
            "[MASTER] Step 9: Sending {} bytes to slave 0x{:02X}...\r",
            DATA_SIZE, SLAVE_ADDRESS
        );
        let _ = writeln!(&mut uart, "  Data to send: {:02X?}...\r", &write_data[..8]);

        // Send data to slave
        let _ = writeln!(&mut uart, "[MASTER] Step 9: Calling i2c.write()...\r");
        match i2c.write(SLAVE_ADDRESS, &write_data) {
            Ok(()) => {
                let _ = writeln!(&mut uart, "[MASTER] Step 9: Write completed successfully ✓\r");
            }
            Err(e) => {
                let _ = writeln!(&mut uart, "[FAIL] Step 9: Write error: {e:?}\r");
                let _ = writeln!(&mut uart, "[INFO] This may be expected if slave is not ready\r");
                // Don't panic, continue to show the issue
            }
        }

        // Small delay to let slave process
        for _ in 0..100_000 {
            cortex_m::asm::nop();
        }

        // Read data back from slave
        let mut read_data = [0u8; DATA_SIZE];
        let _ = writeln!(
            &mut uart,
            "[MASTER] Reading {} bytes from slave 0x{:02X}...\r",
            DATA_SIZE, SLAVE_ADDRESS
        );

        match i2c.read(SLAVE_ADDRESS, &mut read_data) {
            Ok(()) => {
                let _ = writeln!(&mut uart, "[MASTER] Read completed successfully\r");
                let _ = writeln!(&mut uart, "  Data: {:02X?}...\r", &read_data[..8]);
            }
            Err(e) => {
                let _ = writeln!(&mut uart, "[FAIL] Read error: {e:?}\r");
                panic!("Read failed");
            }
        }

        // Verify data
        let mut matches = 0;
        for (idx, (&sent, &received)) in write_data.iter().zip(read_data.iter()).enumerate() {
            if sent == received {
                matches += 1;
            } else {
                let _ = writeln!(
                    &mut uart,
                    "[WARN] Mismatch at index {}: sent=0x{:02X}, received=0x{:02X}\r",
                    idx, sent, received
                );
            }
        }

        let _ = writeln!(
            &mut uart,
            "\n[MASTER] Transfer complete: {}/{} bytes matched\r",
            matches, DATA_SIZE
        );
        let _ = writeln!(&mut uart, "========================================\r\n");

        if matches == DATA_SIZE {
            let _ = writeln!(&mut uart, "[SUCCESS] All data matched!\r");
        } else {
            let _ = writeln!(&mut uart, "[FAIL] Data mismatch detected\r");
        }
    }

    // Halt
    loop {
        cortex_m::asm::wfi();
    }
}

