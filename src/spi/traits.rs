// Licensed under the Apache-2.0 license

use embedded_hal::spi::{ErrorType, SpiBus};

use crate::spi::norflash::SpiNorCommand;

use super::SpiError;

pub trait SpiBusWithCs: SpiBus<u8, Error = SpiError> + ErrorType<Error = SpiError> {
    fn select_cs(&mut self, cs: usize) -> Result<(), SpiError>;
    fn nor_transfer(&mut self, op_info: &mut SpiNorCommand) -> Result<(), SpiError>;
    fn nor_read_init(&mut self, cs: usize, op_info: &SpiNorCommand);
    fn nor_write_init(&mut self, cs: usize, op_info: &SpiNorCommand);

    fn get_device_info(&mut self, cs: usize) -> (u32, u32);
    fn get_master_id(&mut self) -> u32;
}
