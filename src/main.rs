// Licensed under the Apache-2.0 license

#![no_std]
#![no_main]

use core::sync::atomic::AtomicBool;
// use core::arch::asm;
use aspeed_ddk::uart_core::{UartConfig, UartController};
use aspeed_ddk::watchdog::WdtController;
use ast1060_pac::Peripherals;
use ast1060_pac::{Wdt, Wdt1};

use aspeed_ddk::ecdsa::AspeedEcdsa;
use aspeed_ddk::hace_controller::HaceController;
use aspeed_ddk::rsa::AspeedRsa;
use aspeed_ddk::spi;
use aspeed_ddk::syscon::{ClockId, ResetId, SysCon};
use fugit::MillisDurationU32 as MilliSeconds;

use aspeed_ddk::tests::functional::ecdsa_test::run_ecdsa_tests;
use aspeed_ddk::tests::functional::gpio_test;
use aspeed_ddk::tests::functional::hash_test::run_hash_tests;
use aspeed_ddk::tests::functional::hmac_test::run_hmac_tests;
use aspeed_ddk::tests::functional::i2c_core_test::run_i2c_core_tests;
use aspeed_ddk::tests::functional::i2c_master_slave_test::run_master_slave_tests;
use aspeed_ddk::tests::functional::i2c_test;
use aspeed_ddk::tests::functional::rsa_test::run_rsa_tests;
use aspeed_ddk::tests::functional::timer_test::run_timer_tests;
use panic_halt as _;

// Import owned API traits and types
use aspeed_ddk::hash_owned::{Sha2_256, Sha2_384, Sha2_512};
use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

use proposed_traits::system_control::ResetControl;

use core::ptr::{read_volatile, write_volatile};
use cortex_m_rt::entry;
use cortex_m_rt::pre_init;
use embedded_hal::delay::DelayNs;
use embedded_io::Write;

#[pre_init]
unsafe fn pre_init() {
    let jtag_pinmux_offset: u32 = 0x7e6e_2000 + 0x41c;
    let mut reg: u32;
    reg = read_volatile(jtag_pinmux_offset as *const u32);
    reg |= 0x1f << 25;
    write_volatile(jtag_pinmux_offset as *mut u32, reg);

    // Disable Cache
    let cache_ctrl_offset: u32 = 0x7e6e_2a58;
    write_volatile(cache_ctrl_offset as *mut u32, 0);

    // Configure Cache Area and Invalidation
    let cache_area_offset: u32 = 0x7e6e_2a50;
    let cache_val = 0x000f_ffff;
    write_volatile(cache_area_offset as *mut u32, cache_val);

    let cache_inval_offset: u32 = 0x7e6e_2a54;
    let cache_inval_val = 0x8660_0000;
    write_volatile(cache_inval_offset as *mut u32, cache_inval_val);

    // Enable Cache
    write_volatile(cache_ctrl_offset as *mut u32, 1);
}

#[derive(Clone, Default)]
struct DummyDelay;

impl DelayNs for DummyDelay {
    fn delay_ns(&mut self, ns: u32) {
        for _ in 0..ns {
            cortex_m::asm::nop();
        }
    }
}

fn test_wdt(uart: &mut UartController<'_>) {
    //instantiates the controller for the hardware watchdog Wdt and Wdt1
    let mut wdt0 = WdtController::<Wdt>::new();
    let mut wdt1 = WdtController::<Wdt1>::new();
    let mut delay = DummyDelay {};

    // Start watchdog with a timeout of 2000 milliseconds (2 seconds)
    uart.write_all(b"\r\nstart wdt\r\n").unwrap();
    wdt0.start(MilliSeconds::millis(5000));
    wdt1.start(MilliSeconds::millis(10000));
    let mut cnt = 0;

    loop {
        delay.delay_ns(2_000_000);
        uart.write_all(b"wdt feed\r\n").unwrap();
        wdt0.feed(); // petting to prevent reset
        wdt1.feed(); // petting to prevent reset
        cnt += 1;
        if cnt > 30 {
            wdt0.stop();
            wdt1.stop();
            uart.write_all(b"stop wdt\r\n").unwrap();
            break;
        }
    }
}

#[no_mangle]
pub static HALT: AtomicBool = AtomicBool::new(true);

#[macro_export]
macro_rules! debug_halt {
    () => {{
        use core::arch::asm;
        use core::sync::atomic::{AtomicBool, Ordering};

        static HALT: AtomicBool = AtomicBool::new(true);

        while HALT.load(Ordering::SeqCst) {
            unsafe {
                asm!("nop");
            }
        }
    }};
}

/// Test the owned digest API demonstrating move-based resource management
fn test_owned_digest_api(uart: &mut UartController<'_>) {
    writeln!(uart, "\r\nRunning owned digest API tests...\r\n").unwrap();

    // Get a fresh HACE peripheral for owned API testing
    let peripherals = unsafe { Peripherals::steal() };
    let hace = peripherals.hace;

    // Test SHA256 with owned API
    writeln!(uart, "Testing owned SHA256 API...").unwrap();
    test_owned_sha256(uart, hace);

    // Get fresh peripheral for SHA384 test (since it was consumed)
    let peripherals = unsafe { Peripherals::steal() };
    let hace = peripherals.hace;

    writeln!(uart, "Testing owned SHA384 API...").unwrap();
    test_owned_sha384(uart, hace);

    // Get fresh peripheral for SHA512 test
    let peripherals = unsafe { Peripherals::steal() };
    let hace = peripherals.hace;

    writeln!(uart, "Testing owned SHA512 API...").unwrap();
    test_owned_sha512(uart, hace);

    writeln!(uart, "All owned digest API tests completed!\r\n").unwrap();
}

/// Validate digest against known test vector
fn validate_digest(
    actual: &[u32],
    expected: &[u8],
    algorithm: &str,
    uart: &mut UartController<'_>,
) -> bool {
    // Convert actual u32 array to bytes in big-endian format and compare
    let mut matches = true;
    let mut byte_index = 0;

    for &word in actual {
        let word_bytes = word.to_be_bytes();
        for &byte in &word_bytes {
            if byte_index < expected.len() {
                if byte != expected[byte_index] {
                    matches = false;
                    break;
                }
                byte_index += 1;
            }
        }
        if !matches || byte_index >= expected.len() {
            break;
        }
    }

    if matches && byte_index == expected.len() {
        writeln!(uart, "{algorithm} test vector validation: PASSED ✅").unwrap();
        true
    } else {
        writeln!(uart, "{algorithm} test vector validation: FAILED ❌").unwrap();
        write!(uart, "Expected: ").unwrap();
        for &byte in expected {
            write!(uart, "{byte:02x}").unwrap();
        }
        writeln!(uart).unwrap();
        write!(uart, "Actual:   ").unwrap();
        let mut byte_index = 0;
        for &word in actual {
            let word_bytes = word.to_be_bytes();
            for &byte in &word_bytes {
                if byte_index < expected.len() {
                    write!(uart, "{byte:02x}").unwrap();
                    byte_index += 1;
                }
            }
        }
        writeln!(uart).unwrap();
        false
    }
}

/// Test owned SHA256 API demonstrating move semantics
fn test_owned_sha256(uart: &mut UartController<'_>, hace: ast1060_pac::Hace) {
    let controller = HaceController::new(hace);

    // Initialize digest context - controller wrapper is moved
    let context = controller.init(Sha2_256).unwrap();

    // Test with known test vector: "abc" -> SHA256
    // Expected: ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let context = context.update(b"abc").unwrap();

    // Finalize and get both digest and controller back
    let (digest, _recovered_controller) = context.finalize().unwrap();

    writeln!(uart, "SHA256 owned API digest: {:02x?}", &digest.value[..8]).unwrap();

    // Known test vector for "abc"
    let expected_sha256 = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];

    if validate_digest(&digest.value, &expected_sha256, "SHA256", uart) {
        writeln!(uart, "SHA256 owned API: PASSED ✅").unwrap();
    } else {
        writeln!(uart, "SHA256 owned API: FAILED ❌").unwrap();
    }
}

/// Test owned SHA384 API demonstrating controller recovery
fn test_owned_sha384(uart: &mut UartController<'_>, hace: ast1060_pac::Hace) {
    let controller = HaceController::new(hace);

    // Test with known test vector: "abc" -> SHA384
    // Expected: cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7
    let context = controller.init(Sha2_384).unwrap();
    let context = context.update(b"abc").unwrap();

    let (digest, recovered_controller) = context.finalize().unwrap();

    writeln!(uart, "SHA384 owned API digest: {:02x?}", &digest.value[..8]).unwrap();

    // Known test vector for "abc"
    let expected_sha384 = [
        0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50,
        0x07, 0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff,
        0x5b, 0xed, 0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34,
        0xc8, 0x25, 0xa7,
    ];

    if validate_digest(&digest.value, &expected_sha384, "SHA384", uart) {
        writeln!(uart, "SHA384 owned API: PASSED ✅").unwrap();
    } else {
        writeln!(uart, "SHA384 owned API: FAILED ❌").unwrap();
    }

    // Demonstrate controller recovery by using it again
    let context2 = recovered_controller.init(Sha2_384).unwrap();
    let context2 = context2.update(b"Reused controller").unwrap();
    let (_digest2, _final_controller) = context2.finalize().unwrap();

    writeln!(uart, "Controller recovery: PASSED ✅").unwrap();
}

/// Test owned SHA512 API demonstrating cancellation
fn test_owned_sha512(uart: &mut UartController<'_>, hace: ast1060_pac::Hace) {
    let controller = HaceController::new(hace);

    let context = controller.init(Sha2_512).unwrap();
    let context = context.update(b"This will be").unwrap();
    let context = context.update(b" cancelled").unwrap();

    // Demonstrate cancellation - returns controller without computing digest
    let recovered_controller = context.cancel();

    writeln!(uart, "SHA512 cancellation: PASSED ✅").unwrap();

    // Use recovered controller for actual computation with known test vector
    // Test with known test vector: "abc" -> SHA512
    // Expected: ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f
    let context = recovered_controller.init(Sha2_512).unwrap();
    let context = context.update(b"abc").unwrap();

    let (digest, _final_controller) = context.finalize().unwrap();

    writeln!(uart, "SHA512 owned API digest: {:02x?}", &digest.value[..8]).unwrap();

    // Known test vector for "abc"
    let expected_sha512 = [
        0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41,
        0x31, 0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55,
        0xd3, 0x9a, 0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3,
        0xfe, 0xeb, 0xbd, 0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f,
        0xa5, 0x4c, 0xa4, 0x9f,
    ];

    if validate_digest(&digest.value, &expected_sha512, "SHA512", uart) {
        writeln!(uart, "SHA512 owned API: PASSED ✅").unwrap();
    } else {
        writeln!(uart, "SHA512 owned API: FAILED ❌").unwrap();
    }
}

#[entry]
fn main() -> ! {
    let peripherals = unsafe { Peripherals::steal() };
    let uart = peripherals.uart;
    let mut delay = DummyDelay;

    // For jlink attach
    // set aspeed_ddk::__cortex_m_rt_main::HALT.v.value = 0 in gdb
    // debug_halt!();
    // Get UART register block and create controller
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut uart_controller = UartController::new(uart_regs);
    uart_controller.init(&UartConfig::default()).unwrap();

    let hace = peripherals.hace;
    let scu = peripherals.scu;
    let secure = peripherals.secure;

    writeln!(uart_controller, "\r\nHello, world!!\r\n").unwrap();

    let delay = DummyDelay;
    let mut syscon = SysCon::new(delay.clone(), scu);

    // Enable HACE (Hash and Crypto Engine)
    let _ = syscon.enable_clock(ClockId::ClkYCLK as u8);
    let reset_id = ResetId::RstHACE;
    let _ = syscon.reset_deassert(&reset_id);

    let mut hace_controller = HaceController::new(hace);

    run_hash_tests(&mut uart_controller, &mut hace_controller);

    run_hmac_tests(&mut uart_controller, &mut hace_controller);

    // Test the owned digest API
    test_owned_digest_api(&mut uart_controller);

    // Enable RSA and ECC
    let _ = syscon.enable_clock(ClockId::ClkRSACLK as u8);

    let mut ecdsa = AspeedEcdsa::new(&secure, delay.clone());
    run_ecdsa_tests(&mut uart_controller, &mut ecdsa);

    let mut rsa = AspeedRsa::new(&secure, delay);
    run_rsa_tests(&mut uart_controller, &mut rsa);
    gpio_test::test_gpioa(&mut uart_controller);
    i2c_test::test_i2c_master(&mut uart_controller);
    #[cfg(feature = "i2c_target")]
    i2c_test::test_i2c_slave(&mut uart_controller);

    // Run i2c_core functional tests
    run_i2c_core_tests(&mut uart_controller);

    // Run I2C master-slave hardware integration tests
    run_master_slave_tests(&mut uart_controller);

    test_wdt(&mut uart_controller);
    run_timer_tests(&mut uart_controller);

    let test_spicontroller = false;
    if test_spicontroller {
        spi::spitest::test_fmc(&mut uart_controller);
        spi::spitest::test_spi(&mut uart_controller);

        gpio_test::test_gpio_flash_power(&mut uart_controller);
        spi::spitest::test_spi2(&mut uart_controller);
    }
    // Initialize the peripherals here if needed
    loop {
        cortex_m::asm::wfi();
    }
}
