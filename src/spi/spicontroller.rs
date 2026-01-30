// Licensed under the Apache-2.0 license

use super::{
    aspeed_get_spi_freq_div, get_addr_buswidth, get_hclock_rate, get_mid_point_of_longest_one,
    spi_cal_dummy_cycle, spi_calibration_enable, spi_io_mode, spi_io_mode_user, spi_read_data,
    spi_write_data, CtrlType, SpiBusWithCs, SpiConfig, SpiData, SpiError, Write, ASPEED_MAX_CS,
    ASPEED_SPI_NORMAL_READ, ASPEED_SPI_NORMAL_WRITE, ASPEED_SPI_SZ_256M, ASPEED_SPI_SZ_2M,
    ASPEED_SPI_USER, ASPEED_SPI_USER_INACTIVE, SPI_CALIB_LEN, SPI_CTRL_FREQ_MASK,
    SPI_DMA_CALC_CKSUM, SPI_DMA_CALIB_MODE, SPI_DMA_DISCARD_REQ_MAGIC, SPI_DMA_ENABLE,
    SPI_DMA_FLASH_MAP_BASE, SPI_DMA_GET_REQ_MAGIC, SPI_DMA_GRANT, SPI_DMA_RAM_MAP_BASE,
    SPI_DMA_REQUEST, SPI_DMA_STATUS, SPI_DMA_TIMEOUT,
};

#[cfg(feature = "spi_dma")]
use super::{SPI_DMA_TRIGGER_LEN, SPI_NOR_DATA_DIRECT_READ, SPI_NOR_DATA_DIRECT_WRITE};

use crate::dbg;
use crate::spi::{
    SPI_CONF_CE0_ENABLE_WRITE_SHIFT, SPI_CTRL_CEX_4BYTE_MODE_SET, SPI_CTRL_CEX_DUMMY_SHIFT,
    SPI_CTRL_CEX_SPI_CMD_MASK, SPI_CTRL_CEX_SPI_CMD_SHIFT, SPI_DMA_CLK_FREQ_MASK,
    SPI_DMA_CLK_FREQ_SHIFT, SPI_DMA_DELAY_MASK, SPI_DMA_DELAY_SHIFT,
};
use crate::{common::DummyDelay, spi::norflash::SpiNorData, uart_core::UartController};

use embedded_hal::{
    delay::DelayNs,
    spi::{ErrorType, SpiBus},
};
impl<'a> ErrorType for SpiController<'a> {
    type Error = SpiError;
}

pub struct SpiController<'a> {
    regs: &'static ast1060_pac::spi::RegisterBlock,
    current_cs: usize,
    spi_config: SpiConfig,
    spi_data: SpiData,
    pub dbg_uart: Option<&'a mut UartController<'a>>,
}

macro_rules! cs_ctrlreg_w {
    ($this:expr, $cs:expr, $value:expr) => {{
        if $cs == 0 {
            $this.regs.spi010().write(|w| unsafe { w.bits($value) });
        } else if $cs == 1 {
            $this.regs.spi014().write(|w| unsafe { w.bits($value) });
        }
    }};
}

macro_rules! cs_ctrlreg_r {
    ($this:expr, $cs:expr) => {{
        if $cs == 0 {
            $this.regs.spi010().read().bits()
        } else if $cs == 1 {
            $this.regs.spi014().read().bits()
        } else {
            0
        }
    }};
}

impl<'a> SpiController<'a> {
    pub fn new(
        regs: &'static ast1060_pac::spi::RegisterBlock,
        current_cs: usize,
        spi_config: SpiConfig,
        spi_data: SpiData,
        dbg_uart: Option<&'a mut UartController<'a>>,
    ) -> Self {
        SpiController {
            regs,
            current_cs,
            spi_config,
            spi_data,
            dbg_uart,
        }
    }

    pub fn init(&mut self) -> Result<(), SpiError> {
        dbg!(self, "SpiController: init()");

        for cs in 0..self.spi_config.max_cs {
            self.regs.spi000().modify(|r, w| unsafe {
                let current = r.bits();
                w.bits(
                    current | (1 << (SPI_CONF_CE0_ENABLE_WRITE_SHIFT + u32::try_from(cs).unwrap())),
                )
            });

            self.spi_data.cmd_mode[cs].user = ASPEED_SPI_USER;
        }

        self.spi_data.hclk = get_hclock_rate();

        self.decode_range_pre_init();

        Ok(())
    }
    fn decode_range_pre_init(&mut self) {
        let mut max_cs = self.spi_config.max_cs;
        let mut unit_sz = ASPEED_SPI_SZ_2M;
        dbg!(self, "rang pre - init()");
        if self.spi_config.master_idx != 0 {
            max_cs = 1;
            unit_sz = ASPEED_SPI_SZ_256M;
        }

        if self.spi_config.pure_spi_mode_only {
            unit_sz = ASPEED_SPI_SZ_256M / u32::try_from(self.spi_config.max_cs).unwrap();
            unit_sz &= !(ASPEED_SPI_SZ_2M - 1);
        }

        let mut pre_end_addr = 0;
        for cs in 0..max_cs {
            let start_addr = if cs == 0 {
                self.spi_config.mmap_base
            } else {
                pre_end_addr
            };
            let end_addr = start_addr + unit_sz - 1;

            if self.spi_config.mmap_base + ASPEED_SPI_SZ_256M <= end_addr {
                if cs == 0 {
                    self.regs.spi030().write(|w| unsafe { w.bits(0) });
                } else if cs == 1 {
                    self.regs.spi034().write(|w| unsafe { w.bits(0) });
                }
                continue;
            }

            let seg_val = self.segment_compose(start_addr, end_addr);
            if cs == 0 {
                self.regs.spi030().write(|w| unsafe { w.bits(seg_val) });
            } else if cs == 1 {
                self.regs.spi034().write(|w| unsafe { w.bits(seg_val) });
            }

            self.spi_data.decode_addr[cs].start = start_addr;
            self.spi_data.decode_addr[cs].len = unit_sz;
            pre_end_addr = end_addr + 1;
        }
    }

    #[allow(clippy::unused_self)]
    fn segment_start(&self, reg_val: u32) -> u32 {
        (reg_val & 0x0ff0) << 16
    }

    #[allow(clippy::unused_self)]
    fn segment_end(&self, reg_val: u32) -> u32 {
        (reg_val & 0x0ff0_0000) | 0x000f_ffff
    }

    #[allow(clippy::unused_self)]
    fn segment_compose(&self, start: u32, end: u32) -> u32 {
        ((((start >> 20) << 20) >> 16) & 0xffff) | (((end >> 20) << 20) & 0xffff_0000)
    }

    fn decode_range_reinit(&mut self, flash_sz: u32) {
        let mut decode_sz_arr = [0u32; ASPEED_MAX_CS];
        let mut total_decode_range = 0;
        let mut pre_end_addr = 0;
        dbg!(self, "rang reinit() flash size: {:08x}", flash_sz);

        for (cs, size) in decode_sz_arr
            .iter_mut()
            .enumerate()
            .take(self.spi_config.max_cs)
        {
            let tmp = match cs {
                0 => self.regs.spi030().read().bits(),
                1 => self.regs.spi034().read().bits(),
                _ => 0,
            };

            *size = if tmp == 0 {
                0
            } else {
                self.segment_end(tmp) - self.segment_start(tmp) + 1
            };

            total_decode_range += *size;

            dbg!(self, "decode_sz_arr[{}]: {:08x}", cs, *size);
        }

        dbg!(self, "total range: {:08x}", total_decode_range);

        // prepare new decode sz array
        if total_decode_range - decode_sz_arr[self.current_cs] + flash_sz <= ASPEED_SPI_SZ_256M {
            decode_sz_arr[self.current_cs] = flash_sz;
        } else {
            return;
        }

        // 3. Apply new decode config
        for (cs, size) in decode_sz_arr
            .iter()
            .copied()
            .enumerate()
            .take(self.spi_config.max_cs)
        {
            if size == 0 {
                continue;
            }

            let start_addr = if cs == 0 {
                self.spi_config.mmap_base
            } else {
                pre_end_addr
            };

            let end_addr = start_addr + size - 1;
            dbg!(self, "start: {:08x}, end: {:08x}", start_addr, end_addr);
            let value = self.segment_compose(start_addr, end_addr);
            if cs == 0 {
                self.regs.spi030().write(|w| unsafe { w.bits(value) });
            } else if cs == 1 {
                self.regs.spi034().write(|w| unsafe { w.bits(value) });
            }

            self.spi_data.decode_addr[cs].start = start_addr;

            if cs == self.current_cs {
                self.spi_data.decode_addr[cs].len = flash_sz;
            }

            pre_end_addr = end_addr + 1;
        }
    }

    fn spi_nor_read_init(&mut self, cs: usize, op_info: &SpiNorData) {
        dbg!(
            self,
            "spi_nor_read_init() cs:{}  master_idx: {}",
            cs,
            self.spi_config.master_idx
        );

        if self.spi_config.master_idx == 0 && !self.spi_config.pure_spi_mode_only {
            self.decode_range_reinit(op_info.data_len);
        }
        let io_mode = spi_io_mode(op_info.mode);
        let dummy = spi_cal_dummy_cycle(
            u32::from(get_addr_buswidth(op_info.mode as u32)),
            op_info.dummy_cycle,
        );
        let read_cmd = (io_mode
            | ((op_info.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT)
            | (dummy as u32))
            | ASPEED_SPI_NORMAL_READ;
        self.spi_data.cmd_mode[cs].normal_read = read_cmd;
        dbg!(
            self,
            "cs: {:08x}, io_mode: {:08x}, dummy: {:08x}, op: {:08x}, normal read: {:08x}",
            cs,
            io_mode,
            dummy,
            op_info.opcode,
            read_cmd
        );

        cs_ctrlreg_w!(self, cs, read_cmd);
        if op_info.addr_len == 4 {
            self.regs.spi004().modify(|r, w| unsafe {
                let current = r.bits();
                w.bits(current | (SPI_CTRL_CEX_4BYTE_MODE_SET << cs))
            });
        }
        if matches!(self.spi_config.ctrl_type, CtrlType::HostSpi) {
            self.regs.spi06c().modify(|r, w| unsafe {
                let mut current = r.bits();
                if op_info.addr_len == 4 {
                    current = (current & 0xffff_00ff) | (op_info.opcode << 8);
                } else {
                    current = (current & 0xffff_ff00) | op_info.opcode;
                }

                w.bits((current & 0x0fff_ffff) | spi_io_mode(op_info.mode))
            });
        }
        self.timing_calibration(cs);
    }

    fn spi_nor_write_init(&mut self, cs: usize, op_info: &SpiNorData) {
        let io_mode = spi_io_mode(op_info.mode);
        let dummy = 0;
        let write_cmd = (io_mode
            | ((op_info.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT)
            | dummy)
            | ASPEED_SPI_NORMAL_WRITE;
        self.spi_data.cmd_mode[cs].normal_write = write_cmd;

        if matches!(self.spi_config.ctrl_type, CtrlType::HostSpi) {
            self.regs.spi06c().modify(|r, w| unsafe {
                let mut current = r.bits();
                current = (current & 0xf0ff_ffff) | (spi_io_mode(op_info.mode) >> 8);
                w.bits((current & 0x0fff_ffff) | spi_io_mode(op_info.mode))
            });

            self.regs.spi074().modify(|r, w| unsafe {
                let mut current = r.bits();
                if op_info.addr_len == 4 {
                    current = (current & 0xffff_00ff) | (op_info.opcode << 8);
                } else {
                    current = (current & 0xffff_ff00) | op_info.opcode;
                }
                w.bits(current)
            });
        }
    }

    pub fn timing_calibration(&mut self, cs: usize) {
        if self.skip_calibration(cs) {
            return;
        }

        let mut check_buf = [0u8; SPI_CALIB_LEN];
        self.load_flash_calibration_data(cs, &mut check_buf);

        if !spi_calibration_enable(&check_buf) {
            dbg!(self, "Flash data is monotonous, skip calibration");
            self.apply_clock_settings(cs, self.spi_config.frequency);
            return;
        }

        let gold_checksum = self.aspeed_spi_dma_checksum(0, 0);
        let calib_passed = self.run_timing_sweep(cs, gold_checksum);

        if !calib_passed {
            dbg!(self, "Timing sweep failed, using max_freq");
            self.apply_clock_settings(cs, self.spi_config.frequency);
        }
    }

    fn skip_calibration(&mut self, cs: usize) -> bool {
        if self.spi_config.timing_calibration_disabled {
            dbg!(self, "Timing calibration disabled by config");
            self.apply_clock_settings(cs, self.spi_config.frequency);
            return true;
        }

        let already_calibrated = match cs {
            0 => self.regs.spi094().read().bits(),
            1 => self.regs.spi098().read().bits(),
            _ => return true, // invalid CS
        };

        if already_calibrated != 0 {
            dbg!(self, "Calibration already executed for cs {}", cs);
            self.apply_clock_settings(cs, self.spi_config.frequency);
            return true;
        }

        // Skip if mux master_idx != 0 and cs != 0 (as per original logic)
        if self.spi_config.master_idx != 0 && cs != 0 {
            self.apply_clock_settings(cs, self.spi_config.frequency);
            return true;
        }

        // Clear frequency bits
        let mut reg_val = cs_ctrlreg_r!(self, cs);
        reg_val &= !SPI_CTRL_FREQ_MASK;
        cs_ctrlreg_w!(self, cs, reg_val);

        false
    }

    fn load_flash_calibration_data(&self, cs: usize, buf: &mut [u8]) {
        unsafe {
            let flash_ptr = self.spi_data.decode_addr[cs].start as *const u8;
            core::ptr::copy_nonoverlapping(
                flash_ptr.add(self.spi_config.timing_calibration_start_off as usize),
                buf.as_mut_ptr(),
                SPI_CALIB_LEN,
            );
        }
    }

    fn run_timing_sweep(&mut self, cs: usize, gold_checksum: u32) -> bool {
        let hclk_masks = [7u32, 14, 6, 13];
        let mut freq_to_use = self.spi_config.frequency;
        let mut calib_res = [0u8; 6 * 17];

        for (i, &mask) in hclk_masks.iter().enumerate() {
            let div = u32::try_from(i).unwrap() + 2;
            if freq_to_use < self.spi_data.hclk / div {
                continue;
            }

            freq_to_use = self.spi_data.hclk / div;

            let checksum = self.aspeed_spi_dma_checksum(mask, 0);
            let pass = checksum == gold_checksum;
            dbg!(
                self,
                "HCLK/{}, no timing compensation: {}",
                i + 2,
                if pass { "PASS" } else { "FAIL" }
            );

            calib_res.fill(0);

            for hcycle in 0..=5 {
                dbg!(self, "Delay Enable : hcycle {}", hcycle);
                for delay_ns in 0..=0xf {
                    let reg_val = mask | (1 << 3) | hcycle | (delay_ns << 4);
                    let checksum = self.aspeed_spi_dma_checksum(mask, reg_val);
                    let pass = checksum == gold_checksum;
                    let index = (hcycle * 17 + delay_ns) as usize;
                    calib_res[index] = u8::from(pass);
                    dbg!(
                        self,
                        "HCLK/{}, {} HCLK cycle, {} delay_ns : {}",
                        i + 2,
                        hcycle,
                        delay_ns,
                        if pass { "PASS" } else { "FAIL" }
                    );
                }
            }

            if let Some((hcycle, delay_ns)) = self.pick_best_delay(&calib_res) {
                dbg!(self, "Final hcycle: {}, delay_ns: {}", hcycle, delay_ns);
                let final_delay = ((1 << 3) | hcycle | (delay_ns << 4)) << (i * 8);
                self.regs.spi084().write(|w| unsafe { w.bits(final_delay) });

                self.apply_clock_settings(cs, freq_to_use);
                return true;
            }
            dbg!(self, "Cannot get good calibration point.");
        }

        false
    }

    #[allow(clippy::unused_self)]
    fn pick_best_delay(&self, calib_res: &[u8]) -> Option<(u32, u32)> {
        let calib_point = get_mid_point_of_longest_one(calib_res);
        if calib_point < 0 {
            None
        } else {
            let hcycle: u32 = (calib_point / 17).try_into().unwrap();
            let delay_ns: u32 = (calib_point % 17).try_into().unwrap();
            Some((hcycle, delay_ns))
        }
    }

    fn apply_clock_settings(&mut self, cs: usize, max_freq: u32) {
        let hclk_div = aspeed_get_spi_freq_div(self.spi_data.hclk, max_freq);

        let mut reg_val = cs_ctrlreg_r!(self, cs);
        reg_val = (reg_val & !SPI_CTRL_FREQ_MASK) | hclk_div;
        cs_ctrlreg_w!(self, cs, reg_val);

        self.spi_data.cmd_mode[cs].normal_read =
            (self.spi_data.cmd_mode[cs].normal_read & !SPI_CTRL_FREQ_MASK) | hclk_div;

        self.spi_data.cmd_mode[cs].normal_write =
            (self.spi_data.cmd_mode[cs].normal_write & !SPI_CTRL_FREQ_MASK) | hclk_div;

        self.spi_data.cmd_mode[cs].user =
            (self.spi_data.cmd_mode[cs].user & !SPI_CTRL_FREQ_MASK) | hclk_div;

        dbg!(
            self,
            "Configured SPI frequency to {} MHz",
            max_freq / 1_000_000
        );
    }

    pub fn aspeed_spi_dma_checksum(&mut self, div: u32, delay: u32) -> u32 {
        let data = &self.spi_data;
        let config = &self.spi_config;

        let cs = self.current_cs;
        // Request DMA access

        self.regs
            .spi080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        if self.regs.spi080().read().bits() & SPI_DMA_REQUEST != 0 {
            while self.regs.spi080().read().bits() & SPI_DMA_GRANT == 0 {}
        }

        // Set DMA flash start address
        let flash_addr = data.decode_addr[cs].start + config.timing_calibration_start_off;
        self.regs.spi084().write(|w| unsafe { w.bits(flash_addr) });
        // Set DMA length
        self.regs
            .spi08c()
            .write(|w| unsafe { w.bits(u32::try_from(SPI_CALIB_LEN).unwrap()) });
        // Configure DMA control register
        let ctrl_val = SPI_DMA_ENABLE
            | SPI_DMA_CALC_CKSUM
            | SPI_DMA_CALIB_MODE
            | ((delay & SPI_DMA_DELAY_MASK) << SPI_DMA_DELAY_SHIFT)
            | ((div & SPI_DMA_CLK_FREQ_MASK) << SPI_DMA_CLK_FREQ_SHIFT);

        self.regs.spi080().write(|w| unsafe { w.bits(ctrl_val) });
        // Wait until DMA done
        while self.regs.spi008().read().bits() & SPI_DMA_STATUS == 0 {}

        // Read checksum result
        let checksum = self.regs.spi090().read().bits();
        // Clear DMA control and discard request
        self.dma_disable();

        checksum
    }

    fn spi_nor_transceive_user(&mut self, op_info: &mut SpiNorData) {
        let cs: usize = self.current_cs;
        let dummy = [0u8; 12];
        let start_ptr = self.spi_data.decode_addr[cs].start as *mut u32;
        dbg!(
            self,
            "nor_transceive_user cs: {}, ahb start: {:08x}",
            u32::try_from(cs).unwrap(),
            self.spi_data.decode_addr[cs].start
        );

        // Send command
        let cmd_mode = self.spi_data.cmd_mode[cs].user
            | super::spi_io_mode_user(u32::from(super::get_cmd_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, cmd_mode);
        dbg!(self, "write opcode/cmd: 0x{:08x}", op_info.opcode);
        unsafe { super::spi_write_data(start_ptr, &[op_info.opcode.try_into().unwrap()]) };

        // Send address
        let addr_mode = self.spi_data.cmd_mode[cs].user
            | super::spi_io_mode_user(u32::from(super::get_addr_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, addr_mode);

        let mut addr = op_info.addr;
        if op_info.addr_len == 3 {
            addr <<= 8;
        }
        //op_info.addr = sys_cpu_to_be32(op_info.addr);
        let addr_bytes = addr.to_be_bytes();
        unsafe { super::spi_write_data(start_ptr, &addr_bytes[..op_info.addr_len as usize]) };

        // Dummy cycles
        let bus_width: u8 = super::get_addr_buswidth(op_info.mode as u32);
        let dummy_len: u8 = (op_info.dummy_cycle / (8 / u32::from(bus_width)))
            .try_into()
            .unwrap();
        dbg!(self, "write dummy len: 0x{:08x}", dummy_len);
        unsafe { super::spi_write_data(start_ptr, &dummy[..dummy_len as usize]) };

        // Data transfer
        let data_mode = self.spi_data.cmd_mode[cs].user
            | spi_io_mode_user(u32::from(super::get_data_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, data_mode);

        if op_info.data_direct == super::SPI_NOR_DATA_DIRECT_READ {
            unsafe { spi_read_data(start_ptr, op_info.rx_buf) };
        } else {
            unsafe { spi_write_data(start_ptr, op_info.tx_buf) };
        }
    }

    // Helper wrappers would be defined for spi_write_data, spi_read_data, io_mode_user, etc.

    pub fn spi_nor_transceive(&mut self, op_info: &mut SpiNorData) -> Result<(), SpiError> {
        dbg!(self, "spi_nor_transceive()...");

        #[cfg(feature = "spi_dma")]
        {
            dbg!(
                self,
                "spi dma enabled rx_len:[{:p}]{} tx_len[{:p}]: {}",
                op_info.rx_buf.as_ptr(),
                op_info.rx_buf.len(),
                op_info.tx_buf.as_ptr(),
                op_info.tx_buf.len()
            );
            let addr_aligned = op_info.addr % 4 == 0;

            if op_info.data_direct == SPI_NOR_DATA_DIRECT_READ {
                let buf_aligned = (op_info.rx_buf.as_ptr() as usize) % 4 == 0;
                let use_dma = !self.spi_config.pure_spi_mode_only
                    && op_info.rx_buf.len() > SPI_DMA_TRIGGER_LEN as usize
                    && addr_aligned
                    && buf_aligned;
                dbg!(self, "read dma");
                dbg!(
                    self,
                    "use_dma{} rx len: {}, addr_aligned: {}, buf_aligned: {}",
                    use_dma,
                    op_info.rx_buf.len(),
                    addr_aligned,
                    buf_aligned
                );
                if use_dma {
                    return self.read_dma(op_info);
                } else {
                    self.spi_nor_transceive_user(op_info);
                }
            } else if op_info.data_direct == SPI_NOR_DATA_DIRECT_WRITE {
                dbg!(self, "write dma");

                #[cfg(feature = "spi_dma_write")]
                {
                    let buf_aligned = (op_info.tx_buf.as_ptr() as usize) % 4 == 0;
                    let use_dma = !self.spi_config.pure_spi_mode_only
                        && op_info.tx_buf.len() > SPI_DMA_TRIGGER_LEN as usize
                        && addr_aligned
                        && buf_aligned;
                    dbg!(
                        self,
                        "use_dma{} tx len: {}, addr_aligned: {}, buf_aligned: {}",
                        use_dma,
                        op_info.rx_buf.len(),
                        addr_aligned,
                        buf_aligned
                    );
                    if use_dma {
                        return self.write_dma(op_info);
                    } else {
                        self.spi_nor_transceive_user(op_info);
                    }
                } //spi dma write
                #[cfg(not(feature = "spi_dma_write"))]
                self.spi_nor_transceive_user(op_info);
            } //write
            Ok(())
        } // dma

        #[cfg(not(feature = "spi_dma"))]
        {
            dbg!(self, "no dma transceive user");
            self.spi_nor_transceive_user(op_info);
            Ok(())
        }
    }

    fn dma_disable(&mut self) {
        self.regs.spi080().write(|w| unsafe { w.bits(0x0) });

        self.regs
            .spi080()
            .write(|w| unsafe { w.bits(SPI_DMA_DISCARD_REQ_MAGIC) });
    }

    fn wait_for_dma_completion(&mut self, timeout: u32) -> Result<(), SpiError> {
        let mut delay = DummyDelay {};
        let mut to = timeout;
        //wait for_dma done
        while !self.regs.spi008().read().dmastatus().is_dma_finish() {
            delay.delay_ns(500);
            to -= 1;

            if to == 0 {
                self.dma_disable();
                return Err(SpiError::DmaTimeout);
            }
        }
        self.dma_disable();
        Ok(())
    }
    /*
    fn dma_irq_disable(&mut self) {
        // Enable the DMA interrupt bit (bit 3)
        self.regs.spi008().modify(|_, w| w.dmaintenbl().clear_bit());
    }

    fn dma_irq_enable(&mut self) {
        // Enable the DMA interrupt bit (bit 3)
        self.regs.spi008().modify(|_, w| w.dmaintenbl().set_bit());
    }
    */
    pub fn read_dma(&mut self, op: &mut SpiNorData) -> Result<(), SpiError> {
        let cs = self.current_cs;
        dbg!(self, "##### read dma ####");
        dbg!(self, "device size: 0x{:08x} dv start: 0x{:08x}, read len: 0x{:08x}, rx_buf:0x{:08x} op addr: 0x{:08x}",
         self.spi_data.decode_addr[cs].len,
         self.spi_data.decode_addr[cs].start,
        op.rx_buf.len(),
        (op.rx_buf.as_ptr() as u32),
        op.addr);

        // Length check
        if op.rx_buf.len() > self.spi_data.decode_addr[cs].len.try_into().unwrap() {
            return Err(SpiError::Other("Invalid read length"));
        }

        // Alignment check
        if (op.addr % 4 != 0) || ((op.rx_buf.as_ptr() as u32) % 4 != 0) {
            return Err(SpiError::AddressNotAligned(op.addr));
        }

        dbg!(self, "set ctrl ");
        // Construct control value
        let mut ctrl = self.spi_data.cmd_mode[cs].normal_read & SPI_CTRL_FREQ_MASK;
        ctrl |= spi_io_mode(op.mode);
        ctrl |= (op.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT;

        // Calculate dummy cycle bits
        let bus_width = get_addr_buswidth(op.mode as u32);
        let dummy = (op.dummy_cycle / (8 / u32::from(bus_width))) << SPI_CTRL_CEX_DUMMY_SHIFT;
        ctrl |= dummy;
        ctrl |= ASPEED_SPI_NORMAL_READ;

        // Write to CSx control
        cs_ctrlreg_w!(self, cs, ctrl);

        self.regs
            .spi080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        // Wait for grant (busy wait)
        if self.regs.spi080().read().bits() & SPI_DMA_REQUEST == SPI_DMA_REQUEST {
            while self.regs.spi080().read().bits() & SPI_DMA_GRANT != SPI_DMA_GRANT {}
        }

        let flash_start = self.spi_data.decode_addr[cs].start + op.addr - SPI_DMA_FLASH_MAP_BASE;
        dbg!(self, "flash start: 0x{:08x}", flash_start);

        // DMA flash and RAM address
        self.regs.spi084().write(|w| unsafe { w.bits(flash_start) });

        let ram_addr = (op.rx_buf.as_ptr() as usize) + SPI_DMA_RAM_MAP_BASE as usize;
        //let ram_addr = op.rx_buf.as_ptr() as usize;
        dbg!(self, "ram start: 0x{:08x}", ram_addr);
        self.regs
            .spi088()
            .write(|w| unsafe { w.bits(u32::try_from(ram_addr).unwrap()) });
        let read_length = op.rx_buf.len() - 1;
        self.regs
            .spi08c()
            .write(|w| unsafe { w.bits(u32::try_from(read_length).unwrap()) });

        // Enable IRQ
        //self.dma_irq_enable();

        // Start DMA
        // self.regs.spi080().write(|w| unsafe { w.bits(SPI_DMA_ENABLE) });
        self.regs.spi080().modify(|_, w| {
            w.dmaenbl().enable_dma_operation();
            w.dmadirection()
                .read_flash_move_from_flash_to_external_memory()
        });

        dbg!(self, "start wait for dma");
        self.wait_for_dma_completion(SPI_DMA_TIMEOUT)
    }

    #[allow(dead_code)]
    fn write_dma(&mut self, op: &mut SpiNorData) -> Result<(), SpiError> {
        let cs = self.current_cs;
        dbg!(self, "##### write_dma ####");

        // Check alignment and bounds
        if op.addr % 4 != 0 || (op.tx_buf.as_ptr() as usize) % 4 != 0 {
            return Err(SpiError::AddressNotAligned(op.addr));
        }
        if op.tx_buf.len() > self.spi_data.decode_addr[cs].len.try_into().unwrap() {
            return Err(SpiError::Other("Write length exceeds decode region"));
        }

        // Set command register
        let mut ctrl_reg = self.spi_data.cmd_mode[cs].normal_write & SPI_CTRL_FREQ_MASK;
        let bus_width = get_addr_buswidth(op.mode as u32);
        ctrl_reg |= spi_io_mode(op.mode); // you must implement this
        ctrl_reg |= (op.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT;
        ctrl_reg |= (op.dummy_cycle / u32::from(8 / bus_width)) << SPI_CTRL_CEX_DUMMY_SHIFT;
        ctrl_reg |= ASPEED_SPI_NORMAL_WRITE;
        dbg!(
            self,
            "write opcode: {} , addr/offset: {}",
            op.opcode,
            op.addr
        );
        cs_ctrlreg_w!(self, cs, ctrl_reg);

        // Write DMA control (REQ magic)
        self.regs
            .spi080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        if self.regs.spi080().read().bits() & SPI_DMA_REQUEST == SPI_DMA_REQUEST {
            while self.regs.spi080().read().bits() & SPI_DMA_GRANT != SPI_DMA_GRANT {}
        }

        // Program addresses
        self.regs.spi084().write(|w| unsafe {
            w.bits(self.spi_data.decode_addr[cs].start + op.addr - SPI_DMA_FLASH_MAP_BASE)
        });
        self.regs.spi088().write(|w| unsafe {
            w.bits(u32::try_from(op.tx_buf.as_ptr() as usize).unwrap() + SPI_DMA_RAM_MAP_BASE)
        });
        self.regs
            .spi08c()
            .write(|w| unsafe { w.bits(u32::try_from(op.tx_buf.len()).unwrap() - 1) });

        // Enable DMA IRQ if needed
        // self.enable_dma_irq(); // implement if necessary

        // Start DMA with write direction
        self.regs.spi080().modify(|_, w| {
            w.dmaenbl().enable_dma_operation();
            w.dmadirection()
                .write_flash_move_from_external_memory_to_flash()
        });

        self.wait_for_dma_completion(SPI_DMA_TIMEOUT)
    }
}

impl<'a> SpiBus<u8> for SpiController<'a> {
    // we only use mmap for all transaction
    fn read(&mut self, buffer: &mut [u8]) -> Result<(), SpiError> {
        let ahb_addr = self.spi_data.decode_addr[self.current_cs].start as usize as *const u32;
        unsafe { spi_read_data(ahb_addr, buffer) };
        Ok(())
    }

    fn write(&mut self, buffer: &[u8]) -> Result<(), SpiError> {
        let ahb_addr = self.spi_data.decode_addr[self.current_cs].start as usize as *mut u32;
        unsafe { spi_write_data(ahb_addr, buffer) };
        Ok(())
    }

    fn transfer(&mut self, rd_buffer: &mut [u8], wr_buffer: &[u8]) -> Result<(), SpiError> {
        let cs = self.current_cs;
        if !wr_buffer.is_empty() {
            let ahb_addr = self.spi_data.decode_addr[cs].start as usize as *mut u32;
            unsafe { spi_write_data(ahb_addr, wr_buffer) };
        }
        cortex_m::asm::delay(2);
        if !rd_buffer.is_empty() {
            let ahb_addr = self.spi_data.decode_addr[cs].start as usize as *const u32;
            // Read RX buffer
            unsafe { super::spi_read_data(ahb_addr, rd_buffer) };
        }
        Ok(())
    }

    fn transfer_in_place(&mut self, _buffer: &mut [u8]) -> Result<(), SpiError> {
        /*let mut temp = [0u8; 2048]; //TODO:  adjust as needed
        let len = buffer.len();
        temp[..len].copy_from_slice(buffer);
        self.transfer(buffer, &temp[..len])
        */
        todo!()
    }

    fn flush(&mut self) -> Result<(), SpiError> {
        todo!()
    }
}

impl<'a> SpiBusWithCs for SpiController<'a> {
    fn select_cs(&mut self, cs: usize) -> Result<(), SpiError> {
        let user_reg = self.spi_data.cmd_mode[cs].user;
        if cs > self.spi_config.max_cs {
            return Err(SpiError::CsSelectFailed(cs));
        }
        self.current_cs = cs;
        cs_ctrlreg_w!(self, cs, user_reg | ASPEED_SPI_USER_INACTIVE);
        cs_ctrlreg_w!(self, cs, user_reg);
        dbg!(self, "activate cs:{}", u32::try_from(cs).unwrap());
        Ok(())
    }

    fn deselect_cs(&mut self, cs: usize) -> Result<(), SpiError> {
        let user_reg = self.spi_data.cmd_mode[cs].user;
        if cs > self.spi_config.max_cs {
            return Err(SpiError::CsSelectFailed(cs));
        }
        cs_ctrlreg_w!(self, cs, user_reg | ASPEED_SPI_USER_INACTIVE);
        cs_ctrlreg_w!(self, cs, self.spi_data.cmd_mode[cs].normal_read);
        dbg!(self, "deactivate cs:{}", u32::try_from(cs).unwrap());
        dbg!(
            self,
            "normal read:{:08x}",
            self.spi_data.cmd_mode[cs].normal_read
        );
        Ok(())
    }

    fn nor_transfer(&mut self, op_info: &mut SpiNorData) -> Result<(), SpiError> {
        let _ = self.spi_nor_transceive(op_info);
        Ok(())
    }

    fn nor_read_init(&mut self, cs: usize, op_info: &SpiNorData) {
        self.spi_nor_read_init(cs, op_info);
    }

    fn nor_write_init(&mut self, cs: usize, op_info: &SpiNorData) {
        self.spi_nor_write_init(cs, op_info);
    }

    fn get_device_info(&mut self, cs: usize) -> (u32, u32) {
        (
            self.spi_data.decode_addr[cs].len,
            self.spi_config.write_block_size,
        )
    }

    fn get_master_id(&mut self) -> u32 {
        self.spi_config.master_idx
    }
}
