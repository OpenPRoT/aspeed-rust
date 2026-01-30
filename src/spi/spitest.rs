// Licensed under the Apache-2.0 license

use super::device::ChipSelectDevice;
use super::fmccontroller::FmcController;
use super::norflash::{
    Jesd216Mode, SpiNorData, SpiNorDevice, SPI_NOR_CMD_QREAD, SPI_NOR_CMD_READ_FAST_4B,
};
use super::{
    norflash, CommandMode, CtrlType, SpiConfig, SpiData, SpiDecodeAddress,
    SPI_NOR_DATA_DIRECT_READ, SPI_NOR_DATA_DIRECT_WRITE,
};
use crate::common::{DmaBuffer, DummyDelay};
use crate::spi::norflashblockdevice;
use crate::spi::norflashblockdevice::{BlockAddrUsize, NorFlashBlockDevice};
use crate::spi::spicontroller::SpiController;
use crate::spimonitor::{RegionInfo, SpiMonitor, SpimExtMuxSel};
use crate::uart_core::{UartConfig, UartController};
use crate::{astdebug, pinctrl};
use ast1060_pac::{Peripherals, Spipf, Spipf1, Spipf2, Spipf3};
use embedded_hal::delay::DelayNs;
use embedded_hal::spi::SpiDevice;
use embedded_io::Write;
use proposed_traits::block_device::{BlockDevice, BlockRange};

pub const FMC_CTRL_BASE: usize = 0x7e62_0000;
pub const FMC_MMAP_BASE: usize = 0x8000_0000;

pub const SPI0_CTRL_BASE: usize = 0x7e63_0000;
pub const SPI0_MMAP_BASE: usize = 0x9000_0000;

pub const SPI1_CTRL_BASE: usize = 0x7e64_0000;
pub const SPI1_MMAP_BASE: usize = 0xb000_0000;
const SCU_BASE: usize = 0x7E6E_2000;
pub const CTRL_REG_SIZE: usize = 0xc4;

pub const SPIPF1_BASE: usize = 0x7e79_1000;
pub const SPIPF2_BASE: usize = 0x7e79_2000;
pub const SPIPF3_BASE: usize = 0x7e79_3000;
pub const SPIPF4_BASE: usize = 0x7e79_4000;

pub const GPIO_BASE: usize = 0x7e78_0000;

#[derive(Copy, Clone)]
#[deny(dead_code)]
pub enum DeviceId {
    FmcCs0Idx,
    FmcCs1Idx,
    Spi0Cs0Idx,
    Spi0Cs1Idx,
    Spi1Cs0Idx,
    Spi1Cs1Idx,
}

// user define
pub const FMC_CS0_CAPACITY: usize = 0x10_0000;
pub const FMC_CS1_CAPACITY: usize = 0x10_0000;
pub const SPI_CS0_CAPACITY: usize = 0x400_0000; // 64M
pub const SPI_CS1_CAPACITY: usize = 0x400_0000;

/* macronix MX25L8006E Flash device info */
const MACRONIX_PAGE_SIZE: usize = 256;
const MACRONIX_SECTOR_SIZE: usize = 4096;

const DMA_MIN_LENGTH: usize = 128;
const SPI_TOTAL_BUFFER: usize = 2;
const SPI_NC_BUFFER_SIZE: usize = MACRONIX_SECTOR_SIZE;
const TEST_DATA_SIZE: usize = MACRONIX_PAGE_SIZE;

const WRITE_IDX: usize = 0x0;
const READ_IDX: usize = 0x1;

#[link_section = ".ram_nc"]
static mut SPI_NC_BUFFER: [DmaBuffer<SPI_NC_BUFFER_SIZE>; SPI_TOTAL_BUFFER] =
    [DmaBuffer::new(), DmaBuffer::new()];

pub const FMC_CONFIG: SpiConfig = SpiConfig {
    mmap_base: 0x8000_0000,
    max_cs: 2,
    write_block_size: 4096,
    ctrl_type: CtrlType::BootSpi,
    timing_cali_start_off: 2,
    master_idx: 0,
    pure_spi_mode_only: false,
    frequency: 50_000_000,
    timing_calibration_start_off: 0x0,
    timing_calibration_disabled: true,
};

pub const SPI0_CONFIG: SpiConfig = SpiConfig {
    mmap_base: 0x9000_0000,
    max_cs: 1,
    write_block_size: 4096,
    ctrl_type: CtrlType::HostSpi,
    timing_cali_start_off: 2,
    master_idx: 0,
    pure_spi_mode_only: false,
    frequency: 50_000_000,
    timing_calibration_start_off: 0x0,
    timing_calibration_disabled: true,
};

pub const SPI1_CONFIG: SpiConfig = SpiConfig {
    mmap_base: 0xb000_0000,
    max_cs: 2,
    write_block_size: 4096,
    ctrl_type: CtrlType::NormalSpi,
    timing_cali_start_off: 2,
    master_idx: 2,
    pure_spi_mode_only: false,
    frequency: 50_000_000,
    timing_calibration_start_off: 0x0,
    timing_calibration_disabled: false,
};

#[must_use]
pub fn nor_device_read_data<'a>(len: usize) -> SpiNorData<'a> {
    SpiNorData {
        mode: Jesd216Mode::Mode114,
        opcode: SPI_NOR_CMD_QREAD,
        dummy_cycle: 8,
        addr: 0,
        addr_len: 3,
        data_len: u32::try_from(len).unwrap(),
        tx_buf: &[],
        rx_buf: &mut [],
        data_direct: SPI_NOR_DATA_DIRECT_READ,
    }
}

#[must_use]
pub fn nor_device_write_data<'a>(len: usize) -> SpiNorData<'a> {
    SpiNorData {
        mode: Jesd216Mode::Mode111,
        opcode: norflash::SPI_NOR_CMD_PP,
        dummy_cycle: 0,
        addr: 0,
        addr_len: 3,
        data_len: u32::try_from(len).unwrap(),
        tx_buf: &[],
        rx_buf: &mut [],
        data_direct: SPI_NOR_DATA_DIRECT_WRITE,
    }
}

#[must_use]
pub fn nor_device_read_4b_data<'a>(len: usize) -> SpiNorData<'a> {
    SpiNorData {
        mode: Jesd216Mode::Mode111Fast,
        opcode: SPI_NOR_CMD_READ_FAST_4B,
        dummy_cycle: 8,
        addr: 0,
        addr_len: 4,
        data_len: u32::try_from(len).unwrap(),
        tx_buf: &[],
        rx_buf: &mut [],
        data_direct: SPI_NOR_DATA_DIRECT_READ,
    }
}

#[must_use]
pub fn nor_device_write_4b_data<'a>(len: usize) -> SpiNorData<'a> {
    SpiNorData {
        mode: Jesd216Mode::Mode111,
        opcode: norflash::SPI_NOR_CMD_PP_4B,
        dummy_cycle: 0,
        addr: 0,
        addr_len: 4,
        data_len: u32::try_from(len).unwrap(),
        tx_buf: &[],
        rx_buf: &mut [],
        data_direct: SPI_NOR_DATA_DIRECT_WRITE,
    }
}

macro_rules! test_log {
    ($uart:expr, $($arg:tt)*) => {{
        writeln!($uart, $($arg)*).ok();
        write!($uart, "\r").ok();
    }};
}

pub fn test_read_jedec<D: SpiNorDevice<Error = E>, E>(uart: &mut UartController<'_>, dev: &mut D) {
    test_log!(uart, "#############Read Jedec ID############");
    match dev.nor_read_jedec_id() {
        Ok(id) => {
            astdebug::print_array_u8(uart, &id);
        }
        _ => {
            test_log!(uart, "Error:: Failed to read JEDEC ID");
        }
    }
}

#[must_use]
pub fn device_info(dev_idx: DeviceId) -> (usize, usize, usize) {
    match dev_idx {
        DeviceId::FmcCs0Idx => (FMC_CTRL_BASE, FMC_MMAP_BASE, FMC_CS0_CAPACITY),
        DeviceId::FmcCs1Idx => (
            FMC_CTRL_BASE,
            FMC_MMAP_BASE + FMC_CS0_CAPACITY,
            FMC_CS1_CAPACITY,
        ),
        DeviceId::Spi0Cs0Idx => (SPI0_CTRL_BASE, SPI0_MMAP_BASE, SPI_CS0_CAPACITY),
        DeviceId::Spi0Cs1Idx => (
            SPI0_CTRL_BASE,
            SPI0_MMAP_BASE + SPI_CS0_CAPACITY,
            SPI_CS1_CAPACITY,
        ),
        DeviceId::Spi1Cs0Idx => (SPI1_CTRL_BASE, SPI1_MMAP_BASE, SPI_CS0_CAPACITY),
        DeviceId::Spi1Cs1Idx => (
            SPI1_CTRL_BASE,
            SPI1_MMAP_BASE + SPI_CS0_CAPACITY,
            SPI_CS1_CAPACITY,
        ),
    }
}
pub fn test_cs<D: SpiNorDevice<Error = E>, E>(
    uart: &mut UartController<'_>,
    dev: &mut D,
    dev_idx: DeviceId,
    addr: u32,
    len: usize,
    test_write: bool,
) {
    let mut delay1 = DummyDelay {};

    let wbuf: &mut [u8] = unsafe { SPI_NC_BUFFER[WRITE_IDX].as_mut_slice(0, len) };
    let ptr_write: *mut u8 = wbuf.as_mut_ptr();

    let (reg_base, mmap_addr, _cs_capacity) = device_info(dev_idx);

    if len > DMA_MIN_LENGTH {
        test_log!(uart, "################ DMA TEST starts. base: {:08x} addr: {:08x}, len: {:08x}######################", reg_base, addr, len);
    } else {
        test_log!(uart, "################TEST starts. base: {:08x} addr: {:08x}, len: {:08x}######################", reg_base, addr, len);
    }

    test_log!(uart, "write pointer {:p}", ptr_write);

    if test_write {
        test_log!(uart, "##start sector erase");
        let _ = dev.nor_sector_erase(addr);
        delay1.delay_ns(2_000_000);

        test_log!(uart, "##start page_programing");
        for (i, value) in wbuf.iter_mut().enumerate().take(len) {
            *value = u8::try_from(i).unwrap();
        }
        match dev_idx {
            DeviceId::FmcCs0Idx | DeviceId::FmcCs1Idx => {
                let _ = dev.nor_page_program(addr, wbuf);
            }
            _ => {
                let _ = dev.nor_page_program_4b(addr, wbuf);
            }
        }
        delay1.delay_ns(8_000_000);
    }
    // when data size is bigger than 128. use read dma
    // need to be 4-byte aligned for dma
    test_log!(uart, "##start read");
    let rbuf: &mut [u8] = unsafe { SPI_NC_BUFFER[READ_IDX].as_mut_slice(0, len) };
    let ptr_read = rbuf.as_mut_ptr();
    test_log!(uart, "read --- buffer {:p}", ptr_read);
    test_log!(uart, "buf len:{}", rbuf.len());

    match dev_idx {
        DeviceId::FmcCs0Idx | DeviceId::FmcCs1Idx => {
            let _ = dev.nor_read_data(addr, rbuf);
        }
        _ => {
            let _ = dev.nor_read_fast_4b_data(addr, rbuf);
        }
    }
    delay1.delay_ns(2_000_000);

    if test_write {
        let result: bool;
        unsafe {
            result = core::slice::from_raw_parts(ptr_write, len)
                == core::slice::from_raw_parts(ptr_read, len);
        }
        if result {
            test_log!(uart, "read write test passed!");
        } else {
            test_log!(uart, "ERROR:: read write test failed!!");
            test_log!(uart, "write buffer:");
            astdebug::print_array_u8(uart, wbuf);
            test_log!(uart, "read buffer:");
            astdebug::print_array_u8(uart, rbuf);
            test_log!(uart, "Mmap buffer: {:08x}", mmap_addr + addr as usize);
            astdebug::print_reg_u8(uart, mmap_addr + addr as usize, len);
        }
    } else {
        test_log!(uart, "read buffer:");
        astdebug::print_array_u8(uart, rbuf);

        test_log!(uart, "Mmap buffer: {:08x}", mmap_addr + addr as usize);
        astdebug::print_reg_u8(uart, mmap_addr + addr as usize, len);
    }

    if false {
        // len > DMA_MIN_LENGTH {
        test_log!(uart, "Test FIFO read...buf len:  0x20");
        let _ = dev.nor_read_data(addr, &mut rbuf[0..0x20]);
        astdebug::print_array_u8(uart, &rbuf[0..0x20]);
    }
}

pub fn test_fmc(uart: &mut UartController<'_>) {
    let fmc_spi = unsafe { &*ast1060_pac::Fmc::ptr() };
    let base = core::ptr::from_ref(fmc_spi) as usize;
    test_log!(uart, "fmc_spi Base address = 0x{:08X}", base);

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_FMC_QUAD);

    let current_cs = 0x0;
    let fmc_data = SpiData::new();

    let peripherals = unsafe { Peripherals::steal() };
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut fmc_uart_controller = UartController::new(uart_regs);
    fmc_uart_controller.init(&UartConfig::default()).unwrap();

    let mut controller = FmcController::new(
        fmc_spi,
        current_cs,
        FMC_CONFIG,
        fmc_data,
        Some(&mut fmc_uart_controller),
    );

    test_log!(uart, "FMC controller init");
    let _result = controller.init();
    //astdebug::print_reg_u32(uart, FMC_CTRL_BASE, 0xb0);

    let nor_read_data: SpiNorData<'_> = nor_device_read_data(FMC_CS0_CAPACITY);
    let nor_write_data = nor_device_write_data(FMC_CS0_CAPACITY);

    // Wrap controller in a CS device (CS0)
    let mut flash_device0: ChipSelectDevice<'_, FmcController<'_>, Spipf> = ChipSelectDevice {
        bus: &mut controller,
        cs: 0,
        spi_monitor: None,
    };
    test_read_jedec(uart, &mut flash_device0);
    let _ = flash_device0.nor_read_init(&nor_read_data);
    let _ = flash_device0.nor_write_init(&nor_write_data);
    //test_log!(uart, "FMC REG0x{:08x}", FMC_MMAP_BASE);
    //astdebug::print_reg_u32(uart, FMC_MMAP_BASE, 0x80);

    // Wrap controller in a CS device (CS1)
    let mut flash_device1: ChipSelectDevice<'_, FmcController<'_>, Spipf> = ChipSelectDevice {
        bus: &mut controller,
        cs: 1,
        spi_monitor: None,
    };
    test_read_jedec(uart, &mut flash_device1);
    let _ = flash_device1.nor_read_init(&nor_read_data);
    let _ = flash_device1.nor_write_init(&nor_write_data);

    test_cs(
        uart,
        &mut flash_device1,
        DeviceId::FmcCs1Idx,
        0x0,
        TEST_DATA_SIZE,
        true,
    );
    test_cs(
        uart,
        &mut flash_device1,
        DeviceId::FmcCs1Idx,
        0x1000,
        TEST_DATA_SIZE,
        true,
    );
    test_log!(uart, "################# FMC test done ! ###############");
}

#[allow(clippy::too_many_lines)]
pub fn test_spi(uart: &mut UartController<'_>) {
    let spi0 = unsafe { &*ast1060_pac::Spi::ptr() };
    let base = core::ptr::from_ref(spi0) as usize;
    test_log!(uart, "spi0 Base address = 0x{:08X}", base);
    let current_cs = 0;
    let test_block_dev = true;

    test_log!(uart, "SPI Test Starts...");

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPIM0_QUAD_DEFAULT);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPI1_QUAD);
    let scu_qspi_mux: &mut [u32] =
        unsafe { core::slice::from_raw_parts_mut((SCU_BASE + 0xf0) as *mut u32, 4) };
    scu_qspi_mux[0] = 0x0000_fff0;
    //astdebug::print_reg_u32(uart, SCU_BASE + 0x00, 0x100);

    let spi_data = SpiData::new();
    let peripherals = unsafe { Peripherals::steal() };
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut spi_uart_controller = UartController::new(uart_regs);
    spi_uart_controller.init(&UartConfig::default()).unwrap();

    let mut spi_controller = SpiController::new(
        spi0,
        current_cs,
        SPI0_CONFIG,
        spi_data,
        Some(&mut spi_uart_controller),
    );

    let _result = spi_controller.init();

    //astdebug::print_reg_u32(uart, SPI0_CTRL_BASE, 0xb0);
    let mut spi_monitor0 = start_spim0();
    // Wrap controller in a CS device (CS0)
    let mut flash_device = ChipSelectDevice {
        bus: &mut spi_controller,
        cs: 0,
        spi_monitor: Some(&mut spi_monitor0),
    };

    let nor_read_data: SpiNorData<'_> = nor_device_read_4b_data(SPI_CS0_CAPACITY);
    let nor_write_data = nor_device_write_4b_data(SPI_CS0_CAPACITY);
    let _ = flash_device.nor_read_init(&nor_read_data);
    let _ = flash_device.nor_write_init(&nor_write_data);

    if test_block_dev {
        match flash_device.nor_read_jedec_id() {
            Ok(id) => match NorFlashBlockDevice::from_jedec_id(flash_device, id) {
                Ok(mut blockdev) => test_block_device::<_>(&mut blockdev),
                Err(_e) => test_log!(uart, "start block device using jedec id failed"),
            },
            _ => {
                test_log!(uart, "Error:: Failed to read JEDEC ID");
            }
        }
    } else {
        test_read_jedec(uart, &mut flash_device);
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x0,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x100,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x200,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x200,
            0x40,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x1000,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x100,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x2000,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x400,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi0Cs0Idx,
            0x800,
            TEST_DATA_SIZE,
            true,
        );
        //  test_cs(uart, &mut flash_device, DeviceId::Spi0Cs0Idx,0x300, TEST_DATA_SIZE, true);
    }
    test_log!(uart, "################# SPI 1 TEST done ! ###############");
}

pub fn test_block_device<T: SpiNorDevice>(blockdev: &mut NorFlashBlockDevice<T>) {
    let peripherals = unsafe { Peripherals::steal() };
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut uartc = UartController::new(uart_regs);
    let addr = 0x0;

    uartc.init(&UartConfig::default()).unwrap();

    let testsize = 0x400;
    let wbuf: &mut [u8] = unsafe { SPI_NC_BUFFER[WRITE_IDX].as_mut_slice(0, testsize) };

    let rbuf: &mut [u8] = unsafe { SPI_NC_BUFFER[READ_IDX].as_mut_slice(0, testsize) };

    test_log!(
        uartc,
        "###########################page size: {} sector size: {} capacity:{:08X}",
        blockdev.program_size(),
        blockdev.erase_size(),
        blockdev.capacity()
    );

    //blockdev.read(norflashblockdevice::BlockAddrUsize(addr), rbuf);
    //test_log!(uartc, "read buffer:");
    //astdebug::print_array_u8(&mut uartc, rbuf);

    let range = BlockRange {
        start: BlockAddrUsize(0),
        count: 2,
    };
    let ptr_write: *mut u8 = wbuf.as_mut_ptr();
    let ptr_read: *mut u8 = rbuf.as_mut_ptr();

    test_log!(uartc, "write pointer {:p}", ptr_write);
    test_log!(uartc, "read pointer {:p}", ptr_read);

    let mut delay = DummyDelay {};
    test_log!(uartc, "########## start erase ");
    let _ = blockdev.erase(range);

    for (i, value) in wbuf.iter_mut().take(testsize).enumerate() {
        *value = u8::try_from(i % 255).unwrap();
    }
    delay.delay_ns(2_000_000);
    test_log!(
        uartc,
        "########## start block programming size: {:08x} ",
        testsize
    );
    match blockdev.program(norflashblockdevice::BlockAddrUsize(addr), wbuf) {
        Ok(()) => test_log!(uartc, "program successful"),
        Err(_e) => test_log!(uartc, "program failed"),
    }

    let _ = blockdev.read(norflashblockdevice::BlockAddrUsize(addr), rbuf);

    let result: bool;
    unsafe {
        result = core::slice::from_raw_parts(ptr_write, testsize)
            == core::slice::from_raw_parts(ptr_read, testsize);
    }
    if result {
        test_log!(uartc, "read write test passed!");
    } else {
        test_log!(uartc, "ERROR:: read write test failed!!");
        test_log!(uartc, "write buffer:");
        astdebug::print_array_u8(&mut uartc, wbuf);
        test_log!(uartc, "read buffer:");
        astdebug::print_array_u8(&mut uartc, rbuf);
        test_log!(uartc, "Mmap buffer: {:08x}", SPI0_MMAP_BASE + addr);
        astdebug::print_reg_u8(&mut uartc, SPI0_MMAP_BASE + addr, testsize);
    }
}

#[allow(clippy::too_many_lines)]
pub fn test_spi2(uart: &mut UartController<'_>) {
    let spi1 = unsafe { &*ast1060_pac::Spi1::ptr() };
    let current_cs = 0;
    let read_id = true;

    let base = core::ptr::from_ref(spi1) as usize;
    test_log!(uart, "SPI1 Base address = 0x{:08X}", base);

    let spi_data = SpiData {
        decode_addr: [SpiDecodeAddress { start: 0, len: 0 }; 5],
        cmd_mode: [CommandMode {
            normal_read: 0,
            normal_write: 0,
            user: 0,
        }; 5],
        hclk: 0,
        spim_proprietary_pre_config: 0,
    };
    test_log!(uart, "SPI1 PURE xfer Test Starts...");
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPIM2_PINCTRL0);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPIM3_PINCTRL0);
    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_SPI2_QUAD);

    test_log!(uart, "SPI1 set pinctrl");
    test_log!(uart, " SCU:: 0x{:08x}", SCU_BASE);

    let peripherals = unsafe { Peripherals::steal() };
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };

    let mut uart_controller = UartController::new(uart_regs);
    uart_controller.init(&UartConfig::default()).unwrap();

    let mut spi_controller = SpiController::new(
        spi1,
        current_cs,
        SPI1_CONFIG,
        spi_data,
        Some(&mut uart_controller),
    );

    let _result = spi_controller.init();
    astdebug::print_reg_u32(uart, SPI1_CTRL_BASE, 0xb0);
    let nor_read_data: SpiNorData<'_> = nor_device_read_4b_data(SPI_CS0_CAPACITY);
    let nor_write_data = nor_device_read_4b_data(SPI_CS0_CAPACITY);

    if true {
        let mut spi_monitor2 = start_spim2();
        // Wrap controller in a CS device (CS0)
        let mut flash_device = ChipSelectDevice {
            bus: &mut spi_controller,
            cs: 0,
            spi_monitor: Some(&mut spi_monitor2),
        };

        test_read_jedec(uart, &mut flash_device);

        let mut delay1 = DummyDelay {};

        if read_id {
            test_log!(uart, "Raw read Jedec ID:");
            let mut read_buf: [u8; 0x3] = [0u8; 3];
            let write_buf: [u8; 1] = [0x9f];
            let _ = flash_device.transfer(&mut read_buf, &write_buf);
            delay1.delay_ns(2_000_000);
            astdebug::print_array_u8(uart, &read_buf[..3]);
        }

        test_log!(uart, "####### SPI 2@0#######");
        let _ = flash_device.nor_read_init(&nor_read_data);
        delay1.delay_ns(2_000_000);
        let _ = flash_device.nor_write_init(&nor_write_data);
        delay1.delay_ns(2_000_000);
        //astdebug::print_reg_u32(uart, SCU_BASE + 0x690, 0x10);
        //astdebug::print_reg_u32(uart, GPIO_BASE , 0x40);
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi1Cs0Idx,
            0x0,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi1Cs0Idx,
            0x1000,
            TEST_DATA_SIZE,
            true,
        );
        test_cs(
            uart,
            &mut flash_device,
            DeviceId::Spi1Cs0Idx,
            0x2000,
            TEST_DATA_SIZE,
            true,
        );
    }
    {
        test_log!(uart, "####### SPI 2@1#######");
        //NOTE: When SPI2 controller accesses the SPI flash through SPIM3/4 output pins by configuring SCU0F0[3:0],
        // only CS0 decoding address area can be used within this scenario. Thus, CS is fixed to 0.
        let mut spi_monitor3 = start_spim3();
        let mut flash_device2 = ChipSelectDevice {
            bus: &mut spi_controller,
            cs: 0,
            spi_monitor: Some(&mut spi_monitor3),
        };

        let _ = flash_device2.nor_read_init(&nor_read_data);
        let _ = flash_device2.nor_write_init(&nor_write_data);

        test_cs(
            uart,
            &mut flash_device2,
            DeviceId::Spi1Cs1Idx,
            0x0,
            0x20,
            true,
        );
        test_cs(
            uart,
            &mut flash_device2,
            DeviceId::Spi1Cs1Idx,
            0x2000,
            TEST_DATA_SIZE,
            true,
        );
    }
    test_log!(uart, "################# SPI 2 TEST done ! ###############");
}

#[must_use]
pub fn start_spim0() -> SpiMonitor<Spipf> {
    let allow_cmds: [u8; 27] = [
        0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
        0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x15, 0x31, 0x3b, 0x3c,
    ];

    let read_blocked_regions = [RegionInfo {
        /*pfm*/
        start: 0x0400_0000,
        length: 0x0002_0000,
    }];

    let write_blocked_regions = [RegionInfo {
        start: 0x0000_0000,
        length: 0x0800_0000,
    }];
    let mut spi_monitor0 = SpiMonitor::<Spipf>::new(
        true,
        SpimExtMuxSel::SpimExtMuxSel1,
        &allow_cmds,
        u8::try_from(allow_cmds.len()).unwrap(),
        &read_blocked_regions,
        u8::try_from(read_blocked_regions.len()).unwrap(),
        &write_blocked_regions,
        u8::try_from(write_blocked_regions.len()).unwrap(),
    );
    spi_monitor0.spim_sw_rst();
    spi_monitor0.aspeed_spi_monitor_init();

    //TODO: when do we disable the mux?
    //spi_monitor0.spim_ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);
    spi_monitor0
    // print spim pointer value
}

#[must_use]
pub fn start_spim1() -> SpiMonitor<Spipf1> {
    let allow_cmds: [u8; 27] = [
        0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
        0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x15, 0x31, 0x3b, 0x3c,
    ];

    let write_blocked_regions = [RegionInfo {
        start: 0x0000_0000,
        length: 0x0800_0000,
    }];
    let mut spi_monitor1 = SpiMonitor::<Spipf1>::new(
        true,
        SpimExtMuxSel::SpimExtMuxSel1,
        &allow_cmds,
        u8::try_from(allow_cmds.len()).unwrap(),
        &[],
        0,
        &write_blocked_regions,
        u8::try_from(write_blocked_regions.len()).unwrap(),
    );
    spi_monitor1.spim_sw_rst();
    spi_monitor1.aspeed_spi_monitor_init();
    //spi_monitor1.spim_ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);

    spi_monitor1
}

#[must_use]
pub fn start_spim2() -> SpiMonitor<Spipf2> {
    let allow_cmds: [u8; 27] = [
        0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
        0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x15, 0x31, 0x3b, 0x3c,
    ];

    let write_blocked_regions = [RegionInfo {
        start: 0x0000_0000,
        length: 0x0800_0000,
    }];
    let mut spi_monitor2 = SpiMonitor::<Spipf2>::new(
        true,
        SpimExtMuxSel::SpimExtMuxSel1,
        &allow_cmds,
        u8::try_from(allow_cmds.len()).unwrap(),
        &[],
        0,
        &write_blocked_regions,
        u8::try_from(write_blocked_regions.len()).unwrap(),
    );
    spi_monitor2.spim_sw_rst();
    spi_monitor2.aspeed_spi_monitor_init();
    //spi_monitor2.spim_ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);

    spi_monitor2
}

#[must_use]
pub fn start_spim3() -> SpiMonitor<Spipf3> {
    let allow_cmds: [u8; 27] = [
        0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
        0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x15, 0x31, 0x3b, 0x3c,
    ];
    let read_blocked_regions: [RegionInfo; 3] = [
        RegionInfo {
            start: 0x0000_0000,
            length: 0x0001_0000,
        },
        RegionInfo {
            start: 0x0027_4000,
            length: 0x0000_4000,
        },
        RegionInfo {
            start: 0x01E0_0000,
            length: 0x0008_0000,
        },
    ];
    let write_blocked_regions: [RegionInfo; 3] = [
        RegionInfo {
            start: 0x0000_0000,
            length: 0x0001_0000,
        },
        RegionInfo {
            start: 0x013F_C000,
            length: 0x0002_8000,
        },
        RegionInfo {
            start: 0x0FFF_8000,
            length: 0x0000_8000,
        },
    ];
    let mut spi_monitor3 = SpiMonitor::<Spipf3>::new(
        true,
        SpimExtMuxSel::SpimExtMuxSel1,
        &allow_cmds,
        u8::try_from(allow_cmds.len()).unwrap(),
        &read_blocked_regions,
        u8::try_from(read_blocked_regions.len()).unwrap(),
        &write_blocked_regions,
        u8::try_from(write_blocked_regions.len()).unwrap(),
    );
    spi_monitor3.spim_sw_rst();
    spi_monitor3.aspeed_spi_monitor_init();
    //spi_monitor3.spim_ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);

    spi_monitor3
}
