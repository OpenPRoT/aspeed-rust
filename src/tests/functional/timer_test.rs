// Licensed under the Apache-2.0 license

use crate::timer::{TimerController, TimerType};
use crate::uart_core::UartController;
use ast1060_pac::Timer;
use cortex_m::peripheral::NVIC;
use embedded_hal_old::timer::CountDown;
use fugit::MicrosDurationU32;

use embedded_io::Write;

static mut UART_PTR: Option<&'static mut UartController<'static>> = None;
static mut TIMER_INSTANCE: Option<TimerController<Timer>> = None;

#[no_mangle]
pub extern "C" fn timer() {
    unsafe {
        if let Some(uart) = UART_PTR.as_mut() {
            let _ = uart.write_all(b"[ISR] Timer\r\n");
        }
        if let Some(timer) = TIMER_INSTANCE.as_mut() {
            let () = timer.handle_interrupt();
        }
    }
}

pub fn test_timer_isr(uart: &mut UartController<'_>) {
    let mut timer = TimerController::<Timer>::new(50); // tick_per_us
    timer.set_callback(Some(timer_callback), TimerType::Periodic);
    timer.try_start(MicrosDurationU32::millis(1000)).unwrap();

    unsafe {
        UART_PTR = Some(core::mem::transmute::<
            &mut UartController<'_>,
            &'static mut UartController<'static>,
        >(uart));

        TIMER_INSTANCE = Some(timer);
        NVIC::unmask(ast1060_pac::Interrupt::timer);
    }
}

fn timer_callback() {
    unsafe {
        if let Some(uart) = UART_PTR.as_mut() {
            let _ = uart.write_all(b"[CB] Triggered!\r\n");
        }
    }
}

pub fn run_timer_tests(uart: &mut UartController) {
    writeln!(uart, "\r\nRunning Timer ISR test").unwrap();
    test_timer_isr(uart);
}
