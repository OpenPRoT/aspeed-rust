# UART Module Refactoring Plan

This document outlines the plan to replace the current UART implementation in `aspeed-rust-new` with a modern, modular architecture following the patterns established by `i2c_core`.

## Current State Analysis

### Existing Files

| File | Location | Description | Issues |
|------|----------|-------------|--------|
| `uart/` | `src/uart/` | Separate crate | Mismatched documentation (references LPC55), relies on deprecated `embedded-hal` serial traits |
| `uart.rs` | `src/uart.rs` | Flat file module | Duplicated functionality, inconsistent with modular pattern |

### Key Problems

1. **Dual Implementation**: Both `uart/` crate and `uart.rs` exist, causing confusion and maintenance burden
2. **Misleading Documentation**: README references "lpc55-usart" but code is for AST1060
3. **Deprecated Traits**: Uses `embedded_hal::serial::{Read, Write}` which are deprecated in embedded-hal 1.0
4. **Inconsistent Pattern**: Doesn't follow the modular architecture established by `i2c_core/`
5. **Unnecessary Dependencies**: `UartController` requires `&'a mut dyn DelayNs` unnecessarily
6. **Missing Features**: Limited interrupt handling, no flexible configuration, hardcoded baud rates

---

## Proposed Architecture

### Directory Structure

```
src/uart_core/
├── mod.rs            # Module exports, documentation, and re-exports
├── controller.rs     # Core hardware abstraction (UartController struct)
├── config.rs         # Configuration types (baud rate, word length, parity, stop bits)
├── error.rs          # Error types and embedded_io::Error implementation
├── fifo.rs           # FIFO management and trigger level configuration
├── interrupt.rs      # Interrupt handling, sources, and configuration
├── line_status.rs    # LineStatus bitflags and helper methods
├── hal_impl.rs       # embedded_io trait implementations
└── types.rs          # Common type definitions and constants
```

### Module Responsibilities

#### `mod.rs`
- Module-level documentation
- Public re-exports for convenient API access
- Feature flags (if needed)

#### `config.rs`
```rust
/// Baud rate configuration
pub enum BaudRate {
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
    Baud1500000,  // 1.5 MBaud
    Custom(u32),
}

/// Word length configuration
pub enum WordLength {
    Five = 0b00,
    Six = 0b01,
    Seven = 0b10,
    Eight = 0b11,
}

/// Parity configuration
pub enum Parity {
    None,
    Even,
    Odd,
}

/// Stop bits configuration
pub enum StopBits {
    One,
    Two,
}

/// Complete UART configuration
pub struct UartConfig {
    pub baud_rate: BaudRate,
    pub word_length: WordLength,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub fifo_enabled: bool,
    pub rx_trigger_level: FifoTriggerLevel,
    pub clock_hz: u32,  // Input clock frequency (default 24MHz)
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud_rate: BaudRate::Baud115200,
            word_length: WordLength::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            fifo_enabled: true,
            rx_trigger_level: FifoTriggerLevel::EightBytes,
            clock_hz: 24_000_000,
        }
    }
}
```

#### `error.rs`
```rust
/// UART error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UartError {
    /// Framing error - invalid stop bit
    Frame,
    /// Parity error
    Parity,
    /// Receiver overrun - data lost
    Overrun,
    /// Break condition detected
    Break,
    /// Buffer full
    BufferFull,
    /// Timeout waiting for data
    Timeout,
}

impl embedded_io::Error for UartError {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            UartError::Frame | UartError::Parity => embedded_io::ErrorKind::InvalidData,
            UartError::Overrun | UartError::BufferFull => embedded_io::ErrorKind::OutOfMemory,
            UartError::Break => embedded_io::ErrorKind::Interrupted,
            UartError::Timeout => embedded_io::ErrorKind::TimedOut,
        }
    }
}

pub type Result<T> = core::result::Result<T, UartError>;
```

#### `fifo.rs`
```rust
/// Receiver FIFO interrupt trigger level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FifoTriggerLevel {
    OneByte = 0b00,
    FourBytes = 0b01,
    EightBytes = 0b10,
    FourteenBytes = 0b11,
}

/// FIFO configuration
pub struct FifoConfig {
    pub enabled: bool,
    pub rx_trigger_level: FifoTriggerLevel,
}
```

#### `interrupt.rs`
```rust
/// Interrupt source identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptSource {
    ModemStatusChange = 0,
    TxEmpty = 1,
    RxDataAvailable = 2,
    LineStatusChange = 3,
    CharacterTimeout = 6,
    Unknown,
}

/// Interrupt enable configuration
pub struct InterruptConfig {
    pub rx_data_available: bool,
    pub tx_empty: bool,
    pub line_status: bool,
    pub modem_status: bool,
}

impl Default for InterruptConfig {
    fn default() -> Self {
        Self {
            rx_data_available: true,
            tx_empty: true,
            line_status: true,
            modem_status: true,
        }
    }
}
```

#### `line_status.rs`
```rust
use bitflags::bitflags;

bitflags! {
    /// Line Status Register flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LineStatus: u8 {
        /// Error in receiver FIFO (parity, framing, or break)
        const ERROR_IN_FIFO = 0x80;
        /// Transmitter empty (shift register and FIFO/THR empty)
        const TX_EMPTY = 0x40;
        /// Transmitter holding register empty
        const TX_HOLDING_EMPTY = 0x20;
        /// Break interrupt
        const BREAK = 0x10;
        /// Framing error
        const FRAMING_ERROR = 0x08;
        /// Parity error
        const PARITY_ERROR = 0x04;
        /// Overrun error
        const OVERRUN_ERROR = 0x02;
        /// Data ready (receiver has data)
        const DATA_READY = 0x01;
    }
}

impl LineStatus {
    /// Check if any error condition is present
    pub fn has_error(&self) -> bool {
        self.intersects(
            Self::ERROR_IN_FIFO 
            | Self::FRAMING_ERROR 
            | Self::PARITY_ERROR 
            | Self::OVERRUN_ERROR
        )
    }
    
    /// Check if transmitter can accept data
    pub fn can_transmit(&self) -> bool {
        self.contains(Self::TX_HOLDING_EMPTY)
    }
    
    /// Check if receiver has data
    pub fn has_data(&self) -> bool {
        self.contains(Self::DATA_READY)
    }
}
```

#### `controller.rs`
```rust
use ast1060_pac as device;
use crate::uart_core::{UartConfig, UartError, Result, LineStatus, InterruptSource, InterruptConfig};

/// AST1060 UART Controller
pub struct UartController<'a> {
    regs: &'a device::uart::RegisterBlock,
}

impl<'a> UartController<'a> {
    /// Create a new UART controller from register block
    pub fn new(regs: &'a device::uart::RegisterBlock) -> Self {
        Self { regs }
    }

    /// Initialize UART with configuration
    pub fn init(&self, config: &UartConfig) -> Result<()> {
        // Configure FIFO
        self.configure_fifo(config.fifo_enabled, config.rx_trigger_level);
        
        // Set baud rate
        self.set_baud_rate(config.baud_rate, config.clock_hz);
        
        // Configure line control (word length, parity, stop bits)
        self.configure_line_control(config);
        
        Ok(())
    }

    /// Set baud rate divisor
    fn set_baud_rate(&self, rate: BaudRate, clock_hz: u32) { ... }
    
    /// Configure FIFO settings
    fn configure_fifo(&self, enabled: bool, trigger: FifoTriggerLevel) { ... }
    
    /// Configure line control register
    fn configure_line_control(&self, config: &UartConfig) { ... }

    /// Enable/disable interrupts
    pub fn configure_interrupts(&self, config: &InterruptConfig) { ... }

    /// Read line status register
    pub fn line_status(&self) -> LineStatus { ... }

    /// Read interrupt identification
    pub fn interrupt_source(&self) -> InterruptSource { ... }

    /// Check if TX FIFO/THR is full
    pub fn is_tx_full(&self) -> bool { ... }

    /// Check if RX FIFO is empty
    pub fn is_rx_empty(&self) -> bool { ... }

    /// Check if transmitter is idle
    pub fn is_tx_idle(&self) -> bool { ... }

    /// Write a single byte (non-blocking)
    pub fn write_byte(&self, byte: u8) -> Result<()> { ... }

    /// Read a single byte (non-blocking)
    pub fn read_byte(&self) -> Result<u8> { ... }
}
```

#### `hal_impl.rs`
```rust
use embedded_io::{Read, Write, ErrorType};
use core::fmt;

impl ErrorType for UartController<'_> {
    type Error = UartError;
}

impl Write for UartController<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for (n, &byte) in buf.iter().enumerate() {
            if self.is_tx_full() {
                if n == 0 {
                    // Block until at least one byte can be written
                    while self.is_tx_full() {}
                    self.write_byte(byte)?;
                    continue;
                }
                return Ok(n);
            }
            self.write_byte(byte)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        while !self.is_tx_idle() {}
        Ok(())
    }
}

impl Read for UartController<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        
        // Block until at least one byte available
        while self.is_rx_empty() {}
        
        let mut count = 0;
        while !self.is_rx_empty() && count < buf.len() {
            buf[count] = self.read_byte()?;
            count += 1;
        }
        Ok(count)
    }
}

// Support for writeln! macro
impl fmt::Write for UartController<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            while self.is_tx_full() {}
            self.write_byte(byte).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}
```

---

## Migration Plan

### Phase 1: Create New Module (Est. 2 hours)

| Step | Task | Files |
|------|------|-------|
| 1.1 | Create `src/uart_core/` directory | New directory |
| 1.2 | Create `mod.rs` with documentation and exports | `uart_core/mod.rs` |
| 1.3 | Create `error.rs` with error types | `uart_core/error.rs` |
| 1.4 | Create `config.rs` with configuration types | `uart_core/config.rs` |
| 1.5 | Create `fifo.rs` with FIFO types | `uart_core/fifo.rs` |
| 1.6 | Create `interrupt.rs` with interrupt types | `uart_core/interrupt.rs` |
| 1.7 | Create `line_status.rs` with bitflags | `uart_core/line_status.rs` |
| 1.8 | Create `types.rs` with common types | `uart_core/types.rs` |

### Phase 2: Implement Core Driver (Est. 3 hours)

| Step | Task | Files |
|------|------|-------|
| 2.1 | Implement `controller.rs` with hardware abstraction | `uart_core/controller.rs` |
| 2.2 | Implement `hal_impl.rs` with trait implementations | `uart_core/hal_impl.rs` |
| 2.3 | Add module to `src/lib.rs` | `src/lib.rs` |

### Phase 3: Migrate Usage (Est. 1 hour)

| Step | Task | Files |
|------|------|-------|
| 3.1 | Update `main.rs` imports | `src/main.rs` |
| 3.2 | Update any test files | `src/tests/` |
| 3.3 | Verify compilation | - |

### Phase 4: Testing (Est. 2 hours)

| Step | Task | Method |
|------|------|--------|
| 4.1 | Basic TX test | QEMU - print "Hello, UART!" |
| 4.2 | Configuration test | Test different baud rates |
| 4.3 | Interrupt status test | Read interrupt identification |
| 4.4 | Error detection test | Verify frame/parity error handling |
| 4.5 | FIFO trigger test | Test different trigger levels |

### Phase 5: Cleanup (Est. 30 min)

| Step | Task | Files to Remove |
|------|------|-----------------|
| 5.1 | Delete old flat module | `src/uart.rs` |
| 5.2 | Delete old crate | `src/uart/` directory |
| 5.3 | Update lib.rs | Remove `pub mod uart;` |
| 5.4 | Update Cargo.toml | Remove workspace member if applicable |

---

## API Comparison

### Current API (uart.rs)

```rust
// Requires delay dependency
let mut uart = UartController::new(uart_peripheral, &mut delay);

// Unsafe initialization
unsafe { uart.init(&config); }

// Basic operations
uart.send_byte_fifo(b'H');
let byte = uart.read_byte()?;
```

### New API (uart_core)

```rust
// No delay dependency needed
let uart_regs = unsafe { &*ast1060_pac::Uart::ptr() };
let mut uart = UartController::new(uart_regs);

// Safe initialization with builder pattern
let config = UartConfig {
    baud_rate: BaudRate::Baud115200,
    ..Default::default()
};
uart.init(&config)?;

// Use standard traits
use embedded_io::Write;
uart.write_all(b"Hello, UART!\r\n")?;
uart.flush()?;

// Or use writeln! macro
use core::fmt::Write;
writeln!(uart, "Value: {}", 42)?;
```

---

## Breaking Changes

| Change | Migration Path |
|--------|----------------|
| Remove `UartController::new(uart, delay)` | Use `UartController::new(regs)` - no delay needed |
| Remove `send_byte_fifo()` | Use `embedded_io::Write::write()` |
| Remove `embedded_hal::serial::*` traits | Use `embedded_io::{Read, Write}` |
| Change `Usart` to `UartController` | Update type references |
| Move types to submodules | Update import paths |

---

## Dependencies

No new dependencies required. Uses existing workspace dependencies:

```toml
[dependencies]
ast1060-pac = { ... }
embedded-io = "0.6.1"
embedded-hal = "1.0.0"
bitflags = { workspace = true }
nb = "1.1.0"
```

---

## Success Criteria

- [ ] New `uart_core` module compiles without warnings
- [ ] All existing functionality preserved
- [ ] `main.rs` works with new UART module
- [ ] Old `uart.rs` and `uart/` crate removed
- [ ] Documentation matches actual hardware (AST1060)
- [ ] No deprecated trait usage
- [ ] Follows same patterns as `i2c_core`
- [ ] `clippy` passes with no warnings
- [ ] QEMU tests pass

---

## References

- AST1060 Datasheet - UART Chapter
- [embedded-io crate documentation](https://docs.rs/embedded-io)
- [embedded-hal 1.0 migration guide](https://github.com/rust-embedded/embedded-hal/blob/master/docs/migrating-from-0.2-to-1.0.md)
- Existing `i2c_core` implementation in this repository

---

## Revision History

| Date | Version | Description |
|------|---------|-------------|
| 2026-01-30 | 1.0 | Initial plan |
