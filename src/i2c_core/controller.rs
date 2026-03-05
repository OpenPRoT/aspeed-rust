// Licensed under the Apache-2.0 license

//! AST1060 I2C controller implementation

use super::timing::configure_timing;
use super::types::{I2cConfig, I2cController, I2cXferMode};
use super::{constants, error::I2cError};
use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};

/// Main I2C hardware abstraction
#[allow(clippy::struct_excessive_bools)]
pub struct Ast1060I2c<'a> {
    controller: &'a I2cController<'a>,
    /// Transfer mode (visible to other modules for optimization decisions)
    pub(crate) xfer_mode: I2cXferMode,
    multi_master: bool,
    smbus_alert: bool,
    #[allow(dead_code)]
    bus_recover: bool,

    // Transfer state (visible to transfer/master modules)
    /// Current device address being communicated with
    pub(crate) current_addr: u8,
    /// Current transfer length
    pub(crate) current_len: u32,
    /// Bytes transferred so far
    pub(crate) current_xfer_cnt: u32,
    /// Completion flag for synchronous operations
    pub(crate) completion: bool,
}

impl<'a> Ast1060I2c<'a> {
    /// Create a new I2C instance and initialize hardware
    ///
    /// This performs full hardware initialization including:
    /// - I2C global register setup (one-time)
    /// - Controller reset
    /// - Multi-master configuration
    /// - Timing configuration
    /// - Interrupt enable
    ///
    /// Use [`from_initialized`] if hardware is already configured.
    pub fn new(controller: &'a I2cController<'a>, config: I2cConfig) -> Result<Self, I2cError> {
        let mut i2c = Self::from_initialized(controller, config);
        i2c.init_hardware(&config)?;
        Ok(i2c)
    }

    /// Create I2C instance from pre-initialized hardware (NO hardware init)
    ///
    /// This is a lightweight constructor that wraps register pointers without
    /// writing to hardware. Use when:
    /// - Hardware was already initialized by app `main.rs` before kernel start
    /// - Hardware was initialized by a previous `new()` call
    /// - You want to avoid redundant re-initialization overhead
    ///
    /// # Performance
    ///
    /// This is ~50x faster than `new()` as it performs no register writes.
    ///
    /// # Preconditions
    ///
    /// Caller must ensure hardware is already configured:
    /// - I2C global registers (I2CG0C, I2CG10) are set (call [`init_i2c_global()`] ONCE in your app before use)
    /// - Controller is enabled (I2CC00)
    /// - Timing is configured
    /// - Pin mux is configured
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// // In app main.rs - initialize once before kernel
    /// use aspeed_ddk::i2c_core::init_i2c_global;
    /// init_i2c_global();
    ///
    /// // In driver - create lightweight wrapper per operation
    /// let i2c = Ast1060I2c::from_initialized(&ctrl, config);
    /// i2c.write(addr, data)?;
    /// ```
    #[must_use]
    pub fn from_initialized(controller: &'a I2cController<'a>, config: I2cConfig) -> Self {
        Self {
            controller,
            xfer_mode: config.xfer_mode,
            multi_master: config.multi_master,
            smbus_alert: config.smbus_alert,
            bus_recover: true,
            current_addr: 0,
            current_len: 0,
            current_xfer_cnt: 0,
            completion: false,
        }
    }

    /// Get access to registers (visible to other modules)
    #[inline]
    pub(crate) fn regs(&self) -> &RegisterBlock {
        self.controller.registers
    }

    /// Get access to buffer registers (visible to other modules)
    #[inline]
    pub(crate) fn buff_regs(&self) -> &BuffRegisterBlock {
        self.controller.buff_registers
    }

    /// Initialize hardware
    #[inline(never)]
    pub fn init_hardware(&mut self, config: &I2cConfig) -> Result<(), I2cError> {
        // PRECONDITION: I2C global registers must be initialized by app before use.
        // See: i2c_core::global::init_i2c_global().

        // Reset I2C controller
        unsafe {
            self.regs().i2cc00().write(|w| w.bits(0));
        }

        // Configure multi-master
        if !self.multi_master {
            self.regs()
                .i2cc00()
                .modify(|_, w| w.dis_multimaster_capability_for_master_fn_only().set_bit());
        }

        // Enable master function and bus auto-release
        self.regs().i2cc00().modify(|_, w| {
            w.enbl_bus_autorelease_when_scllow_sdalow_or_slave_mode_inactive_timeout()
                .set_bit()
                .enbl_master_fn()
                .set_bit()
        });

        // Configure timing
        configure_timing(self.regs(), config)?;

        // Clear all interrupts
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(0xffff_ffff));
        }

        // Enable interrupts for packet mode
        self.regs().i2cm10().modify(|_, w| {
            w.enbl_pkt_cmd_done_int()
                .set_bit()
                .enbl_bus_recover_done_int()
                .set_bit()
        });

        if self.smbus_alert {
            self.regs()
                .i2cm10()
                .modify(|_, w| w.enbl_smbus_dev_alert_int().set_bit());
        }

        Ok(())
    }

    /// Enable interrupts
    pub fn enable_interrupts(&mut self, mask: u32) {
        unsafe {
            self.regs().i2cm10().write(|w| w.bits(mask));
        }
    }

    /// Clear interrupts
    pub fn clear_interrupts(&mut self, mask: u32) {
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(mask));
        }
    }

    /// Check if bus is busy
    ///
    /// Checks if any I2C transfer is currently active by examining status register bits.
    #[must_use]
    pub fn is_bus_busy(&self) -> bool {
        let status = self.regs().i2cm14().read().bits();
        // Bus is busy if any transfer command bits are set
        (status
            & (constants::AST_I2CM_TX_CMD
                | constants::AST_I2CM_RX_CMD
                | constants::AST_I2CM_START_CMD))
            != 0
    }

    /// Wait for completion with timeout (visible to master/transfer modules)
    pub(crate) fn wait_completion(&mut self, timeout_us: u32) -> Result<(), I2cError> {
        let mut timeout = timeout_us;
        self.completion = false;

        while timeout > 0 && !self.completion {
            self.handle_interrupt()?;
            timeout = timeout.saturating_sub(1);
        }

        if self.completion {
            Ok(())
        } else {
            Err(I2cError::Timeout)
        }
    }
}
