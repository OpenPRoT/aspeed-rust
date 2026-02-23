// Licensed under the Apache-2.0 license

//! UART FIFO configuration and management

/// Receiver FIFO interrupt trigger level
///
/// Determines how many bytes in the RX FIFO will trigger an interrupt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FifoTriggerLevel {
    /// Trigger when 1 byte is in FIFO
    OneByte = 0b00,
    /// Trigger when 4 bytes are in FIFO
    FourBytes = 0b01,
    /// Trigger when 8 bytes are in FIFO (default)
    #[default]
    EightBytes = 0b10,
    /// Trigger when 14 bytes are in FIFO
    FourteenBytes = 0b11,
}

impl FifoTriggerLevel {
    /// Get the register value for this trigger level
    #[must_use]
    pub fn as_bits(&self) -> u8 {
        *self as u8
    }

    /// Get the number of bytes that will trigger an interrupt
    #[must_use]
    pub fn trigger_count(&self) -> u8 {
        match self {
            FifoTriggerLevel::OneByte => 1,
            FifoTriggerLevel::FourBytes => 4,
            FifoTriggerLevel::EightBytes => 8,
            FifoTriggerLevel::FourteenBytes => 14,
        }
    }
}

/// FIFO configuration
#[derive(Debug, Clone, Copy)]
pub struct FifoConfig {
    /// Enable TX/RX FIFOs
    pub enabled: bool,
    /// RX FIFO interrupt trigger level
    pub rx_trigger_level: FifoTriggerLevel,
    /// Clear TX FIFO on init
    pub clear_tx: bool,
    /// Clear RX FIFO on init
    pub clear_rx: bool,
}

impl Default for FifoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rx_trigger_level: FifoTriggerLevel::EightBytes,
            clear_tx: true,
            clear_rx: true,
        }
    }
}

impl FifoConfig {
    /// Create new FIFO configuration with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable FIFOs (byte-by-byte mode)
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set trigger level
    #[must_use]
    pub fn trigger_level(mut self, level: FifoTriggerLevel) -> Self {
        self.rx_trigger_level = level;
        self
    }
}
