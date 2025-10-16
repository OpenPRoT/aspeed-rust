// Licensed under the Apache-2.0 license

use proposed_traits::i3c_master::{Error, ErrorKind, ErrorType, I3c, I3cSpeed};

use embedded_hal::i2c::SevenBitAddress;

use crate::common::Logger;
use crate::i3c::ast1060_i3c::HardwareInterface;
use crate::i3c::ccc;
use crate::i3c::i3c_controller::I3cController;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct I3cMasterError(pub ErrorKind);

impl Error for I3cMasterError {
    #[inline]
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

impl From<ErrorKind> for I3cMasterError {
    #[inline]
    fn from(k: ErrorKind) -> Self {
        I3cMasterError(k)
    }
}

impl<H: HardwareInterface, L: Logger> ErrorType for I3cController<H, L> {
    type Error = I3cMasterError;
}

impl<H: HardwareInterface, L: Logger> I3c for I3cController<H, L> {
    fn assign_dynamic_address(
        &mut self,
        static_address: SevenBitAddress,
    ) -> Result<SevenBitAddress, Self::Error> {
        let slot = self
            .config
            .attached
            .pos_of_addr(static_address)
            .ok_or(I3cMasterError(ErrorKind::DynamicAddressConflict))?;

        let rc = self.hw.do_entdaa(&mut self.config, slot.into());
        match rc {
            Ok(()) => {}
            Err(_) => {
                return Err(I3cMasterError(ErrorKind::DynamicAddressConflict));
            }
        }

        let pid = {
            ccc::ccc_getpid(&mut self.hw, &mut self.config, static_address)
                .map_err(|_| I3cMasterError(ErrorKind::InvalidCcc))?
        };

        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(static_address)
            .ok_or(I3cMasterError(ErrorKind::Other))?;

        let old_pid = {
            self.config
                .attached
                .devices
                .get(dev_idx)
                .ok_or(I3cMasterError(ErrorKind::Other))?
                .pid
        };
        if let Some(op) = old_pid {
            if pid != op {
                return Err(I3cMasterError(ErrorKind::Other));
            }
        }

        let bcr = ccc::ccc_getbcr(&mut self.hw, &mut self.config, static_address)
            .map_err(|_| I3cMasterError(ErrorKind::InvalidCcc))?;

        {
            let dev = self
                .config
                .attached
                .devices
                .get_mut(dev_idx)
                .ok_or(I3cMasterError(ErrorKind::Other))?;

            dev.pid = Some(pid);
            dev.bcr = bcr;
        }

        let dyn_addr: SevenBitAddress = {
            let dev = self
                .config
                .attached
                .devices
                .get(dev_idx)
                .ok_or(I3cMasterError(ErrorKind::Other))?;

            dev.dyn_addr
        };

        let ret = self.hw.ibi_enable(&mut self.config, dyn_addr);
        match ret {
            Ok(()) => {}
            _ => {
                return Err(I3cMasterError(ErrorKind::Other));
            }
        }

        Ok(dyn_addr)
    }

    fn acknowledge_ibi(&mut self, address: SevenBitAddress) -> Result<(), Self::Error> {
        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(address)
            .ok_or(I3cMasterError(ErrorKind::Other))?;

        if self.config.attached.devices[dev_idx].pid.is_none() {
            return Err(I3cMasterError(ErrorKind::Other));
        }

        Ok(())
    }

    fn handle_hot_join(&mut self) -> Result<(), Self::Error> {
        // only need to call assign_dynamic_address after receiving hot-join IBI
        Ok(())
    }

    fn set_bus_speed(&mut self, _speed: I3cSpeed) -> Result<(), Self::Error> {
        // ast1060 i3c c driver doesn't support changing bus speed
        Ok(())
    }

    fn request_mastership(&mut self) -> Result<(), Self::Error> {
        // ast1060 controller doesn't support multi-master
        Ok(())
    }
}
