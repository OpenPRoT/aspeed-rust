// Licensed under the Apache-2.0 license

use crate::modify_reg;
use core::cell::Cell;
use critical_section::Mutex;

use super::consts::{
    PIN_SPIM0_CLK_OUT_BIT, PIN_SPIM1_CLK_OUT_BIT, PIN_SPIM2_CLK_OUT_BIT, PIN_SPIM3_CLK_OUT_BIT,
};

static GPIO_ORI_VAL: Mutex<[Cell<u32>; 4]> = Mutex::new([const { Cell::new(0) }; 4]);

#[must_use]
pub fn get_gpio_ori_val() -> [u32; 4] {
    critical_section::with(|crit| {
        let v = GPIO_ORI_VAL.borrow(crit);
        [v[0].get(), v[1].get(), v[2].get(), v[3].get()]
    })
}

pub fn spim_scu_ctrl_set(mask: u32, val: u32) {
    let scu = unsafe { &*ast1060_pac::Scu::ptr() };
    let mut reg_val = scu.scu0f0().read().bits();
    reg_val &= !mask;
    reg_val |= val;
    scu.scu0f0().write(|w| unsafe { w.bits(reg_val) });
}

pub fn spim_scu_ctrl_clear(clear_bits: u32) {
    let scu = unsafe { &*ast1060_pac::Scu::ptr() };
    let mut reg_val = scu.scu0f0().read().bits();
    reg_val &= !clear_bits;
    scu.scu0f0().write(|w| unsafe { w.bits(reg_val) });
}

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
