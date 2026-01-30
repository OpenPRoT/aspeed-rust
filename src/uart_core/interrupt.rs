// Licensed under the Apache-2.0 license

//! UART interrupt handling

/// Interrupt source identification
///
/// Read from the Interrupt Identification Register (IIR) to determine
/// the highest priority pending interrupt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptSource {
    /// Modem status change (CTS, DSR, RI, DCD)
    ModemStatusChange,
    /// Transmitter holding register empty
    TxEmpty,
    /// Received data available (or trigger level reached in FIFO mode)
    RxDataAvailable,
    /// Receiver line status (error condition)
    LineStatusChange,
    /// Character timeout (no activity with data in FIFO)
    CharacterTimeout,
    /// No interrupt pending
    None,
    /// Unknown interrupt source
    Unknown(u8),
}

impl InterruptSource {
    /// Decode interrupt source from IIR register value
    pub fn from_iir(value: u8) -> Self {
        // Bit 0 = 0 means interrupt pending
        if value & 0x01 != 0 {
            return InterruptSource::None;
        }

        match (value >> 1) & 0x07 {
            0b000 => InterruptSource::ModemStatusChange,
            0b001 => InterruptSource::TxEmpty,
            0b010 => InterruptSource::RxDataAvailable,
            0b011 => InterruptSource::LineStatusChange,
            0b110 => InterruptSource::CharacterTimeout,
            other => InterruptSource::Unknown(other),
        }
    }

    /// Check if this is an error-related interrupt
    pub fn is_error(&self) -> bool {
        matches!(self, InterruptSource::LineStatusChange)
    }

    /// Check if this indicates data is available to read
    pub fn has_rx_data(&self) -> bool {
        matches!(
            self,
            InterruptSource::RxDataAvailable | InterruptSource::CharacterTimeout
        )
    }

    /// Check if the transmitter is ready for more data
    pub fn tx_ready(&self) -> bool {
        matches!(self, InterruptSource::TxEmpty)
    }
}

/// Interrupt enable configuration
///
/// Configure which interrupt sources are enabled.
#[derive(Debug, Clone, Copy)]
pub struct InterruptConfig {
    /// Enable Received Data Available Interrupt (ERBFI)
    pub rx_data_available: bool,
    /// Enable Transmitter Holding Register Empty Interrupt (ETBEI)
    pub tx_empty: bool,
    /// Enable Receiver Line Status Interrupt (ELSI)
    pub line_status: bool,
    /// Enable Modem Status Interrupt (EDSSI)
    pub modem_status: bool,
}

impl Default for InterruptConfig {
    /// Default: all interrupts enabled
    fn default() -> Self {
        Self {
            rx_data_available: true,
            tx_empty: true,
            line_status: true,
            modem_status: true,
        }
    }
}

impl InterruptConfig {
    /// Create configuration with all interrupts disabled
    pub fn none() -> Self {
        Self {
            rx_data_available: false,
            tx_empty: false,
            line_status: false,
            modem_status: false,
        }
    }

    /// Create configuration with only RX interrupt enabled
    pub fn rx_only() -> Self {
        Self {
            rx_data_available: true,
            tx_empty: false,
            line_status: true,
            modem_status: false,
        }
    }

    /// Create configuration with only TX interrupt enabled
    pub fn tx_only() -> Self {
        Self {
            rx_data_available: false,
            tx_empty: true,
            line_status: false,
            modem_status: false,
        }
    }

    /// Enable RX data available interrupt
    pub fn with_rx(mut self) -> Self {
        self.rx_data_available = true;
        self
    }

    /// Enable TX empty interrupt
    pub fn with_tx(mut self) -> Self {
        self.tx_empty = true;
        self
    }

    /// Enable line status interrupt
    pub fn with_line_status(mut self) -> Self {
        self.line_status = true;
        self
    }

    /// Enable modem status interrupt
    pub fn with_modem_status(mut self) -> Self {
        self.modem_status = true;
        self
    }
}
