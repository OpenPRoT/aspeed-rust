// Licensed under the Apache-2.0 license

//! I2C global hardware initialization utility
//!
//! Call this once in your app (e.g., in main.rs) before using any I2C driver instances.
//!
//! # Reference Implementation
//!
//! This follows the initialization sequence from the original working code:
//! - **aspeed-rust/src/i2c/ast1060_i2c.rs** lines 320-360
//!   - Global init guarded by `I2CGLOBAL_INIT` atomic flag
//!   - SCU reset sequence (scu050/scu054)
//!   - I2CG0C and I2CG10 configuration
//!
//! # Register Details
//!
//! ## SCU050 - Module Reset Assert Register
//! - Bit 2: `rst_i2csmbus_ctrl` - Assert I2C/SMBus controller reset
//!
//! ## SCU054 - Module Reset Clear Register  
//! - Write `0x4` (bit 2) to de-assert I2C reset
//!
//! ## I2CG0C - I2C Global Control Register
//! - `clk_divider_mode_sel`: Enable new clock divider mode
//! - `reg_definition_sel`: Select new register definition
//! - `select_the_action_when_slave_pkt_mode_rxbuf_empty`: RX buffer empty action
//!
//! ## I2CG10 - I2C Global Clock Divider Register
//! Value: `0x6222_0803`
//! - Bits [31:24] = 0x62: Base clk4 for auto recovery timeout
//! - Bits [23:16] = 0x22: Base clk3 for Standard-mode (100kHz), tBuf=5.76us
//! - Bits [15:8]  = 0x08: Base clk2 for Fast-mode (400kHz), tBuf=1.6us
//! - Bits [7:0]   = 0x03: Base clk1 for Fast-mode Plus (1MHz), tBuf=0.8us
//!
//! Based on APB clock = 50MHz:
//! ```text
//! div  : scl       : baseclk [APB/((div/2) + 1)] : tBuf [1/bclk * 16]
//! 0x03 : 1MHz      : 20MHz                       : 0.8us  (Fast-mode Plus)
//! 0x08 : 400kHz    : 10MHz                       : 1.6us  (Fast-mode)
//! 0x22 : 99.21kHz  : 2.77MHz                     : 5.76us (Standard-mode)
//! ```

use ast1060_pac;

/// Initialize I2C global registers (one-time init)
///
/// # Reference
///
/// See original implementation in:
/// - `aspeed-rust/src/i2c/ast1060_i2c.rs:320-360` - Global init with I2CGLOBAL_INIT guard
/// - `aspeed-rust/src/i2c/ast1060_i2c.rs:347-360` - Clock divider configuration comments
///
/// # Safety
/// - Must be called only once after reset, before any I2C controller is used.
/// - Not thread-safe; caller must ensure single invocation.
pub fn init_i2c_global() {
    unsafe {
        let scu = &*ast1060_pac::Scu::ptr();
        let i2cg = &*ast1060_pac::I2cglobal::ptr();

        // Reset I2C/SMBus controller (assert reset)
        // Reference: ast1060_i2c.rs:327
        scu.scu050().write(|w| w.rst_i2csmbus_ctrl().set_bit());

        // Small delay for reset to take effect
        // Reference: ast1060_i2c.rs:329 uses delay.delay_ns(1_000_000)
        for _ in 0..1000 {
            core::hint::spin_loop();
        }

        // De-assert reset (SCU054 is reset clear register)
        // rst_i2csmbus_ctrl is bit 2, so we write 0x4 (1 << 2)
        // Reference: ast1060_i2c.rs:330-332
        scu.scu054().write(|w| w.bits(0x4));

        // Small delay
        for _ in 0..1000 {
            core::hint::spin_loop();
        }

        // Configure global settings
        // Reference: ast1060_i2c.rs:336-343
        i2cg.i2cg0c().write(|w| {
            w.clk_divider_mode_sel()
                .set_bit()
                .reg_definition_sel()
                .set_bit()
                .select_the_action_when_slave_pkt_mode_rxbuf_empty()
                .set_bit()
        });

        // Set base clock dividers for different speeds
        // Reference: ast1060_i2c.rs:344-360
        //
        // APB clk: 50MHz
        // I2CG10[31:24] = 0x62: base clk4 for i2c auto recovery timeout counter
        // I2CG10[23:16] = 0x22: base clk3 for Standard-mode (100kHz) min tBuf 4.7us
        // I2CG10[15:8]  = 0x08: base clk2 for Fast-mode (400kHz) min tBuf 1.3us
        // I2CG10[7:0]   = 0x03: base clk1 for Fast-mode Plus (1MHz) min tBuf 0.5us
        i2cg.i2cg10().write(|w| w.bits(0x6222_0803));
    }
}
