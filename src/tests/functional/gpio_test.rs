// Licensed under the Apache-2.0 license

use ast1060_pac::Peripherals;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use embedded_io::Write;

use crate::common::DummyDelay;
use crate::gpio::{gpioa, gpioh, gpiol, gpiom, Floating, GpioExt};
use crate::pinctrl;
use crate::uart_core::UartController;
use embedded_hal::delay::DelayNs;

pub fn test_gpioa(uart: &mut UartController<'_>) {
    let peripherals = unsafe { Peripherals::steal() };
    let gpio = peripherals.gpio;

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOA0);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOA1);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOA3);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOA4);
    let gpioa = gpioa::GPIOA::new(gpio).split();
    uart.write_all(b"\r\n####### GPIO test #######\r\n")
        .unwrap();
    // input test
    let mut pa0 = gpioa.pa0.into_pull_down_input();
    if pa0.is_low().unwrap() {
        uart.write_all(b"\rGPIOA pin0 is low\r\n").unwrap();
    }
    let mut pa1 = gpioa.pa1.into_pull_up_input();
    if pa1.is_high().unwrap() {
        uart.write_all(b"\rGPIOA pin1 is high\r\n").unwrap();
    }
    // output test
    let mut pa3 = gpioa.pa3.into_open_drain_output::<Floating>();
    pa3.set_low().unwrap();
    if pa3.is_set_low().unwrap() {
        uart.write_all(b"\rGPIOA pin3 set low successfully\r\n")
            .unwrap();
    }
    pa3.set_high().unwrap();
    if pa3.is_set_high().unwrap() {
        uart.write_all(b"\rGPIOA pin3 set high successfully\r\n")
            .unwrap();
    }

    let mut pa4 = gpioa.pa4.into_push_pull_output();
    pa4.set_low().unwrap();
    if pa4.is_set_low().unwrap() {
        uart.write_all(b"\rGPIOA pin4 set low successfully\r\n")
            .unwrap();
    }
    pa4.set_high().unwrap();
    if pa4.is_set_high().unwrap() {
        uart.write_all(b"\rGPIOA pin4 set high successfully\r\n")
            .unwrap();
    }
}

pub fn test_gpio_flash_power(uart: &mut UartController<'_>) {
    let mut delay = DummyDelay {};
    if true {
        /* Older demo board required this */
        let peripherals = unsafe { Peripherals::steal() };
        let gpio = peripherals.gpio;
        let gpiol = gpiol::GPIOL::new(gpio).split();
        uart.write_all(b"\r\n####### GPIO flash power #######\r\n")
            .unwrap();

        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOL2);
        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOL3);
        let mut pl2 = gpiol.pl2.into_push_pull_output();
        pl2.set_high().unwrap();
        uart.write_all(b"\r\nGPIOL2 set high\r\n").unwrap();
        let mut pl3 = gpiol.pl3.into_push_pull_output();
        pl3.set_high().unwrap();
        uart.write_all(b"\r\nGPIOL3 set high\r\n").unwrap();
        delay.delay_ns(1_000_000);
    }
}
#[allow(dead_code)]
pub fn test_gpio_bmc_reset(uart: &mut UartController<'_>) {
    {
        let peripherals = unsafe { Peripherals::steal() };
        let gpio = peripherals.gpio;
        let gpiom = gpiom::GPIOM::new(gpio).split();
        uart.write_all(b"\r\n####### GPIO BMC SRST #######\r\n")
            .unwrap();

        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOM5);
        let mut pm5 = gpiom.pm5.into_push_pull_output();
        pm5.set_low().unwrap();
        if pm5.is_set_low().unwrap() {
            uart.write_all(b"\r\nGPIOM pin5 set low successfully\r\n")
                .unwrap();
        }
        pm5.set_high().unwrap();
        if pm5.is_set_high().unwrap() {
            uart.write_all(b"\r\nGPIOM pin5 set high successfully\r\n")
                .unwrap();
        }
    }

    {
        let peripherals = unsafe { Peripherals::steal() };
        let gpio = peripherals.gpio;
        let gpioh = gpioh::GPIOH::new(gpio).split();
        uart.write_all(b"\r\n####### GPIO BMC EXTRST #######\r\n")
            .unwrap();

        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_GPIOH2);
        let mut ph2 = gpioh.ph2.into_push_pull_output();
        ph2.set_low().unwrap();
        if ph2.is_set_low().unwrap() {
            uart.write_all(b"\r\nGPIOH pin2 set low successfully\r\n")
                .unwrap();
        }
        ph2.set_high().unwrap();
        if ph2.is_set_high().unwrap() {
            uart.write_all(b"\r\nGPIOH pin2 set high successfully\r\n")
                .unwrap();
        }
    }
}
