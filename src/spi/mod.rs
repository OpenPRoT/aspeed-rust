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

pub mod error;
pub mod traits;
pub mod types;
pub mod util;

pub mod spim;

pub use error::SpiError;
pub use norflash::{Jesd216Mode, SpiNorCommand, SpiNorDevice};
pub use traits::SpiBusWithCs;
pub use types::{
    AddressWidth, CommandMode, CtrlType, DataDirection, FlashAddress, SpiConfig, SpiData,
    SpiDecodeAddress,
};

pub use spim::{
    spim_proprietary_post_config, spim_proprietary_pre_config, spim_scu_ctrl_clear,
    spim_scu_ctrl_set,
};
pub use util::{
    aspeed_get_spi_freq_div, get_addr_buswidth, get_cmd_buswidth, get_data_buswidth,
    get_hclock_rate, get_mid_point_of_longest_one, spi_cal_dummy_cycle, spi_calibration_enable,
    spi_io_mode, spi_io_mode_user, spi_read_data, spi_write_data,
};
