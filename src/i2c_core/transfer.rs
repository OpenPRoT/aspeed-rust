// Licensed under the Apache-2.0 license

//! Transfer mode implementation

use super::{constants, controller::Ast1060I2c, error::I2cError, types::I2cXferMode};

impl<'a> Ast1060I2c<'a> {
    /// Start a transfer (common setup for byte/buffer modes)
    pub(crate) fn start_transfer(
        &mut self,
        addr: u8,
        is_read: bool,
        len: usize,
    ) -> Result<(), I2cError> {
        if len == 0 || len > 255 {
            return Err(I2cError::Invalid);
        }

        self.current_addr = addr;
        self.current_len = len as u32;
        self.current_xfer_cnt = 0;
        self.completion = false;

        // Clear any previous status
        self.clear_interrupts(0xffff_ffff);

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.start_byte_mode(addr, is_read, len),
            I2cXferMode::BufferMode => self.start_buffer_mode(addr, is_read, len),
        }
    }

    /// Start transfer in byte mode
    fn start_byte_mode(&mut self, addr: u8, is_read: bool, len: usize) -> Result<(), I2cError> {
        // For byte mode, we'll use the master command register
        let mut cmd = constants::AST_I2CM_START_CMD;

        if is_read {
            cmd |= constants::AST_I2CM_RX_CMD;
            if len == 1 {
                cmd |= constants::AST_I2CM_RX_CMD_LAST; // NAK after last byte
            }
        } else {
            cmd |= constants::AST_I2CM_TX_CMD;
        }

        // Set address in data register for byte mode
        unsafe {
            self.regs()
                .i2cc0c()
                .write(|w| w.bits((addr as u32) << 1 | is_read as u32));
        }

        // Issue command
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(cmd));
        }

        Ok(())
    }

    /// Start transfer in buffer mode (up to 32 bytes)
    fn start_buffer_mode(&mut self, addr: u8, is_read: bool, len: usize) -> Result<(), I2cError> {
        if len > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        // Use packet mode for buffer transfers
        let mut cmd = constants::AST_I2CM_PKT_EN;

        if is_read {
            cmd |= constants::AST_I2CM_RX_CMD | constants::AST_I2CM_RX_BUFF_EN;
        } else {
            cmd |= constants::AST_I2CM_TX_CMD | constants::AST_I2CM_TX_BUFF_EN;
        }

        // Always add stop
        cmd |= constants::AST_I2CM_STOP_CMD;

        // Build packet command: address in bits [31:24], length in bits [15:8]
        let pkt_cmd = constants::ast_i2cm_pkt_addr(addr) | ((len as u32) << 8);

        // Set packet command
        unsafe {
            self.regs().i2cm18().write(|w| w.bits(pkt_cmd));
        }

        // Issue command
        unsafe {
            self.regs().i2cm14().write(|w| w.bits(cmd));
        }

        Ok(())
    }

    /// Copy data to hardware buffer (for writes)
    pub(crate) fn copy_to_buffer(&mut self, data: &[u8]) -> Result<(), I2cError> {
        if data.len() > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        let buff_regs = self.buff_regs();
        let mut idx = 0;

        while idx < data.len() {
            // Pack bytes into DWORD (little-endian)
            let mut dword: u32 = 0;
            for byte_pos in 0..4 {
                if idx + byte_pos < data.len() {
                    dword |= (data[idx + byte_pos] as u32) << (byte_pos * 8);
                }
            }

            let dword_idx = idx / 4;
            if dword_idx >= 8 {
                return Err(I2cError::Invalid);
            }

            // Write to buffer register array (AST1060 has 8 DWORDs = 32 bytes)
            unsafe {
                buff_regs.buff(dword_idx).write(|w| w.bits(dword));
            }

            idx += 4;
        }

        Ok(())
    }

    /// Copy data from hardware buffer (for reads)
    pub(crate) fn copy_from_buffer(&self, data: &mut [u8]) -> Result<(), I2cError> {
        if data.len() > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        let buff_regs = self.buff_regs();
        let mut idx = 0;

        while idx < data.len() {
            let dword_idx = idx / 4;
            if dword_idx >= 8 {
                return Err(I2cError::Invalid);
            }

            // Read from buffer register array
            let dword = buff_regs.buff(dword_idx).read().bits();

            // Extract bytes from DWORD (little-endian)
            for byte_pos in 0..4 {
                if idx + byte_pos < data.len() {
                    data[idx + byte_pos] = ((dword >> (byte_pos * 8)) & 0xFF) as u8;
                }
            }

            idx += 4;
        }

        Ok(())
    }
}
