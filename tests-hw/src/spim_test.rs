// Licensed under the Apache-2.0 license

use ast1060_pac::{Spipf, Spipf1, Spipf2, Spipf3};

use aspeed_ddk::astdebug;
use aspeed_ddk::spimonitor::{RegionInfo, SpiMonitor, SpiMonitorInit, SpimExtMuxSel};
use aspeed_ddk::uart_core::UartController;
use embedded_io::Write;

#[allow(dead_code)]
pub const SPIM1_BASE: usize = 0x7e79_1000;
#[allow(dead_code)]
pub const SPIM2_BASE: usize = 0x7e79_2000;
#[allow(dead_code)]
pub const SPIM3_BASE: usize = 0x7e79_3000;
#[allow(dead_code)]
pub const SPIM4_BASE: usize = 0x7e79_4000;

//follow DTS configuration examples
pub fn test_spim0(uart: &mut UartController<'_>) {
    uart.write_all(b"\r\n####### SPIM0 setup #######\r\n")
        .unwrap();
    let allow_cmds: [u8; 27] = [
        0x03, 0x13, 0x0b, 0x0c, 0x6b, 0x6c, 0x01, 0x05, 0x35, 0x06, 0x04, 0x20, 0x21, 0x9f, 0x5a,
        0xb7, 0xe9, 0x32, 0x34, 0xd8, 0xdc, 0x02, 0x12, 0x15, 0x31, 0x3b, 0x3c,
    ];

    let read_blocked_regions = [RegionInfo {
        /*pfm*/
        start: 0x0300_0000,
        length: 0x0004_0000,
    }];

    let write_blocked_regions = [RegionInfo {
        start: 0x0000_0000,
        length: 0x0020_0000,
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
    spi_monitor0.sw_reset();
    spi_monitor0.init();
    spi_monitor0.ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);
    if false {
        astdebug::print_reg_u32(uart, SPIM1_BASE, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x70, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x100, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x300, 0x100);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x400, 0x80);
    }

    let scu_qspi_mux: &mut [u32] =
        unsafe { core::slice::from_raw_parts_mut((SPIM1_BASE) as *mut u32, 4) };
    scu_qspi_mux[0] = 0x5200_0004;
    if false {
        astdebug::print_reg_u32(uart, SPIM1_BASE, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x70, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x100, 0x80);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x200, 0x100);
        astdebug::print_reg_u32(uart, SPIM1_BASE + 0x300, 0x100);
    }

    // print spim pointer value
}

#[allow(dead_code)]
pub fn test_spim1() {
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
    spi_monitor1.sw_reset();
    spi_monitor1.init();
    spi_monitor1.ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);
}

#[allow(dead_code)]
pub fn test_spim2() {
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
    spi_monitor2.sw_reset();
    spi_monitor2.init();
    spi_monitor2.ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);
}

#[allow(dead_code)]
pub fn test_spim3(uart: &mut UartController<'_>) {
    uart.write_all(b"\r\n####### SPIM3 setup #######\r\n")
        .unwrap();
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
    spi_monitor3.sw_reset();
    spi_monitor3.init();
    spi_monitor3.ext_mux_config(SpimExtMuxSel::SpimExtMuxSel0);
}
