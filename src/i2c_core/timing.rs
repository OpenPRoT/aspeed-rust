// Licensed under the Apache-2.0 license

//! Timing configuration for I2C

use super::error::I2cError;
use super::types::{I2cConfig, I2cSpeed};
use ast1060_pac::i2c::RegisterBlock;

/// Configure I2C timing based on speed
pub fn configure_timing(regs: &RegisterBlock, config: &I2cConfig) -> Result<(), I2cError> {
    let base_clk = match config.speed {
        I2cSpeed::Standard => 1, // 100 kHz - use base_clk3
        I2cSpeed::Fast => 2,     // 400 kHz - use base_clk2
        I2cSpeed::FastPlus => 3, // 1 MHz - use base_clk1
    };

    // Set AC timing register based on speed
    unsafe {
        regs.i2cc04().write(|w| w.bits(base_clk));
    }

    // Configure SMBus timeout if enabled
    // This configures the timeout timer to detect hung bus conditions
    // according to SMBus specification (25-35ms timeout)
    //
    // Timeout calculation: (timeout_timer / (HPLL_FREQ / 2^(timeout_base_clk_divisor + 1)))
    // With base_clk_divisor=2, timer=8: 8 / (1GHz / 8) = 64ns * 8 = ~512ns per count
    // Actual timeout depends on base clock configuration
    if config.smbus_timeout {
        unsafe {
            regs.i2cc04().write(|w| {
                w.timeout_base_clk_divisor_tout_base_clk()
                    .bits(2) // Divisor for timeout clock
                    .timeout_timer()
                    .bits(8) // Timeout counter value
            });
        }
    }

    Ok(())
}
