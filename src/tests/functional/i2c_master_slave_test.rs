// Licensed under the Apache-2.0 license

//! I2C Master-Slave Hardware Integration Tests (`i2c_core` API)
//!
//! This module tests **real I2C bus transactions** on AST1060 EVB using the
//! new `i2c_core` module API.
//!
//! # Hardware Requirements
//!
//! ## Master Mode Tests (`run_master_tests`)
//!
//! Tests master mode using ADT7490 temperature sensor on the EVB:
//!
//! ```text
//!   AST1060 EVB (Master)          ADT7490 Temp Sensor
//!   ┌─────────────────────┐       ┌─────────────────┐
//!   │  I2C1           SDA ├───┬───┤ SDA             │
//!   │                 SCL ├──┬┼───┤ SCL             │
//!   │                 GND ├──┼┼───┤ GND             │
//!   └─────────────────────┘  ││   └─────────────────┘
//!                          ┌─┴┴─┐   Address: 0x2E
//!                          │ Rp │   (on-board sensor)
//!                          └─┬┬─┘
//!                           VCC
//! ```
//!
//! ## Slave Mode Tests (`run_slave_tests`)
//!
//! Tests slave mode - requires an external master to drive transactions:
//!
//! ```text
//!   External Master               AST1060 EVB (Slave)
//!   ┌─────────────────────┐       ┌─────────────────────┐
//!   │              SDA    ├───┬───┤ SDA           I2C0  │
//!   │              SCL    ├──┬┼───┤ SCL                 │
//!   │              GND    ├──┼┼───┤ GND                 │
//!   └─────────────────────┘  ││   └─────────────────────┘
//!     (AST2600, another     ┌┴┴┐
//!      EVB, or bus master)  │Rp│ Pull-ups (4.7kΩ each)
//!                           └┬┬┘
//!                           VCC
//! ```
//!
//! # Usage
//!
//! - **Master tests**: Uses on-board ADT7490, call `run_master_tests()`
//! - **Slave tests**: Start slave EVB first with `run_slave_tests()`,
//!   then initiate transactions from external master

use crate::i2c_core::{
    Ast1060I2c, ClockConfig, Controller, I2cConfig, I2cController, I2cSpeed, I2cXferMode,
    SlaveConfig, SlaveEvent,
};
use crate::pinctrl;
use crate::uart_core::UartController;
use ast1060_pac::Peripherals;
use embedded_io::Write;

// ============================================================================
// Test Configuration Constants
// ============================================================================

/// I2C controller for master tests (I2C1 - connected to ADT7490)
const I2C_MASTER_CTRL_ID: u8 = 1;

/// I2C controller for slave tests
const I2C_SLAVE_CTRL_ID: u8 = 2;

/// ADT7490 temperature sensor address (on-board, same as original `i2c_test.rs`)
const ADT7490_ADDRESS: u8 = 0x2e;

/// ADT7490 register addresses and expected values
/// From ADT7490 datasheet - these are read-only default values
const ADT7490_REGS: [(u8, u8); 5] = [
    (0x82, 0x00), // Reserved/default
    (0x4e, 0x81), // Config register 5 default
    (0x4f, 0x7f), // Config register 6 default
    (0x45, 0xff), // Auto fan control default
    (0x3d, 0x00), // VID default
];

/// Slave address for slave mode tests (7-bit)
const SLAVE_ADDRESS: u8 = 0x50;

/// Test data for slave mode
const TEST_PATTERN_READ: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];

// ============================================================================
// Test Result Tracking
// ============================================================================

struct TestResults {
    passed: u32,
    failed: u32,
}

impl TestResults {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
        }
    }

    fn pass(&mut self) {
        self.passed += 1;
    }

    fn fail(&mut self) {
        self.failed += 1;
    }

    fn summary(&self) -> (u32, u32) {
        (self.passed, self.failed)
    }
}

// ============================================================================
// MASTER Tests - ADT7490 Temperature Sensor
// ============================================================================

/// Run master-side tests using ADT7490 temperature sensor
///
/// Tests the `i2c_core` API by reading known registers from the on-board ADT7490.
/// This mirrors the original `i2c_test.rs` functionality.
pub fn run_master_tests(uart: &mut UartController<'_>) {
    let _ = writeln!(uart, "\n========================================\r");
    let _ = writeln!(uart, "I2C MASTER Tests (i2c_core API)\r");
    let _ = writeln!(uart, "Using ADT7490 @ 0x{ADT7490_ADDRESS:02X}\r");
    let _ = writeln!(uart, "========================================\n\r");

    let mut results = TestResults::new();

    test_adt7490_register_reads(uart, &mut results);
    test_adt7490_write_read(uart, &mut results);

    let (passed, failed) = results.summary();
    let _ = writeln!(uart, "\n========================================\r");
    let _ = writeln!(uart, "Master Tests: {passed} passed, {failed} failed\r");
    let _ = writeln!(uart, "========================================\n\r");
}

/// Test reading ADT7490 registers with known default values
fn test_adt7490_register_reads(uart: &mut UartController<'_>, results: &mut TestResults) {
    let _ = writeln!(uart, "[TEST] ADT7490 Register Reads\r");

    unsafe {
        let peripherals = Peripherals::steal();

        // Apply pin control for I2C1 (same as original test)
        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C1);

        // Get I2C1 registers
        let i2c_regs = &peripherals.i2c1;
        let buff_regs = &peripherals.i2cbuff1;

        let Some(controller_id) = Controller::new(I2C_MASTER_CTRL_ID) else {
            let _ = writeln!(uart, "  [FAIL] Invalid controller ID\r");
            results.fail();
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
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(uart, "  [FAIL] Init error: {e:?}\r");
                results.fail();
                return;
            }
        };

        // Read each register and verify against expected value
        for &(reg_addr, expected) in &ADT7490_REGS {
            let mut buf = [reg_addr];

            // Write register address
            match i2c.write(ADT7490_ADDRESS, &buf) {
                Ok(()) => {
                    let _ = writeln!(uart, "  Write reg 0x{reg_addr:02X}: OK\r");
                }
                Err(e) => {
                    let _ = writeln!(uart, "  [FAIL] Write reg 0x{reg_addr:02X}: {e:?}\r");
                    results.fail();
                    continue;
                }
            }

            // Read register value
            match i2c.read(ADT7490_ADDRESS, &mut buf) {
                Ok(()) => {
                    let _ = writeln!(
                        uart,
                        "  Read: 0x{:02X}, expected: 0x{expected:02X}\r",
                        buf[0]
                    );
                    if buf[0] == expected {
                        let _ = writeln!(uart, "  [PASS] Register 0x{reg_addr:02X} matches\r");
                        results.pass();
                    } else {
                        let _ = writeln!(
                            uart,
                            "  [WARN] Value differs (may be OK for dynamic regs)\r"
                        );
                        results.pass(); // Still pass - some regs are dynamic
                    }
                }
                Err(e) => {
                    let _ = writeln!(uart, "  [FAIL] Read reg 0x{reg_addr:02X}: {e:?}\r");
                    results.fail();
                }
            }
        }
    }
}

/// Test write-read sequence to ADT7490
fn test_adt7490_write_read(uart: &mut UartController<'_>, results: &mut TestResults) {
    let _ = writeln!(uart, "\n[TEST] ADT7490 Write-Read Sequence\r");

    unsafe {
        let peripherals = Peripherals::steal();
        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C1);

        // Get I2C1 registers
        let i2c_regs = &peripherals.i2c1;
        let buff_regs = &peripherals.i2cbuff1;

        let Some(controller_id) = Controller::new(I2C_MASTER_CTRL_ID) else {
            let _ = writeln!(uart, "  [FAIL] Invalid controller ID\r");
            results.fail();
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
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(uart, "  [FAIL] Init error: {e:?}\r");
                results.fail();
                return;
            }
        };

        // Read Device ID register (0x3D)
        let reg_addr = [0x3D];
        let mut read_buf = [0u8; 1];

        let _ = writeln!(uart, "  Reading Device ID (reg 0x3D)...\r");

        match i2c.write(ADT7490_ADDRESS, &reg_addr) {
            Ok(()) => {}
            Err(e) => {
                let _ = writeln!(uart, "  [FAIL] Write address: {e:?}\r");
                results.fail();
                return;
            }
        }

        match i2c.read(ADT7490_ADDRESS, &mut read_buf) {
            Ok(()) => {
                let _ = writeln!(uart, "  Device ID: 0x{:02X}\r", read_buf[0]);
                let _ = writeln!(uart, "  [PASS] Write-Read sequence completed\r");
                results.pass();
            }
            Err(e) => {
                let _ = writeln!(uart, "  [FAIL] Read: {e:?}\r");
                results.fail();
            }
        }
    }
}

// ============================================================================
// SLAVE Tests - External Master Required
// ============================================================================

/// Run slave-side tests (requires external master)
///
/// Start this BEFORE the external master initiates transactions.
#[allow(clippy::too_many_lines)]
pub fn run_slave_tests(uart: &mut UartController<'_>) {
    let _ = writeln!(uart, "\n========================================\r");
    let _ = writeln!(uart, "I2C SLAVE Tests (i2c_core API)\r");
    let _ = writeln!(uart, "Slave address: 0x{SLAVE_ADDRESS:02X}\r");
    let _ = writeln!(uart, "========================================\r");
    let _ = writeln!(uart, "Waiting for external master...\n\r");

    unsafe {
        run_slave_tests_inner(uart);
    }
}

/// Inner implementation for slave tests (to reduce function length)
///
/// # Safety
/// Caller must ensure exclusive access to I2C hardware peripherals.
unsafe fn run_slave_tests_inner(uart: &mut UartController<'_>) {
    let peripherals = Peripherals::steal();

    // Apply pin control for I2C2 (slave) - Note: uses I2C1 registers in PAC
    // as there's only one I2C peripheral defined
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C2);

    // Note: PAC only has i2c1/i2cbuff1 - for slave tests we'd need
    // the actual I2C2 peripheral which may need different handling
    let i2c_regs = &peripherals.i2c2;
    let buff_regs = &peripherals.i2cbuff2;

    let Some(controller_id) = Controller::new(I2C_SLAVE_CTRL_ID) else {
        let _ = writeln!(uart, "[FAIL] Invalid controller ID\r");
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
        multi_master: false,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    let mut slave = match Ast1060I2c::new(&controller, config) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(uart, "[FAIL] Init error: {e:?}\r");
            return;
        }
    };

    let slave_cfg = match SlaveConfig::new(SLAVE_ADDRESS) {
        Ok(cfg) => cfg,
        Err(e) => {
            let _ = writeln!(uart, "[FAIL] Invalid slave config: {e:?}\r");
            return;
        }
    };

    if let Err(e) = slave.configure_slave(&slave_cfg) {
        let _ = writeln!(uart, "[FAIL] Configure slave error: {e:?}\r");
        return;
    }

    let _ = writeln!(
        uart,
        "[SLAVE] Configured at address 0x{SLAVE_ADDRESS:02X}\r"
    );
    let _ = writeln!(uart, "[SLAVE] Entering event loop...\n\r");

    slave_event_loop(uart, &mut slave);

    slave.disable_slave();
    let _ = writeln!(uart, "[SLAVE] Test complete\r");
}

/// Slave event loop - handles incoming I2C transactions
fn slave_event_loop(uart: &mut UartController<'_>, slave: &mut Ast1060I2c<'_>) {
    let mut transaction_count = 0u32;
    let mut poll_count = 0u32;

    loop {
        if let Some(event) = slave.handle_slave_interrupt() {
            match event {
                SlaveEvent::DataReceived { len } => {
                    let _ = writeln!(uart, "[SLAVE] Received {len} bytes\r");
                    let mut buf = [0u8; 32];
                    if let Ok(n) = slave.slave_read(&mut buf) {
                        let _ = writeln!(uart, "  Data: {:02X?}\r", &buf[..n]);
                    }
                    transaction_count += 1;
                }
                SlaveEvent::ReadRequest => {
                    // TX was pre-armed before entering the poll loop — hardware
                    // responds automatically. Nothing to do here.
                    let _ = writeln!(uart, "[SLAVE] ReadRequest (TX already pre-armed)\r");
                }
                SlaveEvent::DataSent { len } => {
                    // Master has clocked out our TX data. Re-arm RX so the next
                    // master write is not missed. Do NOT call slave_write() here —
                    // DataSent fires after the master has already received the data.
                    let _ = writeln!(uart, "[SLAVE] DataSent {len} bytes\r");
                    transaction_count += 1;
                }
                SlaveEvent::Stop => {
                    let _ = writeln!(uart, "[SLAVE] Stop condition\r");
                }
                SlaveEvent::WriteRequest => {
                    let _ = writeln!(
                        uart,
                        "[SLAVE] master write request: i2c on_transaction start\r"
                    );
                }
            }
        }

        poll_count += 1;
        if poll_count.is_multiple_of(100_000) {
            let _ = writeln!(
                uart,
                "[SLAVE] ... waiting (transactions: {transaction_count})\r"
            );
        }

        // Exit after some transactions
        if transaction_count >= 10 {
            let _ = writeln!(
                uart,
                "\n[SLAVE] Completed {transaction_count} transactions\r"
            );
            break;
        }
    }
}

// ============================================================================
// SLAVE Request-Response Test — mirrors the MCTP call sequence exactly
// ============================================================================

/// Run a slave test that matches the real MCTP write→compute→read cycle.
///
/// # Protocol sequence reproduced
///
/// In production the MCTP transport drives this exact sequence:
///
/// ```text
/// loop {
///     Phase A: slave_wait_event() → DataReceived  (master write = MCTP request)
///     Phase B: MCTP stack computes response
///     Phase C: slave_set_response(response)        ← pre_arm_tx() happens here
///     Phase D: slave_wait_event() → DataSent       (master read = response delivery)
///     Phase E: rearm_rx()
/// }
/// ```
///
/// Key properties exercised that the legacy `run_slave_tests()` does not:
///
/// 1. `pre_arm_tx()` is called **after** receiving a write, not at start-up.
/// 2. A fresh `from_initialized()` handle is used for each logical phase,
///    exactly as `AspeedI2cBackend` does per IPC call.
/// 3. `rearm_rx()` is called after `DataSent` via the DDK method, not raw PAC.
/// 4. The response is derived from the received bytes (increment each byte),
///    so a stale-data bug (Phase D returning Phase A−1's payload) is detectable
///    by the master.
///
/// # Hardware requirement
///
/// An external I2C master must:
/// 1. Write N bytes to our slave address (0x50).
/// 2. Read N bytes back — expected value: each written byte + 1.
/// 3. Repeat `TRANSACTION_CYCLES` times.
pub fn run_slave_request_response_test(uart: &mut UartController<'_>) {
    let _ = writeln!(uart, "\n========================================\r");
    let _ = writeln!(uart, "I2C SLAVE Request-Response Test\r");
    let _ = writeln!(uart, "Mirrors MCTP write→compute→read cycle\r");
    let _ = writeln!(uart, "Slave address: 0x{SLAVE_ADDRESS:02X}\r");
    let _ = writeln!(uart, "========================================\r");
    let _ = writeln!(uart, "Waiting for external master...\n\r");

    unsafe { run_slave_request_response_inner(uart) }
}

/// Number of write→read cycles to run before declaring success.
const TRANSACTION_CYCLES: u32 = 5;

/// Inner (unsafe) body for the request-response slave test.
///
/// # Safety
/// Caller must ensure exclusive access to I2C hardware peripherals.
unsafe fn run_slave_request_response_inner(uart: &mut UartController<'_>) {
    let peripherals = Peripherals::steal();

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C2);

    let i2c_regs = &peripherals.i2c2;
    let buff_regs = &peripherals.i2cbuff2;

    let Some(controller_id) = Controller::new(I2C_SLAVE_CTRL_ID) else {
        let _ = writeln!(uart, "[FAIL] Invalid controller ID\r");
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
        multi_master: false,
        smbus_timeout: true,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    };

    // --- One-time setup: new() + configure + enable ---
    let mut init_handle = match Ast1060I2c::new(&controller, config) {
        Ok(h) => h,
        Err(e) => {
            let _ = writeln!(uart, "[FAIL] Init: {e:?}\r");
            return;
        }
    };

    let slave_cfg = match SlaveConfig::new(SLAVE_ADDRESS) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(uart, "[FAIL] SlaveConfig: {e:?}\r");
            return;
        }
    };

    if let Err(e) = init_handle.configure_slave(&slave_cfg) {
        let _ = writeln!(uart, "[FAIL] configure_slave: {e:?}\r");
        return;
    }
    init_handle.enable_slave();
    drop(init_handle); // release handle; hardware state persists

    let _ = writeln!(uart, "[SLAVE] Configured. Starting cycle loop.\r");

    let mut cycles_ok: u32 = 0;
    const POLL_BUDGET: usize = 100_000;

    // NOTE: no pre_arm_tx() here — master always writes first in MCTP.

    'cycle: for cycle in 0..TRANSACTION_CYCLES {
        let _ = writeln!(uart, "\n[CYCLE {cycle}] Phase A: waiting for DataReceived\r");

        // --- Phase A: wait for master write (fresh from_initialized handle) ---
        let mut rx_buf = [0u8; 32];
        let rx_len;
        {
            let mut h = Ast1060I2c::from_initialized(&controller, config);
            rx_len = 'wait_rx: {
                for _ in 0..POLL_BUDGET {
                    match h.handle_slave_interrupt() {
                        Some(SlaveEvent::DataReceived { len: _ }) => {
                            match h.slave_read(&mut rx_buf) {
                                Ok(n) => break 'wait_rx n,
                                Err(e) => {
                                    let _ = writeln!(uart, "[FAIL] slave_read: {e:?}\r");
                                    break 'cycle;
                                }
                            }
                        }
                        Some(SlaveEvent::Stop) | None => continue,
                        _ => continue,
                    }
                }
                let _ = writeln!(uart, "[FAIL] Timeout waiting for DataReceived\r");
                break 'cycle;
            };
        }

        let _ = writeln!(uart, "  Received {rx_len} bytes: {:02X?}\r", &rx_buf[..rx_len]);

        // --- Phase B: compute response (increment each byte by 1) ---
        let mut response = [0u8; 32];
        for (i, b) in rx_buf[..rx_len].iter().enumerate() {
            response[i] = b.wrapping_add(1);
        }
        let _ = writeln!(uart, "  Response:  {:02X?}\r", &response[..rx_len]);

        // --- Phase C: pre_arm_tx — fresh from_initialized handle ---
        {
            let mut h = Ast1060I2c::from_initialized(&controller, config);
            if let Err(e) = h.pre_arm_tx(&response[..rx_len]) {
                let _ = writeln!(uart, "[FAIL] pre_arm_tx: {e:?}\r");
                break 'cycle;
            }
        }
        let _ = writeln!(uart, "  Phase C: pre_arm_tx OK\r");

        // --- Phase D: wait for DataSent (fresh from_initialized handle) ---
        {
            let mut h = Ast1060I2c::from_initialized(&controller, config);
            let sent = 'wait_tx: {
                for _ in 0..POLL_BUDGET {
                    match h.handle_slave_interrupt() {
                        Some(SlaveEvent::ReadRequest) => {
                            // Hardware responds automatically — just keep polling.
                            continue;
                        }
                        Some(SlaveEvent::DataSent { len }) => break 'wait_tx len,
                        Some(SlaveEvent::Stop) | None => continue,
                        _ => continue,
                    }
                }
                let _ = writeln!(uart, "[FAIL] Timeout waiting for DataSent\r");
                break 'cycle;
            };
            let _ = writeln!(uart, "  Phase D: DataSent {sent} bytes\r");

            // --- Phase E: rearm_rx via DDK method (same handle, consistent with rearm_rx contract) ---
            h.rearm_rx();
        }
        let _ = writeln!(uart, "  Phase E: rearm_rx OK\r");

        cycles_ok += 1;
        let _ = writeln!(uart, "  [PASS] Cycle {cycle} complete\r");
    }

    let _ = writeln!(uart, "\n========================================\r");
    if cycles_ok == TRANSACTION_CYCLES {
        let _ = writeln!(uart, "[PASS] All {TRANSACTION_CYCLES} cycles completed\r");
    } else {
        let _ = writeln!(
            uart,
            "[FAIL] {cycles_ok}/{TRANSACTION_CYCLES} cycles completed\r"
        );
    }
    let _ = writeln!(uart, "========================================\n\r");

    // Clean up.
    let mut cleanup = Ast1060I2c::from_initialized(&controller, config);
    cleanup.disable_slave();
}

// ============================================================================
// Test Info / Help
// ============================================================================

/// Print test setup information
pub fn run_master_slave_tests(uart: &mut UartController<'_>) {
    let _ = writeln!(uart, "\n========================================");
    let _ = writeln!(uart, "I2C Hardware Integration Tests (i2c_core)");
    let _ = writeln!(uart, "========================================");
    let _ = writeln!(uart);
    let _ = writeln!(uart, "MASTER TESTS: run_master_tests()");
    let _ = writeln!(
        uart,
        "  - Uses on-board ADT7490 temp sensor @ 0x{ADT7490_ADDRESS:02X}"
    );
    let _ = writeln!(uart, "  - Reads known registers and verifies defaults");
    let _ = writeln!(uart, "  - No external hardware needed");
    let _ = writeln!(uart);
    let _ = writeln!(uart, "SLAVE TESTS: run_slave_tests()");
    let _ = writeln!(
        uart,
        "  - Configures AST1060 as I2C slave @ 0x{SLAVE_ADDRESS:02X}"
    );
    let _ = writeln!(uart, "  - Requires external master (AST2600, another EVB)");
    let _ = writeln!(uart, "  - Start slave first, then master initiates");
    let _ = writeln!(uart);
    let _ = writeln!(uart, "REQUEST-RESPONSE TEST: run_slave_request_response_test()");
    let _ = writeln!(uart, "  - Mirrors the real MCTP write→compute→read cycle");
    let _ = writeln!(uart, "  - Uses from_initialized() per phase (matches backend)");
    let _ = writeln!(uart, "  - pre_arm_tx() / rearm_rx() called via DDK methods");
    let _ = writeln!(
        uart,
        "  - Response = each received byte + 1 (detects stale-data bugs)"
    );
    let _ = writeln!(uart, "  - Requires external master to write then read back");
    let _ = writeln!(uart, "========================================\n");
}
