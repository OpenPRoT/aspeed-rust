// Licensed under the Apache-2.0 license

use embedded_hal::spi;

#[derive(Debug)]
pub enum SpiError {
    BusError,
    DmaTimeout,
    CsSelectFailed(usize),
    LengthMismatch,
    CapacityOutOfRange,
    UnsupportedDevice(u8),
    AddressNotAligned(u32),
    InvalidCommand(u8),
    Other(&'static str),
}

/// Required by embedded-hal 1.0
impl spi::Error for SpiError {
    fn kind(&self) -> spi::ErrorKind {
        match self {
            SpiError::BusError
            | SpiError::DmaTimeout
            | SpiError::CsSelectFailed(_)
            | SpiError::LengthMismatch
            | SpiError::CapacityOutOfRange
            | SpiError::UnsupportedDevice(_)
            | SpiError::InvalidCommand(_)
            | SpiError::AddressNotAligned(_)
            | SpiError::Other(_) => spi::ErrorKind::Other,
        }
    }
}
