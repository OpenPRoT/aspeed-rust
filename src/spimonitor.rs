// Licensed under the Apache-2.0 license

use ast1060_pac::Scu;
use core::cmp::min;
use core::fmt;
use core::marker::PhantomData;
//use core::ops::bit;
//use embedded_hal::delay::DelayNs;

#[derive(Debug)]
#[repr(u8)]
pub enum SpiMonitorNum {
    SPIM0 = 0,
    SPIM1 = 1,
    SPIM2 = 2,
    SPIM3 = 3,
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

//Address table selection majic value
pub const SEL_READ_TBL_MAJIC: u32 = 0x52 << 24;
pub const SEL_WRITE_TBL_MAJIC: u32 = 0x57 << 24;
pub const ACCESS_BLOCK_UNIT: u32 = 16 * 1024; //16KB
pub const ACCESS_BLOCK_PER_REG: u32 = 32 * ACCESS_BLOCK_UNIT;
//SPIPFWA size 0x800*8*16KB:256MB
pub const MAX_PRIV_REGION_SIZE: u32 = 256 * 1024 * 1024;
pub const ADDR_LIMIT: u32 = 256 * 1024 * 1024;

/// allow command table control flag
pub const FLAG_CMD_TABLE_VALID: u32 = 0x0000_0000;
pub const FLAG_CMD_TABLE_VALID_ONCE: u32 = 0x0000_0001;
pub const FLAG_CMD_TABLE_LOCK_ALL: u32 = 0x0000_0002;

/// general command 13
pub const CMD_RDID: u8 = 0x9F;
pub const CMD_WREN: u8 = 0x06;
pub const CMD_WRDIS: u8 = 0x04;
pub const CMD_RDSR: u8 = 0x05;
pub const CMD_RDCR: u8 = 0x15;
pub const CMD_RDSR2: u8 = 0x35;
pub const CMD_WRSR: u8 = 0x01;
pub const CMD_WRSR2: u8 = 0x31;
pub const CMD_SFDP: u8 = 0x5A;
pub const CMD_EN4B: u8 = 0xB7;
pub const CMD_EX4B: u8 = 0xE9;
pub const CMD_RDFSR: u8 = 0x70;
pub const CMD_VSR_WREN: u8 = 0x50;

/// read commands 12
pub const CMD_READ_1_1_1_3B: u8 = 0x03;
pub const CMD_READ_1_1_1_4B: u8 = 0x13;
pub const CMD_FREAD_1_1_1_3B: u8 = 0x0B;
pub const CMD_FREAD_1_1_1_4B: u8 = 0x0C;
pub const CMD_READ_1_1_2_3B: u8 = 0x3B;
pub const CMD_READ_1_1_2_4B: u8 = 0x3C;
pub const CMD_READ_1_2_2_3B: u8 = 0xBB;
pub const CMD_READ_1_2_2_4B: u8 = 0xBC;
pub const CMD_READ_1_1_4_3B: u8 = 0x6B;
pub const CMD_READ_1_1_4_4B: u8 = 0x6C;
pub const CMD_READ_1_4_4_3B: u8 = 0xEB;
pub const CMD_READ_1_4_4_4B: u8 = 0xEC;

// write command 6
pub const CMD_PP_1_1_1_3B: u8 = 0x02;
pub const CMD_PP_1_1_1_4B: u8 = 0x12;
pub const CMD_PP_1_1_4_3B: u8 = 0x32;
pub const CMD_PP_1_1_4_4B: u8 = 0x34;
pub const CMD_PP_1_4_4_3B: u8 = 0x38;
pub const CMD_PP_1_4_4_4B: u8 = 0x3E;

// sector erase command 4
pub const CMD_SE_1_1_0_3B: u8 = 0x20;
pub const CMD_SE_1_1_0_4B: u8 = 0x21;
pub const CMD_SE_1_1_0_64_3B: u8 = 0xD8;
pub const CMD_SE_1_1_0_64_4B: u8 = 0xDC;

//Write Extend Address Register
pub const CMD_WREAR: u8 = 0xC5;

// define the total ram size used to record exception log
pub const SPIM_LOG_RAM_TOTAL_SIZE: u32 = 2048;

pub const SPIM_CMD_TABLE_LOCK_MASK: u32 = 1 << 23;
pub const SPIM_CMD_TABLE_VALID_ONCE_BIT: u32 = 1 << 31;
pub const SPIM_CMD_TABLE_VALID_BIT: u32 = 1 << 30;
pub const SPIM_CMD_TABLE_CMD_MASK: u32 = 0xFF;

pub const SPIM0_CS_BIT: u32 = 1 << 6;
pub const SPIM1_CS_BIT: u32 = 1 << 2;
pub const SPIM2_CS_BIT: u32 = 1 << 2;
pub const SPIM3_CS_BIT: u32 = 1 << 16;

pub const SPIM0_EXT_MUX_SEL_BIT_POS: u32 = 12;

//pin control

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CmdTableInfo {
    cmd: u8,
    reserved: [u8; 3],
    cmd_table_val: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct RegionInfo {
    pub start: u32,
    pub length: u32,
}

//#[derive(Debug, Clone, Copy)]
//pub struct GpioInfo {

//}
//compile time - const fn
#[allow(clippy::too_many_arguments)]
#[must_use]
pub const fn cmd_table_value(
    g: u32,
    w: u32,
    r: u32,
    m: u32,
    dat_mode: u32,
    dummy: u32,
    prog_sz: u32,
    addr_len: u32,
    addr_mode: u32,
    cmd: u32,
) -> u32 {
    (g << 29)
        | (w << 28)
        | (r << 27)
        | (m << 26)
        | (dat_mode << 24)
        | (dummy << 16)
        | (prog_sz << 13)
        | (addr_len << 10)
        | (addr_mode << 8)
        | cmd
}

pub fn spim_get_cmd_table_val(cmd: u8) -> Result<u32, SpiMonitorError> {
    for entry in CMDS_ARRAY {
        if entry.cmd == cmd {
            return Ok(entry.cmd_table_val);
        }
    }
    Err(SpiMonitorError::CommandNotFound(cmd))
}

//32 Allow Command Table Entries
//total commands: 36
pub static CMDS_ARRAY: &[CmdTableInfo] = &[
    CmdTableInfo {
        cmd: CMD_READ_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 0, 0, 3, 1, CMD_READ_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 0, 0, 4, 1, CMD_READ_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_FREAD_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 8, 0, 3, 1, CMD_FREAD_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_FREAD_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 1, 8, 0, 4, 1, CMD_FREAD_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_2_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 8, 0, 3, 1, CMD_READ_1_1_2_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_2_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 8, 0, 4, 1, CMD_READ_1_1_2_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_2_2_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 4, 0, 3, 2, CMD_READ_1_2_2_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_2_2_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 2, 4, 0, 4, 2, CMD_READ_1_2_2_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 8, 0, 3, 1, CMD_READ_1_1_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_1_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 8, 0, 4, 1, CMD_READ_1_1_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_4_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 6, 0, 3, 3, CMD_READ_1_4_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_READ_1_4_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 1, 3, 6, 0, 4, 3, CMD_READ_1_4_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_1_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 1, 0, 1, 3, 1, CMD_PP_1_1_1_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_1_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 1, 0, 1, 4, 1, CMD_PP_1_1_1_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_4_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 3, 0, 1, 3, 1, CMD_PP_1_1_4_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_PP_1_1_4_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 3, 0, 1, 4, 1, CMD_PP_1_1_4_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 1, 3, 1, CMD_SE_1_1_0_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 1, 4, 1, CMD_SE_1_1_0_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_64_3B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 5, 3, 1, CMD_SE_1_1_0_64_3B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SE_1_1_0_64_4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 1, 0, 0, 5, 4, 1, CMD_SE_1_1_0_64_4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_WREN,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_WREN as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRDIS,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_WRDIS as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDSR2,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDSR2 as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WRSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_WRSR2,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WRSR2 as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDCR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDCR as u32),
    },
    CmdTableInfo {
        cmd: CMD_EN4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 0, 0, 0, 0, 0, 0, 0, 0, CMD_EN4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_EX4B,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 0, 0, 0, 0, 0, 0, 0, 0, CMD_EX4B as u32),
    },
    CmdTableInfo {
        cmd: CMD_SFDP,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 8, 0, 3, 1, CMD_SFDP as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDID,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDID as u32),
    },
    CmdTableInfo {
        cmd: CMD_RDFSR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 1, 0, 1, 0, 0, 0, 0, CMD_RDFSR as u32),
    },
    CmdTableInfo {
        cmd: CMD_VSR_WREN,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(1, 0, 0, 0, 0, 0, 0, 0, 0, CMD_VSR_WREN as u32),
    },
    CmdTableInfo {
        cmd: CMD_WREAR,
        reserved: [0; 3],
        cmd_table_val: cmd_table_value(0, 1, 0, 0, 1, 0, 0, 0, 0, CMD_WREAR as u32),
    },
];

impl<SPIPF: SpipfInstance> SpiMonitor<SPIPF> {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        force_rel_flash_rst: bool,
        ext_mux_sel: SpimExtMuxSel,
        allow_cmd_list: &[u8],
        allow_cmd_num: u8,
        read_blocked_regions: &[RegionInfo],
        read_blocked_region_num: u8,
        write_blocked_regions: &[RegionInfo],
        write_blocked_region_num: u8,
    ) -> Self {
        assert!(allow_cmd_num as usize <= SPIM_CMD_TABLE_NUM);
        assert!(read_blocked_region_num as usize <= BLOCK_REGION_NUM);
        assert!(write_blocked_region_num as usize <= BLOCK_REGION_NUM);

        let extra_clk_en = false;
        let spi_monitor = unsafe { &*SPIPF::ptr() };
        let scu = unsafe { &*Scu::ptr() };
        let mut allow_cmd_array = [0u8; SPIM_CMD_TABLE_NUM];
        allow_cmd_array[..allow_cmd_num as usize]
            .copy_from_slice(&allow_cmd_list[..allow_cmd_num as usize]);

        let mut read_regions_array = [RegionInfo {
            start: 0,
            length: 0,
        }; BLOCK_REGION_NUM];
        read_regions_array[..read_blocked_region_num as usize]
            .copy_from_slice(&read_blocked_regions[..read_blocked_region_num as usize]);

        let mut write_regions_array = [RegionInfo {
            start: 0,
            length: 0,
        }; BLOCK_REGION_NUM];
        write_regions_array[..write_blocked_region_num as usize]
            .copy_from_slice(&write_blocked_regions[..write_blocked_region_num as usize]);

        Self {
            spi_monitor,
            scu,
            extra_clk_en,
            force_rel_flash_rst,
            ext_mux_sel,
            allow_cmd_list: allow_cmd_array,
            allow_cmd_num,
            read_blocked_regions: read_regions_array,
            read_blocked_region_num,
            write_blocked_regions: write_regions_array,
            write_blocked_region_num,
            _marker: PhantomData,
        }
    }
    pub fn spim_scu_ctrl_set(&mut self, mask: u32, val: u32) {
        let mut reg_val = self.scu.scu0f0().read().bits();
        reg_val &= !mask;
        reg_val |= val;
        self.scu.scu0f0().write(|w| unsafe { w.bits(reg_val) });
    }

    pub fn spim_scu_ctrl_clear(&mut self, clear_bits: u32) {
        let mut reg_val = self.scu.scu0f0().read().bits();
        reg_val &= !clear_bits;
        self.scu.scu0f0().write(|w| unsafe { w.bits(reg_val) });
    }

    //SPI_M1: GPIOA6, SCU610[6], SPI master CS output
    //SPI_M2: GPIOC4, SCU610[20]
    //SPI_M3: GPIOE2, SCU614[2] (dummy, cannot be disabled)
    //SPI_M4: GPIOG0, SCU614[16]
    //disable chip select internal pull down
    pub fn spim_disable_cs_internal_pd(&mut self) {
        //SPIPF::FILTER_ID
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.scu
                    .scu610()
                    .modify(|_, w| w.dis_gpioa6int_pull_down().bit(true));
            }
            SpiMonitorNum::SPIM1 => {
                self.scu
                    .scu610()
                    .modify(|_, w| w.dis_gpioc0int_pull_down().bit(true));
            }
            SpiMonitorNum::SPIM2 => {}
            SpiMonitorNum::SPIM3 => {
                self.scu
                    .scu614()
                    .modify(|_, w| w.dis_gpiog0int_pull_down().bit(true));
            }
        }
    }
    //Enable MISO
    pub fn spim_miso_multi_func_adjust(&mut self, enable: bool) {
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.scu
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor1misoin_fn_pin().bit(enable));
            }
            SpiMonitorNum::SPIM1 => {
                self.scu
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor2misoin_fn_pin().bit(enable));
            }
            SpiMonitorNum::SPIM2 => {
                self.scu
                    .scu690()
                    .modify(|_, w| w.enbl_qspimonitor3misoin_fn_pin().bit(enable));
            }
            SpiMonitorNum::SPIM3 => {
                self.scu
                    .scu694()
                    .modify(|_, w| w.enbl_qspimonitor4misoin_fn_pin().bit(enable));
            }
        }
    }

    //enable/disable pass through
    pub fn spim_scu_passthrough_mode(&mut self, passthrough_en: bool) {
        self.scu.scu0f0().modify(|_, w| match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => w.enbl_passthrough_of_spipf1().bit(passthrough_en),
            SpiMonitorNum::SPIM1 => w.enbl_passthrough_of_spipf2().bit(passthrough_en),
            SpiMonitorNum::SPIM2 => w.enbl_passthrough_of_spipf3().bit(passthrough_en),
            SpiMonitorNum::SPIM3 => w.enbl_passthrough_of_spipf4().bit(passthrough_en),
        });
    }

    //set passthrough mode
    pub fn spim_passthrough_mode_set(&mut self, mode: SpimPassthroughMode) {
        match mode {
            SpimPassthroughMode::SinglePassthrough => {
                self.spi_monitor
                    .spipf000()
                    .modify(|_, w| w.enbl_single_bit_passthrough().bit(true));
            }
            SpimPassthroughMode::MultiPassthrough => {
                self.spi_monitor.spipf000().modify(|_, w| {
                    w.enbl_single_bit_passthrough()
                        .bit(false)
                        .enbl_multiple_bit_passthrough()
                        .bit(true)
                });
            }
        }
    }

    //Internal SPI master selection
    //Select internal SPI master connection
    pub fn spim_spi_ctrl_detour_enable(&mut self, spi_master: SpimSpiMaster, enable: bool) {
        if enable {
            self.scu.scu0f0().modify(|_, w| unsafe {
                w.select_int_spimaster_connection()
                    .bits(SPIPF::FILTER_ID as u8 + 1)
            });
            let mut bit_val: bool = true;
            if spi_master == SpimSpiMaster::SPI1 {
                bit_val = false;
            }
            self.scu
                .scu0f0()
                .modify(|_, w| w.int_spimaster_sel().bit(bit_val));
        } else {
            self.spim_scu_ctrl_clear(0xF);
        }
    }

    pub fn spim_passthrough_config(&mut self, passthrough_en: bool, mode: SpimPassthroughMode) {
        // self.spim_scu_passthrough_enable(passthrough_en);
        if passthrough_en {
            self.spim_passthrough_mode_set(mode);
        } else {
            self.spi_monitor.spipf000().modify(|_, w| {
                w.enbl_single_bit_passthrough()
                    .bit(false)
                    .enbl_multiple_bit_passthrough()
                    .bit(false)
            });
        }
    }
    //External Mux Selection Signal
    pub fn spim_ext_mux_config(&mut self, mux_sel: SpimExtMuxSel) {
        assert!(mux_sel as u32 <= SpimExtMuxSel::SpimExtMuxSel1 as u32);
        let bit_val = mux_sel.to_bool();
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.ext_mux_select_sig_of_spipf1().bit(bit_val));
            }
            SpiMonitorNum::SPIM1 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.ext_mux_select_sig_of_spipf2().bit(bit_val));
            }
            SpiMonitorNum::SPIM2 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.ext_mux_select_sig_of_spipf3().bit(bit_val));
            }
            SpiMonitorNum::SPIM3 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.ext_mux_select_sig_of_spipf4().bit(bit_val));
            }
        }
    }
    //
    //Block Mode: Block a command by one extra CLK
    //Block a command by deasserting CS early
    pub fn spim_block_mode_config(&mut self, block_mode: SpimBlockMode) {
        if block_mode == SpimBlockMode::SpimBlockExtraClk {
            self.spi_monitor
                .spipf000()
                .modify(|_, w| w.block_mode().bit(true));
        } else {
            self.spi_monitor
                .spipf000()
                .modify(|_, w| w.block_mode().bit(false));
        }
    }

    pub fn spim_write_fixed_loc_in_allow_cmd_table(&mut self, cmd: u8, reg_val: u32) -> usize {
        match cmd {
            CMD_EN4B => {
                self.spi_monitor
                    .spipfwt(0)
                    .write(|w| unsafe { w.bits(reg_val) });
                0
            }
            CMD_EX4B => {
                self.spi_monitor
                    .spipfwt(1)
                    .write(|w| unsafe { w.bits(reg_val) });
                1
            }
            CMD_WREAR => {
                self.spi_monitor
                    .spipfwt(MAX_CMD_INDEX)
                    .write(|w| unsafe { w.bits(reg_val) });
                MAX_CMD_INDEX
            }
            _ => MAX_CMD_INDEX + 1,
        }
    }
    //init allow commands table registers
    //0, 1 and 31 are reserved for the particular commands
    pub fn spim_allow_cmd_table_init(&mut self, cmd_list: &[u8], cmd_num: u8, flag: u32) {
        let mut index = 1;
        let list_size = min(cmd_num as usize, cmd_list.len());
        for &cmd in cmd_list.iter().take(list_size) {
            //retrieve Allow Command Table Register Value
            let Ok(mut reg_val) = spim_get_cmd_table_val(cmd) else {
                //eprintln!("Unknown SPI command: 0x{:02X}", cmd);
                continue;
            };
            if (flag & FLAG_CMD_TABLE_VALID_ONCE) > 0 {
                reg_val |= SPIM_CMD_TABLE_VALID_ONCE_BIT;
            } else {
                reg_val |= SPIM_CMD_TABLE_VALID_BIT;
            }
            //attemp to write to a fixed location
            if self.spim_write_fixed_loc_in_allow_cmd_table(cmd, reg_val) <= MAX_CMD_INDEX {
                continue;
            }

            index += 1;

            //write to dynamic slot
            if index < MAX_CMD_INDEX {
                self.spi_monitor
                    .spipfwt(index)
                    .write(|w| unsafe { w.bits(reg_val) });
                //eprintln!("The allowed command number may exceed the expected.");
            }
        }
    }

    pub fn spim_get_empty_allow_cmd_slot(&mut self) -> Result<u32, SpiMonitorError> {
        for index in 2..SPIM_CMD_TABLE_NUM {
            let reg_val = self.spi_monitor.spipfwt(index).read().bits();
            if reg_val == 0 {
                return Ok(u32::try_from(index).unwrap());
            }
        }
        Err(SpiMonitorError::NoAllowCmdSlotAvail(
            u32::try_from(SPIM_CMD_TABLE_NUM).unwrap(),
        ))
    }

    pub fn spim_get_allow_cmd_slot(
        &mut self,
        cmd: u8,
        start_offset: u32,
    ) -> Result<u32, SpiMonitorError> {
        if start_offset >= u32::try_from(SPIM_CMD_TABLE_NUM).unwrap() {
            return Err(SpiMonitorError::InvalidCmdSlotIndex(start_offset));
        }

        for index in start_offset..u32::try_from(SPIM_CMD_TABLE_NUM).unwrap() {
            let reg_val = self.spi_monitor.spipfwt(index as usize).read().bits();
            if (reg_val & SPIM_CMD_TABLE_CMD_MASK) == u32::from(cmd) {
                return Ok(index);
            }
        }

        Err(SpiMonitorError::CommandNotFound(cmd))
    }

    pub fn spim_add_new_command(&mut self, cmd: u8, flag: u32) -> Result<u32, SpiMonitorError> {
        // Retrieve the command table value
        let Ok(mut reg_val) = spim_get_cmd_table_val(cmd) else {
            return Err(SpiMonitorError::CommandNotFound(cmd));
        };
        if (flag & FLAG_CMD_TABLE_VALID_ONCE) > 0 {
            reg_val |= SPIM_CMD_TABLE_VALID_ONCE_BIT;
        } else {
            reg_val |= SPIM_CMD_TABLE_VALID_BIT;
        }

        //Try to write to a fixed location in the command table registers
        let index = self.spim_write_fixed_loc_in_allow_cmd_table(cmd, reg_val);
        if index <= MAX_CMD_INDEX {
            return Ok(u32::try_from(index).unwrap());
        }
        match self.spim_get_empty_allow_cmd_slot() {
            Ok(index) => {
                self.spi_monitor
                    .spipfwt(index as usize)
                    .write(|w| unsafe { w.bits(reg_val) });
                Ok(index)
            }
            Err(_) => Err(SpiMonitorError::NoAllowCmdSlotAvail(u32::from(cmd))),
        }
    }
    //  If the command already exists in allow command table and
    //  it is disabled, it will be enabled by spim_add_allow_command.
    //  If the command already exists in allow command table and
    //  it is lock, it will not be enabled and an error code will
    //   be returned.
    //  If the command doesn't exist in allow command table, an
    //   empty slot will be found and the command info will be
    //   filled into.
    pub fn spim_add_allow_command(&mut self, cmd: u8, flag: u32) -> Result<u32, SpiMonitorError> {
        //check if the command is already in allow command Table
        let mut offset: u32 = 0;

        while offset < u32::try_from(SPIM_CMD_TABLE_NUM).unwrap() {
            match self.spim_get_allow_cmd_slot(cmd, offset) {
                Ok(index) => {
                    //it's locked?
                    let mut reg_val = self.spi_monitor.spipfwt(index as usize).read().bits();
                    //Command can't be enabled in this slot
                    //continue searching for the next slot for the same command
                    if (reg_val & SPIM_CMD_TABLE_LOCK_MASK) != 0 {
                        offset = index + 1;
                        continue;
                    }
                    //found and not locked
                    if (flag & FLAG_CMD_TABLE_VALID_ONCE) > 0 {
                        reg_val |= SPIM_CMD_TABLE_VALID_ONCE_BIT;
                    } else {
                        reg_val |= SPIM_CMD_TABLE_VALID_BIT;
                    }
                    self.spi_monitor
                        .spipfwt(index as usize)
                        .write(|w| unsafe { w.bits(reg_val) });
                    return Ok(index);
                }
                _ => {
                    break;
                }
            }
        } //end while
          //Try adding the new command
        self.spim_add_new_command(cmd, flag)
    }
    //All command table slots where command is equal to "cmd", valid and not locked
    //will be removed
    pub fn spim_remove_allow_command(&mut self, cmd: u8) -> Result<u32, SpiMonitorError> {
        //check if the command is already in allow command Table
        let mut offset: u32 = 0;
        let mut count: u32 = 0;

        while offset < u32::try_from(SPIM_CMD_TABLE_NUM).unwrap() {
            match self.spim_get_allow_cmd_slot(cmd, offset) {
                Ok(index) => {
                    let reg_val = self.spi_monitor.spipfwt(index as usize).read().bits();

                    //Slot is not locked
                    if reg_val & SPIM_CMD_TABLE_LOCK_MASK == 0 {
                        self.spi_monitor
                            .spipfwt(index as usize)
                            .write(|w| unsafe { w.bits(0) });
                        count += 1;
                    } else if reg_val & SPIM_CMD_TABLE_VALID_BIT != 0 {
                        //locked and invalid
                        return Err(SpiMonitorError::AllowCmdSlotInvalid(u32::from(cmd)));
                    }
                    offset = index + 1;
                }
                // No more command not found in command registers
                Err(_) => break,
            }
        }
        if count == 0 {
            Err(SpiMonitorError::CommandNotFound(cmd))
        } else {
            Ok(count)
        }
    }

    //The overall allow command table will be locked when
    //  flag is FLAG_CMD_TABLE_LOCK_ALL.
    //- All command table slot which command is equal to "cmd"
    //  parameter will be locked.
    //
    pub fn spim_lock_allow_command_table(
        &mut self,
        cmd: u8,
        flag: u32,
    ) -> Result<u32, SpiMonitorError> {
        //check if the command is already in allow command Table
        let mut offset: u32 = 0;
        let mut count: u32 = 0;

        if (flag & FLAG_CMD_TABLE_LOCK_ALL) != 0 {
            for index in 0..SPIM_CMD_TABLE_NUM {
                self.spi_monitor
                    .spipfwt(index)
                    .modify(|_, w| w.lock().bit(true));
            }
            return Ok(u32::try_from(SPIM_CMD_TABLE_NUM).unwrap());
        }

        while offset < u32::try_from(SPIM_CMD_TABLE_NUM).unwrap() {
            match self.spim_get_allow_cmd_slot(cmd, offset) {
                Ok(index) => {
                    //it's locked?
                    let reg_val = self.spi_monitor.spipfwt(index as usize).read().bits();
                    if (reg_val & SPIM_CMD_TABLE_LOCK_MASK) == 0 {
                        self.spi_monitor
                            .spipfwt(index as usize)
                            .modify(|_, w| w.lock().bit(true));
                        count += 1;
                    }
                    offset = index + 1;
                }
                // no more command found
                Err(_) => break,
            }
        }
        if count == 0 {
            Err(SpiMonitorError::CommandNotFound(cmd))
        } else {
            Ok(count)
        }
    }
    pub fn spim_is_pri_regs_locked(&mut self, rw_select: AddrPrivRWSel) -> bool {
        match rw_select {
            AddrPrivRWSel::AddrPrivWriteSel => {
                if self.spi_monitor.spipf07c().read().wr_dis_of_spipfwa().bit() {
                    return true;
                }
            }
            AddrPrivRWSel::AddrPrivReadSel => {
                if self.spi_monitor.spipf07c().read().wr_dis_of_spipfra().bit() {
                    return true;
                }
            }
        }
        false
    }
    pub fn spim_addr_priv_access_enable(&mut self, priv_sel: AddrPrivRWSel) {
        let mut reg_val = self.spi_monitor.spipf000().read().bits();
        //mask out the upper 8 bits
        reg_val &= 0x00FF_FFFF;

        match priv_sel {
            AddrPrivRWSel::AddrPrivReadSel => reg_val |= SEL_READ_TBL_MAJIC,
            AddrPrivRWSel::AddrPrivWriteSel => reg_val |= SEL_WRITE_TBL_MAJIC,
        }
        self.spi_monitor
            .spipf000()
            .write(|w| unsafe { w.bits(reg_val) });
    }

    //get aligned length
    pub fn spim_get_adjusted_addr_len(&mut self, addr: u32, len: u32) -> (u32, u32) {
        if len == 0 {
            return (addr, 0);
        }
        let mut adjusted_len = len;
        let mut aligned_addr = addr;
        //start address alignment, protect more
        if !addr.is_multiple_of(ACCESS_BLOCK_UNIT) {
            adjusted_len += addr % ACCESS_BLOCK_UNIT;
            aligned_addr = (addr / ACCESS_BLOCK_UNIT) * ACCESS_BLOCK_UNIT;
        }
        //make len 16KB aligment
        adjusted_len = adjusted_len.div_ceil(ACCESS_BLOCK_UNIT) * ACCESS_BLOCK_UNIT;
        (aligned_addr, adjusted_len)
    }
    //Each bit defines permission of one 16KB block
    //Calculate numbers of 16KB blocks
    //Start address may cross two different 16KB blocks
    pub fn spim_get_total_block_num(&mut self, addr: u32, len: u32) -> u32 {
        let (_aligned_addr, adjusted_len) = self.spim_get_adjusted_addr_len(addr, len);
        adjusted_len / ACCESS_BLOCK_UNIT
    }

    pub fn spim_address_privilege_config(
        &mut self,
        rw_select: AddrPrivRWSel,
        priv_op: AddrPriOp,
        addr: u32,
        len: u32,
    ) -> Result<u32, SpiMonitorError> {
        if addr >= ADDR_LIMIT {
            return Err(SpiMonitorError::AddressInvalid(addr));
        }
        if (len == 0) || (addr + len > ADDR_LIMIT) {
            return Err(SpiMonitorError::LengthInvalid(len));
        }
        if self.spim_is_pri_regs_locked(rw_select) {
            return Err(SpiMonitorError::AddrTblRegsLocked(rw_select as u32));
        }

        let (aligned_addr, adjusted_len) = self.spim_get_adjusted_addr_len(addr, len);
        //Each register SPIPFWA/SPIFRA can protect 512KB = 32*16KB
        let mut reg_off = (aligned_addr / ACCESS_BLOCK_PER_REG) as usize;
        let mut bit_off = (aligned_addr % ACCESS_BLOCK_PER_REG) / ACCESS_BLOCK_UNIT;
        let total_blocks = adjusted_len / ACCESS_BLOCK_UNIT;
        let mut total_bit_num = total_blocks;

        self.spim_addr_priv_access_enable(rw_select);

        while total_bit_num > 0 {
            //reset after incrementing to 32
            if bit_off > 31 {
                bit_off = 0;
                reg_off += 1;
            }
            if (bit_off == 0) && (total_bit_num >= 32) {
                // speed up for large area configuration
                if priv_op == AddrPriOp::FlagAddrPrivEnable {
                    self.spi_monitor
                        .spipfwa(reg_off)
                        .write(|w| unsafe { w.bits(0xffff_ffff) });
                } else {
                    self.spi_monitor
                        .spipfwa(reg_off)
                        .write(|w| unsafe { w.bits(0x0) });
                }
                reg_off += 1;
                total_bit_num -= 32;
            } else {
                let mut reg_val = self.spi_monitor.spipfwa(reg_off).read().bits();
                if priv_op == AddrPriOp::FlagAddrPrivEnable {
                    reg_val |= 1 << bit_off;
                } else {
                    reg_val &= !(1 << bit_off);
                }
                self.spi_monitor
                    .spipfwa(reg_off)
                    .write(|w| unsafe { w.bits(reg_val) });
                //println!(
                //   "Addr Reg index = {}, value = {:#010X}",
                //   reg_off,
                //    self.spi_monitor.spipfwa(reg_off).read().bits();
                // );
                bit_off += 1;
                total_bit_num -= 1;
            }
        }
        Ok(total_blocks)
    }
    //lock all SPIPFWA/RA for writing
    pub fn spim_lock_rw_priv_table(&mut self, rw_select: AddrPrivRWSel) {
        if rw_select == AddrPrivRWSel::AddrPrivWriteSel {
            self.spi_monitor
                .spipf07c()
                .modify(|_, w| w.wr_dis_of_spipfwa().bit(true));
        }
        if rw_select == AddrPrivRWSel::AddrPrivReadSel {
            self.spi_monitor
                .spipf07c()
                .modify(|_, w| w.wr_dis_of_spipfra().bit(true));
        }
    }
    //
    pub fn spim_lock_common(&mut self) {
        self.spim_lock_rw_priv_table(AddrPrivRWSel::AddrPrivReadSel);
        self.spim_lock_rw_priv_table(AddrPrivRWSel::AddrPrivWriteSel);
        let _ = self.spim_lock_allow_command_table(0, FLAG_CMD_TABLE_LOCK_ALL);

        self.spi_monitor.spipf000().modify(|_, w| {
            w.wr_dis_of_spipf000()
                .bit(true)
                .wr_dis_of_spipf0001()
                .bit(true)
        });
        //
        self.spi_monitor.spipf07c().modify(|_, w| {
            w.wr_dis_of_spipf000()
                .bit(true)
                .wr_dis_of_spipf004()
                .bit(true)
                .wr_dis_of_spipf010()
                .bit(true)
                .wr_dis_of_spipf014()
                .bit(true)
        });
    }
    pub fn spim_set_read_blocked_regions(
        &mut self,
        read_blocked_regions: &[RegionInfo],
        read_blocked_region_num: u8,
    ) {
        self.read_blocked_regions[..read_blocked_region_num as usize]
            .copy_from_slice(&read_blocked_regions[..read_blocked_region_num as usize]);
        self.read_blocked_region_num = read_blocked_region_num;
    }

    pub fn spim_set_write_blocked_regions(
        &mut self,
        write_blocked_regions: &[RegionInfo],
        write_blocked_region_num: u8,
    ) {
        self.write_blocked_regions[..write_blocked_region_num as usize]
            .copy_from_slice(&write_blocked_regions[..write_blocked_region_num as usize]);
        self.write_blocked_region_num = write_blocked_region_num;
    }

    // supported command list
    pub fn spim_set_cmd_table(&mut self, allow_cmd_list: &[u8], allow_cmd_num: u8) {
        self.allow_cmd_list[..allow_cmd_num as usize]
            .copy_from_slice(&allow_cmd_list[..allow_cmd_num as usize]);
        self.allow_cmd_num = allow_cmd_num;
    }

    pub fn spim_dump_read_blocked_regions(&mut self) {}
    pub fn spim_dump_write_blocked_regions(&mut self) {}

    //Block read and write to regions
    pub fn spim_rw_perm_init(&mut self) {
        //Enable previliege control for 256MB area
        let _ = self.spim_address_privilege_config(
            AddrPrivRWSel::AddrPrivReadSel,
            AddrPriOp::FlagAddrPrivEnable,
            0x0,
            MAX_PRIV_REGION_SIZE,
        );

        let _ = self.spim_address_privilege_config(
            AddrPrivRWSel::AddrPrivWriteSel,
            AddrPriOp::FlagAddrPrivEnable,
            0x0,
            MAX_PRIV_REGION_SIZE,
        );

        for iter in 0..self.read_blocked_region_num {
            if let Ok(_num_blocks) = self.spim_address_privilege_config(
                AddrPrivRWSel::AddrPrivReadSel,
                AddrPriOp::FlagAddrPrivDisable,
                self.read_blocked_regions[iter as usize].start,
                self.read_blocked_regions[iter as usize].length,
            ) {}
        }
        for iter in 0..self.write_blocked_region_num {
            if let Ok(_num_blocks) = self.spim_address_privilege_config(
                AddrPrivRWSel::AddrPrivWriteSel,
                AddrPriOp::FlagAddrPrivDisable,
                self.write_blocked_regions[iter as usize].start,
                self.write_blocked_regions[iter as usize].length,
            ) {}
        }
    }
    //Enable/disable SPI monitor from SCU
    pub fn spim_scu_monitor_config(&mut self, enable: bool) {
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.enbl_filter_of_spipf1().bit(enable));
            }
            SpiMonitorNum::SPIM1 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.enbl_filter_of_spipf2().bit(enable));
            }
            SpiMonitorNum::SPIM2 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.enbl_filter_of_spipf3().bit(enable));
            }
            SpiMonitorNum::SPIM3 => {
                self.scu
                    .scu0f0()
                    .modify(|_, w| w.enbl_filter_of_spipf4().bit(enable));
            }
        }
    }
    //Enable/disable SPI monitor/filter function
    pub fn spim_ctrl_monitor_config(&mut self, enable: bool) {
        self.spi_monitor
            .spipf000()
            .modify(|_, w| w.enbl_filter_fn().bit(enable));
    }

    pub fn spim_monitor_enable(&mut self, enable: bool) {
        self.spim_ctrl_monitor_config(enable);
        // self.spim_miso_multi_func_adjust(enable);
        self.spim_passthrough_config(enable, SpimPassthroughMode::SinglePassthrough);
    }

    //Use SCU0F0 to enable flash rst
    pub fn spim_release_flash_rst(&mut self) {
        //SCU0F0[23:20]: Reset source selection
        //SCU0F0[27:24]: Enable reset signal output
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf1rst_source_sel()
                        .bit(true)
                        .spipf1rst_output_enbl()
                        .bit(true)
                });
                // delay
                cortex_m::asm::delay(200); // 1us delay at 200MHz
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf1rst_source_sel()
                        .bit(false)
                        .spipf1rst_output_enbl()
                        .bit(false)
                });
            }
            SpiMonitorNum::SPIM1 => {
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf2rst_source_sel()
                        .bit(true)
                        .spipf2rst_output_enbl()
                        .bit(true)
                });
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf2rst_source_sel()
                        .bit(false)
                        .spipf2rst_output_enbl()
                        .bit(false)
                });
            }
            SpiMonitorNum::SPIM2 => {
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf3rst_source_sel()
                        .bit(true)
                        .spipf3rst_output_enbl()
                        .bit(true)
                });
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf3rst_source_sel()
                        .bit(false)
                        .spipf3rst_output_enbl()
                        .bit(false)
                });
            }
            SpiMonitorNum::SPIM3 => {
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf4rst_source_sel()
                        .bit(true)
                        .spipf4rst_output_enbl()
                        .bit(true)
                });
                self.scu.scu0f0().modify(|_, w| {
                    w.spipf4rst_source_sel()
                        .bit(false)
                        .spipf4rst_output_enbl()
                        .bit(false)
                });
            }
        }
    }

    //Enable push pull mode
    pub fn spim_push_pull_mode_config(&mut self) {
        self.spi_monitor
            .spipf004()
            .modify(|_, w| w.enbl_push_pull_mode().bit(true));
        //When AST060 is in unprovision state,
        //SPIPF000[0] and SCU0F0[11:8] should be set for achieving
        //push-pull mode.
        self.spim_passthrough_config(true, SpimPassthroughMode::SinglePassthrough);
        self.spim_scu_monitor_config(true);
    }

    pub fn spim_irq_enable(&mut self) {
        self.spi_monitor.spipf004().modify(|_, w| {
            w.enbl_intof_cmd_block()
                .bit(true)
                .enbl_intof_wr_block()
                .bit(true)
                .enbl_intof_read_block()
                .bit(true)
        });
    }
    pub fn spim_abnormal_log_init(&mut self) {}
    pub fn spim_sw_rst(&mut self) {
        self.spi_monitor
            .spipf000()
            .modify(|_, w| w.sweng_rst().bit(true));
        //delay 5us, 200Mhz
        for _ in 0..(5 * 200) {
            cortex_m::asm::nop();
        }
        self.spi_monitor
            .spipf000()
            .modify(|_, w| w.sweng_rst().bit(false));
    }

    //SCU410, SCU414[27:0], SCU4B4  must keep at value 0x0
    //SPIM0 pin ctrl
    //SCU410/4B0/690[13:0], rstin: SCU414/4B4/694[24]
    pub fn spim_enbl_spim0_pin_ctrl(&mut self) {
        // Clear SCU4B0[13:0]
        let mut reg_val = self.scu.scu4b0().read().bits();
        reg_val &= !0x3FFF;
        self.scu.scu4b0().write(|w| unsafe { w.bits(reg_val) });
        // Set SCU690[13:0]
        reg_val = self.scu.scu690().read().bits();
        reg_val |= 0x3FF7;
        self.scu.scu690().write(|w| unsafe { w.bits(reg_val) });
        // Enable QSPI monitor reset-in function pin
        self.scu
            .scu694()
            .modify(|_, w| w.enbl_qspimonitor1rstin_fn_pin().bit(true));
    }
    //SCU410, SCU41C[7:0], SCU4B4, 4BC[24:0] must keep at value 0x0
    //SPIM1 pin ctrl
    //SCU410/4B0/690[27:14], rstin:41C/4BC/69C[9]
    pub fn spim_enbl_spim1_pin_ctrl(&mut self) {
        // Clear SCU4B0[14:27]
        let mut reg_val = self.scu.scu4b0().read().bits();
        reg_val &= !(0x3FFF << 14);
        self.scu.scu4b0().write(|w| unsafe { w.bits(reg_val) });
        // Set SCU690[14:27]
        reg_val = self.scu.scu690().read().bits();
        reg_val |= 0x3FFF << 14;
        self.scu.scu690().write(|w| unsafe { w.bits(reg_val) });
        // Disable SGPIO Master LD function pin
        self.scu
            .scu41c()
            .modify(|_, w| w.enbl_sgpiomaster_ldfn_pin().bit(false));
        // Enable reset-in function pin
        self.scu
            .scu69c()
            .modify(|_, w| w.enbl_qspimonitor2rstin_fn_pin().bit(true));
    }

    //SCU410, SCU4B4, SCU414[27:0] must keep at value 0x0
    //SPIM2 pin ctrl
    //SCU410/4B0/690[31:28], SCU414/4B4/694[9:0]
    //rstin SCU414/4B4/694[25]
    pub fn spim_enbl_spim2_pin_ctrl(&mut self) {
        //Clear SCU4B0[31:28]
        let mut reg_val = self.scu.scu4b0().read().bits();
        reg_val &= !(0xF << 28);
        self.scu.scu4b0().write(|w| unsafe { w.bits(reg_val) });
        //Set SCU690[31:28]
        reg_val = self.scu.scu690().read().bits();
        reg_val |= 0xF << 28;
        self.scu.scu690().write(|w| unsafe { w.bits(reg_val) });
        //Set SCU694[9:0]
        reg_val = self.scu.scu694().read().bits();
        reg_val |= 0x3FF;
        self.scu.scu694().write(|w| unsafe { w.bits(reg_val) });
        //reset in
        self.scu
            .scu694()
            .modify(|_, w| w.enbl_qspimonitor3rstin_fn_pin().bit(true));
    }
    //SCU410, SCU41C[7:0], SCU4B4, 4BC[24:0] must keep at value 0x0
    //SCU414/4B4/694[23:10]:SPIM3 pin ctrl rstin,41C/4BC/69C[11]
    pub fn spim_enbl_spim3_pin_ctrl(&mut self) {
        //Set SCU694[23:10]
        let mut reg_val = self.scu.scu694().read().bits();
        reg_val |= 0x3FFF << 10;
        self.scu.scu694().write(|w| unsafe { w.bits(reg_val) });
        self.scu
            .scu41c()
            .modify(|_, w| w.enbl_sgpiomaster_difn_pin().bit(false));
        self.scu
            .scu69c()
            .modify(|_, w| w.enbl_qspimonitor4rstin_fn_pin().bit(true));
    }
    //SCU410,SCU414[27:0],SCU41C[7:0],SCU4B4,4BC[24:0], must be kept as 0x0
    pub fn spim_pin_ctrl_config(&mut self) {
        match SPIPF::FILTER_ID {
            SpiMonitorNum::SPIM0 => {
                self.spim_enbl_spim0_pin_ctrl();
            }
            SpiMonitorNum::SPIM1 => {
                self.spim_enbl_spim1_pin_ctrl();
            }
            SpiMonitorNum::SPIM2 => {
                self.spim_enbl_spim2_pin_ctrl();
            }
            SpiMonitorNum::SPIM3 => {
                self.spim_enbl_spim3_pin_ctrl();
            }
        }
    }
    pub fn aspeed_spi_monitor_init(&mut self) {
        let allow_cmd_list = self.allow_cmd_list;
        let allow_cmd_num = self.allow_cmd_num;

        // always enable internal passthrough configuration
        self.spim_scu_passthrough_mode(true);
        //always keep at master mode during booting up stage
        self.spim_ext_mux_config(self.ext_mux_sel);
        //always disable internal pull-down of CS pin
        self.spim_disable_cs_internal_pd();
        //use push-pull mode to improve IO signal quality
        self.spim_push_pull_mode_config();

        if self.extra_clk_en {
            self.spim_block_mode_config(SpimBlockMode::SpimBlockExtraClk);
        }
        self.spim_allow_cmd_table_init(&allow_cmd_list, allow_cmd_num, 0);
        self.spim_rw_perm_init();
        self.spim_monitor_enable(true);

        //log info init
        //self.spim_abnormal_log_init();

        //irq_enable(self.irq_num);
        self.spim_irq_enable();
        self.spim_pin_ctrl_config();

        if self.force_rel_flash_rst {
            self.spim_release_flash_rst();
        }
    }
}
//
// Example trait for enabling SPI filter
//pub trait SpiFilterEnable {
//    fn enable(&mut self);
//    fn disable(&mut self);
//}
/*
//implement APIs/traits
impl<SPIPF: SpipfInstance> SpiFilterEnable for SpiMonitor<SPIPF> {
    fn enable(&mut self) {
        //self.stop();
    }
}
*/
