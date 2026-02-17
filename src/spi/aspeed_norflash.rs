// Licensed under the Apache-2.0 license

//! `embedded-storage` NorFlash implementation for Aspeed SPI NOR devices.
//!
//! Wraps any [`SpiNorDevice`] into the standard [`NorFlash`] /
//! [`ReadNorFlash`] traits from `embedded-storage 0.3`.
//!
//! # Example
//!
//! ```ignore
//! use aspeed_ddk::spi::aspeed_norflash::AspeedNorFlash;
//!
//! let nor = AspeedNorFlash::new(cs_dev).unwrap();
//! // `nor` now implements `embedded_storage::nor_flash::NorFlash`
//! ```

use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

use super::norflash::{SpiNorDevice, SPI_NOR_PAGE_SIZE, SPI_NOR_SECTOR_SIZE};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from [`AspeedNorFlash`] operations.
#[derive(Debug, Clone, Copy)]
pub enum AspeedFlashError {
    /// Access would exceed device capacity.
    OutOfBounds,
    /// Address or length not aligned to required boundary.
    NotAligned,
    /// Underlying SPI transaction failed.
    DeviceError,
}

impl NorFlashError for AspeedFlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Self::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            Self::NotAligned => NorFlashErrorKind::NotAligned,
            Self::DeviceError => NorFlashErrorKind::Other,
        }
    }
}

// ---------------------------------------------------------------------------
// AspeedNorFlash wrapper
// ---------------------------------------------------------------------------

/// Adapts any [`SpiNorDevice`] to the standard `embedded-storage`
/// [`NorFlash`] trait.
///
/// Constructed via JEDEC probe — call [`AspeedNorFlash::new`] with an
/// initialised [`ChipSelectDevice`](super::device::ChipSelectDevice).
pub struct AspeedNorFlash<T: SpiNorDevice> {
    dev: T,
    capacity: usize,
    supports_4byte: bool,
}

impl<T: SpiNorDevice> AspeedNorFlash<T> {
    /// Probe the attached NOR flash via JEDEC ID and construct the wrapper.
    ///
    /// The third byte of the JEDEC ID encodes log₂(capacity).  Devices
    /// larger than 16 MiB automatically use 4-byte addressing.
    pub fn new(mut dev: T) -> Result<Self, AspeedFlashError> {
        let jedec = dev
            .nor_read_jedec_id()
            .map_err(|_| AspeedFlashError::DeviceError)?;

        let cap_code = jedec[2];
        let capacity = 1usize << cap_code;
        let supports_4byte = capacity > 16 * 1024 * 1024; // > 16 MiB

        Ok(Self {
            dev,
            capacity,
            supports_4byte,
        })
    }

    /// Construct with an explicit capacity (skips JEDEC probe).
    pub fn with_capacity(dev: T, capacity: usize) -> Self {
        let supports_4byte = capacity > 16 * 1024 * 1024;
        Self {
            dev,
            capacity,
            supports_4byte,
        }
    }

    /// Returns a reference to the inner device.
    pub fn inner(&self) -> &T {
        &self.dev
    }

    /// Consumes the wrapper and returns the inner device.
    pub fn into_inner(self) -> T {
        self.dev
    }
}

// ---------------------------------------------------------------------------
// embedded-storage trait impls
// ---------------------------------------------------------------------------

impl<T: SpiNorDevice> ErrorType for AspeedNorFlash<T> {
    type Error = AspeedFlashError;
}

impl<T: SpiNorDevice> ReadNorFlash for AspeedNorFlash<T> {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let end = offset as usize + bytes.len();
        if end > self.capacity {
            return Err(AspeedFlashError::OutOfBounds);
        }
        if self.supports_4byte {
            self.dev
                .nor_read_fast_4b_data(offset, bytes)
                .map_err(|_| AspeedFlashError::DeviceError)
        } else {
            self.dev
                .nor_read_data(offset, bytes)
                .map_err(|_| AspeedFlashError::DeviceError)
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T: SpiNorDevice> NorFlash for AspeedNorFlash<T> {
    const WRITE_SIZE: usize = 1; // Sub-page writes OK
    const ERASE_SIZE: usize = SPI_NOR_SECTOR_SIZE; // 4096

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let end = offset as usize + bytes.len();
        if end > self.capacity {
            return Err(AspeedFlashError::OutOfBounds);
        }

        // Split writes across page boundaries (256-byte pages).
        let mut pos = 0usize;
        while pos < bytes.len() {
            let write_addr = offset as usize + pos;
            let page_remaining = SPI_NOR_PAGE_SIZE - (write_addr % SPI_NOR_PAGE_SIZE);
            let chunk_len = core::cmp::min(page_remaining, bytes.len() - pos);
            let chunk = &bytes[pos..pos + chunk_len];
            let addr32 = write_addr as u32;

            if self.supports_4byte {
                self.dev
                    .nor_page_program_4b(addr32, chunk)
                    .map_err(|_| AspeedFlashError::DeviceError)?;
            } else {
                self.dev
                    .nor_page_program(addr32, chunk)
                    .map_err(|_| AspeedFlashError::DeviceError)?;
            }

            pos += chunk_len;
        }
        Ok(())
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if (from as usize) % SPI_NOR_SECTOR_SIZE != 0 || (to as usize) % SPI_NOR_SECTOR_SIZE != 0 {
            return Err(AspeedFlashError::NotAligned);
        }
        if to as usize > self.capacity {
            return Err(AspeedFlashError::OutOfBounds);
        }

        let mut addr = from;
        while addr < to {
            self.dev
                .nor_sector_erase(addr)
                .map_err(|_| AspeedFlashError::DeviceError)?;
            addr += SPI_NOR_SECTOR_SIZE as u32;
        }
        Ok(())
    }
}
