// Licensed under the Apache-2.0 license

//! spidmairqtest.rs - DMA irq read/write test harness using static buffers and chainable callbacks

use super::fmccontroller::FmcController;
use crate::common::{self, DmaBuffer, DummyDelay};
use crate::spi::device::ChipSelectDevice;
use crate::spi::norflash::{SpiNorCommand, SpiNorDevice};
use crate::spi::spicontroller::SpiController;
use crate::spi::spitest::{self, DeviceId, FMC_CONFIG};
use crate::spi::SpiData;
use crate::uart_core::{UartConfig, UartController};
use crate::{astdebug, pinctrl};
use cortex_m::peripheral::NVIC;
use embedded_hal::delay::DelayNs;
use embedded_io::Write;
use heapless::Deque;
use static_cell::StaticCell;

#[macro_export]
macro_rules! log_uart {
    ($uart:expr, $($arg:tt)*) => {{
        writeln!($uart, $($arg)*).ok();
        write!($uart, "\r").ok();
    }};
}

// DMA operation type selector
#[derive(Debug, Copy, Clone)]
pub enum DmaOp {
    Read,
    ReadFast,
    Program,
    ProgramFast,
}

// DMA request struct with callback
#[derive(Debug)]
pub struct DmaRequest {
    pub src_addr: usize,
    pub dst_buf: &'static mut [u8],
    pub len: usize,
    pub op: DmaOp,
    pub verify: bool,   // for test
    pub buf_idx: usize, // for test
    pub on_complete: fn(bool, usize, &[u8]),
}

// Configuration
const MAX_DMA_CHAIN: usize = 4;
const DMA_BUF_SIZE: usize = 256;

struct App {
    uart: UartController<'static>,
    // controllers
    fmc_controller: Option<FmcController<'static>>,
    spi_controller: Option<SpiController<'static>>,

    // dma state
    request_alldone: bool,
    current_dma: Option<DmaRequest>,
    dma_queue: Deque<DmaRequest, MAX_DMA_CHAIN>,
    current_devid: DeviceId,

    // buffers
    read_buffers: [DmaBuffer<DMA_BUF_SIZE>; MAX_DMA_CHAIN],
    write_buffers: [DmaBuffer<DMA_BUF_SIZE>; MAX_DMA_CHAIN],
}

impl App {
    fn new() -> Self {
        let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
        let mut uart = UartController::new(uart_regs);
        uart.init(&UartConfig::default()).unwrap();

        Self {
            uart,
            fmc_controller: None,
            spi_controller: None,

            request_alldone: true,
            current_dma: None,
            dma_queue: Deque::new(),
            current_devid: DeviceId::FmcCs0Idx,

            read_buffers: [const { DmaBuffer::new() }; MAX_DMA_CHAIN],
            write_buffers: [const { DmaBuffer::new() }; MAX_DMA_CHAIN],
        }
    }

    unsafe fn fmc_irq(&mut self) {
        log_uart!(&mut self.uart, "fmc_irq");
        let Some(fmc) = self.fmc_controller.as_mut() else {
            return;
        };
        let irq_result = fmc.handle_interrupt();

        let mut dev = match self.current_devid {
            DeviceId::FmcCs0Idx => ChipSelectDevice {
                bus: fmc,
                cs: 0,
                spim: None,
            },
            DeviceId::FmcCs1Idx => ChipSelectDevice {
                bus: fmc,
                cs: 1,
                spim: None,
            },
            _ => return,
        };

        if irq_result.is_ok() {
            log_uart!(&mut self.uart, "ok");
            if let Some(req) = self.current_dma.take() {
                if matches!(req.op, DmaOp::Program | DmaOp::ProgramFast) {
                    dev.nor_wait_until_ready();
                }
                log_uart!(&mut self.uart, "call on complete");
                (req.on_complete)(req.verify, req.buf_idx, req.dst_buf);
            }
            self.start_next_dma();
        }
    }

    unsafe fn spi_irq(&mut self) {
        log_uart!(&mut self.uart, "spi_irq");
        let Some(spi) = self.spi_controller.as_mut() else {
            return;
        };

        let irq_result = spi.handle_interrupt();

        if irq_result.is_ok() {
            if let Some(req) = self.current_dma.take() {
                (req.on_complete)(req.verify, req.buf_idx, req.dst_buf);
            }
            self.start_next_dma();
        }
    }

    unsafe fn start_next_dma(&mut self) {
        if self.dma_queue.is_empty() {
            self.request_alldone = true;
            return;
        }
        log_uart!(&mut self.uart, "start_next_dma");
        if let Some(mut req) = self.dma_queue.pop_front() {
            match self.current_devid {
                DeviceId::FmcCs0Idx | DeviceId::FmcCs1Idx => {
                    let _ = self.start_dma_fmc_transfer(&mut req);
                }
                DeviceId::Spi0Cs0Idx
                | DeviceId::Spi1Cs0Idx
                | DeviceId::Spi1Cs1Idx
                | DeviceId::Spi0Cs1Idx => {
                    let _ = self.start_dma_spi_transfer(&mut req);
                }
            }
            self.current_dma = Some(req);
        }
    }

    unsafe fn start_dma_spi_transfer(&mut self, req: &mut DmaRequest) -> Result<(), ()> {
        let controller = self.spi_controller.as_mut().unwrap();
        log_uart!(&mut self.uart, "start_dma_spi_transfer");
        let mut dev = match self.current_devid {
            DeviceId::Spi0Cs0Idx => ChipSelectDevice {
                bus: controller,
                cs: 0,
                spim: None,
            },
            _ => return Ok(()),
        };

        let result = match req.op {
            DmaOp::Read => dev.nor_read_data(u32::try_from(req.src_addr).unwrap(), req.dst_buf),
            DmaOp::ReadFast => {
                dev.nor_read_fast_4b_data(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
            DmaOp::Program => {
                dev.nor_page_program(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
            DmaOp::ProgramFast => {
                dev.nor_page_program_4b(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
        };

        result.map_err(|_| ())
    }

    unsafe fn start_dma_fmc_transfer(&mut self, req: &mut DmaRequest) -> Result<(), ()> {
        log_uart!(&mut self.uart, "fmc_irq");
        let controller = self.fmc_controller.as_mut().unwrap();
        let mut dev = match self.current_devid {
            DeviceId::FmcCs0Idx => ChipSelectDevice {
                bus: controller,
                cs: 0,
                spim: None,
            },
            DeviceId::FmcCs1Idx => ChipSelectDevice {
                bus: controller,
                cs: 1,
                spim: None,
            },
            _ => return Ok(()),
        };

        let result = match req.op {
            DmaOp::Read => dev.nor_read_data(u32::try_from(req.src_addr).unwrap(), req.dst_buf),
            DmaOp::ReadFast => {
                dev.nor_read_fast_4b_data(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
            DmaOp::Program => {
                dev.nor_page_program(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
            DmaOp::ProgramFast => {
                dev.nor_page_program_4b(u32::try_from(req.src_addr).unwrap(), req.dst_buf)
            }
        };

        result.map_err(|_| ())
    }
}

// One time globale instance:
static APP: StaticCell<App> = StaticCell::new();
static mut APP_PTR: *mut App = core::ptr::null_mut();

// initialize the APP and return a mutable reference to it
#[allow(clippy::ref_as_ptr)]
pub fn init_spidmairq_app_once() {
    let app = APP.init_with(App::new);
    unsafe {
        APP_PTR = core::ptr::from_mut(app); //app as *mut _;
    }
}
#[inline]
fn app_mut() -> &'static mut App {
    unsafe { APP_PTR.as_mut().expect("APP not initialized") }
}
#[no_mangle]
pub extern "C" fn fmc() {
    unsafe { app_mut().fmc_irq() };
}

#[no_mangle]
pub extern "C" fn spi() {
    unsafe { app_mut().spi_irq() };
}

#[allow(dead_code)]
unsafe fn show_mmap_reg(uart: &mut UartController<'_>) {
    let (_reg_base, mmap_addr, _cs_capacity) = spitest::device_info(app_mut().current_devid);
    log_uart!(uart, "[{:08x}]", mmap_addr);
    astdebug::print_reg_u8(uart, mmap_addr, 0x400);
}
#[must_use]
pub fn verify_dma_buffer_match(uart: &mut UartController<'_>, i: usize) -> bool {
    let read = app_mut().read_buffers[i].as_mut_slice(0, DMA_BUF_SIZE);
    let write = app_mut().write_buffers[i].as_mut_slice(0, DMA_BUF_SIZE);

    if read != write {
        for (j, (&r, &w)) in read.iter().zip(write.iter()).enumerate() {
            if r != w {
                log_uart!(
                    uart,
                    "Mismatch at buffer {}, index {}: read={:02x}, expected={:02x}",
                    i,
                    j,
                    r,
                    w
                );
                break;
            }
        }
        astdebug::print_array_u8(uart, read);
        astdebug::print_array_u8(uart, write);
        return false;
    }
    astdebug::print_array_u8(uart, read);
    astdebug::print_array_u8(uart, write);
    true
}

pub fn on_complete_dma(verify: bool, idx: usize, buf: &[u8]) {
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut uart_controller = UartController::new(uart_regs);
    uart_controller.init(&UartConfig::default()).unwrap();

    if verify {
        if verify_dma_buffer_match(&mut uart_controller, idx) {
            log_uart!(&mut uart_controller, "DMA test passed!!");
        } else {
            log_uart!(&mut uart_controller, "DMA test failed!!");
        }
    } else {
        astdebug::print_array_u8(&mut uart_controller, buf);
    }
}
pub fn fill_dma_buffer(op_req: DmaOp, random: bool, pattern: u32) {
    let mut seed = 0xDEAD_FBEE;
    seed += pattern;

    for i in 0..MAX_DMA_CHAIN {
        let buf: &'static mut [u8] = match op_req {
            DmaOp::Read | DmaOp::ReadFast => {
                app_mut().read_buffers[i].as_mut_slice(0, DMA_BUF_SIZE)
            }
            DmaOp::Program | DmaOp::ProgramFast => {
                app_mut().write_buffers[i].as_mut_slice(0, DMA_BUF_SIZE)
            }
        };

        buf.fill(0x0);
        if random {
            common::fill_random(buf, &mut seed);
        }
    }
}
#[allow(clippy::missing_safety_doc)]
pub unsafe fn dma_irq_chain_test(
    uart: &mut UartController<'_>,
    start_addrs: &[u32],
    op_req: DmaOp,
    verify: bool,
) {
    app_mut().dma_queue.clear();
    app_mut().request_alldone = false;

    for (i, &addr) in start_addrs.iter().enumerate() {
        if i >= MAX_DMA_CHAIN {
            log_uart!(uart, "Too many DMA addresses; max is {}", MAX_DMA_CHAIN);
            break;
        }

        let buf: &'static mut [u8] = match op_req {
            DmaOp::Read | DmaOp::ReadFast => {
                app_mut().read_buffers[i].as_mut_slice(0, DMA_BUF_SIZE)
            }
            DmaOp::Program | DmaOp::ProgramFast => {
                app_mut().write_buffers[i].as_mut_slice(0, DMA_BUF_SIZE)
            }
        };

        let request = DmaRequest {
            src_addr: addr as usize,
            dst_buf: buf,
            len: DMA_BUF_SIZE,
            op: op_req,
            buf_idx: i,
            verify,
            on_complete: on_complete_dma,
        };

        app_mut().dma_queue.push_back(request).unwrap();
        log_uart!(uart, "chaining {}", i);
    }

    unsafe {
        app_mut().start_next_dma();
    }
}

pub fn test_fmc_dma_irq(uart: &mut UartController<'_>) {
    let fmc_spi = unsafe { &*ast1060_pac::Fmc::ptr() };
    let mut delay = DummyDelay {};

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_FMC_QUAD);
    let fmc_data = SpiData::new();

    unsafe {
        NVIC::unmask(ast1060_pac::Interrupt::fmc);
    }

    app_mut().fmc_controller = Some(FmcController::new(fmc_spi, 0, FMC_CONFIG, fmc_data, None));

    let controller = app_mut().fmc_controller.as_mut().unwrap();
    let _ = controller.init();

    let nor_read_data: SpiNorCommand<'_> = spitest::nor_device_read_data(spitest::FMC_CS0_CAPACITY);
    let nor_write_data = spitest::nor_device_write_data(spitest::FMC_CS0_CAPACITY);

    let mut dev0 = ChipSelectDevice {
        bus: controller,
        cs: 0,
        spim: None,
    };

    let _ = dev0.nor_read_init(&nor_read_data);
    let _ = dev0.nor_write_init(&nor_write_data);

    let mut dev1 = ChipSelectDevice {
        bus: controller,
        cs: 1,
        spim: None,
    };

    let _ = dev1.nor_read_init(&nor_read_data);
    let _ = dev1.nor_write_init(&nor_write_data);

    log_uart!(uart, "==== FMC DEV0 DMA Read Test====");
    app_mut().current_devid = DeviceId::FmcCs0Idx;

    let start_addrs = [0x0000_0000, 0x0000_0100, 0x0000_0200, 0x0000_0300];
    // DEV0 read test
    app_mut().current_devid = DeviceId::FmcCs0Idx;
    fill_dma_buffer(DmaOp::Read, true, 0);
    delay.delay_ns(8_000_000);
    unsafe {
        dma_irq_chain_test(uart, &start_addrs, DmaOp::Read, false);
    }
    delay.delay_ns(8_000_000);

    log_uart!(uart, "==== FMC DEV1 DMA Read Write Verify Test====");
    // DEV1 program + verify
    let read_only = false;
    app_mut().current_devid = DeviceId::FmcCs1Idx;

    if read_only {
        fill_dma_buffer(DmaOp::Read, false, 0);
        unsafe {
            dma_irq_chain_test(uart, &start_addrs, DmaOp::Read, false);
        }
    } else {
        fill_dma_buffer(DmaOp::Program, true, 0x3ef1);

        // dev1 erase must be done via APP
        let _ = dev1.nor_sector_erase(0x0000_0000);

        delay.delay_ns(8_000_000);
        unsafe { dma_irq_chain_test(uart, &start_addrs, DmaOp::Program, false) };
        delay.delay_ns(8_000_000);

        let done = app_mut().request_alldone;
        if !done {
            log_uart!(uart, "=ERROR: Programming race condition!!!!=");
        }

        unsafe {
            dma_irq_chain_test(uart, &start_addrs, DmaOp::Read, true);
        }
    }

    delay.delay_ns(8_000_000);
}

pub fn test_spi_dma_irq(uart: &mut UartController<'_>) {
    let spi0 = unsafe { &*ast1060_pac::Spi::ptr() };
    let current_cs = 0;
    let mut delay = DummyDelay {};

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPIM0_QUAD_DEFAULT);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPI1_QUAD);

    let scu_qspi_mux: &mut [u32] =
        unsafe { core::slice::from_raw_parts_mut((spitest::SCU_BASE + 0xf0) as *mut u32, 4) };
    scu_qspi_mux[0] = 0x0000_fff0;

    let spi_data = SpiData::new();

    unsafe {
        NVIC::unmask(ast1060_pac::Interrupt::spi);
    }

    // Setup controller + device inside APP

    app_mut().spi_controller = Some(SpiController::new(
        spi0,
        current_cs,
        spitest::SPI0_CONFIG,
        spi_data,
        None,
    ));

    let controller = app_mut().spi_controller.as_mut().unwrap();
    let _ = controller.init();

    log_uart!(uart, "==== SPI0 DEV0 DMA Test====");

    let nor_read_data: SpiNorCommand<'_> =
        spitest::nor_device_read_4b_data(spitest::SPI_CS0_CAPACITY);
    let nor_write_data = spitest::nor_device_write_4b_data(spitest::SPI_CS0_CAPACITY);

    let mut dev0 = ChipSelectDevice {
        bus: controller,
        cs: 0,
        spim: None,
    };

    let _ = dev0.nor_read_init(&nor_read_data);
    let _ = dev0.nor_write_init(&nor_write_data);

    app_mut().current_devid = DeviceId::Spi0Cs0Idx;

    let start_addrs = [0x0000_0000, 0x0000_0100, 0x0000_0200, 0x0000_0300];
    let read_only = false;

    if read_only {
        fill_dma_buffer(DmaOp::ReadFast, false, 0);
        unsafe { dma_irq_chain_test(uart, &start_addrs, DmaOp::ReadFast, false) };
    } else {
        fill_dma_buffer(DmaOp::Program, true, 0x8e1);

        let _ = dev0.nor_sector_erase(0x0000_0000);

        delay.delay_ns(8_000_000);

        unsafe { dma_irq_chain_test(uart, &start_addrs, DmaOp::ProgramFast, false) };
        delay.delay_ns(8_000_000);

        let done = app_mut().request_alldone;
        if !done {
            log_uart!(uart, "=ERROR: Programming race condition!!!!=");
        }

        unsafe { dma_irq_chain_test(uart, &start_addrs, DmaOp::ReadFast, true) };
    }

    log_uart!(uart, "==== End SPI0 DEV0 DMA Test====");

    scu_qspi_mux[0] = 0x0000_0000;
}
