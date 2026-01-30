// Licensed under the Apache-2.0 license

use crate::hace_controller::HaceController;
use crate::hash::{IntoHashAlgo, Sha256, Sha384, Sha512};
use crate::uart_core::UartController;
use core::any::TypeId;
use embedded_io::Write;
use proposed_traits::digest::{DigestAlgorithm, DigestInit, DigestOp};

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

pub fn run_hash_tests(uart: &mut UartController, hace: &mut HaceController) {
    let input = *b"hello_world";

    run_hash::<Sha256>(uart, hace, &input);
    run_hash::<Sha384>(uart, hace, &input);
    run_hash::<Sha512>(uart, hace, &input);
}

fn run_hash<A>(uart: &mut UartController, ctrl: &mut HaceController, input: &[u8])
where
    A: DigestAlgorithm + IntoHashAlgo + Default + 'static,
    A::DigestOutput: Default + AsRef<[u8]> + AsMut<[u8]>,
{
    let mut ctx = ctrl.init(A::default()).unwrap();
    ctx.update(input).unwrap();
    let output = ctx.finalize().unwrap();

    print_input(uart, core::any::type_name::<A>(), input);
    writeln!(uart, "\r\nOutput:").unwrap();
    print_hex_array(uart, output.as_ref(), 16);

    let expected = if TypeId::of::<A>() == TypeId::of::<Sha256>() {
        Some(
            &[
                // Expected SHA-256 hash of "hello_world"
                0x35, 0x07, 0x2c, 0x1a, 0xe5, 0x46, 0x35, 0x0e, 0x0b, 0xfa, 0x7a, 0xb1, 0x1d, 0x49,
                0xdc, 0x6f, 0x12, 0x9e, 0x72, 0xcc, 0xd5, 0x7e, 0xc7, 0xeb, 0x67, 0x12, 0x25, 0xbb,
                0xd1, 0x97, 0xc8, 0xf1,
            ][..],
        )
    } else if TypeId::of::<A>() == TypeId::of::<Sha384>() {
        Some(
            &[
                0x7f, 0x25, 0x1a, 0x65, 0xac, 0xbe, 0x92, 0xaf, 0x4c, 0x6a, 0x6d, 0x62, 0x4c, 0x08,
                0x60, 0xd9, 0xbe, 0x77, 0x32, 0x9e, 0x10, 0xe5, 0xbe, 0xb3, 0xb9, 0x59, 0x4f, 0x79,
                0x16, 0x12, 0x8c, 0xd9, 0x56, 0x10, 0xa4, 0xd8, 0x4e, 0x3a, 0x83, 0xa2, 0x4a, 0x72,
                0x36, 0x2f, 0x6c, 0x8f, 0x9c, 0x46,
            ][..],
        )
    } else if TypeId::of::<A>() == TypeId::of::<Sha512>() {
        Some(
            &[
                0x94, 0xf4, 0x27, 0xef, 0xef, 0xa7, 0x4c, 0x12, 0x30, 0xc3, 0xe9, 0x3c, 0x35, 0x10,
                0x4d, 0xcb, 0xaa, 0x8f, 0xf7, 0x1b, 0xa4, 0x53, 0x75, 0x83, 0xed, 0x83, 0xc0, 0x44,
                0x9d, 0x60, 0x7c, 0x4e, 0x61, 0xb3, 0x9c, 0x4c, 0x5e, 0xea, 0x55, 0x43, 0xe0, 0x1d,
                0x76, 0xa6, 0x8e, 0x22, 0x3d, 0xa0, 0x2b, 0x50, 0x05, 0x30, 0xa8, 0x21, 0x56, 0x62,
                0x5c, 0xb9, 0x6e, 0xe8, 0xc8, 0xc8, 0x0a, 0x85,
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
