// Licensed under the Apache-2.0 license

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use embedded_hal::delay::DelayNs;
use heapless::Vec;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I3cConfigError {
    AddrInUse,
    AddrExhausted,
    NoFreeSlot,
    DevNotFound,
    DevAlreadyAttached,
    InvalidParam,
    Other,
}

#[derive(Default)]
pub struct CommonState {
    _phantom: PhantomData<()>,
}

#[derive(Default)]
pub struct CommonCfg {
    _phantom: PhantomData<()>,
}

#[derive(Clone, Copy)]
pub struct ResetSpec {
    pub id: u32,
    pub active_high: bool,
}

pub struct I3cTargetConfig {
    pub flags: u8,
    pub addr: Option<u8>,
    pub mdb: u8,
}

impl I3cTargetConfig {
    #[must_use]
    pub const fn new(flags: u8, addr: Option<u8>, mdb: u8) -> Self {
        Self { flags, addr, mdb }
    }
}

pub struct AddrBook {
    pub in_use: [bool; 128],
    pub reserved: [bool; 128],
}

impl Default for AddrBook {
    fn default() -> Self {
        Self::new()
    }
}

impl AddrBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            in_use: [false; 128],
            reserved: [false; 128],
        }
    }

    #[inline]
    #[must_use]
    pub fn is_free(&self, a: u8) -> bool {
        !self.in_use[a as usize] && !self.reserved[a as usize]
    }

    pub fn reserve_defaults(&mut self) {
        for a in 0usize..=7 {
            self.reserved[a] = true;
        }
        self.reserved[0x7E_usize] = true;
        for i in 0..=7 {
            let alt = 0x7E ^ (1u8 << i);
            if alt <= 0x7E {
                self.reserved[alt as usize] = true;
            }
        }
    }

    pub fn alloc_from(&mut self, start: u8) -> Option<u8> {
        let mut a = start.max(8);
        while a < 0x7F {
            if self.is_free(a) {
                // self.in_use[a as usize] = true;
                return Some(a);
            }
            a += 1;
        }
        None
    }

    #[inline]
    pub fn mark_use(&mut self, a: u8, used: bool) {
        if a != 0 {
            self.in_use[a as usize] = used;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevKind {
    I3c,
    I2c,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceEntry {
    pub kind: DevKind,
    pub pid: Option<u64>,
    pub static_addr: u8,
    pub dyn_addr: u8,
    pub desired_da: u8,
    pub bcr: u8,
    pub dcr: u8,
    pub maxrd: u8,
    pub maxwr: u8,
    pub mrl: u16,
    pub mwl: u16,
    pub max_ibi: u8,
    pub ibi_en: bool,
    pub pos: Option<u8>,
}

pub struct Attached {
    pub devices: Vec<DeviceEntry, 8>,
    pub by_pos: [Option<u8>; 8],
}

impl Default for Attached {
    fn default() -> Self {
        Self::new()
    }
}

impl Attached {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            devices: heapless::Vec::new(),
            by_pos: [None; 8],
        }
    }

    pub fn attach(&mut self, dev: DeviceEntry) -> Result<usize, I3cConfigError> {
        let idx = self.devices.len();
        self.devices.push(dev).map_err(|_| I3cConfigError::NoFreeSlot)?;
        Ok(idx)
    }

    pub fn detach(&mut self, dev_idx: usize) {
        if dev_idx >= self.devices.len() {
            return;
        }

        if let Some(pos) = self.devices[dev_idx].pos {
            if let Some(p) = self.by_pos.get_mut(pos as usize) {
                *p = None;
            }
        }

        self.devices.remove(dev_idx);
        for bp in &mut self.by_pos {
            if let Some(idx) = *bp {
                let idx_usize = idx as usize;
                if idx_usize > dev_idx {
                    *bp = Some(u8::try_from(idx_usize - 1).expect("idx too large"));
                }
            }
        }
    }

    pub fn detach_by_pos(&mut self, pos: usize) {
        if let Some(Some(dev_idx)) = self.by_pos.get(pos) {
            self.detach(*dev_idx as usize);
        }
    }
    #[must_use]
    pub fn pos_of(&self, dev_idx: usize) -> Option<u8> {
        self.by_pos
            .iter()
            .position(|&v| v == Some(u8::try_from(dev_idx).expect("dev_idx too large")))
            .and_then(|p| u8::try_from(p).ok())
    }
    #[must_use]
    pub fn find_dev_idx_by_addr(&self, da: u8) -> Option<usize> {
        self.devices.iter().position(|d| d.dyn_addr == da)
    }
    #[must_use]
    pub fn pos_of_addr(&self, da: u8) -> Option<u8> {
        let dev_idx = self.devices.iter().position(|d| d.dyn_addr == da)?;
        self.pos_of(dev_idx)
    }
    #[must_use]
    pub fn pos_of_pid(&self, pid: u64) -> Option<u8> {
        let dev_idx = self.devices.iter().position(|d| d.pid == Some(pid))?;
        self.pos_of(dev_idx)
    }

    #[inline]
    pub fn map_pos(&mut self, pos: u8, idx: u8) {
        self.by_pos[pos as usize] = Some(idx);
    }

    #[inline]
    pub fn unmap_pos(&mut self, pos: u8) {
        self.by_pos[pos as usize] = None;
    }
}

pub struct Completion {
    done: AtomicBool,
}

impl Default for Completion {
    fn default() -> Self {
        Self::new()
    }
}

impl Completion {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn reset(&self) {
        self.done.store(false, Ordering::Release);
    }

    #[inline]
    pub fn complete(&self) {
        self.done.store(true, Ordering::Release);

        cortex_m::asm::sev();
    }

    #[inline]
    pub fn is_completed(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    pub fn wait_for_us<D: DelayNs>(&self, timeout_us: u32, delay: &mut D) -> bool {
        let mut left = timeout_us;
        while !self.is_completed() {
            if left == 0 {
                return false;
            }
            delay.delay_us(1);
            left -= 1;
        }
        true
    }
}

pub struct I3cConfig {
    // Optional: your own “common” higher-level state
    pub common: CommonState,

    pub target_config: Option<I3cTargetConfig>,
    pub addrbook: AddrBook,
    pub attached: Attached,

    // Concurrency
    pub curr_xfer: AtomicPtr<()>,

    // Timing/phy params (ns)
    pub core_period: u32,
    pub i2c_scl_hz: u32,
    pub i3c_scl_hz: u32,
    pub i3c_pp_scl_hi_period_ns: u32,
    pub i3c_pp_scl_lo_period_ns: u32,
    pub i3c_od_scl_hi_period_ns: u32,
    pub i3c_od_scl_lo_period_ns: u32,
    pub sda_tx_hold_ns: u32,
    pub is_secondary: bool,

    // Tables/indices
    pub maxdevs: u16,
    pub free_pos: u32,
    pub need_da: u32,

    pub addrs: [u8; 8],
    pub dcr: u32,

    // Target-mode data
    pub sir_allowed_by_sw: bool,
    pub target_ibi_done: Completion,
    pub target_data_done: Completion,
}

impl Default for I3cConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl I3cConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            common: CommonState::default(),
            target_config: None,
            addrbook: AddrBook::new(),
            attached: Attached::new(),
            curr_xfer: AtomicPtr::new(core::ptr::null_mut()),
            core_period: 0,
            i2c_scl_hz: 0,
            i3c_scl_hz: 0,
            i3c_pp_scl_hi_period_ns: 0,
            i3c_pp_scl_lo_period_ns: 0,
            i3c_od_scl_hi_period_ns: 0,
            i3c_od_scl_lo_period_ns: 0,
            sda_tx_hold_ns: 0,
            is_secondary: false,
            maxdevs: 8,
            free_pos: 0,
            need_da: 0,
            addrs: [0; 8],
            dcr: 0,
            sir_allowed_by_sw: false,
            target_ibi_done: Completion::new(),
            target_data_done: Completion::new(),
        }
    }

    pub fn init_runtime_fields(&mut self) {
        self.addrbook = AddrBook::new();
        self.addrbook.reserve_defaults();
        self.attached = Attached::new();
    }
    pub fn pick_initial_da(&mut self, static_addr: u8, desired: u8) -> Option<u8> {
        if desired != 0 && self.addrbook.is_free(desired) {
            self.addrbook.mark_use(desired, true);
            return Some(desired);
        }
        if static_addr != 0 && self.addrbook.is_free(static_addr) {
            self.addrbook.mark_use(static_addr, true);
            return Some(static_addr);
        }
        self.addrbook.alloc_from(8)
    }

    pub fn reassign_da(&mut self, from: u8, to: u8) -> Result<(), I3cConfigError> {
        if from == to {
            return Ok(());
        }
        if !self.addrbook.is_free(to) {
            return Err(I3cConfigError::AddrInUse);
        }

        self.addrbook.mark_use(from, false);
        self.addrbook.mark_use(to, true);

        if let Some((i, mut e)) = self
            .attached
            .devices
            .iter()
            .enumerate()
            .find_map(|(i, d)| (d.dyn_addr == from).then_some((i, *d)))
        {
            e.dyn_addr = to;
            self.attached.devices[i] = e;
            Ok(())
        } else {
            Err(I3cConfigError::DevNotFound)
        }
    }
}
