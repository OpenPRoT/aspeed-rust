// Licensed under the Apache-2.0 license

//! UART configuration types

use crate::uart_core::fifo::FifoTriggerLevel;

/// Baud rate configuration
///
/// Standard baud rates for AST1060 UART assuming 24MHz clock.
/// Use `Custom(u32)` for non-standard rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaudRate {
    /// 9600 baud
    Baud9600,
    /// 19200 baud
    Baud19200,
    /// 38400 baud
    Baud38400,
    /// 57600 baud
    Baud57600,
    /// 115200 baud
    Baud115200,
    /// 1.5 `MBaud` (1,500,000 baud)
    Baud1500000,
    /// Custom baud rate
    Custom(u32),
}

impl BaudRate {
    /// Get the baud rate value in bits per second
    #[must_use]
    pub fn as_bps(&self) -> u32 {
        match self {
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115_200,
            BaudRate::Baud1500000 => 1_500_000,
            BaudRate::Custom(bps) => *bps,
        }
    }

    /// Calculate divisor for given clock frequency
    ///
    /// Divisor = clock / (13 * 16 * `baud_rate`)
    /// This formula is specific to AST1060.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn calculate_divisor(&self, clock_hz: u32) -> u16 {
        let bps = self.as_bps();
        let divisor = clock_hz / 13 / 16 / bps;
        divisor as u16
    }
}

/// Word length (data bits) configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WordLength {
    /// 5 data bits
    Five = 0b00,
    /// 6 data bits
    Six = 0b01,
    /// 7 data bits
    Seven = 0b10,
    /// 8 data bits (default)
    #[default]
    Eight = 0b11,
}

impl WordLength {
    /// Get the register value for this word length
    #[must_use]
    pub fn as_bits(&self) -> u8 {
        *self as u8
    }
}

/// Parity configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Parity {
    /// No parity (default)
    #[default]
    None,
    /// Even parity
    Even,
    /// Odd parity
    Odd,
}

/// Stop bits configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopBits {
    /// 1 stop bit (default)
    #[default]
    One,
    /// 2 stop bits (1.5 for 5-bit word length)
    Two,
}

/// Complete UART configuration
#[derive(Debug, Clone)]
pub struct UartConfig {
    /// Baud rate
    pub baud_rate: BaudRate,
    /// Word length (data bits)
    pub word_length: WordLength,
    /// Parity setting
    pub parity: Parity,
    /// Stop bits
    pub stop_bits: StopBits,
    /// Enable TX/RX FIFOs
    pub fifo_enabled: bool,
    /// RX FIFO interrupt trigger level
    pub rx_trigger_level: FifoTriggerLevel,
    /// Input clock frequency in Hz (default 24MHz for AST1060)
    pub clock_hz: u32,
}

impl Default for UartConfig {
    /// Default configuration: 115200 8N1 with FIFO enabled
    fn default() -> Self {
        Self {
            baud_rate: BaudRate::Baud115200,
            word_length: WordLength::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            fifo_enabled: true,
            rx_trigger_level: FifoTriggerLevel::EightBytes,
            clock_hz: 24_000_000,
        }
    }
}

impl UartConfig {
    /// Create a new configuration with 115200 8N1
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set baud rate
    #[must_use]
    pub fn baud_rate(mut self, rate: BaudRate) -> Self {
        self.baud_rate = rate;
        self
    }

    /// Set word length
    #[must_use]
    pub fn word_length(mut self, length: WordLength) -> Self {
        self.word_length = length;
        self
    }

    /// Set parity
    #[must_use]
    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    /// Set stop bits
    #[must_use]
    pub fn stop_bits(mut self, bits: StopBits) -> Self {
        self.stop_bits = bits;
        self
    }

    /// Enable or disable FIFO
    #[must_use]
    pub fn fifo_enabled(mut self, enabled: bool) -> Self {
        self.fifo_enabled = enabled;
        self
    }

    /// Set RX FIFO trigger level
    #[must_use]
    pub fn rx_trigger_level(mut self, level: FifoTriggerLevel) -> Self {
        self.rx_trigger_level = level;
        self
    }

    /// Set clock frequency
    #[must_use]
    pub fn clock_hz(mut self, hz: u32) -> Self {
        self.clock_hz = hz;
        self
    }

    /// Create 9600 8N1 configuration
    #[must_use]
    pub fn baud_9600() -> Self {
        Self::default().baud_rate(BaudRate::Baud9600)
    }

    /// Create 19200 8N1 configuration
    #[must_use]
    pub fn baud_19200() -> Self {
        Self::default().baud_rate(BaudRate::Baud19200)
    }

    /// Create 1.5 `MBaud` 8N1 configuration
    #[must_use]
    pub fn baud_1500000() -> Self {
        Self::default().baud_rate(BaudRate::Baud1500000)
    }
}
