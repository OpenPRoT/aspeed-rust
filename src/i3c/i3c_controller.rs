// Licensed under the Apache-2.0 license

use crate::common::Logger;
use crate::i3c::ast1060_i3c::register_i3c_irq_handler;
use crate::i3c::ast1060_i3c::HardwareInterface;
use crate::i3c::i3c_config::{DevKind, DeviceEntry, I3cConfig};

const I3C_BROADCAST_ADDR: u8 = 0x7E;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachError {
    OutOfRange,
    AddrInUse,
}

pub struct I3cController<H: HardwareInterface, L: Logger> {
    pub hw: H,
    pub config: I3cConfig,
    pub logger: L,
}

impl<H: HardwareInterface, L: Logger> I3cController<H, L> {
    pub fn init(&mut self) {
        let ctx = core::ptr::from_mut::<Self>(self) as usize;
        let bus = self.hw.bus_num() as usize;
        register_i3c_irq_handler(bus, Self::irq_trampoline, ctx);

        self.hw.enable_irq();
        self.hw.init(&mut self.config);
    }

    fn irq_trampoline(ctx: usize) {
        let ctrl: &mut Self = unsafe { &mut *(ctx as *mut Self) };
        ctrl.hw.i3c_aspeed_isr(&mut ctrl.config);
    }

    pub fn attach_i3c_dev(&mut self, pid: u64, desired_da: u8, slot: u8) -> Result<(), AttachError> {
        if desired_da == 0 || desired_da >= I3C_BROADCAST_ADDR {
            return Err(AttachError::OutOfRange);
        }

        let dev = DeviceEntry {
            kind: DevKind::I3c,
            pid: Some(pid),
            static_addr: 0,
            dyn_addr: desired_da,
            desired_da,
            bcr: 0,
            dcr: 0,
            maxrd: 0,
            maxwr: 0,
            mrl: 0,
            mwl: 0,
            max_ibi: 0,
            ibi_en: false,
            pos: Some(slot),
        };

        let idx = self
            .config
            .attached
            .attach(dev)
            .map_err(|_| AttachError::AddrInUse)?;
        self.config.attached.map_pos(slot, u8::try_from(idx).map_err(|_| AttachError::OutOfRange)?);
        self.config.addrbook.mark_use(desired_da, true);
        let rc = self.hw.attach_i3c_dev(slot.into(), desired_da);
        match rc {
            Ok(()) => Ok(()),
            Err(_) => {
                Err(AttachError::AddrInUse)
            }
        }
    }

    pub fn detach_i3c_dev(&mut self, pos: usize) {
        self.config.attached.detach_by_pos(pos);
        self.hw.detach_i3c_dev(pos);
    }

    pub fn detach_i3c_dev_by_idx(&mut self, dev_idx: usize) {
        let dev = &self.config.attached.devices[dev_idx];

        if dev.dyn_addr != 0 {
            self.config.addrbook.mark_use(dev.dyn_addr, false);
        }

        if let Some(pos) = dev.pos {
            self.hw.detach_i3c_dev(pos.into());
        }

        self.config.attached.detach(dev_idx);
    }
}
