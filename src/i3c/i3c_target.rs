// Licensed under the Apache-2.0 license

use core::convert::Infallible;

use embedded_hal::i2c::ErrorType as HalI2cErrorType;
use proposed_traits::i2c_target::I2CCoreTarget;
use proposed_traits::i3c_target::{DynamicAddressable, I3CCoreTarget, IBICapable};

use crate::i3c::ast1060_i3c::{HardwareInterface, I3cIbi, I3cIbiType};
use crate::i3c::i3c_config::I3cTargetConfig;
use crate::i3c::i3c_controller::I3cController;

impl<HW: HardwareInterface, L: crate::common::Logger> HalI2cErrorType for I3cController<HW, L> {
    type Error = Infallible;
}

impl<HW: HardwareInterface, L: crate::common::Logger> I2CCoreTarget for I3cController<HW, L> {
    #[inline]
    fn init(&mut self, own_addr: u8) -> Result<(), Self::Error> {
        if let Some(t) = self.config.target_config.as_mut() {
            if t.addr.is_none() {
                t.addr = Some(own_addr);
            }
        } else {
            self.config.target_config =
                Some(I3cTargetConfig::new(0, Some(own_addr), /*mdb*/ 0xae));
        }
        Ok(())
    }

    #[inline]
    fn on_address_match(&mut self, addr: u8) -> bool {
        self.config
            .target_config
            .as_ref()
            .and_then(|t| t.addr)
            .map_or(false, |da| da == addr)
    }

    #[inline]
    fn on_transaction_start(&mut self, _is_read: bool) {}

    #[inline]
    fn on_stop(&mut self) {}
}

impl<HW: HardwareInterface, L: crate::common::Logger> I3CCoreTarget for I3cController<HW, L> {}

impl<HW: HardwareInterface, L: crate::common::Logger> DynamicAddressable for I3cController<HW, L> {
    fn on_dynamic_address_assigned(&mut self, _new_address: u8) {
        self.config.sir_allowed_by_sw = true;
    }
}

impl<HW: HardwareInterface, L: crate::common::Logger> IBICapable for I3cController<HW, L> {
    fn wants_ibi(&self) -> bool {
        true
    }

    fn get_ibi_payload(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let (da, mdb) = match self.config.target_config.as_ref() {
            Some(t) => (
                match t.addr {
                    Some(da) => da,
                    None => return Ok(0),
                },
                t.mdb,
            ),
            None => return Ok(0),
        };

        let addr_rnw = (da << 1) | 0x1;
        let mut crc = crc8_ccitt(0, &[addr_rnw]);
        crc = crc8_ccitt(crc, &[mdb]);

        let payload = [mdb, crc];
        let mut ibi = I3cIbi {
            ibi_type: I3cIbiType::TargetIntr,
            payload: Some(&payload),
        };
        let rc = self
            .hw
            .target_pending_read_notify(&mut self.config, buffer, &mut ibi);

        match rc {
            Ok(()) => Ok(buffer.len() + payload.len()),
            _ => Ok(0),
        }
    }

    fn on_ibi_acknowledged(&mut self) {}
}

#[inline]
fn crc8_ccitt(mut crc: u8, data: &[u8]) -> u8 {
    for &b in data {
        let mut x = crc ^ b;
        for _ in 0..8 {
            x = if (x & 0x80) != 0 {
                (x << 1) ^ 0x07
            } else {
                x << 1
            };
        }
        crc = x;
    }
    crc
}
