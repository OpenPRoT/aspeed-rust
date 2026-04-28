// Licensed under the Apache-2.0 license

//! I2C Slave (Receiver) Application with DMA
//!
//! This application demonstrates I2C slave mode using DMA to receive and send
//! a 64-byte data chunk to/from an I2C master device.
//!
//! # Hardware Setup
//! - Uses I2C2 controller
//! - Slave address: 0x50
//! - Receives 64 bytes from master
//! - Echoes back the same 64 bytes to master

#![no_std]
#![no_main]

use aspeed_ddk::common::DmaBuffer;
use aspeed_ddk::i2c_core::{
    self, Ast1060I2c, ClockConfig, Controller, I2cConfig, I2cController, I2cSpeed, I2cXferMode,
    SlaveConfig, SlaveEvent,
};
use aspeed_ddk::pinctrl;
use aspeed_ddk::uart_core::{UartConfig, UartController};
use ast1060_pac::{self, Peripherals};
use core::ptr::{read_volatile, write_volatile};
use cortex_m_rt::{entry, pre_init};
use embedded_io::Write;
use panic_halt as _;

// I2C Configuration
const I2C_SLAVE_CTRL_ID: u8 = 2;
const SLAVE_ADDRESS: u8 = 0x50;
const DATA_SIZE: usize = 64;
const MAX_TRANSACTIONS: u32 = 2;

// DMA buffer in non-cached RAM section
// In DMA mode, ALL data transfers use this buffer!
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
    let peripherals = unsafe { Peripherals::steal() };

    // Get UART register block and create controller
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut uart = UartController::new(uart_regs);
    uart.init(&UartConfig::default()).unwrap();

    // Early startup message
    let _ = writeln!(&mut uart, "\r\n\r\n***** PROGRAM STARTED *****\r");
    let _ = writeln!(&mut uart, "\r\n========================================\r");
    let _ = writeln!(&mut uart, "I2C Slave DMA Application\r");
    let _ = writeln!(&mut uart, "Version: 1.0\r");
    let _ = writeln!(&mut uart, "========================================\r\n");

    let _ = writeln!(&mut uart, "[SLAVE] Initializing I2C global registers...\r");

    // Initialize I2C global registers (must be called once before using I2C)
    i2c_core::init_i2c_global();

    let _ = writeln!(
        &mut uart,
        "[SLAVE] Initializing I2C2 in DMA mode at address 0x{:02X}...\r",
        SLAVE_ADDRESS
    );

    unsafe {

        // Apply pin control for I2C2
        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C2);

        // Get I2C2 registers
        let i2c_regs = &peripherals.i2c2;
        let buff_regs = &peripherals.i2cbuff2;

        let Some(controller_id) = Controller::new(I2C_SLAVE_CTRL_ID) else {
            let _ = writeln!(&mut uart, "[FAIL] Invalid controller ID\r");
            panic!("Invalid controller ID");
        };

        let controller = I2cController {
            controller: controller_id,
            registers: i2c_regs,
            buff_registers: buff_regs,
        };

        // Configure I2C in DMA mode
        let config = I2cConfig {
            speed: I2cSpeed::Standard,
            xfer_mode: I2cXferMode::DmaMode,
            multi_master: false,
            smbus_timeout: true,
            smbus_alert: false,
            clock_config: ClockConfig::ast1060_default(),
        };

        // Get DMA buffer
        let dma_buf: &mut [u8] = &mut DMA_BUF.buf;

        let mut i2c = match Ast1060I2c::new_with_dma(&controller, config, dma_buf) {
            Ok(i) => {
                let _ = writeln!(&mut uart, "[SLAVE] I2C2 DMA mode initialized successfully\r");
                i
            }
            Err(e) => {
                let _ = writeln!(&mut uart, "[FAIL] I2C2 init error: {e:?}\r");
                panic!("I2C init failed");
            }
        };

        // Configure as slave
        let slave_cfg = match SlaveConfig::new(SLAVE_ADDRESS) {
            Ok(cfg) => cfg,
            Err(e) => {
                let _ = writeln!(&mut uart, "[FAIL] Invalid slave config: {e:?}\r");
                panic!("Slave config failed");
            }
        };

        if let Err(e) = i2c.configure_slave(&slave_cfg) {
            let _ = writeln!(&mut uart, "[FAIL] Configure slave error: {e:?}\r");
            panic!("Configure slave failed");
        }

        let _ = writeln!(
            &mut uart,
            "[SLAVE] Configured at address 0x{:02X}\r",
            SLAVE_ADDRESS
        );

        let _ = writeln!(&mut uart, "[SLAVE] Waiting for master transactions...\r\n");

        // Event loop
        let mut transaction_count = 0u32;
        let mut poll_count = 0u32;

        loop {
            if let Some(event) = i2c.handle_slave_interrupt() {
                match event {
                    SlaveEvent::WriteRequest => {
                        let _ = writeln!(&mut uart, "[SLAVE] Write request from master\r");
                    }
                    SlaveEvent::DataReceived { len } => {
                        let _ = writeln!(&mut uart, "[SLAVE] DataReceived: len={}\r", len);

                        // In DMA mode, data is already in DMA_BUF
                        // Read it to get the actual length and re-arm RX
                        let mut temp_buf = [0u8; 256];
                        if let Ok(n) = i2c.slave_read(&mut temp_buf) {
                            let _ = writeln!(&mut uart, "  Received {} bytes (including address): {:02X?}...\r",
                                n, &temp_buf[..8.min(n)]);

                            // Skip the first byte (address byte 0xA0) when echoing
                            // The hardware saves the address byte due to AST_I2CC_SLAVE_PKT_SAVE_ADDR
                            if n > 1 {
                                let data_bytes = &temp_buf[1..n];
                                let _ = writeln!(&mut uart, "  Data bytes to echo: {} ({:02X?}...)\r",
                                    data_bytes.len(), &data_bytes[..8.min(data_bytes.len())]);

                                if let Err(e) = i2c.slave_write(data_bytes) {
                                    let _ = writeln!(&mut uart, "[WARN] Slave write error: {e:?}\r");
                                } else {
                                    let _ = writeln!(&mut uart, "  Prepared to echo {} bytes\r", data_bytes.len());
                                }
                            }
                        }
                        transaction_count += 1;
                    }
                    SlaveEvent::ReadRequest => {
                        let _ = writeln!(&mut uart, "[SLAVE] Read request from master\r");
                        // Buffer should already be prepared from DataReceived
                    }
                    SlaveEvent::DataSent { len } => {
                        let _ = writeln!(&mut uart, "[SLAVE] Sent {} bytes\r", len);
                        transaction_count += 1;
                    }
                    SlaveEvent::DataReceivedAndSent { rx_len, tx_len } => {
                        let _ = writeln!(
                            &mut uart,
                            "[SLAVE] DataReceivedAndSent: rx_len={}, tx_len={}\r",
                            rx_len, tx_len
                        );

                        // This event means the previous slave_write() was used
                        // tx_len tells us how much was actually sent
                        let _ = writeln!(&mut uart, "  Hardware sent {} bytes from previous slave_write\r", tx_len);

                        // Don't call slave_write again here - the data was already sent!
                        // Just log what happened
                        let _ = writeln!(&mut uart, "  TX completed: {:02X?}...\r", &DMA_BUF.buf[..8.min(tx_len)]);

                        transaction_count += 1;
                    }
                    SlaveEvent::Stop => {
                        let _ = writeln!(&mut uart, "[SLAVE] Stop condition\r");
                    }
                }
            }

            poll_count += 1;
            if poll_count.is_multiple_of(1_000_000) {
                let _ = writeln!(
                    &mut uart,
                    "[SLAVE] Waiting... (transactions: {})\r",
                    transaction_count
                );
            }

            // Exit after some transactions
            if transaction_count >= MAX_TRANSACTIONS {
                let _ = writeln!(
                    &mut uart,
                    "\n[SLAVE] Completed {} transactions\r",
                    transaction_count
                );
                break;
            }
        }

        i2c.disable_slave();
        let _ = writeln!(&mut uart, "[SLAVE] Slave mode disabled\r");
        let _ = writeln!(&mut uart, "========================================\r\n");
        let _ = writeln!(&mut uart, "[SUCCESS] Slave application completed\r");
    }

    // Halt
    loop {
        cortex_m::asm::wfi();
    }
}

