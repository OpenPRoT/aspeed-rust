// Licensed under the Apache-2.0 license

use crate::common::{DummyDelay, UartLogger};
use crate::i3c::config::I3cConfig;
use crate::i3c::config::I3cTargetConfig;
use crate::i3c::constants::I3C_MSG_READ;
use crate::i3c::constants::I3C_MSG_STOP;
use crate::i3c::constants::I3C_MSG_WRITE;
use crate::i3c::controller::I3cController;
use crate::i3c::hardware::Ast1060I3c;
use crate::i3c::hardware::HardwareCore;
use crate::i3c::hardware::HardwareTarget;
use crate::i3c::hardware::HardwareTransfer;
use crate::i3c::ibi::{i3c_ibi_workq_consumer, IbiWork};
use crate::i3c::types::I3cMsg;
use crate::pinctrl;
use crate::uart_core::{UartConfig, UartController};

use core::ptr::read_volatile;
use embedded_hal::delay::DelayNs;
use embedded_io::Write;
use proposed_traits::i3c_master::I3c;
// I3cTarget
use proposed_traits::i3c_target::{DynamicAddressable, IBICapable};

pub fn dump_i3c_controller_registers(uart: &mut UartController<'_>, base: u32) {
    // [7e7a4000] 80000200 00008009 000f40bb 00000000
    // [7e7a4010] 00000000 00000000 00000000 001f0000
    // [7e7a4020] 01010001 00000000 00000000 ffffffff
    unsafe {
        let reg_base = base as *mut u32;
        writeln!(uart, "rust I3C reg dump:\r").unwrap();
        for i in 0..0xc0 {
            let v = read_volatile(reg_base.add(i));
            if i.is_multiple_of(4) {
                write!(uart, "[{:08x}]", base + u32::try_from(i).unwrap() * 4).unwrap();
            }
            write!(uart, " {v:08x}").unwrap();
            if i % 4 == 3 {
                writeln!(uart, "\r").unwrap();
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn test_i3c_master(uart: &mut UartController<'_>) {
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut dbg_uart = UartController::new(uart_regs);

    writeln!(uart, "\r\n####### I3C master test #######\r\n").unwrap();
    dbg_uart.init(&UartConfig::default()).unwrap();

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_HVI3C2);
    let hw = Ast1060I3c::<ast1060_pac::I3c2, UartLogger>::new(UartLogger::new(&mut dbg_uart));

    // Use Builder Pattern for Configuration
    let mut config = I3cConfig::new()
        .core_clk_hz(200_000_000)
        .secondary(false)
        .i2c_scl_hz(1_000_000)
        .i3c_scl_hz(12_500_000)
        .i3c_pp_scl_hi_period_ns(250)
        .i3c_pp_scl_lo_period_ns(250)
        .i3c_od_scl_hi_period_ns(0)
        .i3c_od_scl_lo_period_ns(0)
        .sda_tx_hold_ns(20);

    config.init_runtime_fields();

    // Validate Clock Configuration (Requested Pattern)
    config
        .validate_clock()
        .expect("Invalid clock configuration");

    // Use Constructor that performs initialization
    let mut ctrl = I3cController::new(hw, config);
    ctrl.init_hardware();

    let mut ibi_cons = i3c_ibi_workq_consumer(ctrl.hw.bus_num() as usize).unwrap();
    // let known_pid = 0x07ec_0503_1000u64;
    let known_pid = 0x07ec_a003_2000u64;
    let ctrl_dev_slot0 = 0;

    // ctrl.init_hardware() is already called by I3cController::new()

    let dyn_addr = if let Some(da) = ctrl.config.addrbook.alloc_from(8) {
        ctrl.attach_i3c_dev(known_pid, da, ctrl_dev_slot0).unwrap();
        ctrl.hw.set_ibi_mdb(0); // If MDB was implied by previous 4th arg
        ctrl.hw.ibi_enable(&mut ctrl.config, da).unwrap();
        writeln!(uart, "Pre-attached dev at slot 0, dyn addr {da}\r").unwrap();
        da
    } else {
        writeln!(uart, "no dyn addr\r").unwrap();
        return;
    };

    writeln!(uart, "ctrl dev at slot 0, dyn addr {dyn_addr}\r").unwrap();
    let mut received_count = 0;

    loop {
        if let Some(work) = ibi_cons.dequeue() {
            match work {
                IbiWork::HotJoin => {
                    writeln!(uart, "[IBI] hotjoin\r").unwrap();
                    let _ = ctrl.handle_hot_join();
                    ctrl.assign_dynamic_address(dyn_addr).unwrap();
                }
                IbiWork::Sirq { addr, len, data } => {
                    writeln!(uart, "[IBI] SIRQ from 0x{addr:02x}, len {len}\r").unwrap();
                    writeln!(uart, "  IBI payload:").unwrap();
                    for i in 0..len {
                        write!(uart, " {:02x}", data[i as usize]).unwrap();
                    }
                    writeln!(uart, "\r").unwrap();
                    match ctrl.acknowledge_ibi(addr) {
                        Ok(()) => {}
                        Err(e) => {
                            writeln!(uart, "  acknowledge_ibi failed: {e:?}\r").unwrap();
                        }
                    }
                    let mut rx_buf = [0u8; 128];
                    let mut msgs = [I3cMsg {
                        buf: Some(&mut rx_buf[..]),
                        actual_len: 128,
                        num_xfer: 0,
                        flags: I3C_MSG_READ | I3C_MSG_STOP,
                        hdr_mode: 0,
                        hdr_cmd_mode: 0,
                    }];
                    let _ = ctrl.hw.priv_xfer(&mut ctrl.config, known_pid, &mut msgs);
                    let rx_len = msgs[0].actual_len as usize;
                    writeln!(
                        uart,
                        "[MASTER <== TARGET]  MASTER READ: {:02x?}",
                        &rx_buf[..rx_len]
                    )
                    .unwrap();
                    writeln!(uart, "\r").unwrap();
                    received_count += 1;
                    if received_count > 10 {
                        writeln!(uart, "I3C master test done\r").unwrap();
                        break;
                    }

                    // send data to target
                    let mut tx_buf: [u8; 16] = [
                        0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0x11, 0x22, 0x33, 0x44,
                        0x55, 0x66, 0x77, 0x88,
                    ];
                    let tx_len = u32::try_from(tx_buf.len()).unwrap();

                    let mut tx_msgs = [I3cMsg {
                        buf: Some(&mut tx_buf[..]),
                        actual_len: tx_len,
                        num_xfer: 0,
                        flags: I3C_MSG_WRITE | I3C_MSG_STOP,
                        hdr_mode: 0,
                        hdr_cmd_mode: 0,
                    }];
                    let _ = ctrl.hw.priv_xfer(&mut ctrl.config, known_pid, &mut tx_msgs);
                    writeln!(uart, "[MASTER ==> TARGET]  MASTER WRITE: {tx_buf:02x?}").unwrap();
                    writeln!(uart, "\r").unwrap();
                }
                IbiWork::TargetDaAssignment => {
                    writeln!(uart, "[IBI] TargetDaAssignment\r").unwrap();
                }
            }
        }
    }
}

pub fn test_i3c_target(uart: &mut UartController<'_>) {
    let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    let mut dbg_uart = UartController::new(uart_regs);

    writeln!(uart, "\r\n####### I3C target test #######\r\n").unwrap();
    dbg_uart.init(&UartConfig::default()).unwrap();

    pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_HVI3C2);
    let hw = Ast1060I3c::<ast1060_pac::I3c2, UartLogger>::new(UartLogger::new(&mut dbg_uart));

    let mut config = I3cConfig::new()
        .core_clk_hz(200_000_000)
        .secondary(true)
        .i2c_scl_hz(1_000_000)
        .i3c_scl_hz(12_500_000)
        .i3c_pp_scl_hi_period_ns(36)
        .i3c_pp_scl_lo_period_ns(36)
        .i3c_od_scl_hi_period_ns(0)
        .i3c_od_scl_lo_period_ns(0)
        .sda_tx_hold_ns(0)
        .dcr(0xcc)
        .target_config(I3cTargetConfig::new(0, Some(0), 0xae));

    config.init_runtime_fields();

    // Validate Clock Configuration
    config
        .validate_clock()
        .expect("Invalid clock configuration");

    let mut ctrl = I3cController::new(hw, config);
    ctrl.init_hardware();
    let mut ibi_cons = i3c_ibi_workq_consumer(ctrl.hw.bus_num() as usize).unwrap();

    // ctrl.init_hardware() is already called

    let dyn_addr = 8;
    let dev_idx = 0;
    let _ = ctrl.hw.attach_i3c_dev(dev_idx, dyn_addr);
    writeln!(
        uart,
        "I3C target dev at slot {dev_idx}, dyn addr {dyn_addr}\r"
    )
    .unwrap();
    // Dump I3C2 registers
    // dump_i3c_controller_registers(uart, 0x7e7a_4000);
    let mut delay = DummyDelay {};
    delay.delay_ns(4_000_000_000);
    writeln!(uart, "waiting for dynamic address assignment...\r").unwrap();
    let _ = ctrl.hw.target_ibi_raise_hj(&mut ctrl.config);
    // dump_i3c_controller_registers(uart, 0x7e7a_4000);
    loop {
        if let Some(work) = ibi_cons.dequeue() {
            match work {
                IbiWork::HotJoin => {
                    // do nothing in target mode
                    writeln!(uart, "[IBI] hotjoin\r").unwrap();
                }
                IbiWork::Sirq { addr, len, data: _ } => {
                    // do nothing in target mode
                    writeln!(uart, "[IBI] SIRQ from 0x{addr:02x}, len {len}\r").unwrap();
                }
                IbiWork::TargetDaAssignment => {
                    let mut delay = DummyDelay {};
                    delay.delay_ns(4_000_000_000);
                    writeln!(uart, "[IBI] TargetDaAssignment\r").unwrap();
                    let da = ctrl.config.target_config.as_ref().unwrap().addr;
                    writeln!(
                        uart,
                        "  dyn addr 0x{:02x} was assigned by master\r",
                        da.unwrap()
                    )
                    .unwrap();
                    ctrl.on_dynamic_address_assigned(da.unwrap());
                    break;
                }
            }
        }
    }
    let mut ibi_count = 0;
    loop {
        let mut delay = DummyDelay {};
        delay.delay_ns(4_000_000_000);
        let mut data = [0u8; 16];
        for (i, b) in data.iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap();
        }
        writeln!(uart, "send ibi #{ibi_count}\r").unwrap();
        writeln!(uart, "[MASTER <== TARGET] TARGET WRITE: {data:02x?}").unwrap();
        writeln!(uart, "\r").unwrap();
        ctrl.get_ibi_payload(&mut data).unwrap();
        ibi_count += 1;
        if ibi_count > 10 {
            break;
        }
    }

    writeln!(uart, "I3C target test done\r").unwrap();
}
