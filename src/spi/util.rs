// Licensed under the Apache-2.0 license

use crate::{
    spi::norflash::{Jesd216Mode},
};
use ast1060_pac::Scu;
use super::consts::HPLL_FREQ;

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
    if bus_width == 0 || bus_width > 8 {
        return 0;
    }
    let bits_per_cycle = 8 / bus_width;
    if bits_per_cycle == 0 {
        return 0;
    }
    let dummy_byte = dummy_cycle / bits_per_cycle;
    ((dummy_byte & 0x3) << 6) | (((dummy_byte & 0x4) >> 2) << 14)
}

pub const fn get_cmd_buswidth(v: u32) -> u8 {
    ((v & 0x0000_0F00) >> 8) as u8
}
pub const fn get_addr_buswidth(v: u32) -> u8 {
    ((v & 0x0000_00F0) >> 4) as u8
}
pub const fn get_data_buswidth(v: u32) -> u8 {
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
