// Licensed under the Apache-2.0 license

use crate::common::{DummyDelay, Logger, UartLogger};
use crate::otp::common::{AspeedOtpRegion, StrapStatus};
use crate::otp::{
    OtpController, OTP_CONF_OFFSET, OTP_CONF_PROT_ENBLE, OTP_KEY_PROT_ENBLE, OTP_MEM_LOCK_ENBLE,
    OTP_SECURE_SIZE_BIT_POS, OTP_STRAP_PROT_ENBLE, OTP_USER_ECC_PROT_ENBLE,
};
use crate::uart::{self, Config, UartController};
use ast1060_pac::Peripherals;
use embedded_io::Write;
use proposed_traits::otp::OtpRegions;

/// OTPCFG in OTP memory
/// OTPCFG0-31
fn test_otp_read_conf<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    conf_reg: u32,
) {
    writeln!(uart, "########## test read OTPCFG ######\r").unwrap();

    let mut data: [u32; 32] = [0; 32];
    match otp.read_region(AspeedOtpRegion::Configuration, 0, &mut data) {
        Ok(()) => {
            for (i, each) in data.iter().enumerate() {
                writeln!(
                    uart,
                    "read OTPCFG{:#x} ok: {:#x} (PASS)\r",
                    conf_reg + u32::try_from(i).unwrap(),
                    each
                )
                .unwrap();
            }
        }
        Err(e) => {
            writeln!(uart, "read OTPCFG{conf_reg:#x} err: {e:?} (FAIL)\r").unwrap();
        }
    }
}
#[allow(clippy::too_many_lines)]
fn test_otp_write_conf_b<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    otp_addr: u32,
    otp_bit_offset: u32,
    otp_value: u32,
) {
    let mut conf0: u32 = 0;
    read_otp_conf(uart, otp, 0, &mut conf0);
    if conf0 & OTP_MEM_LOCK_ENBLE != 0 {
        writeln!(uart, "OTP memory is locked!\r").unwrap();
        return;
    }
    if otp_addr != 4 && otp_addr != 10 && otp_addr != 11 && otp_addr < 16 {
        if conf0 & OTP_CONF_PROT_ENBLE != 0 {
            writeln!(uart, "OTP config region is protected!\r").unwrap();
            return;
        }
    } else if otp_addr == 10 || otp_addr == 11 {
        let mut otp_rid: [u32; 2] = [0, 0];
        let mut sw_rid: [u32; 2] = [0, 0];
        read_otp_conf(uart, otp, 10, &mut otp_rid[0]);
        read_otp_conf(uart, otp, 11, &mut otp_rid[1]);
        otp.get_sw_revision(&mut sw_rid);

        if otp_addr == 10 {
            otp_rid[0] |= 1 << otp_bit_offset;
        } else {
            otp_rid[1] |= 1 << otp_bit_offset;
        }

        if otp_rid[1] > sw_rid[1] {
            writeln!(
                uart,
                "update revision id should not be bigger than current SW revision id!\r"
            )
            .unwrap();
            return;
        }
        if otp_rid[1] == sw_rid[1] || otp_rid[0] > sw_rid[0] {
            writeln!(
                uart,
                "update revision id should not be bigger than current SW revision id!\r"
            )
            .unwrap();
            return;
        }
    } else if otp_addr == 4 {
        if conf0 & OTP_KEY_PROT_ENBLE != 0 {
            writeln!(uart, "OTPCFG4 is protected!\r").unwrap();
            return;
        }
        if (otp_bit_offset <= 7) || (16..=23).contains(&otp_bit_offset) {
            let key_num = otp.get_key_count();
            let retire: u32 = if otp_bit_offset >= 16 {
                otp_bit_offset - 16
            } else {
                otp_bit_offset
            };
            if retire >= u32::from(key_num) {
                writeln!(
                    uart,
                    "retire key is equal or greater than current boot key!\r"
                )
                .unwrap();
                return;
            }
        }
    } else if (16..=31).contains(&otp_addr) {
        if conf0 & OTP_STRAP_PROT_ENBLE != 0 {
            writeln!(uart, "OTP strap region is protected!\r").unwrap();
            return;
        }
        if otp_addr < 28 {
            let mut otp_strap_pro: u32 = 0;
            if otp_addr % 2 == 0 {
                read_otp_conf(uart, otp, 30, &mut otp_strap_pro);
            } else {
                read_otp_conf(uart, otp, 31, &mut otp_strap_pro);
            }
            if otp_strap_pro >> otp_bit_offset & 1 != 0 {
                writeln!(
                    uart,
                    "OTPCFG{otp_addr:#x}[{otp_bit_offset:#x}] is protected!\r",
                )
                .unwrap();
            }
        }
    }
    let mut conf: u32 = 0;
    read_otp_conf(uart, otp, otp_addr, &mut conf);
    if (conf >> otp_bit_offset & 0x1) == otp_value {
        writeln!(
            uart,
            "OTPCFG{otp_addr:#x}[{otp_bit_offset:#x}] = 1; no need to program\r",
        )
        .unwrap();
        return;
    }
    if (conf >> otp_bit_offset & 0x1) == 1 && otp_value == 0 {
        writeln!(
            uart,
            "OTPCFG{otp_addr:#x}[{otp_bit_offset:#x}] = 1; cannot be cleared\r",
        )
        .unwrap();
        return;
    }
    let mut prog_addr: u32 = OTP_CONF_OFFSET;
    prog_addr |= (otp_addr / 8) * 0x200;
    prog_addr |= (otp_addr % 8) * 0x2;
    match otp.otp_prog_dc_b(otp_value, prog_addr, otp_bit_offset) {
        Ok(()) => {
            writeln!(
                uart,
                "program OTPCFG{otp_addr:#x}[{otp_bit_offset:#x}] successfully! (PASS)\r",
            )
            .unwrap();
        }
        Err(e) => {
            writeln!(uart, "program OTPCFG{otp_addr:#x} err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn test_otp_write_conf_d<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    offset: u8,
) {
    let data: [u32; 2] = [0xabcd_beef, 0x1234_5678];

    match otp.write_region(AspeedOtpRegion::Configuration, offset as usize, &data) {
        Ok(()) => {
            writeln!(uart, "write OTPCONF{offset:#x} success (PASS)\r").unwrap();
        }
        Err(e) => {
            writeln!(uart, "write OTPCONF{offset:#x} err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn read_otp_conf<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    idx: u32,
    conf: &mut u32,
) {
    let mut data = [0; 1];
    match otp.read_region(AspeedOtpRegion::Configuration, idx as usize, &mut data) {
        Ok(()) => {
            *conf = data[0];
        }
        Err(e) => {
            writeln!(uart, "read OTPCFG0x0 err: {e:?}\r").unwrap();
        }
    }
}

fn read_otp_data<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    offset: u32,
    data: &mut [u32],
) {
    if let Err(e) = otp.read_region(AspeedOtpRegion::Data, 0, data) {
        writeln!(uart, "read OTPDATA{offset:#x} err: {e:?}\r").unwrap();
    }
}

fn test_otp_write_strap_b<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    bit_offset: u8,
    value: u8,
) {
    writeln!(uart, "########## test write OTPSTRAP ######\r").unwrap();
    if bit_offset >= 64 || (value != 0 && value != 1) {
        writeln!(uart, "invalid offset or program value\r").unwrap();
        return;
    }

    let mut conf0: u32 = 0;
    read_otp_conf(uart, otp, 0, &mut conf0);
    if conf0 & OTP_MEM_LOCK_ENBLE != 0 {
        writeln!(uart, "OTP memory is locked!\r").unwrap();
        return;
    }

    if conf0 & OTP_STRAP_PROT_ENBLE != 0 {
        writeln!(uart, "OTP strap region is protected!\r").unwrap();
        return;
    }

    let mut strap_status: [StrapStatus; 64] = [StrapStatus {
        value: false,
        protected: false,
        options: [0; 7],
        remaining_writes: 6,
        writable_option: 0xff,
    }; 64];
    if let Err(e) = otp.otp_strap_status(&mut strap_status) {
        writeln!(uart, "otp_strap_status error: {e:?}\r").unwrap();
        return;
    }
    //test_otp_read_strap(uart, otp, u32::from(bit_offset), 1);
    if value == u8::from(strap_status[bit_offset as usize].value) {
        writeln!(uart, "The value is same as before.\r").unwrap();
        return;
    }
    if strap_status[bit_offset as usize].protected {
        writeln!(uart, "This bit is protected and is not writable.\r").unwrap();
        return;
    }
    if strap_status[bit_offset as usize].remaining_writes == 0 {
        writeln!(uart, "This bit has no remaining chance to write.\r").unwrap();
        return;
    }
    writeln!(
        uart,
        "Write 1 to OTPSTRAP[{:#x}] OPTION[{:#x}], that value changes from {:#x} to {:#x} \r",
        bit_offset,
        strap_status[bit_offset as usize].writable_option + 1,
        u8::from(strap_status[bit_offset as usize].value),
        u8::from(strap_status[bit_offset as usize].value) ^ 1
    )
    .unwrap();

    let mut prog_addr: u32 = OTP_CONF_OFFSET;
    let offset: u32;
    if bit_offset < 32 {
        offset = u32::from(bit_offset);
        prog_addr |=
            ((u32::from(strap_status[bit_offset as usize].writable_option) * 2 + 16) / 8) * 0x200;
        prog_addr |=
            ((u32::from(strap_status[bit_offset as usize].writable_option) * 2 + 16) % 8) * 0x2;
    } else {
        offset = u32::from(bit_offset) - 32;
        prog_addr |=
            ((u32::from(strap_status[bit_offset as usize].writable_option) * 2 + 17) / 8) * 0x200;
        prog_addr |=
            ((u32::from(strap_status[bit_offset as usize].writable_option) * 2 + 17) % 8) * 0x2;
    }
    match otp.otp_prog_dc_b(1, prog_addr, offset) {
        Ok(()) => {
            writeln!(uart, "program OTPSTRAP[{offset:#x}] successfully! (PASS)\r").unwrap();
        }
        Err(e) => {
            writeln!(uart, "program OTPSTRAP err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn test_otp_write_strap_d<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    start_bit: usize,
    strap: &[u32],
) {
    writeln!(uart, "OTPSTRAP start_bit {start_bit:}, strap = {strap:?}\r",).unwrap();
    match otp.write_region(AspeedOtpRegion::Strap, start_bit, strap) {
        Ok(()) => {
            writeln!(uart, "program OTPSTRAP dword success (PASS)\r").unwrap();
        }
        Err(e) => {
            writeln!(uart, "program OTPSTRAP dword err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn test_otp_read_strap<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    start: u32,
    count: u32,
) {
    writeln!(uart, "########## test read OTPSTRAP ######\r").unwrap();
    let remains: u32 = 6;
    let mut strap_status: [StrapStatus; 64] = [StrapStatus {
        value: false,
        protected: false,
        options: [0; 7],
        remaining_writes: 6,
        writable_option: 0xff,
    }; 64];
    writeln!(uart, "BIT(hex)  Value  Option             Status\r").unwrap();
    writeln!(uart, "------------------------------------------\r").unwrap();
    if otp.otp_strap_status(&mut strap_status).is_ok() {
        for i in start..start + count {
            write!(
                uart,
                "0x{:<8x} {:<7} ",
                i,
                u8::from(strap_status[i as usize].value)
            )
            .unwrap();
            for j in 0..remains {
                write!(uart, "{} ", strap_status[i as usize].options[j as usize]).unwrap();
            }
            write!(uart, "    ").unwrap();
            if strap_status[i as usize].protected {
                writeln!(uart, "protected and not writable").unwrap();
            } else {
                write!(uart, "not protected ").unwrap();
                if strap_status[i as usize].remaining_writes == 0 {
                    writeln!(uart, "and no remaining times to write.\r").unwrap();
                } else {
                    writeln!(
                        uart,
                        "and still can write {} times.\r",
                        strap_status[i as usize].remaining_writes
                    )
                    .unwrap();
                }
            }
        }
    } else {
        writeln!(uart, "read otp strap fail! (FAIL)\r").unwrap();
    }

    // trait API
    let mut buffer: [u32; 2] = [0, 0];
    match otp.read_region(AspeedOtpRegion::Strap, 0, &mut buffer) {
        Ok(()) => {
            writeln!(uart, "read OTPSTRAP {buffer:?} (PASS)\r").unwrap();
        }
        Err(e) => {
            writeln!(uart, "read OTPSTRAP err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn test_otp_write_data_b<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    otp_addr: u32,
    bit_offset: u32,
    value: u32,
) {
    let mut conf0: u32 = 0;
    read_otp_conf(uart, otp, 0, &mut conf0);
    if conf0 & OTP_MEM_LOCK_ENBLE != 0 {
        writeln!(uart, "OTP memory is locked!\r").unwrap();
        return;
    }
    let sec_area_size = (conf0 >> OTP_SECURE_SIZE_BIT_POS) & 0x3f;
    if sec_area_size == 0 {
        if conf0 & OTP_USER_ECC_PROT_ENBLE != 0 {
            writeln!(uart, "OTP data region is protected!\r").unwrap();
            return;
        }
    } else if otp_addr < sec_area_size && otp_addr > 16 {
        writeln!(
            uart,
            "OTP secure region is not readable, skip it to prevent unpredictable result!\r"
        )
        .unwrap();
        return;
    } else if otp_addr < sec_area_size {
        writeln!(uart, "OTP secure region is protected!\r").unwrap();
        return;
    } else if conf0 & OTP_USER_ECC_PROT_ENBLE != 0 {
        writeln!(uart, "OTP data region is protected!\r").unwrap();
        return;
    }

    let mut data: [u32; 2] = [0; 2];
    let otp_bit: u32;
    if otp_addr % 2 == 0 {
        read_otp_data(uart, otp, otp_addr, &mut data);
        otp_bit = (data[0] >> bit_offset) & 0x1;
        if otp_bit == 1 && value == 0 {
            writeln!(uart, "OTPDATA{otp_addr:#x}[{bit_offset:#x}] = 1\r").unwrap();
            writeln!(uart, "OTP is programmed, which can't be cleared!\r").unwrap();
            return;
        }
    } else {
        read_otp_data(uart, otp, otp_addr - 1, &mut data);
        otp_bit = (data[1] >> bit_offset) & 0x1;
        if otp_bit == 0 && value == 1 {
            writeln!(uart, "OTPDATA{otp_addr:#x}[{bit_offset:#x}] = 1\r").unwrap();
            writeln!(uart, "OTP is programmed, which can't be written!\r").unwrap();
            return;
        }
    }

    if otp_bit == value {
        writeln!(uart, "OTPDATA{otp_addr:#x}[{bit_offset:#x}] = {value}\r",).unwrap();
        writeln!(uart, "No need to program!\r").unwrap();
        return;
    }

    writeln!(
        uart,
        "Program OTPDATA{otp_addr:#x}[{bit_offset:#x}] to {value}\r",
    )
    .unwrap();

    match otp.otp_prog_dc_b(value, otp_addr, bit_offset) {
        Ok(()) => {
            writeln!(
                uart,
                "program OTPDATA{otp_addr:#x}[{bit_offset:#x}] successfully (PASS)!\r",
            )
            .unwrap();
        }
        Err(e) => {
            writeln!(
                uart,
                "program OTPDATA{otp_addr:#x}[{bit_offset:#x}] err: {e:?} (FAIL)\r",
            )
            .unwrap();
        }
    }
}

fn test_otp_write_data_d<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    otp_addr: u32,
    data: &[u32],
) {
    match otp.write_region(AspeedOtpRegion::Data, otp_addr as usize, data) {
        Ok(()) => {
            writeln!(uart, "write OTPDATA{otp_addr:#x} success (PASS)\r").unwrap();
        }
        Err(e) => {
            writeln!(uart, "write OTPDATA{otp_addr:#x} err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

fn test_otp_read_data<L: Logger>(
    uart: &mut UartController<'_>,
    otp: &mut OtpController<L>,
    conf_reg: u32,
) {
    writeln!(uart, "########## test read OTPDATA ######\r").unwrap();

    let mut data: [u32; 32] = [0; 32];
    match otp.read_region(AspeedOtpRegion::Data, 0, &mut data) {
        Ok(()) => {
            for (i, each) in data.iter().enumerate() {
                writeln!(
                    uart,
                    "read OTPDATA{:#x} ok: {:#x} (PASS)\r",
                    conf_reg + u32::try_from(i).unwrap(),
                    each
                )
                .unwrap();
            }
        }
        Err(e) => {
            writeln!(uart, "read OTPDATA{conf_reg:#x} err: {e:?} (FAIL)\r").unwrap();
        }
    }
}

pub fn test_otp(uart: &mut UartController<'_>) {
    let peripherals = unsafe { Peripherals::steal() };
    let scu = peripherals.scu;
    let mut delay = DummyDelay {};
    let mut dbg_uart = UartController::new(peripherals.uart, &mut delay);
    unsafe {
        dbg_uart.init(&Config {
            baud_rate: 115_200,
            word_length: uart::WordLength::Eight as u8,
            parity: uart::Parity::None,
            stop_bits: uart::StopBits::One,
            clock: 24_000_000,
        });
    }
    let mut otp_controller = OtpController::new(scu, UartLogger::new(&mut dbg_uart));
    //otp_controller.otp_unlock_reg();

    //test otpcfg
    if false {
        test_otp_read_conf(uart, &mut otp_controller, 0);
        test_otp_write_conf_b(uart, &mut otp_controller, 0, 5, 1);
        //test_otp_write_conf_b(uart, &mut otp_controller, 0, 5, 1);
        //test_otp_write_conf_b(uart, &mut otp_controller, 0, 5, 0);
        test_otp_write_conf_d(uart, &mut otp_controller, 5);
        test_otp_read_conf(uart, &mut otp_controller, 0);
    }

    //test otpstrap
    if false {
        test_otp_read_strap(uart, &mut otp_controller, 0, 32);
        for i in 1u8..8 {
            test_otp_write_strap_b(uart, &mut otp_controller, 0, i & 0x1);
        }

        let strap: [u32; 2] = [0xffff_ffff, 0xffff_ffff];
        test_otp_write_strap_d(uart, &mut otp_controller, 30, &strap);

        test_otp_read_strap(uart, &mut otp_controller, 0, 64);
    }

    //test otpdata
    if false {
        test_otp_read_data(uart, &mut otp_controller, 0);
        test_otp_write_data_b(uart, &mut otp_controller, 0, 0, 0);

        let data = [0xabcd_dead, 0x1234_5678];
        test_otp_write_data_d(uart, &mut otp_controller, 4, &data);
        test_otp_read_data(uart, &mut otp_controller, 0);
    }

    //otp_controller.otp_lock_reg();
}
