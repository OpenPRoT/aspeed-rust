// Licensed under the Apache-2.0 license

use super::ast1060_i3c::HardwareInterface;
use super::i3c_config::I3cConfig;

pub const I3C_CCC_RSTDAA: u8 = 0x6;
pub const I3C_CCC_ENTDAA: u8 = 0x7;
pub const I3C_CCC_SETHID: u8 = 0x61;
pub const I3C_CCC_DEVCTRL: u8 = 0x62;
pub const I3C_CCC_SETDASA: u8 = 0x87;
pub const I3C_CCC_SETNEWDA: u8 = 0x88;
pub const I3C_CCC_GETPID: u8 = 0x8d;
pub const I3C_CCC_GETBCR: u8 = 0x8e;
pub const I3C_CCC_GETSTATUS: u8 = 0x90;

pub const I3C_CCC_EVT_INTR: u8 = 1 << 0;
pub const I3C_CCC_EVT_CR: u8 = 1 << 1;
pub const I3C_CCC_EVT_HJ: u8 = 1 << 3;
pub const I3C_CCC_EVT_ALL: u8 = I3C_CCC_EVT_INTR | I3C_CCC_EVT_CR | I3C_CCC_EVT_HJ;

#[derive(Debug)]
pub enum CccError {
    InvalidParam,
    NotFound,
    NoFreeSlot,
    Invalid,
}

#[derive(Debug)]
pub struct CccTargetPayload<'a> {
    /// Target 7‑bit dynamic address (left‑aligned; driver decides if LSB is R/W).
    pub addr: u8,
    /// `false` = write, `true` = read.
    pub rnw: bool,
    /// Data buffer for write (source) or read (destination).
    pub data: Option<&'a mut [u8]>,
    /// Actual bytes transferred (driver fills on return).
    pub num_xfer: usize,
}

#[derive(Debug)]
pub struct Ccc<'a> {
    pub id: u8,
    /// Optional CCC data immediately following the CCC byte.
    pub data: Option<&'a mut [u8]>,
    /// Actual bytes transferred (driver fills on return).
    pub num_xfer: usize,
}

/// One CCC transaction description.
#[derive(Debug)]
pub struct CccPayload<'a, 'b> {
    pub ccc: Option<Ccc<'a>>,
    /// Optional list of direct‑CCC target payloads.
    pub targets: Option<&'b mut [CccTargetPayload<'a>]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CccRstActDefByte {
    CccRstActNoReset = 0x0,
    CccRstActPeriphralOnly = 0x1,
    CccRstActResetWholeTarget = 0x2,
    CccRstActDebugNetworkAdapter = 0x3,
    CccRstActVirtualTargetDetect = 0x4,
}

impl CccRstActDefByte {
    #[inline]
    fn as_byte(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusFormat {
    Fmt1,
    Fmt2(GetStatusDefByte),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusDefByte {
    /// 0x00 - TGTSTAT
    TgtStat,
    /// 0x91 - PRECR
    Precr,
}

impl GetStatusDefByte {
    #[inline]
    fn as_byte(self) -> u8 {
        match self {
            GetStatusDefByte::TgtStat => 0x00,
            GetStatusDefByte::Precr => 0x91,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GetStatusResp {
    Fmt1 {
        status: u16,
    },
    Fmt2 {
        kind: GetStatusDefByte,
        raw_u16: u16,
    },
}

const fn i3c_ccc_enec(broadcast: bool) -> u8 {
    if broadcast {
        0x00
    } else {
        0x80
    }
}

const fn i3c_ccc_disec(broadcast: bool) -> u8 {
    if broadcast {
        0x01
    } else {
        0x81
    }
}

const fn i3c_ccc_rstact(broadcast: bool) -> u8 {
    if broadcast {
        0x2a
    } else {
        0x9a
    }
}

pub fn ccc_events_all_set<H>(hw: &mut H, config: &mut I3cConfig, enable: bool, events: u8) -> Result<(), CccError>
where
    H: HardwareInterface,
{
    let id = if enable {
        i3c_ccc_enec(true)
    } else {
        i3c_ccc_disec(true)
    };
    let rc = hw.do_ccc(
        config,
        &mut CccPayload {
            ccc: Some(Ccc {
                id,
                data: Some(&mut [events]),
                num_xfer: 0,
            }),
            targets: None,
        },
    );

    match rc {
        Ok(()) => Ok(()),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_events_set<H>( hw: &mut H, config: &mut I3cConfig, da: u8, enable: bool, events: u8,) -> Result<(), CccError>
where
    H: HardwareInterface,
{
    if da == 0 {
        return Err(CccError::InvalidParam);
    }

    let mut ev_buf = [events];
    let tgt = CccTargetPayload {
        addr: da,
        rnw: false,
        data: Some(&mut ev_buf[..]),
        num_xfer: 0,
    };

    let mut tgts = [tgt];
    let ccc_id = if enable {
        i3c_ccc_enec(false)
    } else {
        i3c_ccc_disec(false)
    };
    let ccc = Ccc {
        id: ccc_id,
        data: None,
        num_xfer: 0,
    };

    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) => Ok(()),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_rstact_all<H>(hw: &mut H, config: &mut I3cConfig, action: CccRstActDefByte) -> Result<(), CccError>
where
    H: HardwareInterface,
{
    let mut db = [action.as_byte()];
    let ccc = Ccc {
        id: i3c_ccc_rstact(true),
        data: Some(&mut db[..]),
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: None,
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) => Ok(()),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_getbcr<H>(hw: &mut H, config: &mut I3cConfig, dyn_addr: u8) -> Result<u8, CccError>
where
    H: HardwareInterface,
{
    if dyn_addr == 0 {
        return Err(CccError::InvalidParam);
    }

    let mut bcr_buf = [0u8; 1];

    let tgt = CccTargetPayload {
        addr: dyn_addr,
        rnw: true,
        data: Some(&mut bcr_buf[..]),
        num_xfer: 0,
    };
    let mut tgts = [tgt];

    let ccc = Ccc {
        id: I3C_CCC_GETBCR,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) => Ok(bcr_buf[0]),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_setnewda<H>(hw: &mut H, config: &mut I3cConfig, curr_da: u8, new_da: u8) -> Result<(), CccError>
where
    H: HardwareInterface,
{
    if curr_da == 0 || new_da == 0 {
        return Err(CccError::InvalidParam);
    }

    let pos = config.attached.pos_of_addr(curr_da);
    if pos.is_none() {
        return Err(CccError::NotFound);
    }

    if !config.addrbook.is_free(new_da) {
        return Err(CccError::NoFreeSlot);
    }
    let mut new_dyn_addr = (new_da & 0x7F) << 1;
    let tgt = CccTargetPayload {
        addr: curr_da,
        rnw: false,
        data: Some(core::slice::from_mut(&mut new_dyn_addr)),
        num_xfer: 0,
    };
    let mut tgts = [tgt];
    let ccc = Ccc {
        id: I3C_CCC_SETNEWDA,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) =>  Ok(()),
            _ => Err(CccError::Invalid),
    }
}

fn bytes_to_pid(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .take(6)
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
}

pub fn ccc_getpid<H>(hw: &mut H, config: &mut I3cConfig, dyn_addr: u8) -> Result<u64, CccError>
where
    H: HardwareInterface,
{
    let mut pid_buf = [0u8; 6];

    let tgt = CccTargetPayload {
        addr: dyn_addr,
        rnw: true,
        data: Some(&mut pid_buf[..]),
        num_xfer: 0,
    };
    let mut tgts = [tgt];

    let ccc = Ccc {
        id: I3C_CCC_GETPID,
        data: None,
        num_xfer: 0,
    };
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut tgts[..]),
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) => Ok(bytes_to_pid(&pid_buf)),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_getstatus<H>(
    hw: &mut H,
    config: &mut I3cConfig,
    da: u8,
    fmt: GetStatusFormat,
) -> Result<GetStatusResp, CccError>
where
    H: HardwareInterface,
{
    let mut data_buf = [0u8; 2];

    let mut defbyte_buf = [0u8; 1];

    let tgt = CccTargetPayload {
        addr: da,
        rnw: true,
        data: Some(&mut data_buf[..]),
        num_xfer: 0,
    };

    let mut ccc = Ccc {
        id: I3C_CCC_GETSTATUS,
        data: None,
        num_xfer: 0,
    };

    let kind_opt = match fmt {
        GetStatusFormat::Fmt1 => None,
        GetStatusFormat::Fmt2(kind) => {
            defbyte_buf[0] = kind.as_byte();
            ccc.data = Some(&mut defbyte_buf[..]);
            Some(kind)
        }
    };

    let mut targets_arr = [tgt];
    let mut payload = CccPayload {
        ccc: Some(ccc),
        targets: Some(&mut targets_arr[..]),
    };

    let rc = hw.do_ccc(config, &mut payload);
    match rc {
        Ok(()) => {
            let val = u16::from_be_bytes(data_buf);

            let resp = match kind_opt {
                None => GetStatusResp::Fmt1 { status: val },
                Some(kind) => GetStatusResp::Fmt2 { kind, raw_u16: val },
            };
            Ok(resp)
        }

        _ => Err(CccError::Invalid),
    }

}

pub fn ccc_getstatus_fmt1<H>(hw: &mut H, config: &mut I3cConfig, da: u8) -> Result<u16, CccError>
where
    H: HardwareInterface,
{
    match ccc_getstatus(hw, config, da, GetStatusFormat::Fmt1) {
        Ok(GetStatusResp::Fmt1 { status }) => Ok(status),
        _ => Err(CccError::Invalid),
    }
}

pub fn ccc_rstdaa_all<H>(hw: &mut H, config: &mut I3cConfig) -> Result<(), CccError>
where
    H: HardwareInterface,
{
    let rc = hw.do_ccc(
        config,
        &mut CccPayload {
            ccc: Some(Ccc {
                id: I3C_CCC_RSTDAA,
                data: None,
                num_xfer: 0,
            }),
            targets: None,
        },
    );
    match rc {
        Ok(()) => Ok(()),
        _ => Err(CccError::Invalid),
    }
}
