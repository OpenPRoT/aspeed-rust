// Licensed under the Apache-2.0 license

use crate::common::Logger;
use crate::i2c::ast1060_i2c::Ast1060I2c;
use crate::i2c::common::{I2cConfigBuilder, I2cSpeed, I2cXferMode};
use crate::i2c::i2c_controller::{HardwareInterface, I2cController};
use crate::i2c::pfr::swmbx::SwmbxCtrl;
use crate::pinctrl;
use crate::uart::UartController;
use embedded_hal::i2c::ErrorKind;
use embedded_io::Write;
use proposed_traits::i2c_target::{
    I2CCoreTarget, ReadTarget, RegisterAccess, TransactionDirection, WriteReadTarget, WriteTarget,
};

extern "Rust" {
    static mut UART_PTR: Option<&'static mut UartController<'static>>;
}

#[macro_export]
macro_rules! swmbx_target_log {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        if let Some(uart) = unsafe { UART_PTR.as_mut() } {
            let mut buf: heapless::String<64> = heapless::String::new();
            let _ = write!(buf, $($arg)*);
            let _ = uart.write_all(b"[SWMBX_TARGET] ");
            let _ = uart.write_all(buf.as_bytes());
            let _ = uart.write_all(b"\r\n");
        }
    }};
}

#[derive(Debug)]
pub enum SwmbxI2CError {
    OtherError,
}

impl embedded_hal::i2c::Error for SwmbxI2CError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl embedded_hal::i2c::ErrorType for SwmbxI2CTarget {
    type Error = SwmbxI2CError;
}

pub struct SwmbxI2CTarget {
    address: u8,
    read_idx: usize,
    buffer_idx: u8,
    first_write: bool,
    port: usize,
}

impl I2CCoreTarget for SwmbxI2CTarget {
    fn init(&mut self, address: u8) -> Result<(), Self::Error> {
        if address == 0 {
            return Err(SwmbxI2CError::OtherError);
        }
        self.address = address;
        Ok(())
    }
    // read_requested or write_requested
    fn on_transaction_start(
        &mut self,
        direction: TransactionDirection,
        _repeated: bool,
    ) -> Result<Option<u8>, Self::Error> {
        self.read_idx = 0;

        match direction {
            TransactionDirection::Write => {
                swmbx_target_log!("Write requested on port {}", self.port);
                self.first_write = true;
                Ok(None)
            }
            TransactionDirection::Read => {
                let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();
                let val = ctrl.get_msg(self.port, self.buffer_idx);
                swmbx_target_log!("Read requested on port {}: val: {:#02x}", self.port, val);
                return Ok(Some(val));
            }
        }
    }
    fn on_stop(&mut self) {
        swmbx_target_log!("Stop on port {}", self.port);
        let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();
        ctrl.send_stop(self.port);
        self.first_write = true;
    }
    fn on_address_match(&mut self, address: u8) -> bool {
        self.address == address
    }
}

impl ReadTarget for SwmbxI2CTarget {
    // read_processed
    fn on_read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        buffer[0] = 0;
        let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();
        let mut idx = self.buffer_idx as usize;
        idx %= ctrl.buffer_size; // Ensure idx is within bounds
        self.buffer_idx = idx as u8;
        let _val = ctrl.get_msg(self.port, self.buffer_idx);
        swmbx_target_log!("Read processed on port {}: val: {:#02x}", self.port, _val);
        Ok(1)
    }
}

impl WriteTarget for SwmbxI2CTarget {
    // write_received
    fn on_write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        if data.len() == 0 {
            // ignore empty writes
            swmbx_target_log!("Empty write received on port {}", self.port);
            return Ok(());
        }

        swmbx_target_log!("Write received on port {}: data: {:02x?}", self.port, data);
        if self.first_write {
            self.buffer_idx = data[0];
            self.first_write = false;
            let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();
            ctrl.send_start(self.port, self.buffer_idx)
        } else {
            let ctrl = SwmbxCtrl::load_ctrl_ptr_mut();
            ctrl.send_msg(self.port, self.buffer_idx, data[0]);
            let mut idx = self.buffer_idx as usize;
            idx += 1;
            idx %= ctrl.buffer_size; // Ensure idx is within bounds
            self.buffer_idx = idx as u8;
        }

        Ok(())
    }
}

impl WriteReadTarget for SwmbxI2CTarget {}

impl RegisterAccess for SwmbxI2CTarget {
    fn write_register(&mut self, _address: u8, _data: u8) -> Result<(), Self::Error> {
        swmbx_target_log!("Write register called on port {}", self.port);
        Ok(())
    }
    fn read_register(&mut self, _address: u8, _buffer: &mut [u8]) -> Result<usize, Self::Error> {
        swmbx_target_log!("Read register called on port {}", self.port);
        Ok(1)
    }
}

#[cfg(feature = "i2c_target")]
impl SwmbxI2CTarget {
    pub fn new(port: usize, address: u8) -> Result<Self, SwmbxI2CError> {
        if address == 0 {
            return Err(SwmbxI2CError::OtherError);
        }
        Ok(Self {
            address,
            read_idx: 0,
            buffer_idx: 0,
            first_write: false,
            port,
        })
    }
    pub fn attach<LH, LC>(
        &mut self,
        i2c: &mut I2cController<Ast1060I2c<'static, ast1060_pac::I2c, SwmbxI2CTarget, LH>, LC>,
    ) -> Result<(), SwmbxI2CError>
    where
        LH: Logger,
        LC: Logger,
    {
        let mut config = I2cConfigBuilder::new()
            .xfer_mode(I2cXferMode::DmaMode)
            .multi_master(true)
            .smbus_timeout(true)
            .smbus_alert(false)
            .speed(I2cSpeed::Standard)
            .build();

        pinctrl::Pinctrl::apply_pinctrl_group(pinctrl::PINCTRL_I2C0);
        i2c.hardware.init(&mut config);

        let static_self = unsafe { core::mem::transmute::<&mut Self, &'static mut Self>(self) };

        i2c.hardware
            .i2c_aspeed_slave_register(self.address, Some(static_self))
            .map_err(|_| SwmbxI2CError::OtherError)
    }
}
