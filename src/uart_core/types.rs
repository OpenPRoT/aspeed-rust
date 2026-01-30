// Licensed under the Apache-2.0 license

//! Common type definitions for UART driver

/// Default UART clock frequency for AST1060 (24 MHz)
pub const DEFAULT_CLOCK_HZ: u32 = 24_000_000;

/// UART FIFO depth (16 bytes for 16550-compatible)
pub const FIFO_DEPTH: usize = 16;

/// Modem Status Register flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModemStatus(u8);

impl ModemStatus {
    /// Create from register value
    pub fn from_bits(value: u8) -> Self {
        Self(value)
    }

    /// Get raw register value
    pub fn bits(&self) -> u8 {
        self.0
    }

    /// Data Carrier Detect (DCD)
    pub fn dcd(&self) -> bool {
        self.0 & 0x80 != 0
    }

    /// Ring Indicator (RI)
    pub fn ri(&self) -> bool {
        self.0 & 0x40 != 0
    }

    /// Data Set Ready (DSR)
    pub fn dsr(&self) -> bool {
        self.0 & 0x20 != 0
    }

    /// Clear to Send (CTS)
    pub fn cts(&self) -> bool {
        self.0 & 0x10 != 0
    }

    /// Delta DCD (changed since last read)
    pub fn delta_dcd(&self) -> bool {
        self.0 & 0x08 != 0
    }

    /// Trailing Edge RI
    pub fn trailing_ri(&self) -> bool {
        self.0 & 0x04 != 0
    }

    /// Delta DSR (changed since last read)
    pub fn delta_dsr(&self) -> bool {
        self.0 & 0x02 != 0
    }

    /// Delta CTS (changed since last read)
    pub fn delta_cts(&self) -> bool {
        self.0 & 0x01 != 0
    }
}

/// Statistics for UART operations
#[derive(Debug, Clone, Copy, Default)]
pub struct UartStats {
    /// Total bytes transmitted
    pub tx_bytes: u32,
    /// Total bytes received
    pub rx_bytes: u32,
    /// Framing errors encountered
    pub framing_errors: u32,
    /// Parity errors encountered
    pub parity_errors: u32,
    /// Overrun errors encountered
    pub overrun_errors: u32,
    /// Break conditions detected
    pub break_conditions: u32,
}

impl UartStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Total error count
    pub fn total_errors(&self) -> u32 {
        self.framing_errors + self.parity_errors + self.overrun_errors
    }
}
