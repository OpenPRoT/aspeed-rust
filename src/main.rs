// Licensed under the Apache-2.0 license

#![no_std]
#![no_main]

use core::sync::atomic::AtomicBool;
// use core::arch::asm;
use aspeed_ddk::uart::{Config, UartController};
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
use aspeed_ddk::tests::functional::i2c_test;
use aspeed_ddk::tests::functional::rsa_test::run_rsa_tests;

#[cfg(feature = "i2c_target")]
use aspeed_ddk::tests::functional::swmbx_test;
use aspeed_ddk::tests::functional::timer_test::run_timer_tests;

use panic_halt as _;

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

#[entry]
fn main() -> ! {
    let peripherals = unsafe { Peripherals::steal() };
    let uart = peripherals.uart;
    let mut delay = DummyDelay;

    // For jlink attach
    // set aspeed_ddk::__cortex_m_rt_main::HALT.v.value = 0 in gdb
    // debug_halt!();
    let mut uart_controller = UartController::new(uart, &mut delay);
    unsafe {
        uart_controller.init(&Config {
            baud_rate: 115_200,
            word_length: aspeed_ddk::uart::WordLength::Eight as u8,
            parity: aspeed_ddk::uart::Parity::None,
            stop_bits: aspeed_ddk::uart::StopBits::One,
            clock: 24_000_000,
        });
    }

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

    let mut hace_controller = HaceController::new(&hace);

    run_hash_tests(&mut uart_controller, &mut hace_controller);

    run_hmac_tests(&mut uart_controller, &mut hace_controller);

    // Enable RSA and ECC
    let _ = syscon.enable_clock(ClockId::ClkRSACLK as u8);

    let mut ecdsa = AspeedEcdsa::new(&secure, delay.clone());
    run_ecdsa_tests(&mut uart_controller, &mut ecdsa);

    let mut rsa = AspeedRsa::new(&secure, delay);
    run_rsa_tests(&mut uart_controller, &mut rsa);
    // gpio_test::test_gpioa(&mut uart_controller);
    i2c_test::test_i2c_master(&mut uart_controller);

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

    // Start SWMBX test
    #[cfg(feature = "i2c_target")]
    swmbx_test::test_swmbx(&mut uart_controller);

    loop {
        cortex_m::asm::wfi();
    }
}
