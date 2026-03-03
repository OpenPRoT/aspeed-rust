// Licensed under the Apache-2.0 license

use core::fmt;
use core::marker::PhantomData;
use embedded_hal_old::timer::{Cancel, CountDown, Periodic};
use fugit::MicrosDurationU32 as MicroSeconds;

/// Timer type: One-shot or periodic
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerType {
    OneShot,
    Periodic,
}

/// Timer error type
#[derive(Debug)]
pub enum TimerError {
    TimeoutTooLarge,
    InvalidConfig,
}

const MAX_TIMEOUT_MS: u32 = 4_294_967;
const MATCH_DISABLE: u32 = 0xffff_ffff;

/// Trait to abstract timer register base + index
pub trait TimerInstance {
    fn cr() -> &'static ast1060_pac::timer::RegisterBlock;
    fn gr() -> &'static ast1060_pac::timerg::RegisterBlock;
    fn index() -> usize;
}

/// Timer0 instance (only one currently supported)
impl TimerInstance for ast1060_pac::Timer {
    fn cr() -> &'static ast1060_pac::timer::RegisterBlock {
        unsafe { &*ast1060_pac::Timer::ptr() }
    }

    fn gr() -> &'static ast1060_pac::timerg::RegisterBlock {
        unsafe { &*ast1060_pac::Timerg::ptr() }
    }

    fn index() -> usize {
        0
    }
}

/// Timer controller
pub struct TimerController<T: TimerInstance> {
    cr: &'static ast1060_pac::timer::RegisterBlock,
    gr: &'static ast1060_pac::timerg::RegisterBlock,
    tick_per_us: u32,
    callback: Option<fn()>,
    auto_reload: TimerType,
    _marker: PhantomData<T>,
}

impl<T: TimerInstance> fmt::Debug for TimerController<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TimerController")
    }
}

impl<T: TimerInstance> TimerController<T> {
    /// Create a new timer controller
    #[must_use]
    pub fn new(tick_per_us: u32) -> Self {
        Self {
            cr: T::cr(),
            gr: T::gr(),
            tick_per_us,
            callback: None,
            auto_reload: TimerType::OneShot,
            _marker: PhantomData,
        }
    }

    /// Get current counter value
    #[must_use]
    pub fn counter(&self) -> u32 {
        self.cr.timer000().read().bits()
    }

    /// Stop the timer and clear reload
    pub fn stop(&mut self) {
        let index = T::index();
        self.gr
            .timerg03c()
            .write(|w| unsafe { w.bits(1 << (4 * index)) });
        self.cr.timer004().write(|w| unsafe { w.bits(0) });
    }

    /// Handle timer interrupt (user calls this in IRQ handler)
    pub fn handle_interrupt(&mut self) {
        let index = T::index();
        self.gr.timerg034().write(|w| unsafe { w.bits(1 << index) });

        if self.auto_reload == TimerType::OneShot {
            self.stop();
        }

        if let Some(cb) = self.callback {
            cb();
        }
    }

    pub fn set_callback(&mut self, cb: Option<fn()>, periodic: TimerType) {
        self.callback = cb;
        self.auto_reload = periodic;
    }
}

impl<T: TimerInstance> CountDown for TimerController<T> {
    type Time = MicroSeconds;
    type Error = TimerError;

    fn try_start<Time>(&mut self, count: Time) -> Result<(), Self::Error>
    where
        Time: Into<MicroSeconds>,
    {
        let us = count.into().ticks();

        if u64::from(us) >= u64::from(MAX_TIMEOUT_MS) * 1000 {
            return Err(TimerError::TimeoutTooLarge);
        }

        let reload = us * self.tick_per_us;
        let index = T::index();

        self.gr
            .timerg03c()
            .write(|w| unsafe { w.bits(1 << (4 * index)) });
        self.cr.timer004().write(|w| unsafe { w.bits(reload) });
        self.cr
            .timer008()
            .write(|w| unsafe { w.bits(MATCH_DISABLE) });
        self.cr
            .timer00c()
            .write(|w| unsafe { w.bits(MATCH_DISABLE) });

        let ctrl_val = (1 << (4 * index)) | (1 << (4 * index + 2));
        self.gr.timerg030().write(|w| unsafe { w.bits(ctrl_val) });

        Ok(())
    }

    fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
        let index = T::index();
        let status = self.gr.timerg034().read().bits();

        if (status & (1 << index)) != 0 {
            self.gr.timerg034().write(|w| unsafe { w.bits(1 << index) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<T: TimerInstance> Cancel for TimerController<T> {
    fn try_cancel(&mut self) -> Result<(), Self::Error> {
        self.stop();
        Ok(())
    }
}

impl<T: TimerInstance> Periodic for TimerController<T> {}
