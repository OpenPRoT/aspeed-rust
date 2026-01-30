// Licensed under the Apache-2.0 license

//! AST1060 UART bare-metal driver core
//!
//! This module provides a portable, hardware-abstraction layer for the AST1060 UART controller.
//! It is designed to be usable in both bare-metal and RTOS environments without requiring
//! OS-specific dependencies.
//!
//! # Features
//!
//! - 16550-compatible UART interface
//! - Configurable baud rates (9600 to 1.5 MBaud)
//! - 8N1, 7E1, and other line configurations
//! - 16-byte TX/RX FIFO with configurable trigger levels
//! - Interrupt support with multiple sources
//! - `embedded_io` trait implementations
//! - `core::fmt::Write` support for `writeln!` macro
//!
//! # Architecture
//!
//! The driver is split into focused modules:
//!
//! - `controller`: Core hardware abstraction and initialization
//! - `config`: Configuration types (baud rate, word length, parity, stop bits)
//! - `error`: Error types and embedded_io::Error implementation
//! - `fifo`: FIFO management and trigger level configuration
//! - `interrupt`: Interrupt handling and status types
//! - `line_status`: LineStatus bitflags and helpers
//! - `types`: Common type definitions
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use aspeed_ddk::uart_core::*;
//! use ast1060_pac;
//! use embedded_io::Write;
//!
//! // Get UART register block
//! let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
//!
//! // Create controller
//! let mut uart = UartController::new(uart_regs);
//!
//! // Initialize with default config (115200 8N1)
//! uart.init(&UartConfig::default()).unwrap();
//!
//! // Write data using embedded_io traits
//! uart.write_all(b"Hello, UART!\r\n").unwrap();
//! uart.flush().unwrap();
//!
//! // Or use writeln! macro
//! use core::fmt::Write as FmtWrite;
//! writeln!(uart, "Value: {}", 42).unwrap();
//! ```
//!
//! # Configuration Example
//!
//! ```rust,no_run
//! use aspeed_ddk::uart_core::*;
//!
//! let config = UartConfig {
//!     baud_rate: BaudRate::Baud9600,
//!     word_length: WordLength::Seven,
//!     parity: Parity::Even,
//!     stop_bits: StopBits::One,
//!     fifo_enabled: true,
//!     rx_trigger_level: FifoTriggerLevel::FourBytes,
//!     clock_hz: 24_000_000,
//! };
//! ```

mod config;
mod controller;
mod error;
mod fifo;
mod hal_impl;
mod interrupt;
mod line_status;
mod types;

// Re-export public API
pub use config::{BaudRate, Parity, StopBits, UartConfig, WordLength};
pub use controller::UartController;
pub use error::{Result, UartError};
pub use fifo::{FifoConfig, FifoTriggerLevel};
pub use interrupt::{InterruptConfig, InterruptSource};
pub use line_status::LineStatus;
pub use types::*;
