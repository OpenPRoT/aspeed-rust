// Licensed under the Apache-2.0 license

use crate::spi::norflash;
use crate::{
    common::DummyDelay,
    spi::{norflash::SpiNorDevice, SpiError},
};
use core::fmt::Debug;
use embedded_hal::delay::DelayNs;
use proposed_traits::block_device as BD;
use proposed_traits::block_device::{BlockAddress, BlockDevice, BlockRange, ErrorType};

pub struct NorFlashBlockDevice<T: SpiNorDevice> {
    device: T,
    capacity: usize,
    page_size: usize,   // Size of a programmable page (typically 256 bytes)
    sector_size: usize, // Size of an erasable sector (typically 4KB)
    supports_4byte_addr: bool,
}

#[derive(Debug)]
pub enum BlockError {
    ReadError,
    ProgramError,
    EraseError,
    OutOfBounds,
}

/// Required by embedded-hal 1.0
impl BD::Error for BlockError {
    fn kind(&self) -> BD::ErrorKind {
        match self {
            BlockError::ReadError => BD::ErrorKind::ReadError,
            BlockError::ProgramError => BD::ErrorKind::ProgramError,
            BlockError::EraseError => BD::ErrorKind::EraseError,
            BlockError::OutOfBounds => BD::ErrorKind::OutOfBounds,
        }
    }
}

impl<T> ErrorType for NorFlashBlockDevice<T>
where
    T: SpiNorDevice,
{
    type Error = BlockError;
}

impl<T: SpiNorDevice> NorFlashBlockDevice<T> {
    pub fn from_jedec_id(device: T, jedec_id: [u8; 3]) -> Result<Self, SpiError> {
        let capacity_code = jedec_id[2];
        if !(0x10..=0x28).contains(&capacity_code) {
            return Err(SpiError::CapacityOutOfRange);
        }

        let capacity = 1usize << capacity_code;
        let (page_size, sector_size) = match jedec_id[0] {
            norflash::SPI_NOR_MFR_ID_WINBOND | norflash::SPI_NOR_MFR_ID_MXIC => {
                (norflash::SPI_NOR_PAGE_SIZE, norflash::SPI_NOR_SECTOR_SIZE)
            }
            _ => return Err(SpiError::UnsupportedDevice(jedec_id[0])),
        };

        Ok(Self {
            device,
            capacity,
            page_size,
            sector_size,
            supports_4byte_addr: capacity > 16 * 1024 * 1024,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockAddrUsize(pub usize);

impl BlockAddress for BlockAddrUsize {}

impl<T> BlockDevice for NorFlashBlockDevice<T>
where
    T: SpiNorDevice,
{
    //type Address = FlashAddr;
    type Address = BlockAddrUsize;

    /// Returns the size of a readable block in bytes.
    fn read_size(&self) -> usize {
        1
    }

    /// Reads data starting at the given block address.
    ///
    /// # Parameters
    /// - address: The block address to start reading from.
    /// - data: The buffer to store the read data.
    ///
    /// # Returns
    /// A result indicating success or failure.
    fn read(&mut self, address: Self::Address, data: &mut [u8]) -> Result<(), Self::Error> {
        let addr = address.0;
        let end = addr + data.len();

        if end > self.capacity() {
            return Err(BlockError::OutOfBounds);
        }
        if self.supports_4byte_addr {
            if let Err(_e) = self
                .device
                .nor_read_fast_4b_data(addr.try_into().unwrap(), data)
            {
                return Err(BlockError::ReadError);
            }
        } else if let Err(_e) = self.device.nor_read_data(addr.try_into().unwrap(), data) {
            return Err(BlockError::ReadError);
        }

        Ok(())
    }

    fn erase_size(&self) -> usize {
        self.sector_size
    }

    fn erase(&mut self, range: BlockRange<Self::Address>) -> Result<(), Self::Error> {
        let mut addr = range.start.0;
        let end: usize = addr + self.erase_size() * range.count;

        if end > self.capacity() {
            return Err(BlockError::OutOfBounds);
        }

        for _i in 0..range.count {
            if let Err(_e) = self.device.nor_sector_erase(addr.try_into().unwrap()) {
                return Err(BlockError::EraseError);
            }
            addr += self.erase_size();
        }

        Ok(())
    }

    // Returns the size of a programmable block in bytes.
    fn program_size(&self) -> usize {
        self.page_size
    }

    fn program(&mut self, address: Self::Address, data: &[u8]) -> Result<(), Self::Error> {
        let addr = address.0;
        let program_block = self.program_size();
        let end = addr + data.len();

        // Ensure we don't go out of bounds
        if end > self.capacity() {
            return Err(BlockError::OutOfBounds);
        }

        // Ensure data is aligned to full program_size chunks
        if !data.len().is_multiple_of(program_block) {
            return Err(BlockError::ProgramError); // Or define a new `MisalignedWrite` variant
        }

        let mut offset = 0;
        let mut delay = DummyDelay {};
        while offset < data.len() {
            let chunk = &data[offset..offset + program_block];

            let write_addr = addr + offset;

            let result = if self.supports_4byte_addr {
                self.device
                    .nor_page_program_4b(u32::try_from(write_addr).unwrap(), chunk)
            } else {
                self.device
                    .nor_page_program(u32::try_from(write_addr).unwrap(), chunk)
            };

            if result.is_err() {
                return Err(BlockError::ProgramError);
            }
            offset += program_block;
            delay.delay_ns(2_000_000);
        }

        Ok(())
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}
