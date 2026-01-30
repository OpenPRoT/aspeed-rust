// Licensed under the Apache-2.0 license

use crate::hace_controller::HaceController;
use crate::hmac::{IntoHashAlgo, Sha256, Sha384, Sha512};
use crate::uart_core::UartController;
use core::any::TypeId;
use embedded_io::Write;
use proposed_traits::mac::{MacAlgorithm, MacInit, MacOp};

fn print_hex_array(uart: &mut UartController, data: &[u8], bytes_per_line: usize) {
    for (i, b) in data.iter().enumerate() {
        if i % bytes_per_line == 0 {
            writeln!(uart, "\r").unwrap();
        } else {
            write!(uart, " ").unwrap();
        }
        write!(uart, "{b:02x}").unwrap();
    }
    writeln!(uart).unwrap();
}

fn print_input(uart: &mut UartController, algo: &str, input: &[u8]) {
    match core::str::from_utf8(input) {
        Ok(ascii) => {
            write!(uart, "\r\n{algo} of \"{ascii}\" [").unwrap();
        }
        Err(_) => {
            write!(uart, "\r\n{algo} of [").unwrap();
        }
    }

    for (i, b) in input.iter().enumerate() {
        if i > 0 {
            write!(uart, ", ").unwrap();
        }
        write!(uart, "0x{b:02x}").unwrap();
    }
    writeln!(uart, "]:").unwrap();
}

pub fn run_hmac_tests(uart: &mut UartController, hace: &mut HaceController) {
    let key256 = [0xb; 32];
    let key384 = [0xb; 48];
    let key512 = [0xb; 64];
    let message = *b"The quick brown fox jumps over the lazy dog";

    writeln!(uart, "\r\nRunning HMAC tests...").unwrap();
    run_hmac::<Sha256>(uart, hace, &key256, &message);
    run_hmac::<Sha384>(uart, hace, &key384, &message);
    run_hmac::<Sha512>(uart, hace, &key512, &message);
}

fn run_hmac<A>(uart: &mut UartController, ctrl: &mut HaceController, key: &A::Key, input: &[u8])
where
    A: MacAlgorithm + IntoHashAlgo + Default + 'static,
    A::MacOutput: Default + AsRef<[u8]> + AsMut<[u8]>,
    A::Key: AsRef<[u8]>,
{
    let mut ctx = ctrl.init(A::default(), key).unwrap();
    ctx.update(input).unwrap();
    let output = ctx.finalize().unwrap();

    print_input(uart, core::any::type_name::<A>(), input);
    write!(uart, "\r\nKey: ").unwrap();
    print_hex_array(uart, key.as_ref(), 16);
    write!(uart, "\r\nOutput: ").unwrap();
    print_hex_array(uart, output.as_ref(), 16);

    let expected = if TypeId::of::<A>() == TypeId::of::<Sha256>() {
        Some(
            &[
                0xde, 0x60, 0xb1, 0xd4, 0x83, 0xd2, 0x00, 0x11, 0xf1, 0xb4, 0x2f, 0x33, 0x70, 0x0c,
                0xb4, 0x4f, 0xa3, 0x16, 0xc4, 0x43, 0xce, 0x43, 0x03, 0x78, 0xcb, 0x5d, 0x65, 0x42,
                0x7f, 0x64, 0x34, 0x8d,
            ][..],
        )
    } else if TypeId::of::<A>() == TypeId::of::<Sha384>() {
        Some(
            &[
                0xCC, 0xD7, 0xCA, 0xE4, 0x59, 0xE4, 0x23, 0xB9, 0x94, 0x06, 0x9A, 0xC1, 0xD3, 0xF8,
                0x26, 0x41, 0x60, 0x88, 0x85, 0xAF, 0x04, 0x3A, 0x33, 0xAD, 0x51, 0x3E, 0x8E, 0x87,
                0x47, 0x19, 0x66, 0x6C, 0x24, 0xD1, 0xC0, 0xDC, 0x54, 0xAE, 0xBB, 0xB9, 0x3B, 0x84,
                0x34, 0xA1, 0xA9, 0x70, 0x83, 0x6D,
            ][..],
        )
    } else if TypeId::of::<A>() == TypeId::of::<Sha512>() {
        Some(
            &[
                0xf1, 0x10, 0x1d, 0xe2, 0xb4, 0xfa, 0x09, 0xe6, 0xfb, 0x04, 0x9d, 0x2e, 0x12, 0x68,
                0xdb, 0xe7, 0x65, 0x09, 0x22, 0x01, 0x75, 0x1a, 0x42, 0x1c, 0xbc, 0x80, 0x61, 0x09,
                0x36, 0x14, 0x35, 0x81, 0x22, 0x0f, 0x3a, 0x77, 0x1f, 0xff, 0x10, 0xd8, 0xd8, 0x16,
                0x63, 0x19, 0xb6, 0x0d, 0xf0, 0x98, 0xb3, 0x70, 0xdc, 0x5d, 0x1b, 0x01, 0x71, 0x4d,
                0x97, 0x87, 0x12, 0x72, 0x24, 0x60, 0x67, 0x4f,
            ][..],
        )
    } else {
        None
    };

    if let Some(expected) = expected {
        if output.as_ref() == expected {
            writeln!(uart, "\r\n{}: Test passed!", core::any::type_name::<A>()).unwrap();
        } else {
            writeln!(uart, "\r\n{}: Test failed!", core::any::type_name::<A>()).unwrap();
            writeln!(uart, "Expected:").unwrap();
            print_hex_array(uart, expected, 16);
            writeln!(uart, "Got:").unwrap();
            print_hex_array(uart, output.as_ref(), 16);
        }
    } else {
        writeln!(
            uart,
            "\r\n{}: No expected value defined.",
            core::any::type_name::<A>()
        )
        .unwrap();
    }
}
