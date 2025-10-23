// Licensed under the Apache-2.0 license

use crate::common::{DummyDelay, Logger};
use crate::i3c::ccc::{
    ccc_events_set, CccPayload, I3C_CCC_DEVCTRL, I3C_CCC_ENTDAA, I3C_CCC_EVT_INTR, I3C_CCC_SETHID,
};
use crate::i3c::i3c_config::{Completion, I3cConfig};
use crate::i3c::ibi_workq;
use core::cell::RefCell;
use core::marker::PhantomData;
use core::ptr::read_volatile;
use core::sync::atomic::Ordering;
use cortex_m::peripheral::NVIC;
use critical_section::Mutex;
use embedded_hal::delay::DelayNs;

pub const I3C_MSG_WRITE: u8 = 0x0;
pub const I3C_MSG_READ: u8 = 0x1;
pub const I3C_MSG_STOP: u8 = 0x2;

pub const I3C_BUS_I2C_STD_TLOW_MIN_NS: u32 = 4_700;
pub const I3C_BUS_I2C_STD_THIGH_MIN_NS: u32 = 4_000;
pub const I3C_BUS_I2C_STD_TR_MAX_NS: u32 = 1_000;
pub const I3C_BUS_I2C_STD_TF_MAX_NS: u32 = 300;

pub const I3C_BUS_I2C_FM_TLOW_MIN_NS: u32 = 1_300;
pub const I3C_BUS_I2C_FM_THIGH_MIN_NS: u32 = 600;
pub const I3C_BUS_I2C_FM_TR_MAX_NS: u32 = 300;
pub const I3C_BUS_I2C_FM_TF_MAX_NS: u32 = 300;

pub const I3C_BUS_I2C_FMP_TLOW_MIN_NS: u32 = 500;
pub const I3C_BUS_I2C_FMP_THIGH_MIN_NS: u32 = 260;
pub const I3C_BUS_I2C_FMP_TR_MAX_NS: u32 = 120;
pub const I3C_BUS_I2C_FMP_TF_MAX_NS: u32 = 120;

pub const I3C_BUS_THIGH_MAX_NS: u32 = 41;

pub const NSEC_PER_SEC: u32 = 1_000_000_000;
pub const SDA_TX_HOLD_MIN: u32 = 0b001;
pub const SDA_TX_HOLD_MAX: u32 = 0b111;
pub const SDA_TX_HOLD_MASK: u32 = 0x0007_0000; // bits 18:16

pub const SLV_DCR_MASK: u32 = 0x0000_ff00;
pub const SLV_EVENT_CTRL: u32 = 0x38;
pub const SLV_EVENT_CTRL_MWL_UPD: u32 = bit(7);
pub const SLV_EVENT_CTRL_MRL_UPD: u32 = bit(6);
pub const SLV_EVENT_CTRL_HJ_REQ: u32 = bit(3);
pub const SLV_EVENT_CTRL_SIR_EN: u32 = bit(0);

pub const I3CG_REG1_SCL_IN_SW_MODE_VAL: u32 = bit(23);
pub const I3CG_REG1_SDA_IN_SW_MODE_VAL: u32 = bit(27);
pub const I3CG_REG1_SCL_IN_SW_MODE_EN: u32 = bit(28);
pub const I3CG_REG1_SDA_IN_SW_MODE_EN: u32 = bit(29);

pub const CM_TFR_STS_MASTER_HALT: u8 = 0xf;
pub const CM_TFR_STS_TARGET_HALT: u8 = 0x6;

pub const COMMAND_QUEUE_PORT: u32 = 0x0c;

// --- single-bit flags ---
pub const COMMAND_PORT_PEC: u32 = bit(31);
pub const COMMAND_PORT_TOC: u32 = bit(30);
pub const COMMAND_PORT_READ_TRANSFER: u32 = bit(28);
pub const COMMAND_PORT_SDAP: u32 = bit(27);
pub const COMMAND_PORT_ROC: u32 = bit(26);
pub const COMMAND_PORT_DBP: u32 = bit(25);
pub const COMMAND_PORT_CP: u32 = bit(15);
pub const COMMAND_PORT_SPEED: u32 = bits(23, 21);
pub const COMMAND_PORT_DEV_INDEX: u32 = bits(20, 16);
pub const COMMAND_PORT_CMD: u32 = bits(14, 7);
pub const COMMAND_PORT_TID: u32 = bits(6, 3);
pub const COMMAND_PORT_ARG_DB: u32 = bits(15, 8);
pub const COMMAND_PORT_ARG_DATA_LEN: u32 = bits(31, 16);
pub const COMMAND_PORT_ATTR: u32 = bits(2, 0);
pub const COMMAND_PORT_DEV_COUNT: u32 = bits(25, 21);

pub const TID_TARGET_IBI: u32 = 0x1;
pub const TID_TARGET_RD_DATA: u32 = 0x2;
pub const TID_TARGET_MASTER_WR: u32 = 0x8;
pub const TID_TARGET_MASTER_DEF: u32 = 0xf;

pub const COMMAND_ATTR_XFER_CMD: u32 = 0;
pub const COMMAND_ATTR_XFER_ARG: u32 = 1;
pub const COMMAND_ATTR_SHORT_ARG: u32 = 2;
pub const COMMAND_ATTR_ADDR_ASSGN_CMD: u32 = 3;
pub const COMMAND_ATTR_SLAVE_DATA_CMD: u32 = 0;

pub const DEV_ADDR_TABLE_LEGACY_I2C_DEV: u32 = bit(31);
pub const DEV_ADDR_TABLE_DYNAMIC_ADDR: u32 = bits(23, 16);
pub const DEV_ADDR_TABLE_MR_REJECT: u32 = bit(14);
pub const DEV_ADDR_TABLE_SIR_REJECT: u32 = bit(13);
pub const DEV_ADDR_TABLE_IBI_MDB: u32 = bit(12);
pub const DEV_ADDR_TABLE_IBI_PEC: u32 = bit(11);
pub const DEV_ADDR_TABLE_STATIC_ADDR: u32 = bits(6, 0);

pub const IBI_QUEUE_STATUS: u32 = 0x18;
pub const IBIQ_STATUS_IBI_ID: u32 = bits(15, 8);
pub const IBIQ_STATUS_IBI_ID_SHIFT: u32 = 8;
pub const IBIQ_STATUS_IBI_DATA_LEN: u32 = bits(7, 0);
pub const IBIQ_STATUS_IBI_DATA_LEN_SHIFT: u32 = 0;

pub const RESET_CTRL_IBI_QUEUE: u32 = bit(5);
pub const RESET_CTRL_RX_FIFO: u32 = bit(4);
pub const RESET_CTRL_TX_FIFO: u32 = bit(3);
pub const RESET_CTRL_RESP_QUEUE: u32 = bit(2);
pub const RESET_CTRL_CMD_QUEUE: u32 = bit(1);
pub const RESET_CTRL_SOFT: u32 = bit(0);

pub const RESET_CTRL_ALL: u32 = RESET_CTRL_IBI_QUEUE
    | RESET_CTRL_RX_FIFO
    | RESET_CTRL_TX_FIFO
    | RESET_CTRL_RESP_QUEUE
    | RESET_CTRL_CMD_QUEUE
    | RESET_CTRL_SOFT;

pub const RESET_CTRL_QUEUES: u32 = RESET_CTRL_IBI_QUEUE
    | RESET_CTRL_RX_FIFO
    | RESET_CTRL_TX_FIFO
    | RESET_CTRL_RESP_QUEUE
    | RESET_CTRL_CMD_QUEUE;

pub const RESET_CTRL_XFER_QUEUES: u32 =
    RESET_CTRL_RX_FIFO | RESET_CTRL_TX_FIFO | RESET_CTRL_RESP_QUEUE | RESET_CTRL_CMD_QUEUE;

pub const RESPONSE_QUEUE_PORT: u32 = 0x10;
pub const RESPONSE_PORT_ERR_STATUS_SHIFT: u32 = 28;
pub const RESPONSE_PORT_ERR_STATUS_MASK: u32 = genmask(31, 28);
pub const RESPONSE_PORT_TID_SHIFT: u32 = 24;
pub const RESPONSE_PORT_TID_MASK: u32 = genmask(27, 24);
pub const RESPONSE_PORT_DATA_LEN_SHIFT: u32 = 0;
pub const RESPONSE_PORT_DATA_LEN_MASK: u32 = genmask(15, 0);

pub const RESPONSE_NO_ERROR: u32 = 0;
pub const RESPONSE_ERROR_CRC: u32 = 1;
pub const RESPONSE_ERROR_PARITY: u32 = 2;
pub const RESPONSE_ERROR_FRAME: u32 = 3;
pub const RESPONSE_ERROR_IBA_NACK: u32 = 4;
pub const RESPONSE_ERROR_ADDRESS_NACK: u32 = 5;
pub const RESPONSE_ERROR_OVER_UNDER_FLOW: u32 = 6;
pub const RESPONSE_ERROR_TRANSF_ABORT: u32 = 8;
pub const RESPONSE_ERROR_I2C_W_NACK_ERR: u32 = 9;
pub const RESPONSE_ERROR_EARLY_TERMINATE: u32 = 10;
pub const RESPONSE_ERROR_PEC_ERR: u32 = 12;

pub const INTR_STATUS: u32 = 0x3c;
pub const INTR_STATUS_EN: u32 = 0x40;
pub const INTR_SIGNAL_EN: u32 = 0x44;
pub const INTR_FORCE: u32 = 0x48;

pub const INTR_BUSOWNER_UPDATE_STAT: u32 = bit(13);
pub const INTR_IBI_UPDATED_STAT: u32 = bit(12);
pub const INTR_READ_REQ_RECV_STAT: u32 = bit(11);
pub const INTR_DEFSLV_STAT: u32 = bit(10);
pub const INTR_TRANSFER_ERR_STAT: u32 = bit(9);
pub const INTR_DYN_ADDR_ASSGN_STAT: u32 = bit(8);
pub const INTR_CCC_UPDATED_STAT: u32 = bit(6);
pub const INTR_TRANSFER_ABORT_STAT: u32 = bit(5);
pub const INTR_RESP_READY_STAT: u32 = bit(4);
pub const INTR_CMD_QUEUE_READY_STAT: u32 = bit(3);
pub const INTR_IBI_THLD_STAT: u32 = bit(2);
pub const INTR_RX_THLD_STAT: u32 = bit(1);
pub const INTR_TX_THLD_STAT: u32 = bit(0);
pub const I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE: u32 = bit(2);

#[must_use]
pub const fn bit(n: u32) -> u32 {
    1 << n
}
#[must_use]
pub const fn bits(h: u32, l: u32) -> u32 {
    ((1u32 << (h - l + 1)) - 1) << l
}
#[must_use]
pub const fn field_prep(mask: u32, val: u32) -> u32 {
    (val << mask.trailing_zeros()) & mask
}
const fn field_get(val: u32, mask: u32, shift: u32) -> u32 {
    (val & mask) >> shift
}
const fn genmask(msb: u32, lsb: u32) -> u32 {
    let width = msb - lsb + 1;
    if width >= 32 {
        u32::MAX
    } else {
        ((1u32 << width) - 1) << lsb
    }
}

const MAX_CMDS: usize = 32;

#[derive(Debug)]
pub enum I3cDrvError {
    NoDatPos,
    NoMsgs,
    TooManyMsgs,
    InvalidArgs,
    Timeout,
    NoSuchDev,
    Access,
    IoError,
    Invalid,
}

#[derive(Clone, Copy)]
struct Handler {
    func: fn(usize),
    ctx: usize,
}

static BUS_HANDLERS: [Mutex<RefCell<Option<Handler>>>; 4] = [
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
];

pub fn register_i3c_irq_handler(bus: usize, func: fn(usize), ctx: usize) {
    assert!(bus < 4);
    critical_section::with(|cs| {
        *BUS_HANDLERS[bus].borrow(cs).borrow_mut() = Some(Handler { func, ctx });
    });
}

#[inline]
fn dispatch_irq(bus: usize) {
    critical_section::with(|cs| {
        if let Some(h) = *BUS_HANDLERS[bus].borrow(cs).borrow() {
            (h.func)(h.ctx);
        }
    });
}

#[no_mangle]
pub extern "C" fn i3c() {
    dispatch_irq(0);
}
#[no_mangle]
pub extern "C" fn i3c1() {
    dispatch_irq(1);
}
#[no_mangle]
pub extern "C" fn i3c2() {
    dispatch_irq(2);
}
#[no_mangle]
pub extern "C" fn i3c3() {
    dispatch_irq(3);
}

#[repr(u32)]
pub enum SpeedI3c {
    Sdr0 = 0x0,
    Sdr1 = 0x1,
    Sdr2 = 0x2,
    Sdr3 = 0x3,
    Sdr4 = 0x4,
    HdrTs = 0x5,
    HdrDdr = 0x6,
    I2cFmAsI3c = 0x7,
}

#[repr(u32)]
pub enum SpeedI2c {
    Fm = 0x0,
    Fmp = 0x1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tid {
    TargetIbi = 0x1,
    TargetRdData = 0x2,
    TargetMasterWr = 0x8,
    TargetMasterDef = 0xF,
}

pub enum I3cStatus {
    Ok,
    Timeout,
    Busy,
    Pending,
    Invalid,
}

#[derive(Debug)]
pub struct I3cCmd<'a> {
    pub cmd_lo: u32,
    pub cmd_hi: u32,
    pub tx: Option<&'a [u8]>,
    pub rx: Option<&'a mut [u8]>,
    pub tx_len: u32,
    pub rx_len: u32,
    pub ret: i32,
}

pub struct I3cMsg<'a> {
    pub buf: Option<&'a mut [u8]>,
    pub actual_len: u32,
    pub num_xfer: u32,
    pub flags: u8,
    pub hdr_mode: u8,
    pub hdr_cmd_mode: u8,
}

pub struct I3cXfer<'cmds, 'buf> {
    pub cmds: &'cmds mut [I3cCmd<'buf>],
    pub ret: i32,
    pub done: Completion,
}

impl<'cmds, 'buf> I3cXfer<'cmds, 'buf> {
    pub fn new(cmds: &'cmds mut [I3cCmd<'buf>]) -> Self {
        Self {
            cmds,
            ret: 0,
            done: Completion::new(),
        }
    }

    pub fn ncmds(&self) -> usize {
        self.cmds.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I3cPid(pub u64);

impl I3cPid {
    #[must_use]
    pub const fn manuf_id(self) -> u16 {
        ((self.0 >> 33) & 0x1FFF) as u16
    }
    #[must_use]
    pub const fn has_random_lower32(self) -> bool {
        (self.0 & (1u64 << 32)) != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I3cDeviceId {
    pub pid: I3cPid,
}

impl I3cDeviceId {
    #[must_use]
    pub const fn new(pid: u64) -> Self {
        Self { pid: I3cPid(pid) }
    }
}

pub const I3C_BROADCAST_ADDR: u8 = 0x7E;
pub const I3C_MAX_ADDR: u8 = 0x7F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I3cIbiType {
    TargetIntr,
    ControllerRoleRequest,
    HotJoin,
    WorkqueueCb,
}

#[derive(Clone, Copy, Debug)]
pub struct I3cIbi<'a> {
    pub ibi_type: I3cIbiType,
    pub payload: Option<&'a [u8]>,
}

impl<'a> I3cIbi<'a> {
    #[inline]
    #[must_use]
    pub fn payload_len(&self) -> u8 {
        self.payload.map_or(0, |p| {
            u8::try_from(p.len().min(u8::MAX as usize)).unwrap_or(u8::MAX)
        })
    }

    #[must_use]
    pub fn first_byte(&self) -> Option<u8> {
        self.payload.and_then(|p| p.first().copied())
    }
}

pub trait HardwareInterface {
    fn init(&mut self, config: &mut I3cConfig);
    fn bus_num(&self) -> u8;
    fn enable_irq(&mut self);
    fn disable_irq(&mut self);
    fn i3c_enable(&mut self, config: &I3cConfig);
    fn i3c_disable(&mut self, is_secondary: bool);
    fn core_reset_assert(&mut self, bus: u8);
    fn core_reset_deassert(&mut self, bus: u8);
    fn global_reset_assert(&mut self);
    fn global_reset_deassert(&mut self);
    fn clock_on(&mut self, bus: u8);
    fn set_role(&mut self, is_secondary: bool);
    fn init_clock(&mut self, config: &mut I3cConfig);
    fn get_clock_rate(&self) -> u32;
    fn calc_i2c_clk(&mut self, fscl_hz: u32) -> (u32, u32);
    fn init_pid(&mut self, config: &mut I3cConfig, bus: u8);
    fn enter_sw_mode(&mut self);
    fn exit_sw_mode(&mut self);
    fn i3c_toggle_scl_in(&mut self, count: u32);
    fn gen_internal_stop(&mut self);
    fn even_parity(byte: u8) -> bool;
    fn set_ibi_mdb(&mut self, mdb: u8);
    fn exit_halt(&mut self, config: &mut I3cConfig);
    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig);
    fn reset_ctrl(&mut self, reset: u32);
    fn wr_tx_fifo(&mut self, bytes: &[u8]);
    fn rd_fifo<F>(&mut self, read_word: F, out: &mut [u8])
    where
        F: FnMut() -> u32;
    fn drain_fifo<F>(&mut self, read_word: F, len: usize)
    where
        F: FnMut() -> u32;
    fn rd_rx_fifo(&mut self, out: &mut [u8]);
    fn rd_ibi_fifo(&mut self, out: &mut [u8]);
    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cDrvError>;
    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer);
    fn end_xfer(&mut self, config: &mut I3cConfig);
    fn get_addr_pos(&mut self, config: &I3cConfig, addr: u8) -> Option<u8>;
    fn detach_i3c_dev(&mut self, pos: usize);
    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cDrvError>;
    fn do_ccc(&mut self, config: &mut I3cConfig, ccc: &mut CccPayload) -> Result<(), I3cDrvError>;
    fn do_entdaa(&mut self, config: &mut I3cConfig, index: u32) -> Result<(), I3cDrvError>;
    fn priv_xfer_build_cmds<'a>(
        &mut self,
        cmds: &mut [I3cCmd<'a>],
        msgs: &mut [I3cMsg<'a>],
        pos: u8,
    ) -> Result<(), I3cDrvError>;
    fn priv_xfer(
        &mut self,
        config: &mut I3cConfig,
        pid: u64,
        msgs: &mut [I3cMsg],
    ) -> Result<(), I3cDrvError>;
    fn target_tx_write(&mut self, buf: &[u8]);
    fn handle_ibi_sir(&mut self, config: &mut I3cConfig, addr: u8, len: usize);
    fn handle_ibis(&mut self, config: &mut I3cConfig);
    fn i3c_aspeed_isr(&mut self, config: &mut I3cConfig);
    // target apis
    fn target_ibi_raise_hj(&self, config: &mut I3cConfig) -> Result<(), I3cDrvError>;
    fn target_handle_response_ready(&mut self, config: &mut I3cConfig);
    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cDrvError>;
    fn target_handle_ccc_update(&mut self, config: &mut I3cConfig);
}

pub trait Instance {
    fn ptr() -> *const ast1060_pac::i3c::RegisterBlock;
    fn ptr_global() -> *const ast1060_pac::i3cglobal::RegisterBlock;
    fn scu() -> *const ast1060_pac::scu::RegisterBlock;
    const BUS_NUM: u8;
}

macro_rules! macro_i3c {
    ($I3cx: ident, $x: literal) => {
        impl Instance for ast1060_pac::$I3cx {
            fn ptr() -> *const ast1060_pac::i3c::RegisterBlock {
                ast1060_pac::$I3cx::ptr()
            }

            fn ptr_global() -> *const ast1060_pac::i3cglobal::RegisterBlock {
                ast1060_pac::I3cglobal::ptr()
            }

            fn scu() -> *const ast1060_pac::scu::RegisterBlock {
                ast1060_pac::Scu::ptr()
            }
            const BUS_NUM: u8 = $x;
        }
    };
}

macro_i3c!(I3c, 0);
macro_i3c!(I3c1, 1);
macro_i3c!(I3c2, 2);
macro_i3c!(I3c3, 3);

pub struct Ast1060I3c<I3C: Instance, L: Logger> {
    pub i3c: &'static ast1060_pac::i3c::RegisterBlock,
    pub i3cg: &'static ast1060_pac::i3cglobal::RegisterBlock,
    pub scu: &'static ast1060_pac::scu::RegisterBlock,
    pub logger: L,
    _marker: PhantomData<I3C>,
}

impl<I3C: Instance, L: Logger> Ast1060I3c<I3C, L> {
    pub fn new(logger: L) -> Self {
        let i3c = unsafe { &*I3C::ptr() };
        let i3cg = unsafe { &*I3C::ptr_global() };
        let scu = unsafe { &*I3C::scu() };
        Self {
            i3c,
            i3cg,
            scu,
            logger,
            _marker: PhantomData,
        }
    }
}

macro_rules! i3c_debug {
    ($logger:expr, $($arg:tt)*) => {{
        use core::fmt::Write as _;
        let mut buf: heapless::String<128> = heapless::String::new();
        let _ = write!(&mut buf, $($arg)*);
        $logger.debug(buf.as_str());
    }};
}

#[allow(unused_macros)]
macro_rules! read_i3cg_reg0 {
    ($self:expr, $bus:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c010().read().bits(),
            1 => $self.i3cg.i3c020().read().bits(),
            2 => $self.i3cg.i3c030().read().bits(),
            3 => $self.i3cg.i3c040().read().bits(),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! read_i3cg_reg1 {
    ($self:expr, $bus:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c014().read().bits(),
            1 => $self.i3cg.i3c024().read().bits(),
            2 => $self.i3cg.i3c034().read().bits(),
            3 => $self.i3cg.i3c044().read().bits(),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! write_i3cg_reg0 {
    ($self:expr, $bus:expr, |$w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c010().write(|$w| $body),
            1 => $self.i3cg.i3c020().write(|$w| $body),
            2 => $self.i3cg.i3c030().write(|$w| $body),
            3 => $self.i3cg.i3c040().write(|$w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! write_i3cg_reg1 {
    ($self:expr, $bus:expr, |$w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c014().write(|$w| $body),
            1 => $self.i3cg.i3c024().write(|$w| $body),
            2 => $self.i3cg.i3c034().write(|$w| $body),
            3 => $self.i3cg.i3c044().write(|$w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

#[allow(unused_macros)]
macro_rules! modify_i3cg_reg0 {
    ($self:expr, $bus:expr, |$r:ident, $w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c010().modify(|$r, $w| $body),
            1 => $self.i3cg.i3c020().modify(|$r, $w| $body),
            2 => $self.i3cg.i3c030().modify(|$r, $w| $body),
            3 => $self.i3cg.i3c040().modify(|$r, $w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! modify_i3cg_reg1 {
    ($self:expr, $bus:expr, |$r:ident, $w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg.i3c014().modify(|$r, $w| $body),
            1 => $self.i3cg.i3c024().modify(|$r, $w| $body),
            2 => $self.i3cg.i3c034().modify(|$r, $w| $body),
            3 => $self.i3cg.i3c044().modify(|$r, $w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! i3c_dat_read {
    ($self:expr, $pos:expr) => {{
        match ($pos) {
            0 => $self.i3c.i3cd280().read().bits(),
            1 => $self.i3c.i3cd284().read().bits(),
            2 => $self.i3c.i3cd288().read().bits(),
            3 => $self.i3c.i3cd28c().read().bits(),
            4 => $self.i3c.i3cd290().read().bits(),
            5 => $self.i3c.i3cd294().read().bits(),
            6 => $self.i3c.i3cd298().read().bits(),
            7 => $self.i3c.i3cd29c().read().bits(),
            _ => 0,
        }
    }};
}

macro_rules! i3c_dat_write {
    ($self:expr, $pos:expr, |$w:ident| $body:expr) => {{
        match ($pos) {
            0 => {
                $self.i3c.i3cd280().write(|$w| $body);
            }
            1 => {
                $self.i3c.i3cd284().write(|$w| $body);
            }
            2 => {
                $self.i3c.i3cd288().write(|$w| $body);
            }
            3 => {
                $self.i3c.i3cd28c().write(|$w| $body);
            }
            4 => {
                $self.i3c.i3cd290().write(|$w| $body);
            }
            5 => {
                $self.i3c.i3cd294().write(|$w| $body);
            }
            6 => {
                $self.i3c.i3cd298().write(|$w| $body);
            }
            7 => {
                $self.i3c.i3cd29c().write(|$w| $body);
            }
            _ => { /* ignore */ }
        }
    }};
}

#[allow(unused_macros)]
macro_rules! i3c_dat_modify {
    ($self:expr, $pos:expr, |$r:ident, $w:ident| $body:expr) => {{
        match ($pos) {
            0 => {
                $self.i3c.i3cd280().modify(|$r, $w| $body);
            }
            1 => {
                $self.i3c.i3cd284().modify(|$r, $w| $body);
            }
            2 => {
                $self.i3c.i3cd288().modify(|$r, $w| $body);
            }
            3 => {
                $self.i3c.i3cd28c().modify(|$r, $w| $body);
            }
            4 => {
                $self.i3c.i3cd290().modify(|$r, $w| $body);
            }
            5 => {
                $self.i3c.i3cd294().modify(|$r, $w| $body);
            }
            6 => {
                $self.i3c.i3cd298().modify(|$r, $w| $body);
            }
            7 => {
                $self.i3c.i3cd29c().modify(|$r, $w| $body);
            }
            _ => { /* ignore */ }
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollError {
    Timeout,
}

pub fn poll_with_timeout<F, C, D>(
    mut read_reg: F,
    mut condition: C,
    delay: &mut D,
    delay_ns: u32,
    max_iters: u32,
) -> Result<u32, PollError>
where
    F: FnMut() -> u32,
    C: FnMut(u32) -> bool,
    D: embedded_hal::delay::DelayNs,
{
    for _ in 0..max_iters {
        let val = read_reg();
        if condition(val) {
            return Ok(val);
        }
        delay.delay_ns(delay_ns);
    }
    Err(PollError::Timeout)
}

impl<I3C: Instance, L: Logger> HardwareInterface for Ast1060I3c<I3C, L> {
    #[allow(clippy::too_many_lines)]
    fn init(&mut self, config: &mut I3cConfig) {
        i3c_debug!(self.logger, "i3c init");

        // Global reset is shared, so just need to deassert it
        self.global_reset_deassert();

        write_i3cg_reg1!(self, I3C::BUS_NUM, |w| unsafe {
            w.actmode()
                .bits(1)
                .instid()
                .bits(I3C::BUS_NUM)
                .staticaddr()
                .bits(0x74)
        });
        let reg = read_i3cg_reg1!(self, I3C::BUS_NUM);
        i3c_debug!(self.logger, "i3cg_reg1: {:#x}", reg);

        write_i3cg_reg0!(self, I3C::BUS_NUM, |w| unsafe { w.bits(0x0) });
        let reg = read_i3cg_reg0!(self, I3C::BUS_NUM);
        i3c_debug!(self.logger, "i3cg_reg0: {:#x}", reg);

        let mut delay = DummyDelay {};
        self.core_reset_assert(I3C::BUS_NUM);
        self.clock_on(I3C::BUS_NUM);
        self.core_reset_deassert(I3C::BUS_NUM);
        self.i3c_disable(config.is_secondary);
        unsafe {
            let scu090: u32 = 0x7e6e_2090;

            let reg: u32 = read_volatile(scu090 as *const u32);
            i3c_debug!(self.logger, "scu090: {:#x}", reg);

            let scu050: u32 = 0x7e6e_2050;

            let reg: u32 = read_volatile(scu050 as *const u32);
            i3c_debug!(self.logger, "scu050: {:#x}", reg);
        }

        i3c_debug!(
            self.logger,
            "bus num: {}, is_secondary: {}",
            I3C::BUS_NUM,
            config.is_secondary
        );
        // Reset controller
        self.i3c.i3cd034().write(|w| {
            w.ibiqueue_sw_rst()
                .set_bit()
                .rx_buffer_sw_rst()
                .set_bit()
                .tx_buffer_sw_rst()
                .set_bit()
                .response_queue_sw_rst()
                .set_bit()
                .cmd_queue_sw_rst()
                .set_bit()
                .core_sw_rst()
                .set_bit()
        });

        let _ = poll_with_timeout(
            || self.i3c.i3cd034().read().bits(),
            |val| val == 0,
            &mut delay,
            100_000,
            1_000_000,
        );

        self.set_role(config.is_secondary);
        self.init_clock(config);
        // init interrupt mask
        self.i3c.i3cd03c().write(|w| unsafe { w.bits(0xffff_ffff) });
        if config.is_secondary {
            self.i3c.i3cd040().write(|w| {
                w.transfererrstaten()
                    .set_bit()
                    .respreadystatintren()
                    .set_bit()
                    .cccupdatedstaten()
                    .set_bit()
                    .dynaddrassgnstaten()
                    .set_bit()
                    .ibiupdatedstaten()
                    .set_bit()
                    .readreqrecvstaten()
                    .set_bit()
            });

            self.i3c.i3cd044().write(|w| {
                w.transfererrsignalen()
                    .set_bit()
                    .respreadysignalintren()
                    .set_bit()
                    .cccupdatedsignalen()
                    .set_bit()
                    .dynaddrassgnsignalen()
                    .set_bit()
                    .ibiupdatedsignalen()
                    .set_bit()
                    .readreqrecvsignalen()
                    .set_bit()
            });
        } else {
            self.i3c.i3cd040().write(|w| {
                w.transfererrstaten()
                    .set_bit()
                    .respreadystatintren()
                    .set_bit()
            });

            self.i3c.i3cd044().write(|w| {
                w.transfererrsignalen()
                    .set_bit()
                    .respreadysignalintren()
                    .set_bit()
            });
        }

        config.sir_allowed_by_sw = false;

        // Init hardware queues
        self.i3c
            .i3cd01c()
            .write(|w| unsafe { w.ibidata_threshold_value().bits(31) });

        self.i3c
            .i3cd020()
            .modify(|_, w| unsafe { w.rx_buffer_threshold_value().bits(0) });

        // Init PID and DCR for target/secondary mode
        self.init_pid(config, I3C::BUS_NUM);

        // Get max device and DAT start addr
        config.maxdevs = self.i3c.i3cd05c().read().devaddrtabledepth().bits();
        config.free_pos = if config.maxdevs == 32 {
            u32::MAX
        } else {
            (1u32 << config.maxdevs) - 1
        };
        config.need_da = 0;

        // Init DAT
        for i in 0..(config.maxdevs) {
            i3c_dat_write!(self, i, |w| {
                w.sirreject().set_bit().mrreject().set_bit()
            });
        }

        self.i3c.i3cd02c().write(|w| unsafe { w.bits(0xffff_ffff) });
        self.i3c.i3cd030().write(|w| unsafe { w.bits(0xffff_ffff) });
        self.i3c
            .i3cd000()
            .modify(|_, w| w.hot_join_ack_nack_ctrl().set_bit());

        if config.is_secondary {
            self.i3c
                .i3cd004()
                .write(|w| unsafe { w.dev_static_addr().bits(9).static_addr_valid().set_bit() });
        } else {
            self.i3c
                .i3cd004()
                .write(|w| unsafe { w.dev_dynamic_addr().bits(8).dynamic_addr_valid().set_bit() });
        }

        self.i3c_enable(config);

        i3c_debug!(self.logger, "i3c enabled");
        // Enable hot-join
        if !config.is_secondary {
            self.i3c
                .i3cd040()
                .modify(|_, w| w.ibithldstaten().set_bit());
            self.i3c
                .i3cd044()
                .modify(|_, w| w.ibithldsignalen().set_bit());
        }
        self.i3c
            .i3cd000()
            .modify(|_, w| w.hot_join_ack_nack_ctrl().clear_bit());
        i3c_debug!(self.logger, "i3c init done");
        i3c_debug!(
            self.logger,
            "i3c i3cd000: {:#x}",
            self.i3c.i3cd000().read().bits()
        );
    }

    fn bus_num(&self) -> u8 {
        I3C::BUS_NUM
    }

    fn enable_irq(&mut self) {
        unsafe {
            match I3C::BUS_NUM {
                0 => NVIC::unmask(ast1060_pac::Interrupt::i3c),
                1 => NVIC::unmask(ast1060_pac::Interrupt::i3c1),
                2 => NVIC::unmask(ast1060_pac::Interrupt::i3c2),
                3 => NVIC::unmask(ast1060_pac::Interrupt::i3c3),
                _ => {}
            }
        }
    }

    fn disable_irq(&mut self) {
        match I3C::BUS_NUM {
            0 => NVIC::mask(ast1060_pac::Interrupt::i3c),
            1 => NVIC::mask(ast1060_pac::Interrupt::i3c1),
            2 => NVIC::mask(ast1060_pac::Interrupt::i3c2),
            3 => NVIC::mask(ast1060_pac::Interrupt::i3c3),
            _ => {}
        }
    }

    fn i3c_disable(&mut self, is_secondary: bool) {
        i3c_debug!(self.logger, "i3c disable");
        if self.i3c.i3cd000().read().enbl_i3cctrl().bit_is_clear() {
            return;
        }

        if is_secondary {
            // enter sw mode
            self.enter_sw_mode();
        }
        self.i3c
            .i3cd000()
            .modify(|_, w| w.enbl_i3cctrl().clear_bit());

        if is_secondary {
            self.i3c_toggle_scl_in(8);
            self.gen_internal_stop();
            self.exit_sw_mode();
        }
    }

    fn core_reset_assert(&mut self, bus: u8) {
        match bus {
            0 => self.scu.scu050().modify(|_, w| w.rst_i3c0ctrl().set_bit()),
            1 => self.scu.scu050().modify(|_, w| w.rst_i3c1ctrl().set_bit()),
            2 => self.scu.scu050().modify(|_, w| w.rst_i3c2ctrl().set_bit()),
            3 => self.scu.scu050().modify(|_, w| w.rst_i3c3ctrl().set_bit()),
            _ => panic!("invalid I3C bus index: {bus}"),
        };
    }

    fn core_reset_deassert(&mut self, bus: u8) {
        let mask = 1u32 << (8 + u32::from(bus));
        self.scu
            .scu054()
            .modify(|_, w| unsafe { w.scu050sys_rst_ctrl_clear_reg2().bits(mask) });
    }

    fn global_reset_assert(&mut self) {
        self.scu
            .scu050()
            .modify(|_, w| w.rst_i3cregdmactrl().set_bit());
    }

    fn global_reset_deassert(&mut self) {
        self.scu
            .scu054()
            .modify(|_, w| unsafe { w.scu050sys_rst_ctrl_clear_reg2().bits(0x80) });
    }

    fn clock_on(&mut self, bus: u8) {
        let mask = 1u32 << (8 + u32::from(bus));
        self.scu
            .scu094()
            .modify(|_, w| unsafe { w.scu090clk_stop_ctrl_clear_reg_set2().bits(mask) });
    }

    fn set_role(&mut self, is_secondary: bool) {
        if is_secondary {
            self.i3c
                .i3cd0b0()
                .modify(|_, w| unsafe { w.dev_op_mode().bits(1) });
        } else {
            self.i3c
                .i3cd0b0()
                .modify(|_, w| unsafe { w.dev_op_mode().bits(0) });
        }
    }

    fn init_clock(&mut self, config: &mut I3cConfig) {
        let clk_rate = self.get_clock_rate();
        i3c_debug!(self.logger, "i3c clock rate: {} Hz", clk_rate);
        config.core_period = (NSEC_PER_SEC).div_ceil(clk_rate);

        let ns_to_cnt_u8 =
            |ns: u32| -> u8 { u8::try_from(ns.div_ceil(config.core_period)).unwrap_or(u8::MAX) };
        let ns_to_cnt_u16 =
            |ns: u32| -> u16 { u16::try_from(ns.div_ceil(config.core_period)).unwrap_or(u16::MAX) };

        // I2C FM
        let (fm_hi_ns, fm_lo_ns) = self.calc_i2c_clk(config.i2c_scl_hz);
        self.i3c.i3cd0bc().write(|w| unsafe {
            w.i2cfmhcnt()
                .bits(ns_to_cnt_u16(fm_hi_ns))
                .i2cfmlcnt()
                .bits(ns_to_cnt_u16(fm_lo_ns))
        });

        // I2C FMP
        let (i2c_fmp_hi_ns, i2c_fmp_lo_ns) = self.calc_i2c_clk(1_000_000);
        self.i3c.i3cd0c0().write(|w| unsafe {
            w.i2cfmphcnt()
                .bits(ns_to_cnt_u8(i2c_fmp_hi_ns))
                .i2cfmplcnt()
                .bits(ns_to_cnt_u16(i2c_fmp_lo_ns))
        });

        // I3C OD
        let (od_hi_ns, od_lo_ns) =
            if config.i3c_od_scl_hi_period_ns != 0 && config.i3c_od_scl_lo_period_ns != 0 {
                (
                    config.i3c_od_scl_hi_period_ns,
                    config.i3c_od_scl_lo_period_ns,
                )
            } else {
                (i2c_fmp_hi_ns, i2c_fmp_lo_ns)
            };
        self.i3c.i3cd0b4().write(|w| unsafe {
            w.i3codhcnt()
                .bits(ns_to_cnt_u8(od_hi_ns))
                .i3codlcnt()
                .bits(ns_to_cnt_u8(od_lo_ns))
        });

        // I3C PP
        let (i3c_pp_hi_ns, i3c_pp_lo_ns) =
            if config.i3c_pp_scl_hi_period_ns != 0 && config.i3c_pp_scl_lo_period_ns != 0 {
                (
                    config.i3c_pp_scl_hi_period_ns,
                    config.i3c_pp_scl_lo_period_ns,
                )
            } else {
                let total_ns = NSEC_PER_SEC.div_ceil(config.i3c_scl_hz.max(1));
                let hi_ns = core::cmp::min(I3C_BUS_THIGH_MAX_NS, total_ns.saturating_sub(1));
                let lo_ns = total_ns.saturating_sub(hi_ns).max(1);
                (hi_ns, lo_ns)
            };
        self.i3c.i3cd0b8().write(|w| unsafe {
            w.i3cpphcnt()
                .bits(ns_to_cnt_u8(i3c_pp_hi_ns))
                .i3cpplcnt()
                .bits(ns_to_cnt_u8(i3c_pp_lo_ns))
        });

        // SDA TX hold time
        let hold_steps = (config.sda_tx_hold_ns)
            .div_ceil(config.core_period)
            .clamp(SDA_TX_HOLD_MIN, SDA_TX_HOLD_MAX);
        let mut reg = self.i3c.i3cd0d0().read().bits();
        reg = (reg & !SDA_TX_HOLD_MASK) | ((hold_steps & 0x7) << 16);
        self.i3c.i3cd0d0().write(|w| unsafe { w.bits(reg) });

        // BUS_FREE_TIMING
        self.i3c.i3cd0d4().write(|w| unsafe { w.bits(0xffff_007c) });
    }

    fn get_clock_rate(&self) -> u32 {
        200_000_000
    }

    fn calc_i2c_clk(&mut self, fscl_hz: u32) -> (u32, u32) {
        use core::cmp::max;

        debug_assert!(fscl_hz > 0);
        let period_ns: u32 = (1_000_000_000u32).div_ceil(fscl_hz);

        let (lo_min, hi_min): (u32, u32) = if fscl_hz <= 100_000 {
            (
                (I3C_BUS_I2C_STD_TLOW_MIN_NS + I3C_BUS_I2C_STD_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_STD_THIGH_MIN_NS + I3C_BUS_I2C_STD_TR_MAX_NS).div_ceil(period_ns),
            )
        } else if fscl_hz <= 400_000 {
            (
                (I3C_BUS_I2C_FM_TLOW_MIN_NS + I3C_BUS_I2C_FM_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_FM_THIGH_MIN_NS + I3C_BUS_I2C_FM_TR_MAX_NS).div_ceil(period_ns),
            )
        } else {
            (
                (I3C_BUS_I2C_FMP_TLOW_MIN_NS + I3C_BUS_I2C_FMP_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_FMP_THIGH_MIN_NS + I3C_BUS_I2C_FMP_TR_MAX_NS).div_ceil(period_ns),
            )
        };

        let leftover = period_ns.saturating_sub(lo_min + hi_min);
        let lo = lo_min + leftover / 2;
        let hi = max(period_ns.saturating_sub(lo), hi_min);

        (hi as u32, lo)
    }

    fn init_pid(&mut self, config: &mut I3cConfig, bus: u8) {
        self.i3c
            .i3cd070()
            .write(|w| unsafe { w.slvmipimfgid().bits(0x3f6).slvpiddcr().clear_bit() });

        let rev_id: u32 = self.scu.scu004().read().hw_rev_id().bits().into();
        let mut reg: u32 = rev_id << 16 | u32::from(bus) << 12;
        reg |= 0xa000_0000;
        self.i3c.i3cd074().write(|w| unsafe { w.bits(reg) });
        let mut reg: u32 = self.i3c.i3cd078().read().bits();
        reg &= !SLV_DCR_MASK;
        reg |= (config.dcr << 8) | 0x66;
        self.i3c.i3cd078().write(|w| unsafe { w.bits(reg) });
    }

    fn enter_sw_mode(&mut self) {
        i3c_debug!(self.logger, "enter sw mode");
        let bus = I3C::BUS_NUM;
        let mut reg = read_i3cg_reg1!(self, bus);
        reg |= I3CG_REG1_SCL_IN_SW_MODE_VAL | I3CG_REG1_SDA_IN_SW_MODE_VAL;
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
        reg |= I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN;
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
    }

    fn exit_sw_mode(&mut self) {
        let bus = I3C::BUS_NUM;
        let mut reg = read_i3cg_reg1!(self, bus);
        reg &= !(I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN);
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
    }

    fn i3c_enable(&mut self, config: &I3cConfig) {
        i3c_debug!(self.logger, "i3c enable");
        if config.is_secondary {
            i3c_debug!(self.logger, "i3c enable as secondary");
            self.i3c.i3cd038().write(|w| unsafe { w.bits(0) });
            self.enter_sw_mode();
            // Enable hot-join
            self.i3c.i3cd000().modify(|_, w| {
                w.enbl_adaption_of_i2ci3cmode()
                    .clear_bit()
                    .ibipayloaden()
                    .set_bit()
                    .enbl_i3cctrl()
                    .set_bit()
            });
            let wait_cnt = &self.i3c.i3cd0d4().read().i3cibifree().bits();
            let wait_ns = u32::from(*wait_cnt) * config.core_period;
            let mut delay = DummyDelay {};
            delay.delay_ns(wait_ns * 100_u32);
            self.i3c_toggle_scl_in(8);
            if self.i3c.i3cd000().read().enbl_i3cctrl().bit_is_set() {
                self.gen_internal_stop();
            }
            self.exit_sw_mode();
        } else {
            self.i3c.i3cd000().modify(|_, w| {
                w.i3cbroadcast_addr_include()
                    .set_bit()
                    .enbl_i3cctrl()
                    .set_bit()
            });
        }
    }

    fn i3c_toggle_scl_in(&mut self, count: u32) {
        let bus = I3C::BUS_NUM;
        for _ in 0..count {
            modify_i3cg_reg1!(self, bus, |r, w| unsafe {
                w.bits(r.bits() & !I3CG_REG1_SCL_IN_SW_MODE_VAL)
            });
            modify_i3cg_reg1!(self, bus, |r, w| unsafe {
                w.bits(r.bits() | I3CG_REG1_SCL_IN_SW_MODE_VAL)
            });
        }
    }

    fn gen_internal_stop(&mut self) {
        let bus = I3C::BUS_NUM;
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() & !I3CG_REG1_SCL_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() & !I3CG_REG1_SDA_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() | I3CG_REG1_SCL_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() | I3CG_REG1_SDA_IN_SW_MODE_VAL)
        });
    }

    fn even_parity(byte: u8) -> bool {
        let mut parity = false;
        let mut b = byte;

        while b != 0 {
            parity = !parity;
            b &= b - 1;
        }

        !parity
    }

    fn set_ibi_mdb(&mut self, mdb: u8) {
        self.i3c
            .i3cd000()
            .modify(|_, w| unsafe { w.mdb().bits(mdb) });
    }

    fn exit_halt(&mut self, config: &mut I3cConfig) {
        let state = self.i3c.i3cd054().read().cmtfrstatus().bits();
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if state != expected {
            return;
        }

        self.i3c.i3cd000().modify(|_, w| w.i3cresume().set_bit());

        let rc = poll_with_timeout(
            || u32::from(self.i3c.i3cd054().read().cmtfrstatus().bits()),
            |val| val != u32::from(expected),
            &mut DummyDelay {},
            10000,
            1_000_000,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "exit_halt: timeout");
        }
    }

    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig) {
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if by_sw {
            self.i3c.i3cd000().modify(|_, w| w.i3cabort().set_bit());
        }

        let rc = poll_with_timeout(
            || u32::from(self.i3c.i3cd054().read().cmtfrstatus().bits()),
            |val| val == u32::from(expected),
            &mut DummyDelay {},
            10000,
            1_000_000,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "enter_halt: timeout");
        }
    }

    fn reset_ctrl(&mut self, reset: u32) {
        let reg = reset & RESET_CTRL_ALL;

        if reg == 0 {
            return;
        }

        self.i3c.i3cd034().write(|w| unsafe { w.bits(reg) });
        let rc = poll_with_timeout(
            || self.i3c.i3cd034().read().bits(),
            |val| val == 0,
            &mut DummyDelay {},
            10_000,
            1_000_000,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "reset_ctrl: timeout");
        }
    }

    fn wr_tx_fifo(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(4);
        for chunk in &mut chunks {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            self.i3c
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }

        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut tmp = [0u8; 4];
            tmp[..rem.len()].copy_from_slice(rem);
            let word = u32::from_le_bytes(tmp);
            self.i3c
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }
    }

    fn rd_fifo<F>(&mut self, mut read_word: F, out: &mut [u8])
    where
        F: FnMut() -> u32,
    {
        let mut chunks = out.chunks_exact_mut(4);
        for chunk in &mut chunks {
            let val = read_word();
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let val = read_word();
            let bytes = val.to_le_bytes();
            rem.copy_from_slice(&bytes[..rem.len()]);
        }
    }

    fn rd_rx_fifo(&mut self, out: &mut [u8]) {
        self.rd_fifo(|| self.i3c.i3cd014().read().rx_data_port().bits(), out);
    }

    fn rd_ibi_fifo(&mut self, out: &mut [u8]) {
        self.rd_fifo(|| self.i3c.i3cd018().read().bits(), out);
    }

    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cDrvError> {
        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(addr)
            .ok_or(I3cDrvError::NoSuchDev)?;
        i3c_debug!(self.logger, "ibi_enable: dev_idx={}", dev_idx);
        let pos_opt = config
            .attached
            .pos_of(dev_idx)
            .or(config.attached.devices[dev_idx].pos);

        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;
        i3c_debug!(self.logger, "ibi_enable: pos={}", pos);
        let dev = &config.attached.devices[dev_idx];
        let tgt_bcr: u32 = u32::from(dev.bcr);
        let mut reg = i3c_dat_read!(self, u32::from(pos));
        reg &= !DEV_ADDR_TABLE_SIR_REJECT;
        if tgt_bcr & I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE != 0 {
            reg |= DEV_ADDR_TABLE_IBI_MDB | DEV_ADDR_TABLE_IBI_PEC;
        }

        i3c_dat_write!(self, pos, |w| unsafe { w.bits(reg) });

        let mut sir_reject = self.i3c.i3cd030().read().bits();
        sir_reject &= !bit(pos.into());
        self.i3c.i3cd030().write(|w| unsafe { w.bits(sir_reject) });

        self.i3c
            .i3cd040()
            .modify(|_, w| w.ibithldstaten().set_bit());

        self.i3c
            .i3cd044()
            .modify(|_, w| w.ibithldsignalen().set_bit());

        let events = I3C_CCC_EVT_INTR;
        let _ = ccc_events_set(self, config, dev.dyn_addr, true, events);

        i3c_debug!(self.logger, "i3cd030 (SIR reject) = {:#x}", sir_reject);
        i3c_debug!(
            self.logger,
            "i3cd040 (IBI thld) = {:#x}",
            self.i3c.i3cd040().read().bits()
        );
        i3c_debug!(
            self.logger,
            "i3cd044 (IBI thld sig) = {:#x}",
            self.i3c.i3cd044().read().bits()
        );
        i3c_debug!(
            self.logger,
            "i3cd280 dat_addr[{}] = {:#x}",
            pos,
            i3c_dat_read!(self, u32::from(pos))
        );
        i3c_debug!(self.logger, "ibi_enable done");
        Ok(())
    }

    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer) {
        let prev = config
            .curr_xfer
            .swap(core::ptr::from_mut(xfer).cast::<()>(), Ordering::AcqRel);
        if !prev.is_null() {
            i3c_debug!(self.logger, "start_xfer: previous xfer still in flight");
        }

        xfer.ret = -1;
        xfer.done.reset();

        for cmd in xfer.cmds.iter() {
            if let Some(tx) = cmd.tx {
                let take = tx.len().min(cmd.tx_len as usize);
                if take > 0 {
                    i3c_debug!(self.logger, "start_xfer: write {} bytes", take);
                    self.wr_tx_fifo(&tx[..take]);
                }
            }
        }
        self.i3c.i3cd01c().modify(|_, w| unsafe {
            w.response_buffer_threshold_value()
                .bits(u8::try_from(xfer.cmds.len().saturating_sub(1)).unwrap_or(0))
        });

        for cmd in xfer.cmds.iter() {
            i3c_debug!(
                self.logger,
                "start_xfer: cmd: cmd_hi={:#x}, cmd_lo={:#x}",
                cmd.cmd_hi,
                cmd.cmd_lo
            );
            self.i3c.i3cd00c().write(|w| unsafe { w.bits(cmd.cmd_hi) });
            self.i3c.i3cd00c().write(|w| unsafe { w.bits(cmd.cmd_lo) });
        }
    }

    fn end_xfer(&mut self, config: &mut I3cConfig) {
        let p = config
            .curr_xfer
            .swap(core::ptr::null_mut(), Ordering::AcqRel);
        if p.is_null() {
            i3c_debug!(self.logger, "end_xfer: no current xfer");
            return;
        }
        let xfer: &mut I3cXfer = unsafe { &mut *(p.cast::<I3cXfer>()) };

        let nresp = self.i3c.i3cd04c().read().respbufblr().bits() as usize;

        for _ in 0..nresp {
            let resp = self.i3c.i3cd010().read().bits();

            let tid = field_get(resp, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT) as usize;
            let rx_len = field_get(
                resp,
                RESPONSE_PORT_DATA_LEN_MASK,
                RESPONSE_PORT_DATA_LEN_SHIFT,
            ) as usize;
            let err = field_get(
                resp,
                RESPONSE_PORT_ERR_STATUS_MASK,
                RESPONSE_PORT_ERR_STATUS_SHIFT,
            );

            i3c_debug!(
                self.logger,
                "end_xfer: tid={}, rx_len={}, err={}",
                tid,
                rx_len,
                err
            );
            if tid >= xfer.cmds.len() {
                if rx_len > 0 {
                    self.drain_fifo(|| self.i3c.i3cd014().read().rx_data_port().bits(), rx_len);
                }
                continue;
            }

            let cmd = &mut xfer.cmds[tid];
            cmd.rx_len = u32::try_from(rx_len).unwrap_or(0);
            cmd.ret = i32::try_from(err).unwrap_or(-1);

            if rx_len == 0 {
                continue;
            }

            if err == 0 {
                if let Some(rx_buf) = cmd.rx.as_deref_mut() {
                    self.rd_rx_fifo(&mut rx_buf[..rx_len]);
                }
            }
        }
        let mut ret = 0;
        for i in 0..nresp {
            let r = xfer.cmds[i].ret;
            if r != 0 {
                ret = r;
            }
        }

        if ret != 0 {
            self.enter_halt(false, config);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.exit_halt(config);
        }

        xfer.ret = ret;
        xfer.done.complete();
    }

    fn get_addr_pos(&mut self, config: &I3cConfig, addr: u8) -> Option<u8> {
        config
            .addrs
            .iter()
            .take(config.maxdevs as usize)
            .position(|&a| a == addr)
            .and_then(|i| u8::try_from(i).ok())
    }

    fn detach_i3c_dev(&mut self, pos: usize) {
        i3c_dat_write!(self, pos, |w| {
            w.sirreject().set_bit().mrreject().set_bit()
        });
    }

    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cDrvError> {
        let mut da_with_parity = addr;
        if Self::even_parity(addr) {
            da_with_parity |= 1 << 7;
        }

        i3c_dat_write!(self, pos, |w| unsafe {
            w.sirreject()
                .set_bit()
                .mrreject()
                .set_bit()
                .devdynamicaddr()
                .bits(da_with_parity)
        });

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn do_ccc(
        &mut self,
        config: &mut I3cConfig,
        payload: &mut CccPayload<'_, '_>,
    ) -> Result<(), I3cDrvError> {
        // init i3c_cmd to all 0
        let mut cmds = [I3cCmd {
            cmd_lo: 0,
            cmd_hi: 0,
            tx: None,
            rx: None,
            tx_len: 0,
            rx_len: 0,
            ret: 0,
        }];

        let mut pos = 0;
        let mut rnw: bool = false;
        let mut is_broadcast = false;

        let (id, data_len) = {
            let Some(ccc) = payload.ccc.as_ref() else {
                return Err(I3cDrvError::Invalid);
            };
            (ccc.id, ccc.data.as_deref().map_or(0, <[u8]>::len))
        };

        let dbp_is_direct = id > 0x7F;
        let db: u8 = if dbp_is_direct && data_len > 0 {
            payload
                .ccc
                .as_ref()
                .and_then(|c| c.data.as_deref())
                .map_or(0, |d| d[0])
        } else {
            0
        };

        {
            let cmd = &mut cmds[0];

            if id <= 0x7F {
                // -------- Broadcast CCC --------
                is_broadcast = true;

                if data_len > 0 {
                    if let Some(d) = payload.ccc.as_ref().and_then(|c| c.data.as_deref()) {
                        cmd.tx = Some(d);
                        cmd.tx_len = u32::try_from(data_len).map_err(|_| I3cDrvError::Invalid)?;
                    }
                }
            } else {
                let Some(tgt_addr) = payload
                    .targets
                    .as_ref()
                    .and_then(|ts| ts.first())
                    .map(|t| t.addr)
                else {
                    return Err(I3cDrvError::Invalid);
                };
                // -------- Direct CCC --------
                let pos_ops = config.attached.pos_of_addr(tgt_addr);
                i3c_debug!(
                    self.logger,
                    "do_ccc: tgt_addr=0x{:02x}, pos_ops={:?}",
                    tgt_addr,
                    pos_ops
                );
                pos = match pos_ops {
                    Some(p) => p,
                    None => return Err(I3cDrvError::Invalid),
                };
                i3c_debug!(
                    self.logger,
                    "do_ccc: tgt_addr=0x{:02x}, pos={}",
                    tgt_addr,
                    pos
                );

                let Some(tp) = payload.targets.as_deref_mut().and_then(|ts| ts.first_mut()) else {
                    return Err(I3cDrvError::Invalid);
                };

                rnw = tp.rnw;

                if rnw {
                    let len = tp.data.as_deref().map_or(0, <[u8]>::len);
                    if len == 0 {
                        return Err(I3cDrvError::Invalid);
                    }
                    cmd.rx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                    cmd.rx = tp.data.as_deref_mut();
                } else {
                    let (d_opt, len) = match tp.data.as_deref() {
                        Some(d) => (Some(d), d.len()),
                        None => (None, 0),
                    };
                    cmd.tx = d_opt;
                    cmd.tx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                    tp.num_xfer = len;
                }
            }
        }

        let cmd = &mut cmds[0];
        cmd.cmd_hi = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG);

        if dbp_is_direct && data_len > 0 {
            cmd.cmd_lo |= COMMAND_PORT_DBP;
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DB, db.into());
        }

        if rnw {
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DATA_LEN, cmd.rx_len);
        } else {
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DATA_LEN, cmd.tx_len);
        }

        cmd.cmd_lo |= field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_CMD)
            | field_prep(COMMAND_PORT_CMD, id.into())
            | field_prep(COMMAND_PORT_READ_TRANSFER, u32::from(rnw))
            | COMMAND_PORT_CP
            | COMMAND_PORT_ROC
            | COMMAND_PORT_TOC;

        if !is_broadcast {
            cmd.cmd_lo |= field_prep(COMMAND_PORT_DEV_INDEX, u32::from(pos));
        }

        if id == I3C_CCC_SETHID || id == I3C_CCC_DEVCTRL {
            cmd.cmd_lo |= field_prep(COMMAND_PORT_SPEED, SpeedI3c::I2cFmAsI3c as u32);
        }

        let mut xfer = I3cXfer::new(&mut cmds[..]);
        self.start_xfer(config, &mut xfer);

        let mut delay = DummyDelay {};
        if !xfer.done.wait_for_us(1_000_000_000, &mut delay) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
        }

        let ret = xfer.ret;
        if ret == i32::try_from(RESPONSE_ERROR_IBA_NACK).map_err(|_| I3cDrvError::Invalid)? {
            return Ok(());
        }

        if is_broadcast {
            if let Some(ccc_rw) = payload.ccc.as_mut() {
                if let Some(d) = ccc_rw.data.as_deref() {
                    ccc_rw.num_xfer = d.len();
                }
            }
        }

        match ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Invalid),
        }
    }

    fn do_entdaa(&mut self, config: &mut I3cConfig, pos: u32) -> Result<(), I3cDrvError> {
        i3c_debug!(self.logger, "do_entdaa: pos={}", pos);
        let cmd = I3cCmd {
            cmd_lo: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_ADDR_ASSGN_CMD)
                | field_prep(COMMAND_PORT_CMD, u32::from(I3C_CCC_ENTDAA))
                | field_prep(COMMAND_PORT_DEV_COUNT, 1)
                | field_prep(COMMAND_PORT_DEV_INDEX, pos)
                | COMMAND_PORT_ROC
                | COMMAND_PORT_TOC,
            cmd_hi: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG),
            tx: None,
            rx: None,
            tx_len: 0,
            rx_len: 0,
            ret: 0,
        };

        i3c_debug!(
            self.logger,
            "do_entdaa: cmd_lo=0x{:08x}, cmd_hi=0x{:08x}",
            cmd.cmd_lo,
            cmd.cmd_hi
        );
        let mut cmds = [cmd];
        let mut xfer = I3cXfer::new(&mut cmds[..]);
        xfer.ret = -1;

        self.start_xfer(config, &mut xfer);

        let mut delay = DummyDelay {};

        if !xfer.done.wait_for_us(1_000_000_000, &mut delay) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
            return Err(I3cDrvError::Invalid);
        }

        i3c_debug!(self.logger, "do_entdaa: xfer done");
        match xfer.ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Invalid),
        }
    }

    fn priv_xfer_build_cmds<'a>(
        &mut self,
        cmds: &mut [I3cCmd<'a>],
        msgs: &mut [I3cMsg<'a>],
        pos: u8,
    ) -> Result<(), I3cDrvError> {
        let cmds_len = cmds.len();
        if cmds_len != msgs.len() {
            return Err(I3cDrvError::Invalid);
        }

        for i in 0..cmds_len {
            let (is_read, ptr, len) = {
                let m = &mut msgs[i];
                let is_read = (m.flags & I3C_MSG_READ) != 0;

                if is_read {
                    let buf = match m.buf.as_deref_mut() {
                        Some(b) if !b.is_empty() => b,
                        _ => return Err(I3cDrvError::Invalid),
                    };
                    (true, buf.as_mut_ptr(), buf.len())
                } else {
                    let buf = match m.buf.as_deref() {
                        Some(b) if !b.is_empty() => b,
                        _ => return Err(I3cDrvError::Invalid),
                    };
                    m.num_xfer = u32::try_from(buf.len()).map_err(|_| I3cDrvError::Invalid)?;
                    (false, buf.as_ptr().cast_mut(), buf.len())
                }
            };

            let cmd = &mut cmds[i];
            *cmd = I3cCmd {
                cmd_hi: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG)
                    | field_prep(
                        COMMAND_PORT_ARG_DATA_LEN,
                        u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?,
                    ),
                cmd_lo: field_prep(
                    COMMAND_PORT_TID,
                    u32::try_from(i).map_err(|_| I3cDrvError::Invalid)?,
                ) | field_prep(COMMAND_PORT_DEV_INDEX, u32::from(pos))
                    | COMMAND_PORT_ROC,
                tx: None,
                rx: None,
                tx_len: 0,
                rx_len: 0,
                ret: 0,
            };

            if is_read {
                let rx_slice: &'a mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
                cmd.rx = Some(rx_slice);
                cmd.rx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                cmd.cmd_lo |= COMMAND_PORT_READ_TRANSFER;
            } else {
                let tx_slice: &'a [u8] =
                    unsafe { core::slice::from_raw_parts(ptr.cast_const(), len) };
                cmd.tx = Some(tx_slice);
                cmd.tx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
            }

            let is_last = i + 1 == cmds_len;
            if is_last {
                cmd.cmd_lo |= COMMAND_PORT_TOC;
            }
        }

        Ok(())
    }

    fn priv_xfer(
        &mut self,
        config: &mut I3cConfig,
        pid: u64,
        msgs: &mut [I3cMsg],
    ) -> Result<(), I3cDrvError> {
        let pos_opt = config.attached.pos_of_pid(pid);
        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;

        let mut cmds: heapless::Vec<I3cCmd, MAX_CMDS> = heapless::Vec::new();
        for _ in 0..msgs.len() {
            cmds.push(I3cCmd {
                cmd_lo: 0,
                cmd_hi: 0,
                tx: None,
                rx: None,
                tx_len: 0,
                rx_len: 0,
                ret: 0,
            })
            .unwrap();
        }

        let ret = self.priv_xfer_build_cmds(cmds.as_mut_slice(), msgs, pos);
        match ret {
            Ok(()) => {}
            Err(e) => return Err(e),
        }

        let mut xfer = I3cXfer::new(cmds.as_mut_slice());
        self.start_xfer(config, &mut xfer);

        let mut delay = DummyDelay {};
        if !xfer.done.wait_for_us(1_000_000_000, &mut delay) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
            return Err(I3cDrvError::Timeout);
        }

        for (i, m) in msgs.iter_mut().enumerate() {
            if (m.flags & I3C_MSG_READ) != 0 {
                m.actual_len = xfer.cmds[i].rx_len;
            }
        }

        match xfer.ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Timeout),
        }
    }

    fn target_tx_write(&mut self, buf: &[u8]) {
        self.wr_tx_fifo(buf);
        let cmd = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(
                COMMAND_PORT_ARG_DATA_LEN,
                u32::try_from(buf.len()).map_or(0, |v| v),
            )
            | field_prep(COMMAND_PORT_TID, Tid::TargetRdData as u32);

        self.i3c.i3cd00c().write(|w| unsafe { w.bits(cmd) });
    }

    fn drain_fifo<F>(&mut self, mut read_word: F, len: usize)
    where
        F: FnMut() -> u32,
    {
        let nwords = (len + 3) >> 2;
        for _ in 0..nwords {
            let _ = read_word();
        }
    }

    fn handle_ibi_sir(&mut self, config: &mut I3cConfig, addr: u8, len: usize) {
        i3c_debug!(self.logger, "handle_ibi_sir: addr=0x{:02x}", addr);
        let pos = config.attached.pos_of_addr(addr);
        if pos.is_none() {
            i3c_debug!(
                self.logger,
                "handle_ibi_sir: no such addr in attached devices"
            );
            self.drain_fifo(|| self.i3c.i3cd018().read().bits(), len);
        }

        let mut ibi_buf: [u8; 2] = [0u8; 2];
        let take = core::cmp::min(len, ibi_buf.len());
        self.rd_ibi_fifo(&mut ibi_buf[..take]);
        let bus = I3C::BUS_NUM as usize;
        ibi_workq::i3c_ibi_work_enqueue_target_irq(bus, addr, &ibi_buf[..take]);
    }

    fn handle_ibis(&mut self, config: &mut I3cConfig) {
        let nibis = self.i3c.i3cd04c().read().ibistatuscnt().bits();

        i3c_debug!(self.logger, "Number of IBIs: {}", nibis);
        if nibis == 0 {
            return;
        }

        for _ in 0..nibis {
            // i3c_debug!(self.logger, "i3cd018 (IBI Queue status) = {:#x}", self.i3c.i3cd018().read().bits());
            let reg = self.i3c.i3cd018().read().bits();

            let ibi_id = field_get(reg, IBIQ_STATUS_IBI_ID, IBIQ_STATUS_IBI_ID_SHIFT);
            let ibi_data_len = field_get(
                reg,
                IBIQ_STATUS_IBI_DATA_LEN,
                IBIQ_STATUS_IBI_DATA_LEN_SHIFT,
            ) as usize;
            let ibi_addr = (ibi_id >> 1) & 0x7F;
            let rnw = (ibi_id & 1) != 0;
            i3c_debug!(
                self.logger,
                "IBI: addr=0x{:02x}, rnw={}, len={}",
                ibi_addr,
                rnw,
                ibi_data_len
            );
            if ibi_addr != 2 && rnw {
                // sirq
                self.handle_ibi_sir(config, ibi_addr as u8, ibi_data_len);
            } else if ibi_addr == 2 && !rnw {
                // hot-join
                let bus = I3C::BUS_NUM as usize;
                i3c_debug!(self.logger, "Hot-join IBI");
                ibi_workq::i3c_ibi_work_enqueue_hotjoin(bus);
            } else {
                // normal ibi
                i3c_debug!(self.logger, "Normal IBI");
                self.drain_fifo(|| self.i3c.i3cd018().read().bits(), ibi_data_len);
            }
        }
    }

    fn i3c_aspeed_isr(&mut self, config: &mut I3cConfig) {
        self.disable_irq();
        let status = self.i3c.i3cd03c().read().bits();
        i3c_debug!(self.logger, "[ISR] 0x{:08x}", status);
        if status == 0 {
            self.enable_irq();
            return;
        }

        if config.is_secondary {
            if status & INTR_DYN_ADDR_ASSGN_STAT != 0 {
                let da = self.i3c.i3cd004().read().dev_dynamic_addr().bits();
                if let Some(tc) = &mut config.target_config {
                    tc.addr = Some(da);
                }
                ibi_workq::i3c_ibi_work_enqueue_target_da_assignment(I3C::BUS_NUM.into());
            }

            if (status & INTR_RESP_READY_STAT) != 0 {
                self.target_handle_response_ready(config);
            }

            if (status & INTR_CCC_UPDATED_STAT) != 0 {
                self.target_handle_ccc_update(config);
            }
        } else {
            if (status & (INTR_RESP_READY_STAT | INTR_TRANSFER_ERR_STAT | INTR_TRANSFER_ABORT_STAT))
                != 0
            {
                self.end_xfer(config);
            }

            if (status & INTR_IBI_THLD_STAT) != 0 {
                self.handle_ibis(config);
            }
        }

        self.i3c.i3cd03c().write(|w| unsafe { w.bits(status) });
        self.enable_irq();
    }

    fn target_ibi_raise_hj(&self, config: &mut I3cConfig) -> Result<(), I3cDrvError> {
        if !config.is_secondary {
            return Err(I3cDrvError::Invalid);
        }
        let hj_support = self.i3c.i3cd008().read().slvhjcap().bit();
        if !hj_support {
            return Err(I3cDrvError::Invalid);
        }

        let addr_valid = self.i3c.i3cd004().read().dynamic_addr_valid().bit();
        if addr_valid {
            return Err(I3cDrvError::Access);
        }

        self.i3c.i3cd038().write(|w| unsafe { w.bits(8) }); // set HJ request

        Ok(())
    }

    fn target_handle_response_ready(&mut self, config: &mut I3cConfig) {
        let nresp = self.i3c.i3cd04c().read().respbufblr().bits();

        for _ in 0..nresp {
            let resp = self.i3c.i3cd010().read().bits();

            let tid = field_get(resp, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT) as usize;
            let rx_len = field_get(
                resp,
                RESPONSE_PORT_DATA_LEN_MASK,
                RESPONSE_PORT_DATA_LEN_SHIFT,
            ) as usize;
            let err = field_get(
                resp,
                RESPONSE_PORT_ERR_STATUS_MASK,
                RESPONSE_PORT_ERR_STATUS_SHIFT,
            );
            i3c_debug!(
                self.logger,
                "Response: tid={}, rx_len={}, err={}",
                tid,
                rx_len,
                err
            );

            if err != 0 {
                self.enter_halt(false, config);
                self.reset_ctrl(RESET_CTRL_QUEUES);
                self.exit_halt(config);
                continue;
            }

            if rx_len != 0 {
                let mut buf: [u8; 256] = [0u8; 256];
                self.rd_rx_fifo(&mut buf[..rx_len]);
                i3c_debug!(
                    self.logger,
                    "[MASTER ==> TARGET] TARGET READ: {:02x?}",
                    &buf[..rx_len]
                );
            }

            if tid == Tid::TargetIbi as usize {
                config.target_ibi_done.complete();
            }

            if tid == Tid::TargetRdData as usize {
                config.target_data_done.complete();
            }
        }
    }

    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cDrvError> {
        let reg = self.i3c.i3cd038().read().bits();
        if !(config.sir_allowed_by_sw && (reg & SLV_EVENT_CTRL_SIR_EN != 0)) {
            return Err(I3cDrvError::Access);
        }

        let Some(mdb) = notifier.first_byte() else {
            return Err(I3cDrvError::Invalid);
        };

        self.set_ibi_mdb(mdb);
        if let Some(p) = notifier.payload {
            if !p.is_empty() {
                self.wr_tx_fifo(p);
            }
        }

        let payload_len = u32::try_from(notifier.payload.map_or(0, <[u8]>::len))
            .map_err(|_| I3cDrvError::Invalid)?;
        let cmd: u32 = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(COMMAND_PORT_ARG_DATA_LEN, payload_len)
            | field_prep(COMMAND_PORT_TID, Tid::TargetIbi as u32);
        self.i3c.i3cd00c().write(|w| unsafe { w.bits(cmd) });

        config.target_ibi_done.reset();

        self.i3c
            .i3cd01c()
            .modify(|_, w| unsafe { w.response_buffer_threshold_value().bits(0) });

        self.target_tx_write(buf);
        config.target_data_done.reset();

        self.i3c.i3cd08c().write(|w| w.sir().set_bit());

        let mut delay = DummyDelay {};

        if !config
            .target_ibi_done
            .wait_for_us(1_000_000_000, &mut delay)
        {
            i3c_debug!(self.logger, "SIR timeout! Reset I3C controller");
            self.enter_halt(false, config);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.exit_halt(config);
            return Err(I3cDrvError::IoError);
        }

        if !config
            .target_data_done
            .wait_for_us(1_000_000_000, &mut delay)
        {
            i3c_debug!(self.logger, "wait master read timeout! Reset queues");
            self.i3c_disable(config.is_secondary);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.i3c_enable(config);
            return Err(I3cDrvError::Timeout);
        }

        Ok(())
    }

    fn target_handle_ccc_update(&mut self, config: &mut I3cConfig) {
        let event = self.i3c.i3cd038().read().bits();
        self.i3c.i3cd038().write(|w| unsafe { w.bits(event) });
        i3c_debug!(self.logger, "CCC update event: 0x{:08x}", event);
        let reg = self.i3c.i3cd054().read().cmtfrstatus().bits();
        if reg == CM_TFR_STS_TARGET_HALT {
            self.enter_halt(true, config);
            self.exit_halt(config);
        }
    }
}
