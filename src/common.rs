// Licensed under the Apache-2.0 license

use crate::uart_core::UartController;
use core::ops::{Index, IndexMut};
use embedded_io::Write;

pub struct DummyDelay;

impl embedded_hal::delay::DelayNs for DummyDelay {
    fn delay_ns(&mut self, ns: u32) {
        for _ in 0..(ns / 100) {
            cortex_m::asm::nop();
        }
    }
}

#[repr(align(32))]
pub struct DmaBuffer<const N: usize> {
    pub buf: [u8; N],
}

impl<const N: usize> Default for DmaBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> DmaBuffer<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self { buf: [0; N] }
    }

    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }

    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        N
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    #[must_use]
    pub fn as_slice(&self, start: usize, end: usize) -> &[u8] {
        &self.buf[start..end]
    }

    pub fn as_mut_slice(&mut self, start: usize, end: usize) -> &mut [u8] {
        &mut self.buf[start..end]
    }
}

impl<const N: usize> Index<usize> for DmaBuffer<N> {
    type Output = u8;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.buf[idx]
    }
}

impl<const N: usize> IndexMut<usize> for DmaBuffer<N> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.buf[idx]
    }
}

pub trait Logger {
    fn debug(&mut self, msg: &str);
    fn error(&mut self, msg: &str);
}

// No-op implementation for production builds
pub struct NoOpLogger;
impl Logger for NoOpLogger {
    fn debug(&mut self, _msg: &str) {}
    fn error(&mut self, _msg: &str) {}
}

// UART logger adapter (separate concern)
pub struct UartLogger<'a> {
    uart: &'a mut UartController<'a>,
}

impl<'a> UartLogger<'a> {
    pub fn new(uart: &'a mut UartController<'a>) -> Self {
        UartLogger { uart }
    }
}

impl<'a> Logger for UartLogger<'a> {
    fn debug(&mut self, msg: &str) {
        writeln!(self.uart, "{msg}").ok();
        write!(self.uart, "\r").ok();
    }
    fn error(&mut self, msg: &str) {
        writeln!(self.uart, "ERROR: {msg}").ok();
        write!(self.uart, "\r").ok();
    }
}
