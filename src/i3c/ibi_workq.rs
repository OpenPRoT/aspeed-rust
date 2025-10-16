// Licensed under the Apache-2.0 license

use core::cell::RefCell;
use critical_section::Mutex;
use heapless::spsc::Queue;

const IBIQ_DEPTH: usize = 16;
const IBI_DATA_MAX: u8 = 16;

#[derive(Debug, Clone, Copy)]
pub enum IbiWork {
    HotJoin,
    Sirq {
        addr: u8,
        len: u8,
        data: [u8; IBI_DATA_MAX as usize],
    },
    TargetDaAssignment,
}

static mut IBIQ_BUFS: [Queue<IbiWork, IBIQ_DEPTH>; 4] =
    [Queue::new(), Queue::new(), Queue::new(), Queue::new()];

struct IbiBus {
    prod: Option<heapless::spsc::Producer<'static, IbiWork, IBIQ_DEPTH>>,
    cons: Option<heapless::spsc::Consumer<'static, IbiWork, IBIQ_DEPTH>>,
}

static IBI_WORKQS: [Mutex<RefCell<IbiBus>>; 4] = [
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
];

fn ensure_ibiq_split(bus: usize) {
    assert!(bus < 4);
    critical_section::with(|cs| {
        let mut b = IBI_WORKQS[bus].borrow(cs).borrow_mut();
        if b.prod.is_none() || b.cons.is_none() {
            let (p, c) = unsafe { IBIQ_BUFS[bus].split() };
            b.prod = Some(p);
            b.cons = Some(c);
        }
    });
}

#[must_use]
pub fn i3c_ibi_workq_consumer(
    bus: usize,
) -> heapless::spsc::Consumer<'static, IbiWork, IBIQ_DEPTH> {
    assert!(bus < 4);
    critical_section::with(|cs| {
        let mut b = IBI_WORKQS[bus].borrow(cs).borrow_mut();
        if b.prod.is_none() || b.cons.is_none() {
            let (p, c) = unsafe { IBIQ_BUFS[bus].split() };
            b.prod = Some(p);
            b.cons = Some(c);
        }
        b.cons.take().expect("IBI consumer already taken")
    })
}

pub fn i3c_ibi_work_enqueue_target_da_assignment(bus: usize) {
    ensure_ibiq_split(bus);
    critical_section::with(|cs| {
        if let Some(p) = IBI_WORKQS[bus].borrow(cs).borrow_mut().prod.as_mut() {
            let _ = p.enqueue(IbiWork::TargetDaAssignment);
        }
    });
}

#[inline]
pub fn i3c_ibi_work_enqueue_hotjoin(bus: usize) {
    ensure_ibiq_split(bus);
    critical_section::with(|cs| {
        if let Some(p) = IBI_WORKQS[bus].borrow(cs).borrow_mut().prod.as_mut() {
            let _ = p.enqueue(IbiWork::HotJoin);
        }
    });
}

#[inline]
pub fn i3c_ibi_work_enqueue_target_irq(bus: usize, addr: u8, data: &[u8]) {
    ensure_ibiq_split(bus);
    let mut ibi_buf = [0u8; IBI_DATA_MAX as usize];
    let take = core::cmp::min(IBI_DATA_MAX as usize, data.len());
    ibi_buf[..take].copy_from_slice(&data[..take]);
    critical_section::with(|cs| {
        if let Some(p) = IBI_WORKQS[bus].borrow(cs).borrow_mut().prod.as_mut() {
            let _ = p.enqueue(IbiWork::Sirq {
                addr,
                // len: take as u8,
                len: u8::try_from(take).map_err(|_| ())
                    .unwrap_or(IBI_DATA_MAX),
                data: ibi_buf,
            });
        }
    });
}
