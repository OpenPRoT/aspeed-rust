// Licensed under the Apache-2.0 license

use super::{
    aspeed_get_spi_freq_div, get_addr_buswidth, get_hclock_rate, get_mid_point_of_longest_one,
    spi_cal_dummy_cycle, spi_calibration_enable, spi_io_mode, spi_io_mode_user, spi_read_data,
    spi_write_data, CtrlType, SpiBusWithCs, SpiConfig, SpiData, SpiError, get_cmd_buswidth,
    get_data_buswidth, DataDirection, AddressWidth, FlashAddress
};
use super::consts::{ASPEED_MAX_CS,ASPEED_SPI_NORMAL_READ, ASPEED_SPI_NORMAL_WRITE, ASPEED_SPI_SZ_256M, 
    ASPEED_SPI_SZ_2M, ASPEED_SPI_USER, ASPEED_SPI_USER_INACTIVE, SPI_CALIB_LEN, 
    SPI_CTRL_FREQ_MASK, SPI_DMA_CALC_CKSUM, SPI_DMA_CALIB_MODE, SPI_DMA_DISCARD_REQ_MAGIC, 
    SPI_DMA_ENABLE, SPI_DMA_FLASH_MAP_BASE, SPI_DMA_GET_REQ_MAGIC, SPI_DMA_GRANT, 
    SPI_DMA_RAM_MAP_BASE, SPI_DMA_REQUEST, SPI_DMA_STATUS, SPI_DMA_TIMEOUT,
    SPI_CTRL_CEX_4BYTE_MODE_SET, SPI_CTRL_CEX_DUMMY_SHIFT, SPI_CTRL_CEX_SPI_CMD_MASK, 
    SPI_CTRL_CEX_SPI_CMD_SHIFT, SPI_DMA_CLK_FREQ_SHIFT, SPI_DMA_CLK_FREQ_MASK, SPI_DMA_DELAY_MASK, 
    SPI_DMA_DELAY_SHIFT, SPI_CONF_CE0_ENABLE_WRITE_SHIFT};

use embedded_io::Write;

#[cfg(feature = "spi_dma")]
use super::consts::{SPI_DMA_TRIGGER_LEN};

use crate::dbg;

use crate::{common::DummyDelay, spi::norflash::SpiNorCommand, uart::UartController};
use embedded_hal::{
    delay::DelayNs,
    spi::{ErrorType, SpiBus},
};

impl<'a> ErrorType for FmcController<'a> {
    type Error = SpiError;
}

pub struct FmcController<'a> {
    regs: &'static ast1060_pac::fmc::RegisterBlock,
    current_cs: usize,
    spi_config: SpiConfig,
    spi_data: SpiData,
    pub dbg_uart: Option<&'a mut UartController<'a>>,
}

macro_rules! cs_ctrlreg_w {
    ($this:expr, $cs:expr, $value:expr) => {{
        if $cs == 0 {
            $this.regs.fmc010().write(|w| unsafe { w.bits($value) });
        } else if $cs == 1 {
            $this.regs.fmc014().write(|w| unsafe { w.bits($value) });
        }
    }};
}

macro_rules! cs_ctrlreg_r {
    ($this:expr, $cs:expr) => {{
        if $cs == 0 {
            $this.regs.fmc010().read().bits()
        } else if $cs == 1 {
            $this.regs.fmc014().read().bits()
        } else {
            0
        }
    }};
}

impl<'a> FmcController<'a> {
    pub fn new(
        regs: &'static ast1060_pac::fmc::RegisterBlock,
        current_cs: usize,
        spi_config: SpiConfig,
        spi_data: SpiData,
        dbg_uart: Option<&'a mut UartController<'a>>,
    ) -> Self {
        FmcController {
            regs,
            current_cs,
            spi_config,
            spi_data,
            dbg_uart,
        }
    }

    pub fn init(&mut self) -> Result<(), SpiError> {
        dbg!(self, "fmcController: init()");

        for cs in 0..self.spi_config.max_cs {
            self.regs.fmc000().modify(|r, w| unsafe {
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
        let max_cs = self.spi_config.max_cs;
        let mut unit_sz = ASPEED_SPI_SZ_2M;
        dbg!(self, "rang pre - init()");

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
                    self.regs.fmc030().write(|w| unsafe { w.bits(0) });
                } else if cs == 1 {
                    self.regs.fmc034().write(|w| unsafe { w.bits(0) });
                }
                continue;
            }

            let seg_val = self.segment_compose(start_addr, end_addr);
            if cs == 0 {
                self.regs.fmc030().write(|w| unsafe { w.bits(seg_val) });
            } else if cs == 1 {
                self.regs.fmc034().write(|w| unsafe { w.bits(seg_val) });
            }

            self.spi_data.decode_addr[cs].start = start_addr;
            self.spi_data.decode_addr[cs].len = unit_sz;
            pre_end_addr = end_addr + 1;
        }
    }

    #[allow(clippy::unused_self)]
    fn segment_start(&mut self, reg_val: u32) -> u32 {
        (reg_val & 0x0ff8) << 16
    }

    #[allow(clippy::unused_self)]
    fn segment_end(&mut self, reg_val: u32) -> u32 {
        (reg_val & 0x0ff8_0000) | 0x0007_ffff
    }

    #[allow(clippy::unused_self)]
    fn segment_compose(&mut self, start: u32, end: u32) -> u32 {
        ((((start >> 19) << 19) >> 16) & 0xfff8) | (((end >> 19) << 19) & 0xfff8_0000)
    }

    fn decode_range_reinit(&mut self, flash_sz: u32) {
        let mut decode_sz_arr = [0u32; ASPEED_MAX_CS];
        let mut total_decode_range = 0;
        let mut pre_end_addr = 0;
        dbg!(self, "rang reinit() flash size: {}", flash_sz);

        for (cs, size) in decode_sz_arr
            .iter_mut()
            .enumerate()
            .take(self.spi_config.max_cs)
        {
            let tmp = match cs {
                0 => self.regs.fmc030().read().bits(),
                1 => self.regs.fmc034().read().bits(),
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
        dbg!(self, "total range: {}", total_decode_range);

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
                self.regs.fmc030().write(|w| unsafe { w.bits(value) });
            } else if cs == 1 {
                self.regs.fmc034().write(|w| unsafe { w.bits(value) });
            }

            self.spi_data.decode_addr[cs].start = start_addr;

            if cs == self.current_cs {
                self.spi_data.decode_addr[cs].len = flash_sz;
            }

            pre_end_addr = end_addr + 1;
        }
    }

    fn spi_nor_read_init(&mut self, cs: usize, op_info: &SpiNorCommand) {
        dbg!(
            self,
            "spi_nor_read_init() cs:{}  master_idx: {}",
            cs,
            self.spi_config.master_idx
        );

        if !self.spi_config.pure_spi_mode_only {
            self.decode_range_reinit(op_info.data_len);
        }
        let io_mode = spi_io_mode(op_info.mode);
        let dummy = spi_cal_dummy_cycle(
            get_addr_buswidth(op_info.mode as u32).into(),
            op_info.dummy_cycle,
        );
        let read_cmd = (io_mode
            | ((op_info.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT)
            | (dummy as u32))
            | ASPEED_SPI_NORMAL_READ;
        self.spi_data.cmd_mode[cs].normal_read = read_cmd;
        dbg!(
            self,
            "cs: {:08x}, io_mode: {:08x}, dummy: {:08x}, op: {:08x}, normal read: {:08x}, 
            address{:08x}",
            cs,
            io_mode,
            dummy,
            op_info.opcode,
            read_cmd,
            op_info.address.value
        );

        cs_ctrlreg_w!(self, cs, read_cmd);
        if matches!(op_info.address.width, AddressWidth::FourByte) {
            self.regs.fmc004().modify(|r, w| unsafe {
                let current = r.bits();
                w.bits(current | (SPI_CTRL_CEX_4BYTE_MODE_SET << cs))
            });
        }
        if matches!(self.spi_config.ctrl_type, CtrlType::HostSpi) {
            self.regs.fmc06c().modify(|r, w| unsafe {
                let mut current = r.bits();
                if matches!(op_info.address.width, AddressWidth::FourByte){
                    current = (current & 0xffff_00ff) | (op_info.opcode << 8);
                } else {
                    current = (current & 0xffff_ff00) | op_info.opcode;
                }

                w.bits((current & 0x0fff_ffff) | spi_io_mode(op_info.mode))
            });
        }
        self.timing_calibration(cs);
    }

    fn spi_nor_write_init(&mut self, cs: usize, op_info: &SpiNorCommand) {
        let io_mode = spi_io_mode(op_info.mode);
        let dummy = 0;
        let write_cmd = (io_mode
            | ((op_info.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT)
            | dummy)
            | ASPEED_SPI_NORMAL_WRITE;
        self.spi_data.cmd_mode[cs].normal_write = write_cmd;
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
            0 => self.regs.fmc094().read().bits(),
            1 => self.regs.fmc098().read().bits(),
            _ => return true, // invalid CS
        };

        if already_calibrated != 0 {
            dbg!(self, "Calibration already executed for cs {}", cs);
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
                self.regs.fmc084().write(|w| unsafe { w.bits(final_delay) });

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
            .fmc080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        if self.regs.fmc080().read().bits() & SPI_DMA_REQUEST != 0 {
            while self.regs.fmc080().read().bits() & SPI_DMA_GRANT == 0 {}
        }

        // Set DMA flash start address
        let flash_addr = data.decode_addr[cs].start + config.timing_calibration_start_off;
        self.regs.fmc084().write(|w| unsafe { w.bits(flash_addr) });
        // Set DMA length
        self.regs
            .fmc08c()
            .write(|w| unsafe { w.bits(u32::try_from(SPI_CALIB_LEN).unwrap()) });
        // Configure DMA control register
        let ctrl_val = SPI_DMA_ENABLE
            | SPI_DMA_CALC_CKSUM
            | SPI_DMA_CALIB_MODE
            | ((delay & SPI_DMA_DELAY_MASK) << SPI_DMA_DELAY_SHIFT)
            | ((div & SPI_DMA_CLK_FREQ_MASK) << SPI_DMA_CLK_FREQ_SHIFT);

        self.regs.fmc080().write(|w| unsafe { w.bits(ctrl_val) });
        // Wait until DMA done
        while self.regs.fmc008().read().bits() & SPI_DMA_STATUS == 0 {}

        // Read checksum result
        let checksum = self.regs.fmc090().read().bits();
        // Clear DMA control and discard request
        self.dma_disable();

        checksum
    }

    fn spi_nor_transceive_user(&mut self, op_info: &mut SpiNorCommand) {
        let cs: usize = self.current_cs;
        let dummy = [0u8; 12];
        let start_ptr = self.spi_data.decode_addr[cs].start as *mut u32;
        dbg!(
            self,
            "nor_transceive_user cs: {}, ahb start: {:08x}",
            u32::try_from(cs).unwrap(),
            self.spi_data.decode_addr[cs].start
        );
        self.activate_user();
        // Send command
        let cmd_mode = self.spi_data.cmd_mode[cs].user
            | spi_io_mode_user(u32::from(get_cmd_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, cmd_mode);
        dbg!(self, "write opcode/cmd: 0x{:08x}", op_info.opcode);
        unsafe { spi_write_data(start_ptr, &[op_info.opcode.try_into().unwrap()]) };

        // Send address
        let addr_mode = self.spi_data.cmd_mode[cs].user
            | spi_io_mode_user(u32::from(get_addr_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, addr_mode);

        let be = op_info.address.value.to_be_bytes();

        let bytes = match op_info.address.width {
             AddressWidth::ThreeByte => &be[1..], // last 3 bytes (24-bit)
             AddressWidth::FourByte => &be[..],
             AddressWidth::None => &[],
        };       

        unsafe {
            super::spi_write_data(start_ptr, bytes);
        }


        // Dummy cycles
        let bus_width: u8 = get_addr_buswidth(op_info.mode as u32);
        let dummy_len: u8 = (op_info.dummy_cycle / (8 / u32::from(bus_width)))
            .try_into()
            .unwrap();
        dbg!(self, "write dummy len: 0x{:08x}", dummy_len);
        unsafe { spi_write_data(start_ptr, &dummy[..dummy_len as usize]) };

        // Data transfer
        let data_mode = self.spi_data.cmd_mode[cs].user
            | spi_io_mode_user(u32::from(get_data_buswidth(op_info.mode as u32)));
        cs_ctrlreg_w!(self, cs, data_mode);

        if matches!(op_info.data_direct, DataDirection::DRead) {
            unsafe { spi_read_data(start_ptr, op_info.rx_buf) };
        } else {
            unsafe { spi_write_data(start_ptr, op_info.tx_buf) };
        }
        self.deactivate_user();
    }

    // Helper wrappers would be defined for spi_write_data, spi_read_data, io_mode_user, etc.

    pub fn spi_nor_transceive(&mut self, op_info: &mut SpiNorCommand) -> Result<(), SpiError> {
        dbg!(self, "spi_nor_transceive()...");

        #[cfg(feature = "spi_dma")]
        {
            dbg!(self, "spi dma enabled rx_len:{}", op_info.rx_buf.len());
            let addr_aligned = op_info.address.value % 4 == 0;

            if matches!(op_info.data_direct, DataDirection::DRead) {
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
            } else if matches!(op_info.data_direct, DataDirection::DWrite) {
                dbg!(self, "write dma");
                #[cfg(feature = "spi_dma_write")]
                {
                    let buf_aligned = (op_info.tx_buf.as_ptr() as usize) % 4 == 0;
                    let use_dma = !self.spi_config.pure_spi_mode_only
                        && op_info.tx_buf.len() > SPI_DMA_TRIGGER_LEN as usize
                        && addr_aligned
                        && buf_aligned;
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

    pub fn dma_disable(&mut self) {
        dbg!(self, "dma disable");
        self.regs.fmc080().write(|w| unsafe { w.bits(0x0) });

        self.regs
            .fmc080()
            .write(|w| unsafe { w.bits(SPI_DMA_DISCARD_REQ_MAGIC) });
    }

    pub fn wait_for_dma_completion(&mut self, timeout: u32) -> Result<(), SpiError> {
        let mut delay = DummyDelay {};
        let mut to = timeout;
        //wait for_dma done
        #[cfg(not(feature = "spi_dma_irq"))]
        {
            dbg!(self, "wait_for_dma_completion");
            while !self.regs.fmc008().read().dmastatus().is_dma_finish() {
                delay.delay_ns(500);
                to -= 1;

                if to == 0 {
                    self.dma_disable();
                    return Err(SpiError::DmaTimeout);
                }
            }
            self.dma_disable();
        }
        Ok(())
    }

    fn dma_irq_disable(&mut self) {
        // disable the DMA interrupt bit (bit 3)
        dbg!(self, "dma_irq_disable");
        self.regs.fmc008().modify(|_, w| w.dmaintenbl().clear_bit());
    }
    #[allow(dead_code)]
    fn dma_irq_enable(&mut self) {
        // Enable the DMA interrupt bit (bit 3)
        dbg!(self, "dma_irq_enable");
        self.regs.fmc008().modify(|_, w| w.dmaintenbl().set_bit());
    }
    #[allow(dead_code)]
    fn dbg_fmc_dma(&mut self) {
        dbg!(self, "reg 0x14: {:14x}", self.regs.fmc014().read().bits());
        dbg!(self, "reg 0x80: {:08x}", self.regs.fmc080().read().bits());
        dbg!(self, "reg 0x84: {:08x}", self.regs.fmc084().read().bits());
        dbg!(self, "reg 0x88: {:08x}", self.regs.fmc088().read().bits());
        dbg!(self, "reg 0x8c: {:08x}", self.regs.fmc08c().read().bits());
    }

    pub fn handle_interrupt(&mut self) -> Result<(), SpiError> {
        dbg!(self, "handle interrupt");
        if !self.regs.fmc008().read().dmastatus().is_dma_finish() {
            return Err(SpiError::Other("irq error"));
        }
        /* disable IRQ */
        self.dma_irq_disable();

        /* disable DMA */
        self.dma_disable();

        // Set it to normal read again
        let cs = self.current_cs;
        cs_ctrlreg_w!(self, cs, self.spi_data.cmd_mode[cs].normal_read);
        Ok(())
    }

    pub fn read_dma(&mut self, op: &mut SpiNorCommand) -> Result<(), SpiError> {
        let cs = self.current_cs;
        dbg!(self, "##### fmc read dma ####");
        dbg!(self, "device size: 0x{:08x} dv start: 0x{:08x}, read len: 0x{:08x}, rx_buf:0x{:08x} op addr: 0x{:08x}",
         self.spi_data.decode_addr[cs].len,
         self.spi_data.decode_addr[cs].start,
        op.rx_buf.len(),
        (op.rx_buf.as_ptr() as u32),
        op.address.value);

        // Length check
        if op.rx_buf.len() > self.spi_data.decode_addr[cs].len.try_into().unwrap() {
            return Err(SpiError::Other("Invalid read length"));
        }

        // Alignment check
        if (op.address.value % 4 != 0) || ((op.rx_buf.as_ptr() as u32) % 4 != 0) {
            return Err(SpiError::AddressNotAligned(op.address.value));
        }
        // Construct control value
        let mut ctrl = self.spi_data.cmd_mode[cs].normal_read & SPI_CTRL_FREQ_MASK;
        ctrl |= spi_io_mode(op.mode);
        ctrl |= (op.opcode & SPI_CTRL_CEX_SPI_CMD_MASK) << SPI_CTRL_CEX_SPI_CMD_SHIFT;

        // Calculate dummy cycle bits
        let bus_width = get_addr_buswidth(op.mode as u32);
        let dummy = (op.dummy_cycle / u32::from(8 / bus_width)) << SPI_CTRL_CEX_DUMMY_SHIFT;
        ctrl |= dummy;
        ctrl |= ASPEED_SPI_NORMAL_READ;

        // Write to CSx control
        cs_ctrlreg_w!(self, cs, ctrl);

        self.regs
            .fmc080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        // Wait for grant (busy wait)
        if self.regs.fmc080().read().bits() & SPI_DMA_REQUEST == SPI_DMA_REQUEST {
            while self.regs.fmc080().read().bits() & SPI_DMA_GRANT != SPI_DMA_GRANT {}
        }

        let flash_start = self.spi_data.decode_addr[cs].start + op.address.value - SPI_DMA_FLASH_MAP_BASE;
        dbg!(self, "flash start: 0x{:08x}", flash_start);
        // DMA flash and RAM address
        self.regs.fmc084().write(|w| unsafe { w.bits(flash_start) });
        let ram_addr = (op.rx_buf.as_ptr() as usize) + SPI_DMA_RAM_MAP_BASE as usize;
        dbg!(self, "ram start: 0x{:08x}", ram_addr);
        self.regs
            .fmc088()
            .write(|w| unsafe { w.bits(u32::try_from(ram_addr).unwrap()) });
        let read_length = op.rx_buf.len() - 1;
        self.regs
            .fmc08c()
            .write(|w| unsafe { w.bits(u32::try_from(read_length).unwrap()) });

        // Enable IRQ
        #[cfg(feature = "spi_dma_irq")]
        {
            self.dma_irq_enable();
        }
        //self.dbg_fmc_dma();

        // Start DMA
        dbg!(self, "start dma");
        self.regs.fmc080().modify(|_, w| {
            w.dmaenbl().enable_dma_operation();
            w.dmadirection()
                .read_flash_move_from_flash_to_external_memory()
        });

        self.wait_for_dma_completion(SPI_DMA_TIMEOUT)
    }

    #[allow(dead_code)]
    fn write_dma(&mut self, op: &mut SpiNorCommand) -> Result<(), SpiError> {
        let cs = self.current_cs;
        dbg!(self, "##### write_dma ####");
        dbg!(self, "device size: 0x{:08x} dv start: 0x{:08x}, read len: 0x{:08x}, tx_buf:0x{:08x} op addr: 0x{:08x}",
         self.spi_data.decode_addr[cs].len,
         self.spi_data.decode_addr[cs].start,
        op.tx_buf.len(),
        (op.tx_buf.as_ptr() as u32),
        op.address.value);
        //self.dbg_fmc_dma();
        // Check alignment and bounds
        if op.address.value % 4 != 0 || (op.tx_buf.as_ptr() as usize) % 4 != 0 {
            return Err(SpiError::AddressNotAligned(op.address.value));
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

        cs_ctrlreg_w!(self, cs, ctrl_reg);

        // Write DMA control (REQ magic)
        self.regs
            .fmc080()
            .write(|w| unsafe { w.bits(SPI_DMA_GET_REQ_MAGIC) });
        if self.regs.fmc080().read().bits() & SPI_DMA_REQUEST == SPI_DMA_REQUEST {
            while self.regs.fmc080().read().bits() & SPI_DMA_GRANT != SPI_DMA_GRANT {}
        }

        // Program addresses
        self.regs.fmc084().write(|w| unsafe {
            w.bits(self.spi_data.decode_addr[cs].start + op.address.value - SPI_DMA_FLASH_MAP_BASE)
        });
        self.regs.fmc088().write(|w| unsafe {
            w.bits(u32::try_from(op.tx_buf.as_ptr() as usize).unwrap() + SPI_DMA_RAM_MAP_BASE)
        });
        self.regs
            .fmc08c()
            .write(|w| unsafe { w.bits(u32::try_from(op.tx_buf.len()).unwrap() - 1) });
        //self.dbg_fmc_dma();

        // Enable DMA IRQ if needed
        #[cfg(feature = "spi_dma_irq")]
        {
            self.dma_irq_enable();
        }

        // Start DMA with write direction
        self.regs.fmc080().modify(|_, w| {
            w.dmaenbl().enable_dma_operation();
            w.dmadirection()
                .write_flash_move_from_external_memory_to_flash()
        });
        self.wait_for_dma_completion(SPI_DMA_TIMEOUT)
    }

    fn activate_user(&mut self) {
        let cs = self.current_cs;
        let user_reg = self.spi_data.cmd_mode[cs].user;
        cs_ctrlreg_w!(self, cs, user_reg | ASPEED_SPI_USER_INACTIVE);
        cs_ctrlreg_w!(self, cs, user_reg);
        dbg!(self, "activate cs:{}", u32::try_from(cs).unwrap());
    }

    fn deactivate_user(&mut self) {
        let cs = self.current_cs;
        let user_reg = self.spi_data.cmd_mode[cs].user;

        cs_ctrlreg_w!(self, cs, user_reg | ASPEED_SPI_USER_INACTIVE);
        cs_ctrlreg_w!(self, cs, self.spi_data.cmd_mode[cs].normal_read);
        dbg!(self, "deactivate cs:{}", u32::try_from(cs).unwrap());
        dbg!(
            self,
            "normal read:{:08x}",
            self.spi_data.cmd_mode[cs].normal_read
        );
    }
}

impl<'a> SpiBus<u8> for FmcController<'a> {
    // we only use mmap for all transaction
    fn read(&mut self, buffer: &mut [u8]) -> Result<(), SpiError> {
        let ahb_addr = self.spi_data.decode_addr[self.current_cs].start as usize as *const u32;
        self.activate_user();
        unsafe { spi_read_data(ahb_addr, buffer) };
        self.deactivate_user();
        Ok(())
    }

    fn write(&mut self, buffer: &[u8]) -> Result<(), SpiError> {
        let ahb_addr = self.spi_data.decode_addr[self.current_cs].start as usize as *mut u32;
        self.activate_user();
        unsafe { spi_write_data(ahb_addr, buffer) };
        self.deactivate_user();
        Ok(())
    }

    fn transfer(&mut self, rd_buffer: &mut [u8], wr_buffer: &[u8]) -> Result<(), SpiError> {
        let cs = self.current_cs;
        self.activate_user();
        if !wr_buffer.is_empty() {
            let ahb_addr = self.spi_data.decode_addr[cs].start as usize as *mut u32;
            unsafe { spi_write_data(ahb_addr, wr_buffer) };
        }
        cortex_m::asm::delay(2);
        if !rd_buffer.is_empty() {
            let ahb_addr = self.spi_data.decode_addr[cs].start as usize as *const u32;
            // Read RX buffer
            unsafe { spi_read_data(ahb_addr, rd_buffer) };
        }
        self.deactivate_user();
        Ok(())
    }

    fn transfer_in_place(&mut self, _buffer: &mut [u8]) -> Result<(), SpiError> {
        Err(SpiError::Other("transfer_in_place not supported"))
    }

    fn flush(&mut self) -> Result<(), SpiError> {
        Err(SpiError::Other("flush not supported"))
    }
}

impl<'a> SpiBusWithCs for FmcController<'a> {
    fn select_cs(&mut self, cs: usize) -> Result<(), SpiError> {
        if cs >= self.spi_config.max_cs {
            return Err(SpiError::CsSelectFailed(cs));
        }
        self.current_cs = cs;
        Ok(())
    }

    fn nor_transfer(&mut self, op_info: &mut SpiNorCommand) -> Result<(), SpiError> {
        let _ = self.spi_nor_transceive(op_info);
        Ok(())
    }

    fn nor_read_init(&mut self, cs: usize, op_info: &SpiNorCommand) {
        self.spi_nor_read_init(cs, op_info);
    }

    fn nor_write_init(&mut self, cs: usize, op_info: &SpiNorCommand) {
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
