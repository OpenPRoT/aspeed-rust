// Licensed under the Apache-2.0 license

//! Aspeed SPI HAL module

pub mod device;
pub mod fmccontroller;
pub mod norflash;
pub mod norflashblockdevice;
pub mod spicontroller;
pub mod spidmairqtest;
pub mod spitest;

pub(crate) mod consts;

pub mod util;
pub mod error;
pub mod traits;
pub mod types;

pub mod spim;

pub use error::SpiError;
pub use traits::SpiBusWithCs;
pub use types::{CommandMode, CtrlType, DataDirection, SpiConfig, SpiDecodeAddress, SpiData,
    FlashAddress, AddressWidth};
pub use norflash::{Jesd216Mode, SpiNorCommand, SpiNorDevice};

pub use util::{spi_read_data, spi_write_data, aspeed_get_spi_freq_div, get_mid_point_of_longest_one, 
    spi_cal_dummy_cycle, spi_calibration_enable, get_hclock_rate, spi_io_mode, spi_io_mode_user,
    get_addr_buswidth, get_cmd_buswidth, get_data_buswidth};
pub use spim::{spim_proprietary_post_config, spim_proprietary_pre_config, spim_scu_ctrl_clear, spim_scu_ctrl_set};
