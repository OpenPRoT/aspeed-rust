// Licensed under the Apache-2.0 license

// Enforce Copilot coding guidelines - prevent panic-prone patterns in production code only
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::indexing_slicing))]
#![cfg_attr(not(test), warn(clippy::expect_used))]
#![cfg_attr(not(test), no_std)]
pub mod astdebug;
pub mod common;
pub mod ecdsa;
pub mod gpio;
pub mod hace_controller;
pub mod hash;
pub mod hash_owned;
pub mod hmac;
pub mod i2c;
pub mod i2c_core;
pub mod i3c;
pub mod pinctrl;
pub mod rsa;
pub mod spi;
pub mod spimonitor;
pub mod syscon;
pub mod tests;
pub mod timer;
pub mod uart;
pub mod uart_core;
pub mod watchdog;
