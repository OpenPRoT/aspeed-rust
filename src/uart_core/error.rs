// Licensed under the Apache-2.0 license

//! UART error types

/// UART error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    /// Framing error - invalid stop bit detected
    Frame,
    /// Parity error - parity check failed
    Parity,
    /// Receiver overrun - data lost due to full buffer
    Overrun,
    /// Break condition detected on the line
    Break,
    /// Buffer full - cannot accept more data
    BufferFull,
    /// Timeout waiting for data or completion
    Timeout,
}

impl embedded_io::Error for UartError {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            UartError::Frame | UartError::Parity => embedded_io::ErrorKind::InvalidData,
            UartError::Overrun | UartError::BufferFull => embedded_io::ErrorKind::OutOfMemory,
            UartError::Break => embedded_io::ErrorKind::Interrupted,
            UartError::Timeout => embedded_io::ErrorKind::TimedOut,
        }
    }
}

impl core::fmt::Display for UartError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UartError::Frame => write!(f, "framing error"),
            UartError::Parity => write!(f, "parity error"),
            UartError::Overrun => write!(f, "receiver overrun"),
            UartError::Break => write!(f, "break condition"),
            UartError::BufferFull => write!(f, "buffer full"),
            UartError::Timeout => write!(f, "timeout"),
        }
    }
}

/// Result type alias for UART operations
pub type Result<T> = core::result::Result<T, UartError>;
