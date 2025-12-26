// Licensed under the Apache-2.0 license

//! AST1060 I2C controller implementation

use super::timing::configure_timing;
use super::types::{I2cConfig, I2cController, I2cXferMode};
use super::{constants, error::I2cError};
use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};

/// Main I2C hardware abstraction
pub struct Ast1060I2c<'a> {
    controller: &'a I2cController<'a>,
    /// Transfer mode (visible to other modules for optimization decisions)
    pub(crate) xfer_mode: I2cXferMode,
    multi_master: bool,
    smbus_alert: bool,
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
    /// Create a new I2C instance
    pub fn new(controller: &'a I2cController<'a>, config: I2cConfig) -> Result<Self, I2cError> {
        let mut i2c = Self {
            controller,
            xfer_mode: config.xfer_mode,
            multi_master: config.multi_master,
            smbus_alert: config.smbus_alert,
            bus_recover: true,
            current_addr: 0,
            current_len: 0,
            current_xfer_cnt: 0,
            completion: false,
        };

        i2c.init_hardware(&config)?;
        Ok(i2c)
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
        // Initialize I2C global registers (one-time init)
        init_i2c_global()?;

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

        if !self.completion {
            Err(I2cError::Timeout)
        } else {
            Ok(())
        }
    }
}

/// Initialize I2C global registers (one-time init)
fn init_i2c_global() -> Result<(), I2cError> {
    use core::sync::atomic::{AtomicBool, Ordering};
    static I2CGLOBAL_INIT: AtomicBool = AtomicBool::new(false);

    if I2CGLOBAL_INIT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
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

    Ok(())
}
