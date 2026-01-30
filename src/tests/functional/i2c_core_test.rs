// Licensed under the Apache-2.0 license

//! Functional tests for the `i2c_core` module
//!
//! These tests run on-target (QEMU or hardware) and exercise the
//! refactored I2C driver through actual register access.
//!
//! # Design: Decoupled Logging
//!
//! Unlike the original `i2c` module which has `UartLogger` coupled into the
//! hardware struct, `i2c_core` has no logging dependencies. All diagnostic
//! output is handled at this test layer via the UART controller passed to
//! each test function. This keeps the driver portable and testable.

use crate::i2c_core::{
    Ast1060I2c, ClockConfig, Controller, I2cConfig, I2cController, I2cError, I2cSpeed, I2cXferMode,
    SlaveConfig,
};
use crate::pinctrl;
use crate::uart_core::UartController;
use ast1060_pac::Peripherals;
use embedded_io::Write;

// ============================================================================
// Test Logging Helpers (adapter pattern - keeps i2c_core decoupled)
// ============================================================================

/// Log an I2C operation result with context
fn log_i2c_result<T: core::fmt::Debug>(
    uart: &mut UartController<'_>,
    op: &str,
    result: &Result<T, I2cError>,
) {
    match result {
        Ok(val) => {
            writeln!(uart, "  [LOG] {op} ok: {val:?}\r").ok();
        }
        Err(e) => {
            writeln!(uart, "  [LOG] {op} err: {e:?}\r").ok();
        }
    }
}

/// Log I2C write operation with address and data
fn log_write(uart: &mut UartController<'_>, addr: u8, data: &[u8]) {
    write!(uart, "  [LOG] write to 0x{addr:02X}: [").ok();
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            write!(uart, ", ").ok();
        }
        write!(uart, "0x{b:02X}").ok();
    }
    writeln!(uart, "]\r").ok();
}

/// Log I2C read operation with address and received data
fn log_read(uart: &mut UartController<'_>, addr: u8, data: &[u8]) {
    write!(uart, "  [LOG] read from 0x{addr:02X}: [").ok();
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            write!(uart, ", ").ok();
        }
        write!(uart, "0x{b:02X}").ok();
    }
    writeln!(uart, "]\r").ok();
}

// ============================================================================
// Functional Tests
// ============================================================================

/// Test I2C core initialization and timing configuration
pub fn test_i2c_core_init(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n####### I2C Core Init Test #######\r").ok();

    let peripherals = unsafe { Peripherals::steal() };

    // Get I2C1 registers (commonly used for master mode)
    let i2c_regs = &peripherals.i2c1;
    let buff_regs = &peripherals.i2cbuff1;

    // Create I2cController wrapper
    let Some(controller_id) = Controller::new(1) else {
        writeln!(uart, "FAIL: Invalid controller ID\r").ok();
        return;
    };
    let controller = I2cController {
        controller: controller_id,
        registers: i2c_regs,
        buff_registers: buff_regs,
    };

    // Test 1: Default configuration
    writeln!(uart, "Test 1: Default I2cConfig...").ok();
    let config = I2cConfig::default();
    writeln!(
        uart,
        "  Speed: {:?}, XferMode: {:?}, SMBus timeout: {}\r",
        config.speed, config.xfer_mode, config.smbus_timeout
    )
    .ok();

    // Test 2: Create controller with Standard speed
    writeln!(uart, "Test 2: Standard mode (100kHz)...").ok();
    let config_std = I2cConfig {
        speed: I2cSpeed::Standard,
        xfer_mode: I2cXferMode::BufferMode,
        multi_master: true,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    match Ast1060I2c::new(&controller, config_std) {
        Ok(_i2c) => writeln!(uart, "  PASS: Standard mode init OK\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Standard mode init error: {e:?}\r").ok(),
    };

    // Test 3: Create controller with Fast speed
    writeln!(uart, "Test 3: Fast mode (400kHz)...").ok();
    let config_fast = I2cConfig {
        speed: I2cSpeed::Fast,
        xfer_mode: I2cXferMode::BufferMode,
        multi_master: true,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    match Ast1060I2c::new(&controller, config_fast) {
        Ok(_i2c) => writeln!(uart, "  PASS: Fast mode init OK\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Fast mode init error: {e:?}\r").ok(),
    };

    // Test 4: Create controller with Fast-plus speed
    writeln!(uart, "Test 4: Fast-plus mode (1MHz)...").ok();
    let config_fp = I2cConfig {
        speed: I2cSpeed::FastPlus,
        xfer_mode: I2cXferMode::BufferMode,
        multi_master: false,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    match Ast1060I2c::new(&controller, config_fp) {
        Ok(_i2c) => writeln!(uart, "  PASS: Fast-plus mode init OK\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Fast-plus mode init error: {e:?}\r").ok(),
    };

    writeln!(uart, "####### I2C Core Init Test Complete #######\r\n").ok();
}

/// Test I2C master mode operations
pub fn test_i2c_core_master(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n####### I2C Core Master Test #######\r").ok();

    let peripherals = unsafe { Peripherals::steal() };

    let i2c_regs = &peripherals.i2c1;
    let buff_regs = &peripherals.i2cbuff1;

    let Some(controller_id) = Controller::new(1) else {
        writeln!(uart, "FAIL: Invalid controller ID\r").ok();
        return;
    };
    let controller = I2cController {
        controller: controller_id,
        registers: i2c_regs,
        buff_registers: buff_regs,
    };

    let config = I2cConfig {
        speed: I2cSpeed::Standard,
        xfer_mode: I2cXferMode::BufferMode,
        multi_master: true,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    let mut i2c = match Ast1060I2c::new(&controller, config) {
        Ok(i) => {
            writeln!(uart, "I2C1 initialized\r").ok();
            i
        }
        Err(e) => {
            writeln!(uart, "FAIL: I2C1 init error: {e:?}\r").ok();
            return;
        }
    };

    // Apply pin control for I2C1
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C1);

    // Test device: ADT7490 temperature sensor at 0x2e (common on ASPEED boards)
    // Note: In QEMU, no I2C devices are emulated, so we expect NoAcknowledge
    let addr: u8 = 0x2e;
    let verbose = false; // Set to true for detailed logging

    // Test 1: Write register address
    writeln!(uart, "Test 1: Write to device 0x{addr:02X}...").ok();
    let write_buf = [0x4e_u8]; // Device ID register
    if verbose {
        log_write(uart, addr, &write_buf);
    }
    let result = i2c.write(addr, &write_buf);
    if verbose {
        log_i2c_result(uart, "write", &result);
    }
    match result {
        Ok(()) => writeln!(uart, "  PASS: Write OK\r").ok(),
        Err(I2cError::NoAcknowledge) => {
            writeln!(uart, "  INFO: NoAck (expected in QEMU - no device)\r").ok()
        }
        Err(I2cError::Timeout) => writeln!(uart, "  INFO: Timeout (expected in QEMU)\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Write error: {e:?}\r").ok(),
    };

    // Test 2: Read from device
    writeln!(uart, "Test 2: Read from device 0x{addr:02X}...").ok();
    let mut read_buf = [0u8; 1];
    let result = i2c.read(addr, &mut read_buf);
    if verbose {
        log_read(uart, addr, &read_buf);
        log_i2c_result(uart, "read", &result);
    }
    match result {
        Ok(()) => writeln!(uart, "  PASS: Read OK, data=0x{:02X}\r", read_buf[0]).ok(),
        Err(I2cError::NoAcknowledge) => {
            writeln!(uart, "  INFO: NoAck (expected in QEMU - no device)\r").ok()
        }
        Err(I2cError::Timeout) => writeln!(uart, "  INFO: Timeout (expected in QEMU)\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Read error: {e:?}\r").ok(),
    };

    // Test 3: Write-read combined
    writeln!(uart, "Test 3: Write-read combined...").ok();
    let reg_addr = [0x4e_u8];
    let mut data = [0u8; 1];
    if verbose {
        log_write(uart, addr, &reg_addr);
    }
    let result = i2c.write_read(addr, &reg_addr, &mut data);
    if verbose {
        log_read(uart, addr, &data);
        log_i2c_result(uart, "write_read", &result);
    }
    match result {
        Ok(()) => writeln!(uart, "  PASS: Write-read OK, data=0x{:02X}\r", data[0]).ok(),
        Err(I2cError::NoAcknowledge) => {
            writeln!(uart, "  INFO: NoAck (expected in QEMU - no device)\r").ok()
        }
        Err(I2cError::Timeout) => writeln!(uart, "  INFO: Timeout (expected in QEMU)\r").ok(),
        Err(e) => writeln!(uart, "  FAIL: Write-read error: {e:?}\r").ok(),
    };

    // Test 4: Check bus busy status
    writeln!(uart, "Test 4: Bus busy status...").ok();
    let busy = i2c.is_bus_busy();
    writeln!(uart, "  Bus busy: {busy}\r").ok();
    if busy {
        writeln!(uart, "  WARN: Bus still busy\r").ok();
    } else {
        writeln!(uart, "  PASS: Bus not busy after operations\r").ok();
    }

    writeln!(uart, "####### I2C Core Master Test Complete #######\r\n").ok();
}

/// Test I2C slave configuration (setup only - actual slave requires external master)
pub fn test_i2c_core_slave_config(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n####### I2C Core Slave Config Test #######\r").ok();

    // Test 1: Valid slave address
    writeln!(uart, "Test 1: Valid slave address 0x42...").ok();
    match SlaveConfig::new(0x42) {
        Ok(config) => writeln!(
            uart,
            "  PASS: SlaveConfig created, addr=0x{:02X}\r",
            config.address
        )
        .ok(),
        Err(e) => writeln!(uart, "  FAIL: SlaveConfig error: {e:?}\r").ok(),
    };

    // Test 2: Invalid slave address (>0x7F)
    writeln!(uart, "Test 2: Invalid slave address 0x80...").ok();
    match SlaveConfig::new(0x80) {
        Ok(_) => writeln!(uart, "  FAIL: Should have rejected 0x80\r").ok(),
        Err(I2cError::InvalidAddress) => {
            writeln!(uart, "  PASS: Correctly rejected invalid address\r").ok()
        }
        Err(e) => writeln!(uart, "  FAIL: Unexpected error: {e:?}\r").ok(),
    };

    // Test 3: Reserved address 0x00
    writeln!(uart, "Test 3: Reserved address 0x00...").ok();
    match SlaveConfig::new(0x00) {
        Ok(_) => writeln!(uart, "  INFO: Address 0x00 accepted (general call)\r").ok(),
        Err(e) => writeln!(uart, "  INFO: Address 0x00 rejected: {e:?}\r").ok(),
    };

    // Test 4: Boundary address 0x7F
    writeln!(uart, "Test 4: Boundary address 0x7F...").ok();
    match SlaveConfig::new(0x7F) {
        Ok(config) => writeln!(
            uart,
            "  PASS: SlaveConfig created, addr=0x{:02X}\r",
            config.address
        )
        .ok(),
        Err(e) => writeln!(uart, "  FAIL: SlaveConfig error: {e:?}\r").ok(),
    };

    writeln!(
        uart,
        "####### I2C Core Slave Config Test Complete #######\r\n"
    )
    .ok();
}

/// Test `ClockConfig` values
pub fn test_i2c_core_clocks(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n####### I2C Core Clock Config Test #######\r").ok();

    // Test 1: Default AST1060 clocks
    writeln!(uart, "Test 1: AST1060 default clocks...").ok();
    let clocks = ClockConfig::ast1060_default();
    writeln!(uart, "  APB:       {} Hz\r", clocks.apb_clock_hz).ok();
    writeln!(
        uart,
        "  base_clk1: {} Hz (Fast-plus)\r",
        clocks.base_clk1_hz
    )
    .ok();
    writeln!(uart, "  base_clk2: {} Hz (Fast)\r", clocks.base_clk2_hz).ok();
    writeln!(uart, "  base_clk3: {} Hz (Standard)\r", clocks.base_clk3_hz).ok();
    writeln!(uart, "  base_clk4: {} Hz (Recovery)\r", clocks.base_clk4_hz).ok();

    // Verify expected values
    if clocks.apb_clock_hz == 50_000_000 {
        writeln!(uart, "  PASS: APB clock is 50MHz as expected\r").ok();
    } else {
        writeln!(uart, "  WARN: APB clock differs from expected 50MHz\r").ok();
    }

    // Test 2: Read from hardware
    writeln!(uart, "Test 2: Read clocks from hardware...").ok();
    let hw_clocks = ClockConfig::from_hardware();
    writeln!(uart, "  APB:       {} Hz\r", hw_clocks.apb_clock_hz).ok();
    writeln!(uart, "  base_clk1: {} Hz\r", hw_clocks.base_clk1_hz).ok();
    writeln!(uart, "  base_clk2: {} Hz\r", hw_clocks.base_clk2_hz).ok();
    writeln!(uart, "  base_clk3: {} Hz\r", hw_clocks.base_clk3_hz).ok();
    writeln!(uart, "  base_clk4: {} Hz\r", hw_clocks.base_clk4_hz).ok();

    writeln!(
        uart,
        "####### I2C Core Clock Config Test Complete #######\r\n"
    )
    .ok();
}

/// Run all `i2c_core` functional tests
pub fn run_i2c_core_tests(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\n========================================\r").ok();
    writeln!(uart, "       I2C CORE FUNCTIONAL TESTS\r").ok();
    writeln!(uart, "========================================\r\n").ok();

    test_i2c_core_clocks(uart);
    test_i2c_core_slave_config(uart);
    test_i2c_core_init(uart);
    test_i2c_core_master(uart);

    writeln!(uart, "\r\n========================================\r").ok();
    writeln!(uart, "   I2C CORE TESTS COMPLETE\r").ok();
    writeln!(uart, "========================================\r\n").ok();
}
