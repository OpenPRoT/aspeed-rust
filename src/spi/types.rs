// Licensed under the Apache-2.0 license

use super::consts::ASPEED_MAX_CS;

#[derive(Clone, Copy)]
pub enum DataDirection {
    DRead,
    DWrite,
}

pub struct FlashAddress {
    pub value: u32,
    pub width: AddressWidth,
}

pub enum AddressWidth {
    ThreeByte,
    FourByte,
    None,
}

#[derive(Clone, Copy)]
pub enum CtrlType {
    BootSpi,
    HostSpi,
    NormalSpi,
}

#[derive(Clone, Copy)]
pub struct CommandMode {
    pub normal_read: u32,
    pub normal_write: u32,
    pub user: u32,
}

#[derive(Default, Clone, Copy)]
pub struct SpiDecodeAddress {
    pub start: u32,
    pub len: u32,
}

/// Static SPI controller configuration information
pub struct SpiConfig {
    pub mmap_base: u32,
    pub max_cs: usize,
    pub write_block_size: u32,
    pub ctrl_type: CtrlType,
    pub timing_cali_start_off: u32,
    pub master_idx: u32,
    pub pure_spi_mode_only: bool,
    pub frequency: u32,
    pub timing_calibration_start_off: u32,
    pub timing_calibration_disabled: bool,
}

/// Controller state structure
pub struct SpiData {
    pub decode_addr: [SpiDecodeAddress; ASPEED_MAX_CS],
    pub cmd_mode: [CommandMode; ASPEED_MAX_CS],
    pub hclk: u32,
    pub spim_proprietary_pre_config: u32,
}

impl Default for SpiData {
    fn default() -> Self {
        Self::new()
    }
}

impl SpiData {
    #[must_use]
    pub const fn new() -> Self {
        const ZERO_ADDR: SpiDecodeAddress = SpiDecodeAddress { start: 0, len: 0 };
        const ZERO_CMD: CommandMode = CommandMode {
            normal_read: 0,
            normal_write: 0,
            user: 0,
        };

        Self {
            decode_addr: [ZERO_ADDR; ASPEED_MAX_CS],
            cmd_mode: [ZERO_CMD; ASPEED_MAX_CS],
            hclk: 0,
            spim_proprietary_pre_config: 0,
        }
    }
}

