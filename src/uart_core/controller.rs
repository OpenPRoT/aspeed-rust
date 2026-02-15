// Licensed under the Apache-2.0 license

//! UART Controller - Core hardware abstraction

use ast1060_pac as device;

use crate::uart_core::config::{Parity, StopBits, UartConfig};
use crate::uart_core::error::{Result, UartError};
use crate::uart_core::fifo::FifoTriggerLevel;
use crate::uart_core::interrupt::{InterruptConfig, InterruptSource};
use crate::uart_core::line_status::LineStatus;
use crate::uart_core::types::ModemStatus;

/// AST1060 UART Controller
///
/// Provides hardware abstraction for the 16550-compatible UART peripheral.
///
/// # Example
///
/// ```rust,no_run
/// use aspeed_ddk::uart_core::{UartController, UartConfig};
/// use ast1060_pac;
///
/// let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
/// let mut uart = UartController::new(uart_regs);
/// uart.init(&UartConfig::default()).unwrap();
/// ```
pub struct UartController<'a> {
    regs: &'a device::uart::RegisterBlock,
}

impl<'a> UartController<'a> {
    /// Create a new UART controller from register block
    ///
    /// # Arguments
    ///
    /// * `regs` - Reference to the UART register block
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
    /// let uart = UartController::new(uart_regs);
    /// ```
    pub fn new(regs: &'a device::uart::RegisterBlock) -> Self {
        Self { regs }
    }

    /// Initialize UART with the given configuration
    ///
    /// This configures:
    /// - Baud rate divisor
    /// - Word length, parity, and stop bits
    /// - FIFO settings and trigger level
    ///
    /// # Arguments
    ///
    /// * `config` - UART configuration settings
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `Err(UartError)` on failure
    pub fn init(&mut self, config: &UartConfig) -> Result<()> {
        // Configure FIFO first
        self.configure_fifo(config.fifo_enabled, config.rx_trigger_level);

        // Set baud rate (requires DLAB manipulation)
        self.set_baud_rate_divisor(config.baud_rate.calculate_divisor(config.clock_hz));

        // Configure line control (word length, parity, stop bits)
        self.configure_line_control(config);

        Ok(())
    }

    /// Set the baud rate divisor
    ///
    /// # Arguments
    ///
    /// * `divisor` - The 16-bit baud rate divisor value
    fn set_baud_rate_divisor(&self, divisor: u16) {
        // Enable DLAB to access divisor latch registers
        self.regs.uartlcr().modify(|_, w| w.dlab().set_bit());

        // Set divisor latch low byte
        self.regs.uartdll().write(|w| unsafe {
            w.the_lsbof_the_bd_divisor_latch()
                .bits((divisor & 0xFF) as u8)
        });

        // Set divisor latch high byte
        self.regs.uartdlh().write(|w| unsafe {
            w.the_msbof_the_bd_divisor_latch()
                .bits((divisor >> 8) as u8)
        });

        // Disable DLAB to access other registers
        self.regs.uartlcr().modify(|_, w| w.dlab().clear_bit());
    }

    /// Configure FIFO settings
    ///
    /// # Arguments
    ///
    /// * `enabled` - Enable or disable FIFOs
    /// * `rx_trigger` - RX FIFO interrupt trigger level
    fn configure_fifo(&self, enabled: bool, rx_trigger: FifoTriggerLevel) {
        unsafe {
            self.regs.uartfcr().write(|w| {
                if enabled {
                    w.enbl_uartfifo().set_bit();
                } else {
                    w.enbl_uartfifo().clear_bit();
                }
                w.rx_fiforst().set_bit(); // Clear RX FIFO
                w.tx_fiforst().set_bit(); // Clear TX FIFO
                w.define_the_rxr_fifointtrigger_level()
                    .bits(rx_trigger.as_bits())
            });
        }
    }

    /// Configure line control register
    ///
    /// Sets word length, stop bits, and parity.
    fn configure_line_control(&self, config: &UartConfig) {
        self.regs.uartlcr().write(|w| {
            // Word length - unsafe due to PAC API
            unsafe { w.cls().bits(config.word_length.as_bits()) };

            // Stop bits
            w.stop().bit(config.stop_bits == StopBits::Two);

            // Parity
            match config.parity {
                Parity::None => w.pen().clear_bit(),
                Parity::Even => {
                    w.pen().set_bit();
                    w.eps().set_bit()
                }
                Parity::Odd => {
                    w.pen().set_bit();
                    w.eps().clear_bit()
                }
            }
        });
    }

    /// Configure interrupt enables
    ///
    /// # Arguments
    ///
    /// * `config` - Interrupt configuration
    pub fn configure_interrupts(&self, config: &InterruptConfig) {
        self.regs.uartier().write(|w| {
            if config.rx_data_available {
                w.erbfi().set_bit();
            } else {
                w.erbfi().clear_bit();
            }
            if config.tx_empty {
                w.etbei().set_bit();
            } else {
                w.etbei().clear_bit();
            }
            if config.line_status {
                w.elsi().set_bit();
            } else {
                w.elsi().clear_bit();
            }
            if config.modem_status {
                w.edssi().set_bit();
            } else {
                w.edssi().clear_bit();
            }
            w
        });
    }

    /// Enable all interrupts
    pub fn enable_all_interrupts(&self) {
        self.configure_interrupts(&InterruptConfig::default());
    }

    /// Disable all interrupts
    pub fn disable_all_interrupts(&self) {
        self.configure_interrupts(&InterruptConfig::none());
    }

    /// Enable TX empty interrupt
    pub fn enable_tx_interrupt(&self) {
        self.regs.uartier().modify(|_, w| w.etbei().set_bit());
    }

    /// Disable TX empty interrupt
    pub fn disable_tx_interrupt(&self) {
        self.regs.uartier().modify(|_, w| w.etbei().clear_bit());
    }

    /// Enable RX data available interrupt
    pub fn enable_rx_interrupt(&self) {
        self.regs.uartier().modify(|_, w| w.erbfi().set_bit());
    }

    /// Disable RX data available interrupt
    pub fn disable_rx_interrupt(&self) {
        self.regs.uartier().modify(|_, w| w.erbfi().clear_bit());
    }

    /// Read line status register
    ///
    /// Note: Reading LSR clears some error flags.
    #[inline]
    pub fn line_status(&self) -> LineStatus {
        LineStatus::from_bits_truncate(self.regs.uartlsr().read().bits() as u8)
    }

    /// Read interrupt identification register
    ///
    /// Returns the highest priority pending interrupt source.
    #[inline]
    pub fn interrupt_source(&self) -> InterruptSource {
        InterruptSource::from_iir(self.regs.uartiir().read().intdecoding_table().bits())
    }

    /// Read modem status register
    #[inline]
    pub fn modem_status(&self) -> ModemStatus {
        ModemStatus::from_bits(self.regs.uartmsr().read().bits() as u8)
    }

    /// Check if TX FIFO/THR is full
    ///
    /// Returns true if the transmit holding register cannot accept more data.
    #[inline]
    pub fn is_tx_full(&self) -> bool {
        !self.regs.uartlsr().read().thre().bit()
    }

    /// Check if RX FIFO is empty
    ///
    /// Returns true if no data is available to read.
    #[inline]
    pub fn is_rx_empty(&self) -> bool {
        !self.regs.uartlsr().read().dr().bit()
    }

    /// Check if transmitter is completely idle
    ///
    /// Returns true when both shift register and FIFO/THR are empty.
    #[inline]
    pub fn is_tx_idle(&self) -> bool {
        self.regs.uartlsr().read().txter_empty().bit_is_set()
    }

    /// Write a single byte (non-blocking)
    ///
    /// # Arguments
    ///
    /// * `byte` - The byte to transmit
    ///
    /// # Returns
    ///
    /// `Ok(())` if the byte was written, `Err(UartError::BufferFull)` if TX is full
    #[inline]
    pub fn write_byte(&self, byte: u8) -> Result<()> {
        if self.is_tx_full() {
            return Err(UartError::BufferFull);
        }
        self.regs
            .uartthr()
            .write(|w| unsafe { w.bits(byte as u32) });
        Ok(())
    }

    /// Write a single byte (blocking)
    ///
    /// Waits until TX has space, then writes the byte.
    #[inline]
    pub fn write_byte_blocking(&self, byte: u8) {
        while self.is_tx_full() {}
        self.regs
            .uartthr()
            .write(|w| unsafe { w.bits(byte as u32) });
    }

    /// Read a single byte (non-blocking)
    ///
    /// # Returns
    ///
    /// `Ok(byte)` if data available, `Err(UartError)` on error or empty
    #[inline]
    pub fn read_byte(&self) -> Result<u8> {
        let status = self.line_status();

        if !status.has_data() {
            return Err(UartError::BufferFull); // Using BufferFull to indicate "would block"
        }

        // Check for errors
        if status.is_framing_error() {
            return Err(UartError::Frame);
        }
        if status.is_parity_error() {
            return Err(UartError::Parity);
        }
        if status.is_overrun_error() {
            return Err(UartError::Overrun);
        }
        if status.is_break() {
            return Err(UartError::Break);
        }

        Ok(self.regs.uartrbr().read().bits() as u8)
    }

    /// Read a single byte (blocking)
    ///
    /// Waits until data is available, then reads it.
    ///
    /// # Returns
    ///
    /// `Ok(byte)` on success, `Err(UartError)` on receive error
    pub fn read_byte_blocking(&self) -> Result<u8> {
        while self.is_rx_empty() {}
        self.read_byte()
    }

    /// Set RX FIFO trigger level
    ///
    /// # Arguments
    ///
    /// * `level` - The new trigger level
    pub fn set_rx_trigger_level(&self, level: FifoTriggerLevel) {
        unsafe {
            self.regs.uartfcr().modify(|_, w| {
                w.define_the_rxr_fifointtrigger_level()
                    .bits(level.as_bits())
            });
        }
    }

    /// Reset TX FIFO
    pub fn reset_tx_fifo(&self) {
        self.regs.uartfcr().modify(|_, w| w.tx_fiforst().set_bit());
    }

    /// Reset RX FIFO
    pub fn reset_rx_fifo(&self) {
        self.regs.uartfcr().modify(|_, w| w.rx_fiforst().set_bit());
    }
}

/// Allow creating controller from register block reference
impl<'a> From<&'a device::uart::RegisterBlock> for UartController<'a> {
    fn from(regs: &'a device::uart::RegisterBlock) -> Self {
        Self::new(regs)
    }
}
