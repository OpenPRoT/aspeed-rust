// Licensed under the Apache-2.0 license

use crate::uart_core::UartController;
use embedded_io::Write;

pub fn print_array_u32(uart: &mut UartController<'_>, data: &[u32]) {
    let bytes_per_line = 0x4;
    for (i, dw) in data.iter().enumerate() {
        if i % bytes_per_line == 0 {
            writeln!(uart, "\r").unwrap();
        } else {
            write!(uart, " ").unwrap();
        }
        write!(uart, "{dw:08x}").unwrap();
    }
    writeln!(uart, "\r").unwrap();
}

pub fn print_array_u8(uart: &mut UartController<'_>, data: &[u8]) {
    let bytes_per_line = 0x8;
    for (i, b) in data.iter().enumerate() {
        if i % bytes_per_line == 0 {
            writeln!(uart, "\r").unwrap();
        } else {
            write!(uart, " ").unwrap();
        }
        write!(uart, "{b:02x}").unwrap();
    }
    writeln!(uart, "\r").unwrap();
}

pub fn print_reg_u8(uart: &mut UartController<'_>, reg_base: usize, size: usize) {
    let bytes_per_line = 0x8;
    let scu_bytes: &[u8] = unsafe { core::slice::from_raw_parts(reg_base as *const u8, size) };

    for (i, b) in scu_bytes.iter().enumerate() {
        if i % bytes_per_line == 0 {
            writeln!(uart, "\r").unwrap();
        } else {
            write!(uart, ", ").unwrap();
        }
        write!(uart, "0x{b:02x}").unwrap();
    }
    writeln!(uart, "\r").unwrap();
}

pub fn print_reg_u32(uart: &mut UartController<'_>, reg_base: usize, size: usize) {
    let words_per_line = 4; // 4 u32 values per line
    let reg_words: &[u32] =
        unsafe { core::slice::from_raw_parts(reg_base as *const u32, size / 4) };

    for (i, word) in reg_words.iter().enumerate() {
        if i % words_per_line == 0 {
            writeln!(uart, "\r\n[{:08x}]:", reg_base + i * 4).unwrap(); // Print base address
        } else {
            write!(uart, ", ").unwrap();
        }
        write!(uart, "0x{word:08x}").unwrap();
    }

    writeln!(uart, "\r").unwrap();
}
