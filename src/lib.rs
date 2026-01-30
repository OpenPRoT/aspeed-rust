// Licensed under the Apache-2.0 license

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
pub mod pinctrl;
pub mod rsa;
pub mod spi;
pub mod spimonitor;
pub mod syscon;
pub mod tests;
pub mod timer;
pub mod uart_core;
pub mod watchdog;
