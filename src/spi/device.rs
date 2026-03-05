// Licensed under the Apache-2.0 license

use super::SpiBusWithCs;
use super::SpiError;
use crate::spimonitor::{SpiMonitor, SpipfInstance};
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

#[derive(Debug)]
pub struct ChipSelectDevice<'a, B, SPIPF>
where
    B: SpiBusWithCs,
    SPIPF: SpipfInstance,
{
    pub bus: &'a mut B,
    pub cs: usize,
    pub spi_monitor: Option<&'a mut SpiMonitor<SPIPF>>,
}

impl<B, SPIPF> ErrorType for ChipSelectDevice<'_, B, SPIPF>
where
    B: SpiBusWithCs,
    SPIPF: SpipfInstance,
{
    type Error = B::Error;
}

impl<B, SPIPF> SpiDevice for ChipSelectDevice<'_, B, SPIPF>
where
    B: SpiBusWithCs,
    SPIPF: SpipfInstance,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), SpiError> {
        self.bus.select_cs(self.cs)?;
        if let Some(spim) = self.spi_monitor.as_mut() {
            if self.bus.get_master_id() != 0 {
                spim.spim_scu_ctrl_set(0x8, 0x8);
                spim.spim_scu_ctrl_set(0x7, 1 + SPIPF::FILTER_ID as u32);
            }
            super::spim_proprietary_pre_config();
        }

        for op in operations {
            match op {
                Operation::Read(buf) => self.bus.read(buf)?,
                Operation::Write(buf) => self.bus.write(buf)?,
                Operation::Transfer(read, write) => self.bus.transfer(read, write)?,
                Operation::TransferInPlace(buf) => self.bus.transfer_in_place(buf)?,
                Operation::DelayNs(_) => todo!(),
            }
        }

        super::spim_proprietary_post_config();
        if let Some(spim) = self.spi_monitor.as_mut() {
            if self.bus.get_master_id() != 0 {
                spim.spim_scu_ctrl_clear(0xf);
            }
        }
        self.bus.deselect_cs(self.cs)?;
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
