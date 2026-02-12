// Licensed under the Apache-2.0 license

// Constants (unchanged)
pub(crate) const SPI_CONF_CE0_ENABLE_WRITE_SHIFT: u32 = 16;

pub(crate) const SPI_CTRL_FREQ_MASK: u32 = 0x0F00_0F00;
pub(crate) const SPI_CTRL_CEX_SPI_CMD_SHIFT: u32 = 16;
pub(crate) const SPI_CTRL_CEX_SPI_CMD_MASK: u32 = 0xff;
pub(crate) const SPI_CTRL_CEX_DUMMY_SHIFT: u32 = 6;
pub(crate) const SPI_CTRL_CEX_4BYTE_MODE_SET: u32 = 0x11; // bit0: 4byte mode, bit4: 4byte mode cmd

pub(crate) const SPI_DMA_DELAY_SHIFT: u32 = 8;
pub(crate) const SPI_DMA_DELAY_MASK: u32 = 0xff;
pub(crate) const SPI_DMA_CLK_FREQ_SHIFT: u32 = 16;
pub(crate) const SPI_DMA_CLK_FREQ_MASK: u32 = 0xf;

pub(crate) const SPI_DMA_GET_REQ_MAGIC: u32 = 0xaeed_0000;
pub(crate) const SPI_DMA_DISCARD_REQ_MAGIC: u32 = 0xdeea_0000;
pub(crate) const SPI_DMA_RAM_MAP_BASE: u32 = 0x8000_0000;
pub(crate) const SPI_DMA_FLASH_MAP_BASE: u32 = 0x6000_0000;

pub(crate) const SPI_CALIB_LEN: usize = 0x400;

#[cfg(feature = "spi_dma")]
pub(crate) const SPI_DMA_TRIGGER_LEN: u32 = 128;

#[cfg(feature = "spi_dma")]
pub(crate) const SPI_DMA_WRITE: u32 = 1 << 1;

pub(crate) const SPI_DMA_REQUEST: u32 = 1 << 31;
pub(crate) const SPI_DMA_GRANT: u32 = 1 << 30;
pub(crate) const SPI_DMA_CALIB_MODE: u32 = 1 << 3;
pub(crate) const SPI_DMA_CALC_CKSUM: u32 = 1 << 2;

pub(crate) const SPI_DMA_ENABLE: u32 = 1 << 0;
pub(crate) const SPI_DMA_STATUS: u32 = 1 << 11;

pub(crate) const ASPEED_MAX_CS: usize = 5; // Must be usize for array indexing

pub(crate) const ASPEED_SPI_NORMAL_READ: u32 = 0x1;
pub(crate) const ASPEED_SPI_NORMAL_WRITE: u32 = 0x2;
pub(crate) const ASPEED_SPI_USER: u32 = 0x3;
pub(crate) const ASPEED_SPI_USER_INACTIVE: u32 = 0x4;

pub(crate) const ASPEED_SPI_SZ_2M: u32 = 0x0020_0000;
pub(crate) const ASPEED_SPI_SZ_256M: u32 = 0x1000_0000;

pub(crate) const HPLL_FREQ: u32 = 1_000_000_000;

pub(crate) const SPI_DMA_TIMEOUT: u32 = 0x10000;

// SPIM clock output pin bits
pub(crate) const PIN_SPIM0_CLK_OUT_BIT: u32 = 7;
pub(crate) const PIN_SPIM1_CLK_OUT_BIT: u32 = 21;
pub(crate) const PIN_SPIM2_CLK_OUT_BIT: u32 = 3;
pub(crate) const PIN_SPIM3_CLK_OUT_BIT: u32 = 17;

//pub const SPI_NOR_DATA_DIRECT_READ: u32 = 0x0000_0001;
//pub const SPI_NOR_DATA_DIRECT_WRITE: u32 = 0x0000_0002;
