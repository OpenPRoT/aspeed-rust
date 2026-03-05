// Licensed under the Apache-2.0 license

use crate::{
    modify_reg,
    spi::norflash::{Jesd216Mode, SpiNorData},
};
use ast1060_pac::Scu;
use embedded_hal::spi;
use embedded_hal::spi::ErrorType;
use embedded_hal::spi::SpiBus;
use embedded_io::Write;

pub mod device;
pub mod fmccontroller;
pub mod norflash;
pub mod norflashblockdevice;
pub mod spicontroller;
pub mod spitest;

#[derive(Debug)]

pub enum SpiError {
    BusError,
    DmaTimeout,
    CsSelectFailed(usize),
    LengthMismatch,
    CapacityOutOfRange,
    UnsupportedDevice(u8),
    AddressNotAligned(u32),
    InvalidCommand(u8),
    Other(&'static str),
}

/// Required by embedded-hal 1.0
impl spi::Error for SpiError {
    fn kind(&self) -> spi::ErrorKind {
        match self {
            SpiError::BusError
            | SpiError::DmaTimeout
            | SpiError::CsSelectFailed(_)
            | SpiError::LengthMismatch
            | SpiError::CapacityOutOfRange
            | SpiError::UnsupportedDevice(_)
            | SpiError::InvalidCommand(_)
            | SpiError::AddressNotAligned(_)
            | SpiError::Other(_) => spi::ErrorKind::Other,
        }
    }
}

pub trait SpiBusWithCs: SpiBus<u8, Error = SpiError> + ErrorType<Error = SpiError> {
    fn select_cs(&mut self, cs: usize) -> Result<(), SpiError>;
    fn deselect_cs(&mut self, cs: usize) -> Result<(), SpiError>;
    fn nor_transfer(&mut self, op_info: &mut SpiNorData) -> Result<(), SpiError>;
    fn nor_read_init(&mut self, cs: usize, op_info: &SpiNorData);
    fn nor_write_init(&mut self, cs: usize, op_info: &SpiNorData);

    fn get_device_info(&mut self, cs: usize) -> (u32, u32);
    fn get_master_id(&mut self) -> u32;
}

// Constants (unchanged)
const SPI_CONF_CE0_ENABLE_WRITE_SHIFT: u32 = 16;

const SPI_CTRL_FREQ_MASK: u32 = 0x0F00_0F00;
const SPI_CTRL_CEX_SPI_CMD_SHIFT: u32 = 16;
const SPI_CTRL_CEX_SPI_CMD_MASK: u32 = 0xff;
const SPI_CTRL_CEX_DUMMY_SHIFT: u32 = 6;
const SPI_CTRL_CEX_4BYTE_MODE_SET: u32 = 0x11; // bit0: 4byte mode, bit4: 4byte mode cmd

const SPI_DMA_DELAY_SHIFT: u32 = 8;
const SPI_DMA_DELAY_MASK: u32 = 0xff;
const SPI_DMA_CLK_FREQ_SHIFT: u32 = 16;
const SPI_DMA_CLK_FREQ_MASK: u32 = 0xf;

const SPI_DMA_GET_REQ_MAGIC: u32 = 0xaeed_0000;
const SPI_DMA_DISCARD_REQ_MAGIC: u32 = 0xdeea_0000;
const SPI_DMA_RAM_MAP_BASE: u32 = 0x8000_0000;
const SPI_DMA_FLASH_MAP_BASE: u32 = 0x6000_0000;

const SPI_CALIB_LEN: usize = 0x400;

#[cfg(feature = "spi_dma")]
const SPI_DMA_TRIGGER_LEN: u32 = 128;
//const SPI_DMA_STS: u32 = 1 << 11;
//const SPI_DMA_IRQ_EN: u32 = 1 << 3;
#[cfg(feature = "spi_dma")]
const SPI_DMA_WRITE: u32 = 1 << 1;

const SPI_DMA_REQUEST: u32 = 1 << 31;
const SPI_DMA_GRANT: u32 = 1 << 30;
const SPI_DMA_CALIB_MODE: u32 = 1 << 3;
const SPI_DMA_CALC_CKSUM: u32 = 1 << 2;

const SPI_DMA_ENABLE: u32 = 1 << 0;
const SPI_DMA_STATUS: u32 = 1 << 11;

const ASPEED_MAX_CS: usize = 5; // Must be usize for array indexing

const ASPEED_SPI_NORMAL_READ: u32 = 0x1;
const ASPEED_SPI_NORMAL_WRITE: u32 = 0x2;
const ASPEED_SPI_USER: u32 = 0x3;
const ASPEED_SPI_USER_INACTIVE: u32 = 0x4;

const ASPEED_SPI_SZ_2M: u32 = 0x0020_0000;
const ASPEED_SPI_SZ_256M: u32 = 0x1000_0000;

const HPLL_FREQ: u32 = 1_000_000_000;
//const HCLK_DIV_SEL_MASK: u32 = 0b111 << 28;

//const SPI_NOR_MAX_ID_LEN: u32 = 3;

const SPI_DMA_TIMEOUT: u32 = 0x10000;
const SPI_NOR_DATA_DIRECT_READ: u32 = 0x0000_0001;
const SPI_NOR_DATA_DIRECT_WRITE: u32 = 0x0000_0002;

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

//Static  spi controller configuration information
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

// Struct holding segment behavior as trait object
// controller state structure
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

#[macro_export]
macro_rules! dbg {
    ($self:expr, $($arg:tt)*) => {{
        if let Some(ref mut uart) = $self.dbg_uart {
            writeln!(uart, $($arg)*).unwrap();
            write!(uart, "\r").unwrap();
        }
    }};
}

#[inline]
fn hclk_div_reg_to_val(x: u32) -> u32 {
    if x == 0 {
        2
    } else {
        x + 1
    }
}

#[must_use]
pub fn get_hclock_rate() -> u32 {
    let scu_reg = unsafe { &*Scu::ptr() };
    let raw_div = scu_reg.scu314().read().hclkdivider_sel().bits();
    let clk_div = hclk_div_reg_to_val(u32::from(raw_div));

    HPLL_FREQ / clk_div
}

#[must_use]
pub fn spi_io_mode(mode: Jesd216Mode) -> u32 {
    match mode {
        //Jesd216Mode::Mode111 | Jesd216Mode::Mode111Fast => 0x0000_0000,
        Jesd216Mode::Mode112 => 0x2000_0000,
        Jesd216Mode::Mode122 => 0x3000_0000,
        Jesd216Mode::Mode114 => 0x4000_0000,
        Jesd216Mode::Mode144 => 0x5000_0000,
        _ => 0,
    }
}
#[must_use]
pub fn spi_io_mode_user(bus_width: u32) -> u32 {
    match bus_width {
        4 => 0x4000_0000,
        2 => 0x2000_0000,
        _ => 0x0000_0000,
    }
}
#[must_use]
pub fn spi_cal_dummy_cycle(bus_width: u32, dummy_cycle: u32) -> u32 {
    let dummy_byte = dummy_cycle / (8 / bus_width);
    ((dummy_byte & 0x3) << 6) | (((dummy_byte & 0x4) >> 2) << 14)
}

const fn get_cmd_buswidth(v: u32) -> u8 {
    ((v & 0x0000_0F00) >> 8) as u8
}
const fn get_addr_buswidth(v: u32) -> u8 {
    ((v & 0x0000_00F0) >> 4) as u8
}
const fn get_data_buswidth(v: u32) -> u8 {
    (v & 0x0000_000F) as u8
}

/// Calculate the SPI frequency division setting based on bus clock and max frequency.
///
/// # Arguments
/// * `bus_clk` - The bus clock frequency in Hz.
/// * `max_freq` - The maximum desired SPI frequency in Hz.
///
/// # Returns
/// A 32-bit value encoding the frequency divider,
/// or 0 if no valid divider found.
#[must_use]
pub fn aspeed_get_spi_freq_div(bus_clk: u32, max_freq: u32) -> u32 {
    // Division mapping array matching C div_arr
    let div_arr = [15, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 0];

    for i in 0..0x0f {
        for (j, div_val) in div_arr.iter().copied().enumerate() {
            if i == 0 && j == 0 {
                continue;
            }
            let divisor = j + 1 + (i * 16);
            let freq = bus_clk / u32::try_from(divisor).unwrap();

            if max_freq >= freq {
                #[allow(clippy::cast_sign_loss)]
                return ((i << 24) | ((div_val as u32) << 8) as usize)
                    .try_into()
                    .unwrap();
            }
        }
    }
    // If not found, log error and return 0 (adjust logging as needed)
    //log eprintln!("aspeed_get_spi_freq_div: cannot get correct frequency division.");
    0
}

/// Finds the midpoint of the longest consecutive sequence of 1's in a buffer.
///
/// Returns the midpoint index if the longest run is at least length 4,
/// otherwise returns -1.
///
/// # Arguments
/// * `buf` - slice of bytes (each should be 0 or 1).
#[must_use]
pub fn get_mid_point_of_longest_one(buf: &[u8]) -> i32 {
    let mut start = 0;
    let mut mid_point = 0;
    let mut max_cnt = 0;
    let mut cnt = 0;

    for (i, &val) in buf.iter().enumerate() {
        if val == 1 {
            cnt += 1;
        } else {
            cnt = 0;
            start = i;
        }

        if cnt > max_cnt {
            max_cnt = cnt;
            mid_point = start + (cnt / 2);
        }
    }

    if max_cnt < 4 {
        -1
    } else {
        i32::try_from(mid_point).unwrap()
    }
}

#[must_use]
pub fn spi_calibration_enable(buf: &[u8]) -> bool {
    if buf.len() < 4 {
        return false;
    }

    let mut valid_count = 0;

    // Process 4 bytes at a time
    for chunk in buf.chunks_exact(4) {
        // Convert 4 bytes to u32 in little-endian order
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

        if word != 0 && word != 0xFFFF_FFFF {
            valid_count += 1;
        }
        if valid_count > 100 {
            return true;
        }
    }

    false
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn spi_read_data(ahb_addr: *const u32, read_arr: &mut [u8]) {
    let len = read_arr.len();
    let mut i = 0;

    // Read full u32 words
    while i + 4 <= len {
        let word = core::ptr::read_volatile(ahb_addr.add(i / 4));
        read_arr[i..i + 4].copy_from_slice(&word.to_le_bytes()); // adjust for BE if needed
        i += 4;
    }

    // Remaining bytes
    while i < len {
        read_arr[i] = core::ptr::read_volatile((ahb_addr.cast::<u8>()).add(i));
        i += 1;
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn spi_write_data(ahb_addr: *mut u32, write_arr: &[u8]) {
    if write_arr.is_empty() {
        return;
    }

    let len = write_arr.len();
    let mut i = 0;

    // Write in u32 words as long as possible
    while i + 4 <= len {
        let word = u32::from_le_bytes([
            write_arr[i],
            write_arr[i + 1],
            write_arr[i + 2],
            write_arr[i + 3],
        ]);
        core::ptr::write_volatile(ahb_addr.add(i / 4), word);
        i += 4;
    }

    // Write remaining bytes (if any)
    let ahb_addr_u8 = ahb_addr.cast::<u8>();
    while i < len {
        core::ptr::write_volatile(ahb_addr_u8.add(i), write_arr[i]);
        i += 1;
    }
}
pub static mut GPIO_ORI_VAL: [u32; 4] = [0; 4];
fn get_gpio_ori_val() -> [u32; 4] {
    unsafe { GPIO_ORI_VAL }
}
const PIN_SPIM0_CLK_OUT_BIT: u32 = 7;
const PIN_SPIM1_CLK_OUT_BIT: u32 = 21;
const PIN_SPIM2_CLK_OUT_BIT: u32 = 3;
const PIN_SPIM3_CLK_OUT_BIT: u32 = 17;

pub fn spim_proprietary_pre_config() {
    let scu = unsafe { &*ast1060_pac::Scu::ptr() };
    let gpio = unsafe { &*ast1060_pac::Gpio::ptr() };

    // If no SPIM in use, return
    #[allow(clippy::verbose_bit_mask)]
    if scu.scu0f0().read().bits() & 0x7 == 0 {
        return;
    }

    let spim_idx = (scu.scu0f0().read().bits() & 0x7) - 1;
    if spim_idx > 3 {
        return;
    }
    let clear = true;
    for (idx, ori_val) in get_gpio_ori_val().iter_mut().enumerate() {
        if u32::try_from(idx).unwrap() == spim_idx {
            continue;
        }

        match idx {
            0 => {
                modify_reg!(scu.scu690(), PIN_SPIM0_CLK_OUT_BIT, clear);
                *ori_val = gpio.gpio004().read().bits();
                modify_reg!(gpio.gpio004(), PIN_SPIM0_CLK_OUT_BIT, clear);
            }
            1 => {
                modify_reg!(scu.scu690(), PIN_SPIM1_CLK_OUT_BIT, clear);
                *ori_val = gpio.gpio004().read().bits();
                modify_reg!(gpio.gpio004(), PIN_SPIM1_CLK_OUT_BIT, clear);
            }
            2 => {
                modify_reg!(scu.scu694(), PIN_SPIM2_CLK_OUT_BIT, clear);
                *ori_val = gpio.gpio024().read().bits();
                modify_reg!(gpio.gpio024(), PIN_SPIM2_CLK_OUT_BIT, clear);
            }
            3 => {
                modify_reg!(scu.scu694(), PIN_SPIM3_CLK_OUT_BIT, clear);
                *ori_val = gpio.gpio024().read().bits();
                modify_reg!(gpio.gpio024(), PIN_SPIM3_CLK_OUT_BIT, clear);
            }
            _ => (),
        }
    }
}

pub fn spim_proprietary_post_config() {
    let scu = unsafe { &*ast1060_pac::Scu::ptr() };
    let gpio = unsafe { &*ast1060_pac::Gpio::ptr() };

    // If no SPIM in use, return
    let bits = scu.scu0f0().read().bits();
    if bits.trailing_zeros() >= 3 {
        return;
    }

    let spim_idx = (scu.scu0f0().read().bits() & 0x7) - 1;
    if spim_idx > 3 {
        return;
    }
    let clear = false;
    for (idx, ori_val) in get_gpio_ori_val().iter().copied().enumerate() {
        if u32::try_from(idx).unwrap() == spim_idx {
            continue;
        }

        match idx {
            0 => {
                gpio.gpio004().modify(|r, w| unsafe {
                    let mut current = r.bits();
                    current &= !(1 << PIN_SPIM0_CLK_OUT_BIT);
                    current |= ori_val;
                    w.bits(current)
                });
                modify_reg!(scu.scu690(), PIN_SPIM0_CLK_OUT_BIT, clear);
            }
            1 => {
                gpio.gpio004().modify(|r, w| unsafe {
                    let mut current = r.bits();
                    current &= !(1 << PIN_SPIM1_CLK_OUT_BIT);
                    current |= ori_val;
                    w.bits(current)
                });
                modify_reg!(gpio.gpio004(), PIN_SPIM1_CLK_OUT_BIT, clear);
            }
            2 => {
                gpio.gpio024().modify(|r, w| unsafe {
                    let mut current = r.bits();
                    current &= !(1 << PIN_SPIM2_CLK_OUT_BIT);
                    current |= ori_val;
                    w.bits(current)
                });
                modify_reg!(scu.scu694(), PIN_SPIM2_CLK_OUT_BIT, clear);
            }
            3 => {
                gpio.gpio024().modify(|r, w| unsafe {
                    let mut current = r.bits();
                    current &= !(1 << PIN_SPIM3_CLK_OUT_BIT);
                    current |= ori_val;
                    w.bits(current)
                });
                modify_reg!(scu.scu694(), PIN_SPIM3_CLK_OUT_BIT, clear);
            }
            _ => (),
        }
    }
}
