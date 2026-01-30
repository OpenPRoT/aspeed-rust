// Licensed under the Apache-2.0 license

//! Trait implementations for embedded_io and core::fmt

use embedded_io::{ErrorType, Read, Write};

use crate::uart_core::controller::UartController;
use crate::uart_core::error::UartError;

// ============================================================================
// embedded_io trait implementations
// ============================================================================

impl ErrorType for UartController<'_> {
    type Error = UartError;
}

impl Write for UartController<'_> {
    /// Write bytes to the UART
    ///
    /// This implementation:
    /// - Blocks on the first byte if TX is full (per embedded_io spec)
    /// - Returns early if TX becomes full after writing at least one byte
    /// - Never returns Ok(0) unless the input buffer is empty
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut written = 0;

        for &byte in buf {
            if self.is_tx_full() {
                if written == 0 {
                    // Must block until at least one byte can be written
                    while self.is_tx_full() {}
                } else {
                    // Already wrote some bytes, return what we have
                    return Ok(written);
                }
            }

            self.write_byte_blocking(byte);
            written += 1;
        }

        Ok(written)
    }

    /// Flush the transmit buffer
    ///
    /// Blocks until all buffered data has been transmitted.
    fn flush(&mut self) -> Result<(), Self::Error> {
        while !self.is_tx_idle() {}
        Ok(())
    }
}

impl Read for UartController<'_> {
    /// Read bytes from the UART
    ///
    /// This implementation:
    /// - Blocks until at least one byte is available (per embedded_io spec)
    /// - Returns available bytes up to buffer length
    /// - Checks for receive errors on each byte
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        // Block until at least one byte is available
        while self.is_rx_empty() {}

        let mut count = 0;

        while !self.is_rx_empty() && count < buf.len() {
            match self.read_byte() {
                Ok(byte) => {
                    buf[count] = byte;
                    count += 1;
                }
                Err(e) => {
                    // If we've read some bytes, return them
                    // Otherwise propagate the error
                    if count > 0 {
                        return Ok(count);
                    }
                    return Err(e);
                }
            }
        }

        Ok(count)
    }
}

// ============================================================================
// core::fmt::Write implementation for writeln! macro support
// ============================================================================

impl core::fmt::Write for UartController<'_> {
    /// Write a string to the UART
    ///
    /// This enables use of `write!` and `writeln!` macros:
    ///
    /// ```rust,no_run
    /// use core::fmt::Write;
    /// writeln!(uart, "Hello, {}!", "world").unwrap();
    /// ```
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte_blocking(byte);
        }
        Ok(())
    }
}

// ============================================================================
// nb-based non-blocking implementations (for embedded_hal compatibility)
// ============================================================================

impl UartController<'_> {
    /// Non-blocking write using nb::Result
    ///
    /// Returns `nb::Error::WouldBlock` if TX is full.
    pub fn nb_write(&mut self, byte: u8) -> nb::Result<(), UartError> {
        if self.is_tx_full() {
            Err(nb::Error::WouldBlock)
        } else {
            self.write_byte_blocking(byte);
            Ok(())
        }
    }

    /// Non-blocking read using nb::Result
    ///
    /// Returns `nb::Error::WouldBlock` if RX is empty.
    pub fn nb_read(&mut self) -> nb::Result<u8, UartError> {
        if self.is_rx_empty() {
            Err(nb::Error::WouldBlock)
        } else {
            self.read_byte().map_err(nb::Error::Other)
        }
    }

    /// Non-blocking flush using nb::Result
    ///
    /// Returns `nb::Error::WouldBlock` if TX is not idle.
    pub fn nb_flush(&mut self) -> nb::Result<(), UartError> {
        if self.is_tx_idle() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

#[cfg(test)]
mod tests {
    // Unit tests would go here if we had a mock UART
}
