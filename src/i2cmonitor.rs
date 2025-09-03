// Licensed under the Apache-2.0 license

use crate::common::Logger;
use ast1060_pac::{I2cFilterThr, I2cFilterThr1, I2cFilterThr2, I2cFilterThr3, I2cfilter};
use core::fmt;
use core::fmt::Write;

type I2cFilterRegBlock = ast1060_pac::i2cfilter::RegisterBlock;
type I2cFilterThrRegBlock = ast1060_pac::i2c_filter_thr::RegisterBlock;

//filter capability define
const AST_I2C_F_COUNT: usize = 4;
const AST_I2C_F_REMAP_SIZE: usize = 16;
const AST_I2C_F_ELEMENT_SIZE: usize = 8;
const AST_CFG_CLOCK0: u32 = 100;
const AST_CFG_CLOCK1: u32 = 400;

#[derive(Debug, Copy, Clone)]
pub struct AstI2cThrData {
    filter_en: bool,
    wlist_en: bool,
    filter_idx: [u8; AST_I2C_F_REMAP_SIZE],
}

#[derive(Debug, Copy, Clone)]
pub struct AstI2cFBitmap {
    pub element: [u32; AST_I2C_F_ELEMENT_SIZE],
}

#[derive(Debug, Copy, Clone)]
pub struct AstI2cFMTbl {
    filter_mtbl: [AstI2cFBitmap; AST_I2C_F_REMAP_SIZE + 1],
}

/// Wraps the `I2C_filter` peripheral
pub struct I2cMonitor<L: Logger> {
    i2cfilter_glb: &'static I2cFilterRegBlock,
    i2cfilter_thrs: [&'static I2cFilterThrRegBlock; AST_I2C_F_COUNT],
    i2cfilter_tbl: [AstI2cFMTbl; AST_I2C_F_COUNT],
    i2cfilter_data: [AstI2cThrData; AST_I2C_F_COUNT],
    logger: L,
}

impl<L: Logger> fmt::Debug for I2cMonitor<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("I2cMonitor")
    }
}

macro_rules! i2cf_debug {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        write!(buf, $($arg)*).unwrap();
        $logger.debug(buf.as_str());
    };
}

macro_rules! i2cf_error {
    ($logger:expr, $($arg:tt)*) => {
        let mut buf: heapless::String<64> = heapless::String::new();
        write!(buf, $($arg)*).unwrap();
        $logger.error(buf.as_str());
    };
}

impl<L: Logger> I2cMonitor<L> {
    pub fn new(logger: L) -> Self {
        // SAFETY: I2C filter registers are memory-mapped and guaranteed
        // to be valid by the hardware spec.
        let i2cfilter_glb = unsafe { &*I2cfilter::PTR };
        let i2cfilter_thrs: [&'static I2cFilterThrRegBlock; AST_I2C_F_COUNT] = unsafe {
            [
                &*I2cFilterThr::PTR,
                &*I2cFilterThr1::PTR,
                &*I2cFilterThr2::PTR,
                &*I2cFilterThr3::PTR,
            ]
        };

        Self {
            i2cfilter_glb,
            i2cfilter_thrs,
            i2cfilter_tbl: [AstI2cFMTbl {
                filter_mtbl: [AstI2cFBitmap {
                    element: [0; AST_I2C_F_ELEMENT_SIZE],
                }; AST_I2C_F_REMAP_SIZE + 1],
            }; AST_I2C_F_COUNT],
            i2cfilter_data: [AstI2cThrData {
                filter_en: false,
                wlist_en: false,
                filter_idx: [0; AST_I2C_F_REMAP_SIZE],
            }; AST_I2C_F_COUNT],
            logger,
        }
    }

    pub fn dump_regs(&mut self) {
        i2cf_debug!(self.logger, "******* i2cf registers ******");
        i2cf_debug!(
            self.logger,
            "i2cfilter008 {:#x}",
            self.i2cfilter_glb.i2cfilter008().read().bits()
        );
        i2cf_debug!(
            self.logger,
            "i2cfilter00c {:#x}",
            self.i2cfilter_glb.i2cfilter00c().read().bits()
        );
        for i in 0..4 {
            i2cf_debug!(self.logger, "******* i2cf thr {i:} registers ******");
            i2cf_debug!(
                self.logger,
                "i2cfilterthr04 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr04().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr08 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr08().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr0c {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr0c().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr10 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr10().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr14 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr14().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr18 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr18().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr20 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr20().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr24 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr24().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr40 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr40().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr44 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr44().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr48 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr48().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr4c {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr4c().read().bits()
            );
            i2cf_debug!(
                self.logger,
                "i2cfilterthr60 {:#x}",
                self.i2cfilter_thrs[i].i2cfilterthr60().read().bits()
            );
        }
        i2cf_debug!(self.logger, "*****************************");
    }

    #[allow(clippy::similar_names)]
    pub fn ast_i2c_filter_isr(&mut self) {
        let sts_global: u32 = self.i2cfilter_glb.i2cfilter00c().read().bits();

        for index in 0..AST_I2C_F_COUNT {
            if sts_global & (1 << index) == 1 << index {
                let count: u8;
                let int_sts = self.i2cfilter_thrs[index].i2cfilterthr18().read().bits();
                if int_sts > 0 {
                    let info_wp = self.i2cfilter_thrs[index]
                        .i2cfilterthr20()
                        .read()
                        .failwpt()
                        .bits();
                    let info_rp = self.i2cfilter_thrs[index]
                        .i2cfilterthr20()
                        .read()
                        .failrpt()
                        .bits();
                    //calculate the information count
                    if info_wp > info_rp {
                        count = info_wp - info_rp;
                    } else {
                        count = (info_wp + 0x10) - info_rp;
                    }
                    //read back
                    for _i in 0..count {
                        //filter block number and information
                        let value = self.i2cfilter_thrs[index].i2cfilterthr60().read().bits();
                        i2cf_debug!(self.logger, "fail log info: {value:#x}");
                    }
                    //clear status
                    self.i2cfilter_thrs[index]
                        .i2cfilterthr18()
                        .write(|w| unsafe { w.bits(int_sts) });
                }
            }
        }
    }
    pub fn get_pclk(&mut self) -> u32 {
        50_000_000
    }
    pub fn close_filter(&mut self, index: usize) {
        self.i2cfilter_thrs[index]
            .i2cfilterthr04()
            .write(|w| w.en().clear_bit());
        self.i2cfilter_thrs[index]
            .i2cfilterthr0c()
            .write(|w| unsafe { w.bits(0x0) });
    }

    pub fn set_initial_timing(&mut self, index: usize, cfg_clock: u32) {
        if cfg_clock == AST_CFG_CLOCK0 || cfg_clock == AST_CFG_CLOCK1 {
            let mut timeout_count = self.get_pclk() / (cfg_clock * 3 * 1000);

            timeout_count |= timeout_count << 16;
            self.i2cfilter_thrs[index]
                .i2cfilterthr10()
                .write(|w| unsafe { w.bits(timeout_count) });
        } else {
            i2cf_error!(self.logger, "i2c filter invalid clock");
        }
    }

    pub fn clr_local_interrupt(&mut self, index: usize) {
        self.i2cfilter_thrs[index]
            .i2cfilterthr18()
            .write(|w| unsafe { w.bits(0x1) });
    }
    pub fn enable_local_interrupt(&mut self, index: usize) {
        self.i2cfilter_thrs[index]
            .i2cfilterthr14()
            .write(|w| unsafe { w.inten().bits(0x1) });
    }
    pub fn enable_global_interrupt(&mut self, index: usize) {
        self.i2cfilter_glb
            .i2cfilter008()
            .modify(|r, w| unsafe { w.topirqen().bits(r.topirqen().bits() | (1 << index)) });
    }
    //4x4
    fn clr_remap_index(&mut self, index: usize) {
        self.i2cfilter_thrs[index]
            .i2cfilterthr40()
            .write(|w| unsafe { w.map0().bits(0x0) });
        self.i2cfilter_thrs[index]
            .i2cfilterthr44()
            .write(|w| unsafe { w.map1().bits(0x0) });
        self.i2cfilter_thrs[index]
            .i2cfilterthr48()
            .write(|w| unsafe { w.map2().bits(0x0) });
        self.i2cfilter_thrs[index]
            .i2cfilterthr4c()
            .write(|w| unsafe { w.map3().bits(0x0) });
        for i in 0..AST_I2C_F_REMAP_SIZE {
            self.i2cfilter_data[index].filter_idx[i] = 0;
        }
    }

    //set white list buffer into device
    fn set_dev_white_list_tbl(&mut self, index: usize) {
        let table_ptr = core::ptr::from_ref::<AstI2cFMTbl>(&self.i2cfilter_tbl[index]) as u32;
        self.i2cfilter_thrs[index]
            .i2cfilterthr08()
            .write(|w| unsafe { w.addr().bits(table_ptr) });
    }
    //clear white list table
    fn clr_white_list_tbl(&mut self, index: usize) {
        self.i2cfilter_thrs[index]
            .i2cfilterthr08()
            .write(|w| unsafe { w.addr().bits(0) });

        let tbl_addr: u32 = self.i2cfilter_thrs[index].i2cfilterthr08().read().bits();
        let tbl_ptr = tbl_addr as *mut AstI2cFMTbl;
        //clear bitmap table
        unsafe {
            //make sure the address is valid and points to a properly aligned AstI2cFMTbl
            if !tbl_ptr.is_null() {
                let tbl_ref: &mut AstI2cFMTbl = &mut *tbl_ptr;
                for bitmap in &mut tbl_ref.filter_mtbl {
                    for elem in &mut bitmap.element {
                        *elem = 0;
                    }
                }
            }
        }
    }
    //
    pub fn ast_i2c_filter_default(
        &mut self,
        idx: usize,
        pass: u8,
        index: usize,
    ) -> Result<i32, &'static str> {
        if idx >= AST_I2C_F_REMAP_SIZE {
            return Err("Invalid filter table index");
        }
        let mut value = 0;
        if pass != 0 {
            value = 0xffff_ffff;
        }
        //fill bitmap table (pass or block)
        for i in 0..AST_I2C_F_ELEMENT_SIZE {
            self.i2cfilter_tbl[index].filter_mtbl[idx].element[i] = value;
        }
        Ok(0)
    }

    pub fn ast_i2c_filter_update(
        &mut self,
        idx: usize,
        addr: u8,
        table: &mut AstI2cFBitmap,
        index: usize,
    ) -> Result<i32, &'static str> {
        if idx >= AST_I2C_F_REMAP_SIZE {
            return Err("Invalid filter table index");
        }
        //always put mapped address in the table
        self.i2cfilter_data[index].filter_idx[idx] = addr;

        //byte index
        let start_index = (idx >> 2) << 2;
        let addr_4bytes = u32::from_le_bytes([
            self.i2cfilter_data[index].filter_idx[start_index],
            self.i2cfilter_data[index].filter_idx[start_index + 1],
            self.i2cfilter_data[index].filter_idx[start_index + 2],
            self.i2cfilter_data[index].filter_idx[start_index + 3],
        ]);
        // 4-byte address based on 4 indexes per u32/dw map[0-3]
        // Write the mapped address to the correct map register
        match idx >> 2 {
            0 => {
                self.i2cfilter_thrs[index]
                    .i2cfilterthr40()
                    .write(|w| unsafe { w.map0().bits(addr_4bytes) });
            }
            1 => {
                self.i2cfilter_thrs[index]
                    .i2cfilterthr44()
                    .write(|w| unsafe { w.map1().bits(addr_4bytes) });
            }
            2 => {
                self.i2cfilter_thrs[index]
                    .i2cfilterthr48()
                    .write(|w| unsafe { w.map2().bits(addr_4bytes) });
            }
            3 => {
                self.i2cfilter_thrs[index]
                    .i2cfilterthr4c()
                    .write(|w| unsafe { w.map3().bits(addr_4bytes) });
            }
            _ => return Err("index out of range"), // This should be unreachable
        }
        //fill bitmap table (pass or block)
        for i in 0..AST_I2C_F_ELEMENT_SIZE {
            self.i2cfilter_tbl[index].filter_mtbl[idx].element[i] = table.element[i];
        }
        Ok(0)
    }

    //index: filter thread index
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn ast_i2c_filter_en(
        &mut self,
        filter_en: bool,
        wlist_en: bool,
        clr_idx: bool,
        clr_tbl: bool,
        index: usize,
    ) -> Result<i32, &'static str> {
        self.i2cfilter_data[index].filter_en = filter_en;
        self.i2cfilter_data[index].wlist_en = wlist_en;

        if filter_en && wlist_en {
            i2cf_debug!(self.logger, "ast_i2c_filter_en: set_dev_white_list_tbl");
            self.set_dev_white_list_tbl(index);
        }
        //clear re-map index
        if clr_idx {
            i2cf_debug!(self.logger, "ast_i2c_filter_en: clr_remap_index");
            self.clr_remap_index(index);
        }
        if clr_tbl {
            i2cf_debug!(self.logger, "ast_i2c_filter_en: clr_white_list_tbl");
            self.clr_white_list_tbl(index);
        }
        //apply filter setting
        self.i2cfilter_thrs[index]
            .i2cfilterthr04()
            .write(|w| w.en().bit(filter_en));
        self.i2cfilter_thrs[index]
            .i2cfilterthr0c()
            .write(|w| unsafe { w.cfg().bits(u32::from(wlist_en)) });
        Ok(0)
    }

    //init a filter/thread
    pub fn ast_i2c_filter_init(&mut self, index: usize) {
        self.close_filter(index);
        self.set_initial_timing(index, AST_CFG_CLOCK0);
        self.clr_local_interrupt(index);
        self.enable_local_interrupt(index);
        self.enable_global_interrupt(index);
    }
}
