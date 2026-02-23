// Licensed under the Apache-2.0 license

//! UART Line Status Register abstraction

use bitflags::bitflags;

bitflags! {
    /// Line Status Register (LSR) flags
    ///
    /// The LSR provides status information about the UART transmitter and receiver.
    /// Reading this register clears some error flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LineStatus: u8 {
        /// Error in receiver FIFO
        ///
        /// At least one parity error, framing error, or break indication
        /// exists in the FIFO. Only active when FIFOs are enabled.
        /// Cleared when LSR is read.
        const ERROR_IN_FIFO = 0x80;

        /// Transmitter Empty (TEMT)
        ///
        /// Both the transmitter shift register and FIFO (or THR) are empty.
        /// Set when the last bit of data has been transmitted.
        const TX_EMPTY = 0x40;

        /// Transmitter Holding Register Empty (THRE)
        ///
        /// The THR (or TX FIFO) is empty and ready to accept new data.
        /// This triggers a THRE interrupt if enabled.
        const TX_HOLDING_EMPTY = 0x20;

        /// Break Interrupt (BI)
        ///
        /// The serial input has been held at logic 0 for longer than
        /// a full character time (start + data + parity + stop).
        const BREAK = 0x10;

        /// Framing Error (FE)
        ///
        /// The received character did not have a valid stop bit.
        /// Cleared when LSR is read.
        const FRAMING_ERROR = 0x08;

        /// Parity Error (PE)
        ///
        /// The received character has incorrect parity.
        /// Cleared when LSR is read.
        const PARITY_ERROR = 0x04;

        /// Overrun Error (OE)
        ///
        /// A new character was received while the receive buffer was full.
        /// The new character is lost. Cleared when LSR is read.
        const OVERRUN_ERROR = 0x02;

        /// Data Ready (DR)
        ///
        /// At least one character is available in the receive buffer.
        const DATA_READY = 0x01;
    }
}

impl LineStatus {
    /// Check if any error condition is present
    ///
    /// Returns true if framing, parity, overrun, or FIFO error is detected.
    #[inline]
    #[must_use]
    pub fn has_error(&self) -> bool {
        self.intersects(
            Self::ERROR_IN_FIFO | Self::FRAMING_ERROR | Self::PARITY_ERROR | Self::OVERRUN_ERROR,
        )
    }

    /// Check if transmitter can accept data
    ///
    /// Returns true if the transmit holding register (or TX FIFO) has space.
    #[inline]
    #[must_use]
    pub fn can_transmit(&self) -> bool {
        self.contains(Self::TX_HOLDING_EMPTY)
    }

    /// Check if receiver has data available
    ///
    /// Returns true if at least one byte is in the receive buffer.
    #[inline]
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.contains(Self::DATA_READY)
    }

    /// Check if transmitter is completely idle
    ///
    /// Returns true if both shift register and holding register/FIFO are empty.
    #[inline]
    #[must_use]
    pub fn is_tx_idle(&self) -> bool {
        self.contains(Self::TX_EMPTY)
    }

    /// Check if a break condition was detected
    #[inline]
    #[must_use]
    pub fn is_break(&self) -> bool {
        self.contains(Self::BREAK)
    }

    /// Check for framing error specifically
    #[inline]
    #[must_use]
    pub fn is_framing_error(&self) -> bool {
        self.contains(Self::FRAMING_ERROR)
    }

    /// Check for parity error specifically
    #[inline]
    #[must_use]
    pub fn is_parity_error(&self) -> bool {
        self.contains(Self::PARITY_ERROR)
    }

    /// Check for overrun error specifically
    #[inline]
    #[must_use]
    pub fn is_overrun_error(&self) -> bool {
        self.contains(Self::OVERRUN_ERROR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_detection() {
        let status = LineStatus::FRAMING_ERROR;
        assert!(status.has_error());
        assert!(status.is_framing_error());
        assert!(!status.is_parity_error());
    }

    #[test]
    fn test_tx_ready() {
        let status = LineStatus::TX_HOLDING_EMPTY | LineStatus::TX_EMPTY;
        assert!(status.can_transmit());
        assert!(status.is_tx_idle());
    }

    #[test]
    fn test_rx_ready() {
        let status = LineStatus::DATA_READY;
        assert!(status.has_data());
        assert!(!status.has_error());
    }
}
