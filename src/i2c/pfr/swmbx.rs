// Licensed under the Apache-2.0 license

use crate::uart::UartController;
use core::cell::Cell;
use core::mem::MaybeUninit;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, Ordering};
use embedded_io::Write;
use heapless::spsc::Queue;

// Constants
pub const SWMBX_DEV_COUNT: usize = 2;
pub const SWMBX_NODE_COUNT: usize = 256;
pub const SWMBX_FIFO_COUNT: usize = 4;
pub const SWMBX_BUF_BASE: usize = 0x7e7b_0e00;
pub const SWMBX_BUF_SIZE: usize = 256;
pub const SWMBX_FIFO_DEPTH: usize = 256;
pub const SWMBX_INFO_BASE: usize = 0x7e7b_0f00;

// Behavior flags
pub const SWMBX_PROTECT: u8 = 1 << 0;
pub const SWMBX_NOTIFY: u8 = 1 << 1;
pub const SWMBX_FIFO: u8 = 1 << 2;
pub const FLAG_MASK: u8 = SWMBX_PROTECT | SWMBX_NOTIFY | SWMBX_FIFO;

// FIFO notify flags
pub const SWMBX_FIFO_NOTIFY_START: u8 = 1 << 0;
pub const SWMBX_FIFO_NOTIFY_STOP: u8 = 1 << 1;
pub const FIFO_NOTIFY_MASK: u8 = SWMBX_FIFO_NOTIFY_START | SWMBX_FIFO_NOTIFY_STOP;

pub static mut SWMBX_CTRL: MaybeUninit<SwmbxCtrl> = MaybeUninit::uninit();
static INIT_DONE: AtomicBool = AtomicBool::new(false);

extern "Rust" {
    static mut UART_PTR: Option<&'static mut UartController<'static>>;
}

#[macro_export]
macro_rules! swmbx_log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        if let Some(uart) = unsafe { UART_PTR.as_mut() } {
            let mut buf: heapless::String<64> = heapless::String::new();
            let _ = write!(buf, $($arg)*);
            let _ = uart.write_all(b"[SWMBX] ");
            let _ = uart.write_all(buf.as_bytes());
            let _ = uart.write_all(b"\r\n");
        }
    }};
}

// --- FIFO Entry ---
#[derive(Copy, Clone, Debug)]
pub struct SwmbxFifoEntry {
    pub value: u8,
}

// --- Node entry per device+addr ---
#[derive(Copy, Clone, Debug, Default)]
pub struct SwmbxNode {
    pub notify_flag: bool,
    pub enabled_flags: u8, // uses SWMBX_PROTECT / SWMBX_NOTIFY bitmask
}

// --- FIFO buffer per index ---
pub struct SwmbxFifo<const N: usize> {
    pub queue: Queue<SwmbxFifoEntry, N>,
    pub notify_flag: u8, // FIFO_NOTIFY_START/STOP
    pub notify_start: bool,
    pub fifo_write: bool,
    pub fifo_offset: u8,
    pub enabled: bool,
    pub msg_index: usize,
    pub max_msg_count: u8,
}

impl<const N: usize> SwmbxFifo<N> {
    pub const fn new() -> Self {
        Self {
            queue: Queue::new(),
            notify_flag: 0,
            notify_start: false,
            fifo_write: false,
            fifo_offset: 0,
            enabled: false,
            msg_index: 0,
            max_msg_count: SWMBX_FIFO_DEPTH as u8,
        }
    }

    pub fn append_write(&mut self, value: u8) -> Result<(), ()> {
        if self.queue.len() < self.max_msg_count as usize {
            self.queue
                .enqueue(SwmbxFifoEntry { value })
                .map_err(|_| ())?;
            if self.msg_index == (self.max_msg_count - 1) as usize {
                self.msg_index = 0;
            } else {
                self.msg_index = (self.msg_index + 1) % self.max_msg_count as usize;
            }
            swmbx_log!(
                "append_write: value {:#x} added to FIFO, current index: {}",
                value,
                self.msg_index
            );
            Ok(())
        } else {
            swmbx_log!("append_write: FIFO full, cannot add value {:#x}", value);
            Err(())
        }
    }

    pub fn peek_read(&mut self) -> Result<u8, ()> {
        if let Some(entry) = self.queue.dequeue() {
            swmbx_log!("peek_read: value {:#x} read from FIFO", entry.value,);
            Ok(entry.value)
        } else {
            Err(())
        }
    }
    pub fn flush(&mut self) {
        while self.queue.dequeue().is_some() {}
        self.msg_index = 0;
    }
}

// --- Main SWMBX controller data ---
pub struct SwmbxCtrl {
    pub mbx_en: Cell<u8>,

    pub node: [[SwmbxNode; SWMBX_NODE_COUNT]; SWMBX_DEV_COUNT],
    pub fifo: [SwmbxFifo<SWMBX_FIFO_DEPTH>; SWMBX_FIFO_COUNT],

    pub mbx_fifo_execute: [bool; SWMBX_DEV_COUNT],
    pub mbx_fifo_addr: [u8; SWMBX_DEV_COUNT],
    pub mbx_fifo_idx: [u8; SWMBX_DEV_COUNT],
    pub buffer_size: usize,
}

impl SwmbxCtrl {
    pub fn init(buffer_size: usize) -> &'static mut SwmbxCtrl {
        if INIT_DONE.load(Ordering::SeqCst) {
            return unsafe { SWMBX_CTRL.assume_init_mut() };
        }

        let ctrl = SwmbxCtrl {
            mbx_en: Cell::new(0),
            node: [[SwmbxNode::default(); SWMBX_NODE_COUNT]; SWMBX_DEV_COUNT],
            fifo: [
                SwmbxFifo::new(),
                SwmbxFifo::new(),
                SwmbxFifo::new(),
                SwmbxFifo::new(),
            ],
            mbx_fifo_execute: [false; SWMBX_DEV_COUNT],
            mbx_fifo_addr: [0; SWMBX_DEV_COUNT],
            mbx_fifo_idx: [0; SWMBX_DEV_COUNT],
            buffer_size,
        };

        let ctrl_ref = unsafe {
            SWMBX_CTRL.write(ctrl);
            SWMBX_CTRL.assume_init_mut()
        };

        INIT_DONE.store(true, Ordering::SeqCst);
        ctrl_ref
    }

    pub fn store_ctrl_ptr(ctrl: &'static Self) {
        unsafe {
            let ptr = SWMBX_INFO_BASE as *mut *const SwmbxCtrl;
            ptr.write_volatile(ctrl as *const _);
        }
    }

    pub fn load_ctrl_ptr() -> &'static Self {
        unsafe {
            let ptr = SWMBX_INFO_BASE as *const *const SwmbxCtrl;
            &**ptr
        }
    }

    pub fn load_ctrl_ptr_mut() -> &'static mut Self {
        unsafe {
            let ptr = SWMBX_INFO_BASE as *const *mut SwmbxCtrl;
            &mut **ptr
        }
    }

    pub fn update_notify(&mut self, port: usize, addr: u8, enable: bool) -> Result<(), ()> {
        if port >= SWMBX_DEV_COUNT {
            return Err(());
        }

        let node = &mut self.node[port][addr as usize];

        if enable {
            node.enabled_flags |= SWMBX_NOTIFY;
        } else {
            node.enabled_flags &= !SWMBX_NOTIFY;
        }

        Ok(())
    }
    pub fn update_fifo(
        &mut self,
        index: usize,
        addr: u8,
        depth: u8,
        notify: u8,
        enable: bool,
    ) -> Result<(), ()> {
        if index >= SWMBX_FIFO_COUNT {
            return Err(());
        }

        let fifo = &mut self.fifo[index];
        fifo.enabled = enable;

        if enable {
            fifo.fifo_offset = addr;
            fifo.max_msg_count = depth;
            fifo.notify_flag = notify;
            fifo.msg_index = 0;
            fifo.queue = Queue::new();
        } else {
            fifo.queue = Queue::new();
        }

        Ok(())
    }

    pub fn flush_fifo(&mut self, index: usize) -> Result<(), ()> {
        if index >= SWMBX_FIFO_COUNT {
            return Err(());
        }

        self.fifo[index].flush();
        Ok(())
    }
    pub fn enable_behavior(&self, flag: u8, enable: bool) -> Result<(), ()> {
        if (flag & FLAG_MASK) == 0 {
            return Err(());
        }

        let old = self.mbx_en.get();
        if enable {
            self.mbx_en.set(old | flag);
        } else {
            self.mbx_en.set(old & !flag);
        }
        swmbx_log!("enable_behavior: {:#x} -> {:#x}", old, self.mbx_en.get());

        Ok(())
    }
    pub fn update_protect(&mut self, port: usize, addr: u8, enable: bool) -> Result<(), ()> {
        if port >= SWMBX_DEV_COUNT {
            return Err(());
        }

        let node = &mut self.node[port][addr as usize];
        if enable {
            node.enabled_flags |= SWMBX_PROTECT;
        } else {
            node.enabled_flags &= !SWMBX_PROTECT;
        }

        Ok(())
    }

    pub fn apply_protect(
        &mut self,
        port: usize,
        bitmap: &[u32],
        start_idx: usize,
    ) -> Result<(), ()> {
        if port >= SWMBX_DEV_COUNT || start_idx + bitmap.len() > 8 {
            return Err(());
        }

        for (i, &val) in bitmap.iter().enumerate() {
            let base = (start_idx + i) << 5;
            for bit in 0..32 {
                let addr = base + bit;
                if addr >= SWMBX_NODE_COUNT {
                    break;
                }
                let node = &mut self.node[port][addr];
                if (val >> bit) & 1 != 0 {
                    node.enabled_flags |= SWMBX_PROTECT;
                } else {
                    node.enabled_flags &= !SWMBX_PROTECT;
                }
            }
        }
        Ok(())
    }

    pub fn get_msg(&mut self, port: usize, addr: u8) -> u8 {
        if (port as usize) >= SWMBX_DEV_COUNT {
            swmbx_log!("get_msg invalid port {}", port);
            return 0;
        }

        if self.mbx_fifo_execute[port] && (self.mbx_en.get() & SWMBX_FIFO) != 0 {
            let fifo_index = self.mbx_fifo_idx[port as usize] as usize;

            match self.fifo[fifo_index].peek_read() {
                Ok(val) => {
                    return val;
                }
                Err(_) => {
                    // In C: give semaphore here — not implemented in Rust
                    swmbx_log!("FIFO empty at index {} (port {})", fifo_index, port);
                    return 0;
                }
            }
        }

        let val = SwmbxBuffer::read(addr);
        swmbx_log!("get_msg port: {}, addr: {:#x}, val: {:#x}", port, addr, val);
        val
    }

    pub fn send_msg(&mut self, port: usize, addr: u8, val: u8) {
        if port >= SWMBX_DEV_COUNT {
            return;
        }

        swmbx_log!(
            "send_msg port: {}, addr: {:#x}, val: {:#x}",
            port,
            addr,
            val
        );

        let mut write_to_buffer = false;

        if self.mbx_fifo_execute[port] && (self.mbx_en.get() & SWMBX_FIFO) != 0 {
            let fifo_addr = self.mbx_fifo_addr[port];
            let fifo_index = self.mbx_fifo_idx[port] as usize;

            if self.fifo[fifo_index].append_write(val).is_err() {
                self.node[port][addr as usize].notify_flag = true;
                return;
            }

            if (self.mbx_en.get() & SWMBX_NOTIFY) != 0
                && (self.fifo[fifo_index].notify_flag & SWMBX_FIFO_NOTIFY_START) != 0
                && !self.fifo[fifo_index].notify_start
                && (self.node[port][fifo_addr as usize].enabled_flags & SWMBX_NOTIFY) != 0
            {
                self.node[port][fifo_addr as usize].notify_flag = true;
                self.fifo[fifo_index].notify_start = true;
            }

            if !self.fifo[fifo_index].fifo_write {
                self.fifo[fifo_index].fifo_write = true;
            }
        } else {
            let node = &mut self.node[port][addr as usize];

            if (node.enabled_flags & SWMBX_PROTECT) == 0 || (self.mbx_en.get() & SWMBX_PROTECT) == 0
            {
                write_to_buffer = true;
            }

            if (self.mbx_en.get() & SWMBX_NOTIFY) != 0 && (node.enabled_flags & SWMBX_NOTIFY) != 0 {
                node.notify_flag = true;
            }
        }

        if write_to_buffer {
            SwmbxBuffer::write(addr, val);
        }
    }
    pub fn send_start(&mut self, port: usize, addr: u8) {
        if port >= SWMBX_DEV_COUNT {
            return;
        }

        swmbx_log!("send_start port: {}, addr: {:#x}", port, addr);
        if let Some(fifo_index) = self.check_fifo(addr) {
            self.mbx_fifo_execute[port] = true;
            self.mbx_fifo_addr[port] = addr;
            self.mbx_fifo_idx[port] = fifo_index as u8;
        } else {
            self.mbx_fifo_execute[port] = false;
        }
    }

    pub fn check_fifo(&self, addr: u8) -> Option<usize> {
        self.fifo
            .iter()
            .position(|f| f.fifo_offset == addr && f.enabled)
    }

    pub fn send_stop(&mut self, port: usize) {
        if port >= SWMBX_DEV_COUNT {
            return;
        }

        swmbx_log!("send_stop port: {}", port);
        if self.mbx_fifo_execute[port] {
            let fifo_addr = self.mbx_fifo_addr[port];
            let fifo_index = self.mbx_fifo_idx[port] as usize;

            if (self.mbx_en.get() & SWMBX_NOTIFY) != 0
                && (self.fifo[fifo_index].notify_flag & SWMBX_FIFO_NOTIFY_STOP) != 0
                && self.fifo[fifo_index].fifo_write
            {
                if (self.node[port][fifo_addr as usize].enabled_flags & SWMBX_NOTIFY) != 0 {
                    self.node[port][fifo_addr as usize].notify_flag = true;
                }
            }

            self.fifo[fifo_index].notify_start = false;
            self.fifo[fifo_index].fifo_write = false;
            self.mbx_fifo_execute[port] = false;
            self.mbx_fifo_addr[port] = 0;
            self.mbx_fifo_idx[port] = 0;
        }
    }
    pub fn swmbx_write(&mut self, fifo: bool, addr: u8, val: u8) -> Result<(), ()> {
        if fifo {
            if let Some(index) = self.check_fifo(addr) {
                self.fifo[index].append_write(val)?;
                Ok(())
            } else {
                Err(())
            }
        } else {
            SwmbxBuffer::write(addr, val);
            Ok(())
        }
    }

    pub fn swmbx_read(&mut self, fifo: bool, addr: u8) -> Result<u8, ()> {
        if fifo {
            if let Some(index) = self.check_fifo(addr) {
                self.fifo[index].peek_read()
            } else {
                Err(())
            }
        } else {
            Ok(SwmbxBuffer::read(addr))
        }
    }
}

// --- SWMBX buffer abstraction ---
pub struct SwmbxBuffer;

impl SwmbxBuffer {
    #[inline]
    pub fn read(addr: u8) -> u8 {
        assert!((addr as usize) < SWMBX_BUF_SIZE);
        unsafe { read_volatile((SWMBX_BUF_BASE + addr as usize) as *const u8) }
    }

    #[inline]
    pub fn write(addr: u8, val: u8) {
        assert!((addr as usize) < SWMBX_BUF_SIZE);
        unsafe { write_volatile((SWMBX_BUF_BASE + addr as usize) as *mut u8, val) };
    }
}
