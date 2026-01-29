// Licensed under the Apache-2.0 license

//! I2C global hardware initialization utility
//!
//! Call this once in your app (e.g., in main.rs) before using any I2C driver instances.

use ast1060_pac;

/// Initialize I2C global registers (one-time init)
///
/// # Safety
/// - Must be called only once after reset, before any I2C controller is used.
/// - Not thread-safe; caller must ensure single invocation.
pub fn init_i2c_global() {
    unsafe {
        let scu = &*ast1060_pac::Scu::ptr();
        let i2cg = &*ast1060_pac::I2cglobal::ptr();

        // Reset I2C/SMBus controller (assert reset)
        scu.scu050().write(|w| w.rst_i2csmbus_ctrl().set_bit());

        // Small delay for reset to take effect
        for _ in 0..1000 {
            core::hint::spin_loop();
        }

        // De-assert reset (SCU054 is reset clear register)
        // rst_i2csmbus_ctrl is bit 2, so we write 0x4 (1 << 2)
        scu.scu054().write(|w| w.bits(0x4));

        // Small delay
        for _ in 0..1000 {
            core::hint::spin_loop();
        }

        // Configure global settings
        i2cg.i2cg0c().write(|w| {
            w.clk_divider_mode_sel()
                .set_bit()
                .reg_definition_sel()
                .set_bit()
                .select_the_action_when_slave_pkt_mode_rxbuf_empty()
                .set_bit()
        });

        // Set base clock dividers for different speeds
        // Based on 50MHz APB clock:
        // - Fast-plus (1MHz): base clk1 = 0x03 → 20MHz
        // - Fast (400kHz): base clk2 = 0x08 → 10MHz
        // - Standard (100kHz): base clk3 = 0x22 → 2.77MHz
        // - Recovery timeout: base clk4 = 0x62
        i2cg.i2cg10().write(|w| w.bits(0x6222_0803));
    }
}
