// Licensed under the Apache-2.0 license

//! AST1060 I2C bare-metal driver core
//!
//! This module provides a portable, hardware-abstraction layer for the AST1060 I2C controller.
//! It is designed to be usable in both bare-metal and RTOS environments without requiring
//! OS-specific dependencies.
//!
//! # Features
//!
//! - Multi-master support
//! - Master and slave (target) mode
//! - Buffer mode with 32-byte hardware FIFO
//! - Byte-by-byte mode for simple transfers
//! - Clock stretching and bus recovery
//! - `SMBus` alert support
//! - Configurable speeds: Standard (100kHz), Fast (400kHz), Fast-plus (1MHz)
//!
//! # Architecture
//!
//! The driver is split into focused modules:
//!
//! - `controller`: Core hardware abstraction and initialization
//! - `master`: Master mode operations (read/write)
//! - `slave`: Slave (target) mode operations
//! - `transfer`: Low-level transfer state machine
//! - `timing`: Clock timing configuration
//! - `recovery`: Bus recovery mechanisms
//! - `types`: Core type definitions
//! - `error`: Error types
//! - `constants`: Hardware register constants
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use aspeed_ddk::i2c_core::*;
//! use ast1060_pac;
//!
//! // Initialize I2C global registers ONCE before any controller use
//! init_i2c_global();
//!
//! // Create controller reference
//! let i2c_regs = unsafe { &*ast1060_pac::I2c1::ptr() };
//! let buff_regs = unsafe { &*ast1060_pac::I2cbuff1::ptr() };
//! let controller = I2cController::new(i2c_regs, buff_regs);
//!
//! // Initialize with config
//! let config = I2cConfig {
//!     speed: I2cSpeed::Fast,
//!     xfer_mode: I2cXferMode::BuffMode,
//!     multi_master: true,
//!     smbus_alert: false,
//! };
//!
//! let mut i2c = Ast1060I2c::new(&controller, config)?;
//!
//! // Perform master read
//! let mut data = [0u8; 4];
//! i2c.master_read(0x50, &mut data)?;
//! ```

mod constants;
mod controller;
mod error;
mod global;
mod hal_impl;
mod master;
mod recovery;
mod slave;
mod timing;
mod transfer;
mod types;

// Re-export public API
pub use constants::*;
pub use controller::Ast1060I2c;
pub use error::I2cError;
pub use global::init_i2c_global;
pub use slave::{SlaveBuffer, SlaveConfig, SlaveEvent};
pub use types::*;

// Re-export HAL implementations for external use
#[allow(unused_imports)]
pub use hal_impl::*;
