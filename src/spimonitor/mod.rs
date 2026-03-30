// Licensed under the Apache-2.0 license

use core::fmt;
use core::marker::PhantomData;
pub mod hardware;

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum SpiMonitorNum {
    SPIM0 = 0,
    SPIM1 = 1,
    SPIM2 = 2,
    SPIM3 = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpimSpiMaster {
    SPI1 = 0,
    SPI2 = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SpimPassthroughMode {
    SinglePassthrough = 0,
    MultiPassthrough = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpimExtMuxSel {
    SpimExtMuxSel0 = 0,
    SpimExtMuxSel1 = 1,
}
impl SpimExtMuxSel {
    #[must_use]
    pub fn to_bool(self) -> bool {
        self as u8 != 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpimBlockMode {
    SpimDeassertCsEearly = 0,
    SpimBlockExtraClk = 1,
}

//address privilege table control
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AddrPrivRWSel {
    AddrPrivReadSel,
    AddrPrivWriteSel,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AddrPriOp {
    FlagAddrPrivEnable,
    FlagAddrPrivDisable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpiMonitorError {
    CommandNotFound(u8),
    NoAllowCmdSlotAvail(u32),
    InvalidCmdSlotIndex(u32),
    AllowCmdSlotLocked(u32),
    AllowCmdSlotInvalid(u32),
    AddressInvalid(u32),
    LengthInvalid(u32),
    AddrTblRegsLocked(u32),
}

//abstracts register base access for different instances
pub trait SpipfInstance {
    fn ptr() -> *const ast1060_pac::spipf::RegisterBlock;
    const FILTER_ID: SpiMonitorNum;
}

macro_rules! macro_spif {
    ($Spipfx: ident, $x: path) => {
        impl SpipfInstance for ast1060_pac::$Spipfx {
            fn ptr() -> *const ast1060_pac::spipf::RegisterBlock {
                ast1060_pac::$Spipfx::ptr()
            }
            const FILTER_ID: SpiMonitorNum = $x;
        }
    };
}
macro_spif!(Spipf, SpiMonitorNum::SPIM0);
macro_spif!(Spipf1, SpiMonitorNum::SPIM1);
macro_spif!(Spipf2, SpiMonitorNum::SPIM2);
macro_spif!(Spipf3, SpiMonitorNum::SPIM3);

#[derive(Debug, Clone, Copy)]
pub struct RegionInfo {
    pub start: u32,
    pub length: u32,
}

//Allow command table information
pub const SPIM_CMD_TABLE_NUM: usize = 32;
pub const MAX_CMD_INDEX: usize = 31;
pub const BLOCK_REGION_NUM: usize = 32;
//generic type
pub struct SpiMonitor<SPIPF: SpipfInstance> {
    pub spi_monitor: &'static ast1060_pac::spipf::RegisterBlock,
    pub scu: &'static ast1060_pac::scu::RegisterBlock,
    pub extra_clk_en: bool,
    pub force_rel_flash_rst: bool,
    pub ext_mux_sel: SpimExtMuxSel,
    pub allow_cmd_list: [u8; SPIM_CMD_TABLE_NUM],
    pub allow_cmd_num: u8,
    pub read_blocked_regions: [RegionInfo; BLOCK_REGION_NUM],
    pub read_blocked_region_num: u8,
    pub write_blocked_regions: [RegionInfo; BLOCK_REGION_NUM],
    pub write_blocked_region_num: u8,
    _marker: PhantomData<SPIPF>,
}

impl<SPIPF: SpipfInstance> fmt::Debug for SpiMonitor<SPIPF> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SpiMonitor")
    }
}

// public traits for spimonitor
// Keep these traits tiny and object-safe (no generics, no Self returns).
pub trait SpiMonitorInit {
    fn init(&mut self);
    fn sw_reset(&mut self);
    fn ext_mux_config(&mut self, mux_sel: SpimExtMuxSel);
}

pub trait SpiMonitorOps {
    fn enable(&mut self);
    fn diable(&mut self);
    fn passthrough_config(&mut self, passthrough_en: bool, mode: SpimPassthroughMode);
    fn spi_ctrl_detour_enable(&mut self, spi_master: SpimSpiMaster, enable: bool);
    fn block_mode_config(&mut self, block_mode: SpimBlockMode);
    fn lock_common(&mut self);
}

pub trait PrivilegeCtrl {
    fn addr_priv_enable(&mut self, rw: AddrPrivRWSel);
    fn address_privilege_config(&mut self, rw: AddrPrivRWSel, op: AddrPriOp, addr: u32, len: u32);
    fn lock_rw_privilege_table(&mut self, rw: AddrPrivRWSel);
}

pub trait AllowCmdCtrl {
    fn get_cmd_table_val(&mut self, cmd: u8) -> Result<u32, SpiMonitorError>;
    fn set_cmd_table(&mut self, cmd_list: &[u8], cmd_num: u8);
    fn init_allow_cmd_table(&mut self, cmd_list: &[u8], cmd_num: u8, flags: u32);
    fn first_empty_slot(&mut self) -> Result<u32, SpiMonitorError>;
    fn find_allow_cmd_slot(&mut self, cmd: u8, start_offset: u32) -> Result<u32, SpiMonitorError>;
    fn add_allow_cmd(&mut self, cmd: u8, flags: u32) -> Result<u32, SpiMonitorError>;
    fn remove_allow_cmd(&mut self, cmd: u8) -> Result<u32, SpiMonitorError>;
    fn lock_allow_cmd(&mut self, cmd: u8, flags: u32) -> Result<u32, SpiMonitorError>;
}

// calling the functions within the same crate, so use inline
impl<SPIPF: SpipfInstance> SpiMonitorInit for SpiMonitor<SPIPF> {
    #[inline]
    fn init(&mut self) {
        self.aspeed_spi_monitor_init();
    }
    #[inline]
    fn sw_reset(&mut self) {
        self.spim_sw_rst();
    }
    #[inline]
    fn ext_mux_config(&mut self, mux_sel: SpimExtMuxSel) {
        self.spim_ext_mux_config(mux_sel);
    }
}

impl<SPIPF: SpipfInstance> SpiMonitorOps for SpiMonitor<SPIPF> {
    #[inline]
    fn passthrough_config(&mut self, passthrough_en: bool, mode: SpimPassthroughMode) {
        self.spim_passthrough_config(passthrough_en, mode);
    }
    #[inline]
    fn spi_ctrl_detour_enable(&mut self, spi_master: SpimSpiMaster, enable: bool) {
        self.spim_spi_ctrl_detour_enable(spi_master, enable);
    }
    #[inline]
    fn block_mode_config(&mut self, block_mode: SpimBlockMode) {
        self.spim_block_mode_config(block_mode);
    }
    #[inline]
    fn enable(&mut self) {
        self.spim_enable(true);
    }
    #[inline]
    fn diable(&mut self) {
        self.spim_enable(false);
    }
    #[inline]
    fn lock_common(&mut self) {
        self.spim_lock_common();
    }
}
/*
impl<SPIPF: SpipfInstance> AllowCmdCtrl for SpiMonitor<SPIPF> {
    #[inline]
    fn get_cmd_table_val(&mut self, cmd: u8) -> Result<u32, SpiMonitorError> {
        self.spim_get_cmd_table_val(cmd)
    }
    #[inline]
    fn set_cmd_table(&mut self, list: &[u8], num: u8) {
        self.spim_set_cmd_table(list, num)
    }
    #[inline]
    fn init_allow_cmd_table(&mut self, list: &[u8], num: u8, flags: u32) {
        self.spim_allow_cmd_table_init(list, num, flags)
    }
    #[inline]
    fn first_empty_slot(&mut self) -> Result<u32, SpiMonitorError> {
        self.spim_get_empty_allow_cmd_slot()
    }
    #[inline]
    fn find_allow_cmd_slot(&mut self, cmd: u8, start: u32) -> Result<u32, SpiMonitorError> {
        self.spim_get_allow_cmd_slot(cmd, start)
    }
    #[inline]
    fn add_allow_cmd(&mut self, cmd: u8, flags: u32) -> Result<u32, SpiMonitorError> {
        self.spim_add_allow_command(cmd, flags)
    }
    #[inline]
    fn remove_allow_cmd(&mut self, cmd: u8) -> Result<u32, SpiMonitorError> {
        self.spim_remove_allow_command(cmd)
    }
    #[inline]
    fn lock_allow_cmd(&mut self, cmd: u8, flags: u32) -> Result<u32, SpiMonitorError> {
        self.spim_lock_allow_command_table(cmd, flags)
    }
}

impl<SPIPF: SpipfInstance> PrivilegeCtrl for SpiMonitor<SPIPF> {
    #[inline]
    fn addr_priv_enable(&mut self, rw: AddrPrivRWSel) {
        self.spim_addr_priv_access_enable(rw);
    }
    #[inline]
    fn address_privilege_config(&mut self, rw: AddrPrivRWSel, op: AddrPriOp, addr: u32, len: u32) {
        self.spim_address_privilege_config(rw, op, addr, len);
    }
    #[inline]
    fn lock_rw_privilege_table(&mut self, rw: AddrPrivRWSel) {
        self.spim_lock_rw_priv_table(rw);
    }
}
*/
