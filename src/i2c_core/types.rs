// Licensed under the Apache-2.0 license

//! Core types for AST1060 I2C driver
//!
//! These types are portable and have no OS dependencies.

use ast1060_pac;

/// I2C controller identifier (0-13 for AST1060)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Controller(pub u8);

impl Controller {
    /// Create a new controller instance
    pub const fn new(id: u8) -> Option<Self> {
        if id < 14 {
            Some(Self(id))
        } else {
            None
        }
    }

    /// Get the controller ID
    pub const fn id(&self) -> u8 {
        self.0
    }
}

// Implement conversions for compatibility
impl From<Controller> for u8 {
    fn from(c: Controller) -> u8 {
        c.0
    }
}

impl From<u8> for Controller {
    fn from(id: u8) -> Self {
        Self(id)
    }
}

/// I2C controller configuration
pub struct I2cController<'a> {
    pub controller: Controller,
    pub registers: &'a ast1060_pac::i2c::RegisterBlock,
    pub buff_registers: &'a ast1060_pac::i2cbuff::RegisterBlock,
}

/// I2C configuration
///
/// Default configuration is optimized for MCTP-over-I2C:
/// - Fast mode (400 kHz) - standard MCTP speed
/// - Buffer mode - efficient for MCTP packet transfers
/// - SMBus timeout enabled - required for robust MCTP operation
#[derive(Debug, Clone, Copy)]
pub struct I2cConfig {
    /// Transfer mode (byte-by-byte or buffer)
    pub xfer_mode: I2cXferMode,
    /// Bus speed (Standard/Fast/FastPlus)
    pub speed: I2cSpeed,
    /// Enable multi-master support
    pub multi_master: bool,
    /// Enable SMBus timeout detection (25-35ms per SMBus spec)
    pub smbus_timeout: bool,
    /// Enable SMBus alert interrupt
    pub smbus_alert: bool,
}

impl Default for I2cConfig {
    fn default() -> Self {
        Self {
            xfer_mode: I2cXferMode::BufferMode,
            speed: I2cSpeed::Fast,
            multi_master: false,
            smbus_timeout: true,
            smbus_alert: false,
        }
    }
}

/// Transfer mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cXferMode {
    /// Byte-by-byte mode (1 byte at a time)
    ByteMode,
    /// Buffer mode (up to 32 bytes via hardware buffer)
    BufferMode,
}

/// I2C bus speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cSpeed {
    /// Standard mode: 100 kHz
    Standard,
    /// Fast mode: 400 kHz
    Fast,
    /// Fast-plus mode: 1 MHz
    FastPlus,
}
