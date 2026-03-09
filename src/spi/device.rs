// Licensed under the Apache-2.0 license

use crate::spimonitor::SpiMonitorNum;

use super::SpiBusWithCs;
use super::SpiError;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

#[derive(Debug)]
pub struct ChipSelectDevice<'a, B>
where
    B: SpiBusWithCs,
{
    pub bus: &'a mut B,
    pub cs: usize,
    pub spim: Option<SpiMonitorNum>,
}

impl<B> ErrorType for ChipSelectDevice<'_, B>
where
    B: SpiBusWithCs,
{
    type Error = B::Error;
}

impl From<SpiMonitorNum> for u32 {
    #[inline]
    fn from(v: SpiMonitorNum) -> u32 {
        v as u32
    }
}

impl<B> SpiDevice for ChipSelectDevice<'_, B>
where
    B: SpiBusWithCs,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), SpiError> {
        self.bus.select_cs(self.cs)?;
        if let Some(spim) = self.spim {
            if self.bus.get_master_id() != 0 {
                super::spim_scu_ctrl_set(0x8, 0x8);
                super::spim_scu_ctrl_set(0x7, 1 + u32::from(spim));
            }
            super::spim_proprietary_pre_config();
        }

        for op in operations {
            match op {
                Operation::Read(buf) => self.bus.read(buf)?,
                Operation::Write(buf) => self.bus.write(buf)?,
                Operation::Transfer(read, write) => self.bus.transfer(read, write)?,
                Operation::TransferInPlace(buf) => self.bus.transfer_in_place(buf)?,
                Operation::DelayNs(_) => {} // Ignore delay, as the SPI controller will handle timing
            }
        }
        if let Some(_spim) = self.spim {
            super::spim_proprietary_post_config();
            if self.bus.get_master_id() != 0 {
                super::spim_scu_ctrl_clear(0xf);
            }
        }
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(), SpiError> {
        self.transaction(&mut [Operation::Read(buf)])
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), SpiError> {
        self.transaction(&mut [Operation::Write(buf)])
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), SpiError> {
        self.transaction(&mut [Operation::Transfer(read, write)])
    }

    fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), SpiError> {
        self.transaction(&mut [Operation::TransferInPlace(buf)])
    }
}
