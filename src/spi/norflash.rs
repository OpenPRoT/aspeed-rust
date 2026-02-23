// Licensed under the Apache-2.0 license

use super::device::ChipSelectDevice;
use super::SpiBusWithCs;
use super::{norflash, SpiError, SPI_NOR_DATA_DIRECT_READ, SPI_NOR_DATA_DIRECT_WRITE};
use crate::common::DummyDelay;
use crate::spimonitor::SpipfInstance;
use embedded_hal::delay::DelayNs;

/* Flash opcodes */
pub const SPI_NOR_CMD_WRSR: u32 = 0x01; /* Write status register */
pub const SPI_NOR_CMD_RDSR: u32 = 0x05; /* Read status register */
pub const SPI_NOR_CMD_WRSR2: u32 = 0x31; /* Write status register 2 */
pub const SPI_NOR_CMD_RDSR2: u32 = 0x35; /* Read status register 2 */
pub const SPI_NOR_CMD_RDSR3: u32 = 0x15; /* Read status register 3 */
pub const SPI_NOR_CMD_WRSR3: u32 = 0x11; /* Write status register 3 */
pub const SPI_NOR_CMD_READ: u32 = 0x03; /* Read data */
pub const SPI_NOR_CMD_READ_FAST: u32 = 0x0B; /* Read data */
pub const SPI_NOR_CMD_DREAD: u32 = 0x3B; /* Read data (1-1-2) */
pub const SPI_NOR_CMD_2READ: u32 = 0xBB; /* Read data (1-2-2) */
pub const SPI_NOR_CMD_QREAD: u32 = 0x6B; /* Read data (1-1-4) */
pub const SPI_NOR_CMD_4READ: u32 = 0xEB; /* Read data (1-4-4) */
pub const SPI_NOR_CMD_WREN: u32 = 0x06; /* Write enable */
pub const SPI_NOR_CMD_WRDI: u32 = 0x04; /* Write disable */
pub const SPI_NOR_CMD_PP: u32 = 0x02; /* Page program */
pub const SPI_NOR_CMD_PP_1_1_2: u32 = 0xA2; /* Dual Page program (1-1-2) */
pub const SPI_NOR_CMD_PP_1_1_4: u32 = 0x32; /* Quad Page program (1-1-4) */
pub const SPI_NOR_CMD_PP_1_4_4: u32 = 0x38; /* Quad Page program (1-4-4) */
pub const SPI_NOR_CMD_RDCR: u32 = 0x15; /* Read control register */
pub const SPI_NOR_CMD_SE: u32 = 0x20; /* Sector erase */
pub const SPI_NOR_CMD_SE_4B: u32 = 0x21; /* Sector erase 4 byte address*/
pub const SPI_NOR_CMD_BE_32K: u32 = 0x52; /* Block erase 32KB */
pub const SPI_NOR_CMD_BE: u32 = 0xD8; /* Block erase */
pub const SPI_NOR_CMD_BE_32K_4B: u32 = 0x5C; /* Block erase 32KB */
pub const SPI_NOR_CMD_BE_4B: u32 = 0xDC; /* Block erase */
pub const SPI_NOR_CMD_CE: u32 = 0xC7; /* Chip erase */
pub const SPI_NOR_CMD_RDID: u32 = 0x9F; /* Read JEDEC ID */
pub const SPI_NOR_CMD_ULBPR: u32 = 0x98; /* Global Block Protection Unlock */
pub const SPI_NOR_CMD_4BA: u32 = 0xB7; /* Enter 4-Byte Address Mode */
pub const SPI_NOR_CMD_EXIT_4BA: u32 = 0xE9; /* Exit 4-Byte Address Mode */
pub const SPI_NOR_CMD_DPD: u32 = 0xB9; /* Deep Power Down */

pub const SPI_NOR_CMD_READ_4B: u32 = 0x13; /* Read data 4 Byte Address */
pub const SPI_NOR_CMD_READ_FAST_4B: u32 = 0x0C; /* Fast Read 4 Byte Address */
pub const SPI_NOR_CMD_DREAD_4B: u32 = 0x3C; /* Read data (1-1-2) 4 Byte Address */
pub const SPI_NOR_CMD_2READ_4B: u32 = 0xBC; /* Read data (1-2-2) 4 Byte Address */
pub const SPI_NOR_CMD_QREAD_4B: u32 = 0x6C; /* Read data (1-1-4) 4 Byte Address */
pub const SPI_NOR_CMD_4READ_4B: u32 = 0xEC; /* Read data (1-4-4) 4 Byte Address */
pub const SPI_NOR_CMD_PP_4B: u32 = 0x12; /* Page Program 4 Byte Address */
pub const SPI_NOR_CMD_PP_1_1_4_4B: u32 = 0x34; /* Quad Page program (1-1-4) 4 Byte Address */
pub const SPI_NOR_CMD_PP_1_4_4_4B: u32 = 0x3e; /* Quad Page program (1-4-4) 4 Byte Address */

pub const SPI_NOR_CMD_RESET_EN: u32 = 0x66; /* Reset Enable */
pub const SPI_NOR_CMD_RESET_MEM: u32 = 0x99; /* Reset Memory */

pub const SPI_NOR_CMD_RDSFDP: u32 = 0x5A; /* Read SFDP */
/* Status register bits */
pub const SPI_NOR_WIP_BIT: u32 = 0x1; /* Write in progress */
pub const SPI_NOR_WEL_BIT: u32 = 0x2; /* Write enable latch */

pub const SPI_NOR_MFR_ID_WINBOND: u8 = 0xEF;
pub const SPI_NOR_MFR_ID_MXIC: u8 = 0xC2;
pub const SPI_NOR_MFR_ID_ST: u8 = 0x20;
pub const SPI_NOR_MFR_ID_MICRON: u8 = 0x2C;
pub const SPI_NOR_MFR_ID_ISSI: u8 = 0x9D;
pub const SPI_NOR_MFR_ID_GIGADEVICE: u8 = 0xC8;
pub const SPI_NOR_MFR_ID_CYPRESS: u8 = 0x34;

pub const SPI_NOR_PAGE_SIZE: usize = 256;
pub const SPI_NOR_SECTOR_SIZE: usize = 4096;

#[derive(Clone, Copy)]
pub enum Jesd216Mode {
    Mode044 = 0x0000_0044, /* implied instruction, execute in place */
    Mode088 = 0x0000_0088,
    Mode111 = 0x0000_0111,
    Mode111Fast = 0x1000_0111,
    Mode112 = 0x0000_0112,
    Mode114 = 0x0000_0114,
    Mode118 = 0x0000_0118,
    Mode122 = 0x0000_0122,
    Mode144 = 0x0000_0144,
    Mode188 = 0x0000_0188,
    Mode222 = 0x0000_0222,
    Mode444 = 0x0000_0444,
    Mode888 = 0x0000_0888,
    Mode44D4D = 0x2000_0444,
    Mode8D8D8D = 0x2000_0888,
    Unknown = 0xFFF_FFFF,
}

pub struct SpiNorData<'a> {
    pub mode: Jesd216Mode,
    pub opcode: u32,
    pub dummy_cycle: u32,
    pub addr_len: u32,
    pub addr: u32,
    pub data_len: u32,
    pub tx_buf: &'a [u8],
    pub rx_buf: &'a mut [u8],
    pub data_direct: u32,
}

pub trait SpiNorDevice {
    type Error;
    fn nor_read_init(&mut self, data: &SpiNorData) -> Result<(), Self::Error>;
    fn nor_write_init(&mut self, data: &SpiNorData) -> Result<(), Self::Error>;
    fn nor_write_enable(&mut self) -> Result<(), Self::Error>;
    fn nor_write_disable(&mut self) -> Result<(), Self::Error>;
    fn nor_read_jedec_id(&mut self) -> Result<[u8; 3], Self::Error>;
    fn nor_sector_erase(&mut self, address: u32) -> Result<(), Self::Error>;
    fn nor_page_program(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error>;
    fn nor_page_program_4b(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error>;
    fn nor_read_data(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Self::Error>;
    fn nor_read_fast_4b_data(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Self::Error>;
    fn nor_sector_aligned(&mut self, address: u32) -> bool;
    fn nor_wait_until_ready(&mut self);
    fn nor_reset(&mut self) -> Result<(), Self::Error>;
    fn nor_reset_enable(&mut self) -> Result<(), Self::Error>;
}

macro_rules! start_transfer {
    ($this:expr, $data:expr) => {{
        let _ = (|| -> Result<(), SpiError> {
            $this.bus.select_cs($this.cs)?;
            // SPIM config
            if let Some(spim) = $this.spi_monitor.as_mut() {
                if $this.bus.get_master_id() != 0 {
                    spim.spim_scu_ctrl_set(0x8, 0x8);
                    spim.spim_scu_ctrl_set(0x7, 1 + SPIPF::FILTER_ID as u32);
                }
                super::spim_proprietary_pre_config();
            }

            $this.bus.nor_transfer($data)?;
            $this.bus.deselect_cs($this.cs)?;
            //SPIM deconfig
            super::spim_proprietary_post_config();
            if let Some(spim) = $this.spi_monitor.as_mut() {
                if $this.bus.get_master_id() != 0 {
                    spim.spim_scu_ctrl_clear(0xf);
                }
            }
            Ok(())
        })();
    }};
}

//TODO: add 4byte address mode support
impl<B, SPIPF> SpiNorDevice for ChipSelectDevice<'_, B, SPIPF>
where
    B: SpiBusWithCs,
    SPIPF: SpipfInstance,
{
    type Error = B::Error;

    fn nor_write_enable(&mut self) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: SPI_NOR_CMD_WREN,
            dummy_cycle: 0,
            addr: 0,
            addr_len: 0,
            data_len: 0,
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
            tx_buf: &[],
            rx_buf: &mut [],
        };
        start_transfer!(self, &mut nor_data);
        Ok(())
    }

    fn nor_write_disable(&mut self) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: SPI_NOR_CMD_WRDI,
            dummy_cycle: 0,
            addr: 0,
            addr_len: 0,
            data_len: 0,
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
            tx_buf: &[],
            rx_buf: &mut [],
        };
        start_transfer!(self, &mut nor_data);
        Ok(())
    }

    fn nor_read_jedec_id(&mut self) -> Result<[u8; 3], Self::Error> {
        let mut read_buf: [u8; 3] = [0, 0, 0];
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: 0x9F,
            dummy_cycle: 0,
            addr: 0,
            addr_len: 0,
            data_len: 0,
            rx_buf: &mut read_buf,
            tx_buf: &[],
            data_direct: SPI_NOR_DATA_DIRECT_READ,
        };
        start_transfer!(self, &mut nor_data);
        Ok([read_buf[0], read_buf[1], read_buf[2]])
    }

    fn nor_sector_erase(&mut self, address: u32) -> Result<(), Self::Error> {
        self.nor_write_enable()?;
        if self.nor_sector_aligned(address) {
            let mut nor_data = SpiNorData {
                mode: Jesd216Mode::Mode111,
                opcode: norflash::SPI_NOR_CMD_SE,
                dummy_cycle: 0,
                addr: address,
                addr_len: 3,
                data_len: 0,
                tx_buf: &[],
                rx_buf: &mut [],
                data_direct: SPI_NOR_DATA_DIRECT_WRITE,
            };
            start_transfer!(self, &mut nor_data);
            self.nor_wait_until_ready();
            Ok(())
        } else {
            Err(SpiError::AddressNotAligned(address))
        }
    }

    fn nor_page_program(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error> {
        self.nor_write_enable()?;
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: norflash::SPI_NOR_CMD_PP,
            dummy_cycle: 0,
            addr: address,
            addr_len: 3,
            data_len: u32::try_from(data.len()).unwrap(),
            tx_buf: data,
            rx_buf: &mut [],
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
        };
        start_transfer!(self, &mut nor_data);
        self.nor_wait_until_ready();
        Ok(())
    }

    fn nor_page_program_4b(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error> {
        self.nor_write_enable()?;
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: norflash::SPI_NOR_CMD_PP_4B,
            dummy_cycle: 0,
            addr: address,
            addr_len: 4,
            data_len: u32::try_from(data.len()).unwrap(),
            tx_buf: data,
            rx_buf: &mut [],
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
        };
        start_transfer!(self, &mut nor_data);
        self.nor_wait_until_ready();
        Ok(())
    }

    fn nor_read_data(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode114,
            opcode: SPI_NOR_CMD_QREAD,
            dummy_cycle: 8,
            addr: address,
            addr_len: 3,
            data_len: u32::try_from(buf.len()).unwrap(), // it is not in used.
            tx_buf: &[],
            rx_buf: buf,
            data_direct: SPI_NOR_DATA_DIRECT_READ,
        };
        start_transfer!(self, &mut nor_data);

        Ok(())
    }

    fn nor_read_fast_4b_data(&mut self, address: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111Fast,
            opcode: SPI_NOR_CMD_READ_FAST_4B,
            dummy_cycle: 8,
            addr: address,
            addr_len: 4,
            data_len: u32::try_from(buf.len()).unwrap(), // it is not in used.
            tx_buf: &[],
            rx_buf: buf,
            data_direct: SPI_NOR_DATA_DIRECT_READ,
        };
        start_transfer!(self, &mut nor_data);

        Ok(())
    }

    fn nor_reset_enable(&mut self) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: SPI_NOR_CMD_RESET_EN,
            dummy_cycle: 0,
            addr: 0x0,
            addr_len: 0x0,
            data_len: 0x0, // it is not in used.
            tx_buf: &[],
            rx_buf: &mut [],
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
        };
        start_transfer!(self, &mut nor_data);

        Ok(())
    }

    fn nor_reset(&mut self) -> Result<(), Self::Error> {
        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: SPI_NOR_CMD_RESET_MEM,
            dummy_cycle: 0,
            addr: 0x0,
            addr_len: 0x0,
            data_len: 0x0, // it is not in used.
            tx_buf: &[],
            rx_buf: &mut [],
            data_direct: SPI_NOR_DATA_DIRECT_WRITE,
        };
        start_transfer!(self, &mut nor_data);

        Ok(())
    }

    fn nor_read_init(&mut self, nor_data: &SpiNorData) -> Result<(), Self::Error> {
        if let Some(spim) = self.spi_monitor.as_mut() {
            if self.bus.get_master_id() != 0 {
                spim.spim_scu_ctrl_set(0x8, 0x8);
                spim.spim_scu_ctrl_set(0x7, 1 + SPIPF::FILTER_ID as u32);
            }
            super::spim_proprietary_pre_config();
        }

        self.bus.nor_read_init(self.cs, nor_data);

        super::spim_proprietary_post_config();
        if let Some(spim) = self.spi_monitor.as_mut() {
            if self.bus.get_master_id() != 0 {
                spim.spim_scu_ctrl_clear(0xf);
            }
        }
        Ok(())
    }

    fn nor_write_init(&mut self, nor_data: &SpiNorData) -> Result<(), Self::Error> {
        self.bus.nor_write_init(self.cs, nor_data);
        Ok(())
    }

    fn nor_sector_aligned(&mut self, address: u32) -> bool {
        //let (flash_sz, sector_sz) = self.bus.get_device_info(self.cs);
        let bits = 12;
        let mask = (1 << bits) - 1;
        (address & mask) == 0
    }

    fn nor_wait_until_ready(&mut self) {
        let mut delay = DummyDelay {};
        let mut buf: [u8; 1] = [0u8];

        let mut nor_data = SpiNorData {
            mode: Jesd216Mode::Mode111,
            opcode: SPI_NOR_CMD_RDSR,
            dummy_cycle: 0,
            addr: 0,
            addr_len: 0,
            data_len: 1, // it is not in used.
            tx_buf: &[],
            rx_buf: &mut buf,
            data_direct: SPI_NOR_DATA_DIRECT_READ,
        };
        loop {
            start_transfer!(self, &mut nor_data);
            delay.delay_ns(1_000);
            if (u32::from(nor_data.rx_buf[0]) & SPI_NOR_WIP_BIT) == 0 {
                break;
            }
        }
    }
}
