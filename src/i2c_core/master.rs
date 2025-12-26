// Licensed under the Apache-2.0 license

//! Master mode operations

use super::{constants, controller::Ast1060I2c, error::I2cError, types::I2cXferMode};

impl<'a> Ast1060I2c<'a> {
    /// Write bytes to an I2C device
    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
        if bytes.is_empty() {
            return Ok(());
        }

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.write_byte_mode(addr, bytes),
            I2cXferMode::BufferMode => self.write_buffer_mode(addr, bytes),
        }
    }

    /// Read bytes from an I2C device
    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        if buffer.is_empty() {
            return Ok(());
        }

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.read_byte_mode(addr, buffer),
            I2cXferMode::BufferMode => self.read_buffer_mode(addr, buffer),
        }
    }

    /// Write then read (combined transaction)
    pub fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        // Write phase
        self.write(addr, bytes)?;

        // Read phase
        self.read(addr, buffer)?;

        Ok(())
    }

    /// Write in byte mode (for small transfers)
    fn write_byte_mode(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
        for (i, &byte) in bytes.iter().enumerate() {
            let is_last = i == bytes.len() - 1;

            // Write data byte
            unsafe {
                self.regs().i2cc0c().write(|w| w.bits(byte as u32));
            }

            // Start transfer
            self.start_transfer(addr, false, 1)?;

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_TX_NAK != 0 {
                return Err(I2cError::NoAcknowledge);
            }

            // Add stop on last byte
            if is_last {
                unsafe {
                    self.regs()
                        .i2cm14()
                        .write(|w| w.bits(constants::AST_I2CM_STOP_CMD));
                }
            }
        }

        Ok(())
    }

    /// Read in byte mode
    fn read_byte_mode(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let buffer_len = buffer.len();
        for (i, byte) in buffer.iter_mut().enumerate() {
            let is_last = i == buffer_len - 1;

            // Start transfer
            self.start_transfer(addr, true, 1)?;

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Read data
            let data = self.regs().i2cc0c().read().bits();
            *byte = (data & 0xFF) as u8;

            // Check status
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_TX_NAK != 0 {
                return Err(I2cError::NoAcknowledge);
            }

            // Add stop on last byte
            if is_last {
                unsafe {
                    self.regs()
                        .i2cm14()
                        .write(|w| w.bits(constants::AST_I2CM_STOP_CMD));
                }
            }
        }

        Ok(())
    }

    /// Write in buffer mode (optimal for 2-32 bytes)
    fn write_buffer_mode(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
        // For transfers larger than buffer size, chunk them
        let mut offset = 0;

        while offset < bytes.len() {
            let chunk_len = core::cmp::min(constants::BUFFER_MODE_SIZE, bytes.len() - offset);
            let chunk = &bytes[offset..offset + chunk_len];

            // Copy data to hardware buffer
            self.copy_to_buffer(chunk)?;

            // Start transfer
            self.start_transfer(addr, false, chunk_len)?;

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            offset += chunk_len;
        }

        Ok(())
    }

    /// Read in buffer mode
    fn read_buffer_mode(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let mut offset = 0;

        while offset < buffer.len() {
            let chunk_len = core::cmp::min(constants::BUFFER_MODE_SIZE, buffer.len() - offset);

            // Start transfer
            self.start_transfer(addr, true, chunk_len)?;

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            // Copy from hardware buffer
            let chunk = &mut buffer[offset..offset + chunk_len];
            self.copy_from_buffer(chunk)?;

            offset += chunk_len;
        }

        Ok(())
    }

    /// Handle interrupt (process completion status)
    pub fn handle_interrupt(&mut self) -> Result<(), I2cError> {
        let status = self.regs().i2cm14().read().bits();

        // Check for packet mode completion
        if status & constants::AST_I2CM_PKT_DONE != 0 {
            self.completion = true;
            self.clear_interrupts(constants::AST_I2CM_PKT_DONE);

            // Check for errors
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                if status & constants::AST_I2CM_ARBIT_LOSS != 0 {
                    return Err(I2cError::ArbitrationLoss);
                }
                if status & constants::AST_I2CM_ABNORMAL != 0 {
                    return Err(I2cError::Abnormal);
                }
                return Err(I2cError::Bus);
            }

            return Ok(());
        }

        // Check for byte mode completion
        if status & (constants::AST_I2CM_TX_ACK | constants::AST_I2CM_RX_DONE) != 0 {
            self.completion = true;
            self.clear_interrupts(status);
            return Ok(());
        }

        // Check for errors
        if status & constants::AST_I2CM_TX_NAK != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::NoAcknowledge);
        }

        if status & constants::AST_I2CM_ABNORMAL != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::Abnormal);
        }

        if status & constants::AST_I2CM_ARBIT_LOSS != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::ArbitrationLoss);
        }

        if status & constants::AST_I2CM_SCL_LOW_TO != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::Timeout);
        }

        Ok(())
    }
}
