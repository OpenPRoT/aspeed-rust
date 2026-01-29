// Licensed under the Apache-2.0 license

//! AST1060 I2C Slave/Target Mode Implementation
//!
//! This module provides slave (target) mode functionality for the AST1060 I2C controllers.
//! In slave mode, the controller responds to requests from an external I2C master.

use super::{constants, controller::Ast1060I2c, error::I2cError};

/// Hardware buffer size (32 bytes / 8 DWORDs)
const BUFFER_SIZE: usize = 32;

/// Maximum slave receive buffer size (hardware limitation)
pub const SLAVE_BUFFER_SIZE: usize = 256;

/// Slave mode configuration
#[derive(Debug, Clone, Copy)]
pub struct SlaveConfig {
    /// Primary slave address (7-bit)
    pub address: u8,
    /// Enable packet mode for slave
    pub packet_mode: bool,
    /// Use buffer mode (32 bytes) vs byte mode (1 byte)
    pub buffer_mode: bool,
}

impl SlaveConfig {
    /// Create a new slave configuration
    pub fn new(address: u8) -> Result<Self, I2cError> {
        if address > 0x7F {
            return Err(I2cError::InvalidAddress);
        }

        Ok(Self {
            address,
            packet_mode: true, // Recommended for performance
            buffer_mode: true, // Recommended for performance
        })
    }
}

/// Slave mode events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaveEvent {
    /// Master is requesting to read from us (we need to send data)
    ReadRequest,
    /// Master is writing to us (we're receiving data)
    WriteRequest,
    /// Data received from master
    DataReceived { len: usize },
    /// Data sent to master
    DataSent { len: usize },
    /// Stop condition received
    Stop,
    /// Address matched
    AddressMatch,
}

/// Slave mode data buffer for application-level buffering
pub struct SlaveBuffer {
    data: [u8; SLAVE_BUFFER_SIZE],
    len: usize,
}

impl Default for SlaveBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SlaveBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: [0u8; SLAVE_BUFFER_SIZE],
            len: 0,
        }
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
    }

    pub fn set_len(&mut self, len: usize) {
        self.len = len.min(SLAVE_BUFFER_SIZE);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let to_copy = data.len().min(SLAVE_BUFFER_SIZE);
        self.data[..to_copy].copy_from_slice(&data[..to_copy]);
        self.len = to_copy;
        to_copy
    }
}

impl<'a> Ast1060I2c<'a> {
    /// Configure the controller for slave mode
    pub fn configure_slave(&mut self, config: &SlaveConfig) -> Result<(), I2cError> {
        // Ensure master mode is disabled first
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_master_fn().clear_bit());

        // Set slave address
        self.regs()
            .i2cs40()
            .write(|w| unsafe { w.bits(u32::from(config.address & 0x7F)) });

        // Clear slave interrupts
        self.clear_slave_interrupts();

        // Configure slave mode
        let mut cmd = 0u32;

        if config.packet_mode {
            cmd |= constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_ACTIVE_ALL;
        }

        if config.buffer_mode {
            cmd |= constants::AST_I2CS_RX_BUFF_EN;
        }

        // Enable slave mode
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_slave_fn().set_bit());

        // Set slave command register
        unsafe {
            self.regs().i2cs28().write(|w| w.bits(cmd));
        }

        // Enable slave interrupts
        self.enable_slave_interrupts();

        Ok(())
    }

    /// Enable slave mode interrupts
    fn enable_slave_interrupts(&mut self) {
        let mask = constants::AST_I2CS_PKT_DONE
            | constants::AST_I2CS_PKT_ERROR
            | constants::AST_I2CS_RX_DONE
            | constants::AST_I2CS_SLAVE_MATCH
            | constants::AST_I2CS_STOP;

        unsafe {
            self.regs().i2cs20().write(|w| w.bits(mask));
        }
    }

    /// Clear slave mode interrupts
    fn clear_slave_interrupts(&mut self) {
        unsafe {
            self.regs().i2cs24().write(|w| w.bits(0xFFFF_FFFF));
        }
    }

    /// Disable slave mode
    pub fn disable_slave(&mut self) {
        // Disable interrupts
        unsafe {
            self.regs().i2cs20().write(|w| w.bits(0));
        }

        // Clear interrupts
        self.clear_slave_interrupts();

        // Disable slave mode
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_slave_fn().clear_bit());
    }

    /// Check if slave has received data
    #[must_use]
    pub fn slave_has_data(&self) -> bool {
        let status = self.regs().i2cs24().read().bits();
        (status & constants::AST_I2CS_RX_DONE) != 0
    }

    /// Read data received in slave mode
    pub fn slave_read(&mut self, buffer: &mut [u8]) -> Result<usize, I2cError> {
        let status = self.regs().i2cs24().read().bits();

        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            return Err(I2cError::SlaveError);
        }

        if (status & constants::AST_I2CS_RX_DONE) == 0 {
            return Ok(0);
        }

        // Get receive length from DMA/buffer length register
        let len = self.regs().i2cs2c().read().bits() as usize;
        let to_read = len.min(buffer.len()).min(BUFFER_SIZE);

        // Read from buffer
        self.copy_from_buffer(&mut buffer[..to_read])?;

        // Clear RX_DONE interrupt
        unsafe {
            self.regs()
                .i2cs24()
                .write(|w| w.bits(constants::AST_I2CS_RX_DONE));
        }

        Ok(to_read)
    }

    /// Write data to send in slave mode (in response to read request)
    pub fn slave_write(&mut self, data: &[u8]) -> Result<usize, I2cError> {
        if data.is_empty() {
            return Ok(0);
        }

        let to_write = data.len().min(BUFFER_SIZE);

        // Copy data to buffer
        self.copy_to_buffer(&data[..to_write])?;

        // Set transfer length
        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            self.regs().i2cs2c().write(|w| w.bits(to_write as u32));
        }

        // Trigger slave transmit
        let mut cmd = self.regs().i2cs28().read().bits();
        cmd |= constants::AST_I2CS_TX_BUFF_EN;
        cmd |= constants::AST_I2CS_TX_CMD;

        unsafe {
            self.regs().i2cs28().write(|w| w.bits(cmd));
        }

        Ok(to_write)
    }

    /// Handle slave mode interrupt
    pub fn handle_slave_interrupt(&mut self) -> Option<SlaveEvent> {
        let status = self.regs().i2cs24().read().bits();

        if status == 0 {
            return None;
        }

        // Check for errors first
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            return None;
        }

        // Address match - master has addressed us
        if (status & constants::AST_I2CS_SLAVE_MATCH) != 0 {
            unsafe {
                self.regs()
                    .i2cs24()
                    .write(|w| w.bits(constants::AST_I2CS_SLAVE_MATCH));
            }
            return Some(SlaveEvent::AddressMatch);
        }

        // Check for read request (master wants to read from us)
        if (status & constants::AST_I2CS_WAIT_TX_DMA) != 0 {
            return Some(SlaveEvent::ReadRequest);
        }

        // Check for write complete (we received data)
        if (status & constants::AST_I2CS_RX_DONE) != 0 {
            let len = self.regs().i2cs2c().read().bits() as usize;
            return Some(SlaveEvent::DataReceived { len });
        }

        // Check for transmit complete
        if (status & constants::AST_I2CS_TX_ACK) != 0 {
            let len = self.regs().i2cs2c().read().bits() as usize;
            unsafe {
                self.regs()
                    .i2cs24()
                    .write(|w| w.bits(constants::AST_I2CS_TX_ACK));
            }
            return Some(SlaveEvent::DataSent { len });
        }

        // Stop condition
        if (status & constants::AST_I2CS_STOP) != 0 {
            unsafe {
                self.regs()
                    .i2cs24()
                    .write(|w| w.bits(constants::AST_I2CS_STOP));
            }
            return Some(SlaveEvent::Stop);
        }

        // Packet done
        if (status & constants::AST_I2CS_PKT_DONE) != 0 {
            unsafe {
                self.regs()
                    .i2cs24()
                    .write(|w| w.bits(constants::AST_I2CS_PKT_DONE));
            }
        }

        None
    }
}
