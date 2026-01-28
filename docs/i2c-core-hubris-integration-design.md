# AST1060 I2C Core: Layered Architecture Design

## Executive Summary

This document describes the architecture for making `aspeed-rust/i2c_core` the single source of truth for AST1060 I2C functionality, with Hubris-specific wrappers providing RTOS integration. This eliminates code duplication and ensures consistent behavior across bare-metal and Hubris environments.

**Key Reference**: This design integrates with the existing `drv-openprot-i2c-server` architecture, which provides a vendor-agnostic I2C server using the `I2cHardware` trait from `drv-i2c-types`. The `i2c_core` module becomes the AST1060 implementation behind that trait.

## Goals

1. **Single Implementation**: One I2C driver codebase for all environments
2. **Portable Core**: Zero OS dependencies in the core driver
3. **Thin Wrappers**: Hubris integration adds only IPC/notification glue
4. **Testability**: Core can be tested on host or QEMU without Hubris
5. **Ecosystem Compatibility**: Core implements `embedded-hal` traits
6. **OpenPRoT Integration**: Works seamlessly with `drv-openprot-i2c-server` and `I2cHardware` trait

## Interrupt Architecture Summary

| Mode | Mechanism | Description |
|------|-----------|-------------|
| **Master** | Polling | Busy-polls status register until completion (same as original driver) |
| **Slave** | Hardware IRQ | Real interrupt-driven I/O via Hubris notifications |

- **Master mode**: Each `write_read()` call polls the status register in a loop until the transfer completes. No hardware interrupts are used.
- **Slave mode**: When an external master sends data, the AST1060 hardware generates an IRQ. Hubris kernel delivers this as a notification to the I2C server task, which then calls `handle_slave_interrupt()`.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Hubris Application                         │
│                    (e.g., mctp-i2c-task)                       │
└─────────────────────────────────────────────────────────────────┘
                              │ IPC (idol)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│               drv-openprot-i2c-server (existing)                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  main.rs - Vendor-agnostic IPC handling                   │  │
│  │  - WriteRead, WriteReadBlock                              │  │
│  │  - ConfigureSlaveAddress, EnableSlaveReceive              │  │
│  │  - Notification-based slave message delivery              │  │
│  │  - Lease management for zero-copy transfers               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  hardware.rs - Feature-gated vendor dispatch              │  │
│  │  #[cfg(feature = "ast1060")]                              │  │
│  │  pub fn create_driver() -> impl I2cHardware               │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 drv-ast1060-i2c (Hubris wrapper)                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Ast1060I2cDriver                                         │  │
│  │  - impl I2cHardware (from drv-i2c-types)                  │  │
│  │  - Pin mux configuration                                  │  │
│  │  - Peripheral ownership (I2cPeripherals)                  │  │
│  │  - ResponseCode error mapping                             │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │ delegates to
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    aspeed-ddk::i2c_core                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Ast1060I2c (Portable Core Driver)                        │  │
│  │  - Hardware register abstraction                          │  │
│  │  - Master mode (read/write/write_read)                    │  │
│  │  - Slave/target mode                                      │  │
│  │  - Buffer mode (32-byte FIFO)                             │  │
│  │  - Timing configuration                                   │  │
│  │  - Bus recovery                                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  embedded-hal Implementation                              │  │
│  │  - impl I2c<SevenBitAddress> for Ast1060I2c               │  │
│  │  - impl Error for I2cError                                │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       ast1060-pac                               │
│                  (SVD-generated registers)                      │
└─────────────────────────────────────────────────────────────────┘
```

## Layer Responsibilities

### Layer 1: `aspeed-ddk::i2c_core` (This Crate)

**Location**: `aspeed-rust/src/i2c_core/`

**Purpose**: Portable, hardware-level I2C driver with no OS dependencies.

**Dependencies**:
- `ast1060-pac` - Register definitions
- `embedded-hal` - Trait implementations
- `core` only (no `std`, no `alloc`)

**Modules**:
```
i2c_core/
├── mod.rs              # Public API re-exports
├── controller.rs       # Ast1060I2c main struct
├── master.rs           # Master mode operations
├── slave.rs            # Slave/target mode operations
├── transfer.rs         # Low-level transfer state machine
├── timing.rs           # Clock configuration
├── recovery.rs         # Bus recovery
├── constants.rs        # Hardware register constants
├── error.rs            # I2cError enum
├── types.rs            # I2cConfig, I2cSpeed, etc.
├── hal_impl.rs         # embedded-hal trait impls
└── target_adapter.rs   # I2CTarget trait adapter
```

**Public API**:
```rust
// Core types
pub struct Ast1060I2c<'a> { ... }
pub struct I2cController<'a> { ... }
pub struct I2cConfig { ... }
pub enum I2cSpeed { Standard, Fast, FastPlus }
pub enum I2cXferMode { ByteMode, BufferMode }
pub enum I2cError { ... }

// Slave types
pub struct SlaveConfig { ... }
pub struct SlaveBuffer { ... }
pub enum SlaveEvent { ... }

// Core operations (on Ast1060I2c)
impl Ast1060I2c {
    // Construction
    pub fn new(controller: &I2cController, config: I2cConfig) -> Result<Self, I2cError>;
    pub fn from_initialized(controller: &I2cController, config: I2cConfig) -> Self;  // No hardware init
    pub fn init_hardware(&mut self, config: &I2cConfig) -> Result<(), I2cError>;
    
    // Master operations (uses polling internally)
    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError>;
    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError>;
    pub fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), I2cError>;
    
    // Slave operations (called from real IRQ context in Hubris)
    pub fn configure_slave(&mut self, config: &SlaveConfig) -> Result<(), I2cError>;
    pub fn handle_slave_interrupt(&mut self) -> Option<SlaveEvent>;  // Called from IRQ
    pub fn slave_read(&mut self, buffer: &mut [u8]) -> Result<usize, I2cError>;
    pub fn slave_write(&mut self, data: &[u8]) -> Result<usize, I2cError>;
    pub fn slave_has_data(&self) -> bool;
    
    // Status register access (used by polling and IRQ paths)
    pub fn handle_interrupt(&mut self) -> Result<(), I2cError>;  // Reads/clears status
    pub fn is_bus_busy(&self) -> bool;
}
```

**Design Decisions**:

1. **No global state for peripherals**: The caller provides `I2cController` references, allowing Hubris task ownership model.

2. **Master mode uses polling**: `wait_completion()` busy-polls the status register. This matches the original `drv-ast1060-i2c` behavior. The `notification` field was intended for future interrupt-driven master transfers but was never implemented.

3. **Slave mode uses real interrupts**: The server's main loop receives hardware IRQs via Hubris notifications, then calls `handle_slave_interrupt()`. This is true interrupt-driven I/O.

4. **No pin mux**: Pin configuration is board-specific and handled by the platform layer.

### Interrupt vs Polling Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SLAVE MODE (Interrupt-Driven)                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  External I2C Master sends data to AST1060                          │
│         │                                                            │
│         ▼                                                            │
│  AST1060 I2C hardware ───► IRQ line (i2c2.irq)                      │
│         │                                                            │
│         ▼                                                            │
│  Hubris kernel receives interrupt                                    │
│         │                                                            │
│         ▼                                                            │
│  Kernel posts notification to I2C server task (i2c-irq mask)        │
│         │                                                            │
│         ▼                                                            │
│  sys_recv_open() returns with sender = KERNEL                       │
│         │                                                            │
│         ▼                                                            │
│  Server calls driver.handle_slave_interrupt()                       │
│         │                                                            │
│         ▼                                                            │
│  i2c_core reads slave data from hardware buffer                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    MASTER MODE (Polling)                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Client calls write_read()                                          │
│         │                                                            │
│         ▼                                                            │
│  Driver starts transfer, then busy-polls status register            │
│         │                                                            │
│         ▼                                                            │
│  while (!completion) { handle_interrupt(); }  ← Reads status, no IRQ│
│         │                                                            │
│         ▼                                                            │
│  Returns when hardware sets completion bit                          │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**app.toml wiring for slave interrupts:**
```toml
[tasks.i2c]
notifications = ["i2c-irq"]      # Server receives this notification

[tasks.i2c.interrupts]
"i2c2.irq" = "i2c-irq"          # Hardware IRQ → notification mapping
```

**Note**: The `handle_interrupt()` function name is misleading for master mode - it's actually polling the status register, not handling a real hardware interrupt. In slave mode, it IS called from real interrupt context via the server's main loop.

### Pre-Kernel Hardware Initialization

The app's `main.rs` configures I2C hardware **before the Hubris kernel starts**. This allows the driver to use `from_initialized()` for lightweight per-operation wrappers.

**What app main.rs configures:**

```rust
// Runs before kernel starts
fn configure_i2c_hardware(peripherals: &Peripherals) {
    // Step 1: Enable I2C clock gate
    peripherals.scu.scu040().modify(|_, w| w.enbl_i2c_clk().set_bit());
    
    // Step 2: Configure I2C global timing (I2CG10)
    // Sets base clocks for 100kHz, 400kHz, 1MHz speeds
    i2cg.i2cg10().write(|w| unsafe { w.bits(0x6222_0803) });
    
    // Step 3: Configure I2C pin mux (SCU418 for I2C2)
    configure_i2c_pins(peripherals);
}

fn configure_i2c_pins(peripherals: &Peripherals) {
    // I2C2: SCU418[0:1] = SCL2/SDA2
    peripherals.scu.scu418().modify(|r, w| unsafe {
        w.bits(r.bits() | (0b11 << 0))
    });
}
```

**Division of responsibility:**

| Component | Responsibility |
|-----------|----------------|
| **App main.rs** | Clock gates, global timing (I2CG10), pin mux (SCU4xx) |
| **i2c_core** | Controller-specific init (I2CC00, timing, interrupts) |
| **drv-ast1060-i2c-refactor** | Owns peripherals, maps errors, delegates to i2c_core |

**Pin mux register mapping (AST1060):**

| Controller | Register | Bits |
|------------|----------|------|
| I2C0 | SCU414 | [28:29] |
| I2C1 | SCU414 | [30:31] |
| I2C2-7 | SCU418 | [0:11] (2 bits each) |
| I2C8-13 | SCU41C | varies |

---

### Layer 2: `drv-ast1060-i2c` (Hubris Wrapper)

**Location**: `hubris/drv/ast1060-i2c/` (existing, to be refactored)

**Purpose**: Implements `I2cHardware` trait from `drv-i2c-types` by wrapping `i2c_core`.

**Key Insight**: The existing `drv-openprot-i2c-server` already provides vendor-agnostic IPC handling via the `I2cHardware` trait. Our job is to implement that trait using `i2c_core`.

**Dependencies**:
- `aspeed-ddk` (the `i2c_core` module)
- `drv-i2c-types` - `I2cHardware` trait definition
- `ast1060-pac` - Register access
- `userlib` - Hubris syscalls (optional, for notifications)

**Structure**:
```
drv-ast1060-i2c/
├── Cargo.toml
├── src/
│   ├── lib.rs            # Exports Ast1060I2cDriver, I2cPeripherals
│   ├── server_driver.rs  # impl I2cHardware for Ast1060I2cDriver
│   ├── pinmux.rs         # AST1060 pin mux configuration
│   └── mux/              # I2C multiplexer drivers (optional)
│       ├── mod.rs
│       ├── pca9545.rs
│       └── pca9548.rs
```

**I2cHardware Implementation** (bridges to `i2c_core`):

```rust
// server_driver.rs
use aspeed_ddk::i2c_core::{Ast1060I2c, I2cConfig, I2cController, I2cError};
use drv_i2c_types::{traits::I2cHardware, Controller, ResponseCode, SlaveConfig};

/// Hubris driver wrapping the portable i2c_core
pub struct Ast1060I2cDriver {
    peripherals: I2cPeripherals,
}

impl Ast1060I2cDriver {
    pub fn new(peripherals: I2cPeripherals) -> Self {
        Self { peripherals }
    }

    fn get_i2c(&self, controller: Controller) -> Result<Ast1060I2c<'_>, ResponseCode> {
        let (regs, buffs) = self.peripherals
            .controller_and_buffer(controller as u8)
            .ok_or(ResponseCode::BadController)?;

        let ctrl = I2cController {
            registers: regs,
            buff_registers: buffs,
        };

        let config = I2cConfig::default();
        Ast1060I2c::new(&ctrl, config).map_err(|e| e.into())
    }
}

impl I2cHardware for Ast1060I2cDriver {
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        let mut i2c = self.get_i2c(controller)?;
        
        // Note: Pin mux is pre-configured by app main.rs before kernel starts.
        // No pinmux call needed here.
        
        // Delegate to portable core
        i2c.write_read(addr, write_data, read_buffer)
            .map_err(ResponseCode::from)?;
        
        Ok(read_buffer.len())
    }

    fn write_read_block(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        // SMBus block read: first byte is length
        let mut i2c = self.get_i2c(controller)?;
        
        // Write command, then read length + data
        let mut block_buf = [0u8; 256];
        i2c.write_read(addr, write_data, &mut block_buf[..read_buffer.len() + 1])
            .map_err(ResponseCode::from)?;
        
        let len = block_buf[0] as usize;
        let actual_len = len.min(read_buffer.len());
        read_buffer[..actual_len].copy_from_slice(&block_buf[1..actual_len + 1]);
        
        Ok(actual_len)
    }

    fn configure_slave_mode(
        &mut self,
        controller: Controller,
        config: &SlaveConfig,
    ) -> Result<(), ResponseCode> {
        let mut i2c = self.get_i2c(controller)?;
        
        // Convert drv-i2c-types SlaveConfig to i2c_core SlaveConfig
        let core_config = aspeed_ddk::i2c_core::slave::SlaveConfig::new(config.address)
            .map_err(|_| ResponseCode::BadArg)?;
        
        i2c.configure_slave(&core_config).map_err(ResponseCode::from)
    }

    fn enable_slave_receive(&mut self, controller: Controller) -> Result<(), ResponseCode> {
        let mut i2c = self.get_i2c(controller)?;
        i2c.enable_slave_receive().map_err(ResponseCode::from)
    }

    fn disable_slave_receive(&mut self, controller: Controller) -> Result<(), ResponseCode> {
        let mut i2c = self.get_i2c(controller)?;
        i2c.disable_slave_receive().map_err(ResponseCode::from)
    }

    fn configure_timing(
        &mut self,
        controller: Controller,
        speed: drv_i2c_types::traits::I2cSpeed,
    ) -> Result<(), ResponseCode> {
        // Map drv-i2c-types speed to i2c_core speed
        let core_speed = match speed {
            drv_i2c_types::traits::I2cSpeed::Standard => aspeed_ddk::i2c_core::I2cSpeed::Standard,
            drv_i2c_types::traits::I2cSpeed::Fast => aspeed_ddk::i2c_core::I2cSpeed::Fast,
            drv_i2c_types::traits::I2cSpeed::FastPlus => aspeed_ddk::i2c_core::I2cSpeed::FastPlus,
            _ => return Err(ResponseCode::BadArg),
        };
        
        let mut i2c = self.get_i2c(controller)?;
        i2c.configure_timing(core_speed).map_err(ResponseCode::from)
    }

    fn reset_bus(&mut self, controller: Controller) -> Result<(), ResponseCode> {
        let mut i2c = self.get_i2c(controller)?;
        i2c.recover_bus().map_err(ResponseCode::from)
    }

    fn enable_controller(&mut self, controller: Controller) -> Result<(), ResponseCode> {
        // Note: Pin mux is pre-configured by app main.rs before kernel starts.
        // This just initializes the controller hardware.
        let mut i2c = self.get_i2c(controller)?;
        i2c.init_hardware(&I2cConfig::default()).map_err(ResponseCode::from)
    }

    fn disable_controller(&mut self, _controller: Controller) -> Result<(), ResponseCode> {
        // AST1060 doesn't need explicit disable
        Ok(())
    }
}
```

---

### Layer 3: `drv-openprot-i2c-server` (Existing - No Changes Needed)

**Location**: `hubris/drv/openprot-i2c-server/`

**Purpose**: Vendor-agnostic I2C server handling IPC, leases, and slave notifications.

**Key Files**:
- `main.rs` - IPC dispatch loop, **DO NOT MODIFY**
- `hardware.rs` - Feature-gated vendor selection
- `i2c_topology.rs` - Controller/port/mux lookup

**Integration Point** (`hardware.rs`):

```rust
//! Hardware-specific driver selection

use drv_i2c_types::traits::I2cHardware;

#[cfg(feature = "ast1060")]
pub fn create_driver() -> impl I2cHardware {
    use drv_ast1060_i2c::{Ast1060I2cDriver, I2cPeripherals};
    
    // Safety: Task owns I2C peripherals exclusively per app.toml
    let peripherals = unsafe { I2cPeripherals::new() };
    Ast1060I2cDriver::new(peripherals)
}

#[cfg(not(any(feature = "ast1060")))]
compile_error!("No hardware vendor feature enabled. Enable one of: ast1060");
```

**Cargo.toml** features:

```toml
[features]
default = ["ast1060"]
ast1060 = ["drv-ast1060-i2c", "ast1060-pac"]
slave = ["drv-ast1060-i2c?/slave"]
```

---

## I2C Hardware Pre-Configuration

The AST1060 I2C peripheral requires global configuration before use:

1. **Application pre-configures** clocks and pins at startup
2. **Driver reads from hardware** to calculate timing

This separation provides:
- Clear ownership: App owns hardware setup, driver owns I2C operations
- Board-specific configuration: Different apps can use different pins/clocks
- No hardcoded assumptions: Driver adapts to actual hardware state

### Clock Hierarchy

```
HPLL (1 GHz)
    └── CPU clock (200 MHz)
            └── AHB clock (200 MHz)
                    └── APB clock (50 MHz) ──► I2C peripherals
```

### Application Pre-Configuration

The application configures I2C hardware **before starting the kernel**:

```rust
// In app/*/src/main.rs - BEFORE kernel start
fn configure_i2c_hardware(peripherals: &Peripherals) {
    // Step 1: Reset I2C controller
    peripherals.scu.scu050().write(|w| w.rst_i2csmbus_ctrl().set_bit());
    peripherals.scu.scu054().write(|w| unsafe { w.bits(0x4) });
    
    // Step 2: Configure global clock settings
    let i2cg = unsafe { &*ast1060_pac::I2cglobal::ptr() };
    i2cg.i2cg0c().write(|w| {
        w.clk_divider_mode_sel().set_bit()
         .reg_definition_sel().set_bit()
    });
    i2cg.i2cg10().write(|w| unsafe { w.bits(0x6222_0803) });
    
    // Step 3: Configure pin mux for I2C controllers used by this app
    // I2C2: SCU418[0:1]
    peripherals.scu.scu418().modify(|r, w| unsafe {
        w.bits(r.bits() | (0b11 << 0))
    });
}
```

### I2C Pin Mapping (AST1060)

| Controller | SCU Register | Bits    | Notes              |
|------------|--------------|---------|-------------------|
| I2C0       | SCU414       | [28:29] | SCL0/SDA0         |
| I2C1       | SCU414       | [30:31] | SCL1/SDA1         |
| I2C2       | SCU418       | [0:1]   | SCL2/SDA2         |
| I2C3       | SCU418       | [2:3]   | SCL3/SDA3         |
| I2C4       | SCU418       | [4:5]   | SCL4/SDA4         |
| I2C5       | SCU418       | [6:7]   | SCL5/SDA5         |
| I2C6       | SCU418       | [8:9]   | SCL6/SDA6         |
| I2C7       | SCU418       | [10:11] | SCL7/SDA7         |
| I2C8-13    | SCU41C       | varies  | Check datasheet   |

### I2CG10 Base Clock Configuration

| Field      | Value | APB Divisor | Base Clock | Purpose                |
|------------|-------|-------------|------------|------------------------|
| `[7:0]`    | 0x03  | (3+2)/2=2.5 | 20 MHz     | Fast-plus (1 MHz)     |
| `[15:8]`   | 0x08  | (8+2)/2=5   | 10 MHz     | Fast (400 kHz)        |
| `[23:16]`  | 0x22  | (34+2)/2=18 | 2.77 MHz   | Standard (100 kHz)    |
| `[31:24]`  | 0x62  | (98+2)/2=50 | 1 MHz      | Recovery timeout      |

### Driver Reads from Hardware

When creating an I2C driver instance, `I2cConfig::default()` reads actual clock values:

```rust
impl Default for I2cConfig {
    fn default() -> Self {
        Self {
            clock_config: ClockConfig::from_hardware(), // Reads I2CG10!
            // ...
        }
    }
}
```

See [app/ast1060-i2c-example/src/main.rs](../../transfer-work/app/ast1060-i2c-example/src/main.rs) for the complete implementation.

---

## The I2cHardware Trait (from drv-i2c-types)

The existing `I2cHardware` trait in `drv-i2c-types/src/traits.rs` defines the contract between the vendor-agnostic server and hardware implementations:

```rust
pub trait I2cHardware {
    type Error: Into<ResponseCode>;

    // Master operations
    fn write_read(&mut self, controller: Controller, addr: u8,
                  write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, Self::Error>;
    fn write_read_block(&mut self, controller: Controller, addr: u8,
                        write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, Self::Error>;

    // Configuration
    fn configure_timing(&mut self, controller: Controller, speed: I2cSpeed) -> Result<(), Self::Error>;
    fn reset_bus(&mut self, controller: Controller) -> Result<(), Self::Error>;
    fn enable_controller(&mut self, controller: Controller) -> Result<(), Self::Error>;
    fn disable_controller(&mut self, controller: Controller) -> Result<(), Self::Error>;

    // Slave mode (for MCTP)
    fn configure_slave_mode(&mut self, controller: Controller, config: &SlaveConfig) -> Result<(), Self::Error>;
    fn enable_slave_receive(&mut self, controller: Controller) -> Result<(), Self::Error>;
    fn disable_slave_receive(&mut self, controller: Controller) -> Result<(), Self::Error>;
    fn get_slave_message(&mut self, controller: Controller) -> Result<Option<SlaveMessage>, Self::Error>;
}
```

**This trait is already defined** - we just need to implement it using `i2c_core`.

---

## Integration Points

### 1. Interrupt Handling

**i2c_core** provides:
```rust
impl Ast1060I2c {
    /// Process pending interrupts, update internal state
    pub fn handle_interrupt(&mut self) -> Result<(), I2cError>;
    
    /// Check if operation completed (non-blocking)
    pub fn is_complete(&self) -> bool;
    
    /// Get raw interrupt status for diagnostics
    pub fn pending_interrupts(&self) -> u32;
}
```

**drv-openprot-i2c-server** `main.rs` already handles this:
```rust
// From main.rs - interrupt notification handling
if msginfo.sender == TaskId::KERNEL {
    if msginfo.operation & notifications::I2C_IRQ_MASK != 0 {
        handle_slave_interrupt(&mut driver, &mut pending_slave_msg, &notification_client);
        sys_irq_control(notifications::I2C_IRQ_MASK, true);
    }
    continue;
}
```

### 2. Slave Message Flow

The server already implements notification-based slave message delivery:

```rust
// Server maintains pending slave message state
let mut pending_slave_msg: Option<(u8, SlaveMessage)> = None;
let mut notification_client: Option<(TaskId, u32)> = None;

// When slave event occurs, notify client
fn handle_slave_interrupt<D: I2cHardware>(
    driver: &mut D,
    pending_slave_msg: &mut Option<(u8, SlaveMessage)>,
    notification_client: &Option<(TaskId, u32)>,
) {
    // Poll hardware for slave message
    if let Ok(Some(msg)) = driver.get_slave_message(controller) {
        *pending_slave_msg = Some((controller_id, msg));
        
        // Notify waiting client
        if let Some((task_id, mask)) = notification_client {
            sys_post(*task_id, *mask);
        }
    }
}
```

### 3. Zero-Copy Lease Handling

The server uses Hubris leases for efficient data transfer:

```rust
// Read write data from caller's lease (zero-copy)
sys_borrow_read(msginfo.sender, i, pos, &mut write_data[pos..pos + 1]);

// Write read data back to caller's lease
sys_borrow_write(msginfo.sender, i + 1, pos, &read_slice[pos..pos + 1]);
```

This stays in the server - the hardware driver just works with slices.

---

## Cargo Configuration

### aspeed-ddk/Cargo.toml

```toml
[package]
name = "aspeed-ddk"
version = "0.1.0"
edition = "2021"

[features]
default = []
i2c = ["dep:embedded-hal"]

[dependencies]
ast1060-pac = { path = "../ast1060-pac" }
embedded-hal = { version = "1.0", optional = true }
```

### drv-ast1060-i2c/Cargo.toml (Hubris wrapper)

```toml
[package]
name = "drv-ast1060-i2c"
version = "0.1.0"
edition = "2021"

[dependencies]
aspeed-ddk = { path = "../../aspeed-ddk", features = ["i2c"] }
ast1060-pac = { workspace = true }
drv-i2c-types = { path = "../i2c-types" }

[features]
default = []
slave = []  # Enable slave mode support
```

### drv-openprot-i2c-server/Cargo.toml (existing)

```toml
[dependencies]
# Hardware vendor dependencies (conditionally compiled)
drv-ast1060-i2c = { path = "../ast1060-i2c", optional = true }
ast1060-pac = { workspace = true, optional = true }

[features]
default = ["ast1060"]
ast1060 = ["drv-ast1060-i2c", "ast1060-pac"]
slave = ["drv-ast1060-i2c?/slave"]
```

---

## Migration Path

### Phase 1: Stabilize i2c_core (Current)
- [x] Core driver implementation
- [x] embedded-hal trait implementation  
- [x] Slave mode support
- [x] Target adapter for trait-based callbacks

### Phase 2: Build Container App + Refactor Driver
- [x] Create `app/ast1060-i2c-refactor-example/` based on `ast1060-i2c-example`
- [x] Create `drv/ast1060-i2c-refactor/` with `aspeed-ddk` dependency
- [x] Implement `I2cHardware` trait by delegating to `i2c_core`
- [x] Configure app.toml to use the refactored driver
- [x] Build with `cargo xtask dist app/ast1060-i2c-refactor-example/app.toml`
- [x] Verify compilation and fix any API mismatches
- [x] Add `from_initialized()` optimization to avoid per-operation re-init

**Note:** Hubris driver crates cannot be built standalone with `cargo check -p`.
They must be built in the context of a Hubris app which provides:
- Memory layout and linker scripts
- Kernel and syscall dependencies  
- Target-specific build configuration

#### Implementation Patterns

**IMPORTANT:** Follow the same patterns as `drv-ast1060-i2c/src/server_driver.rs`:

1. **Lifetime Management**: Create `I2cController` and `Ast1060I2c` inline within
   each `I2cHardware` trait method - do NOT use a helper method that returns
   `Ast1060I2c<'_>` (causes lifetime issues).

   ```rust
   // CORRECT - both structs created in same scope
   fn write_read(&mut self, controller: Controller, ...) -> Result<...> {
       let (regs, buffs) = self.peripherals.controller_and_buffer(controller as u8)?;
       let ctrl = I2cController { registers: regs, buff_registers: buffs, ... };
       let mut i2c = Ast1060I2c::from_initialized(&ctrl, config);  // lightweight
       i2c.write_read(addr, write_data, read_buffer)?;
       Ok(...)
   }

   // WRONG - lifetime error: ctrl doesn't live long enough
   fn get_i2c(&self, controller: Controller) -> Result<Ast1060I2c<'_>, ...> {
       let ctrl = I2cController { ... };  // local variable
       Ast1060I2c::new(&ctrl, config)      // returns ref to local!
   }
   ```

2. **Performance Optimization - `from_initialized()` vs `new()`**:
   
   The `Ast1060I2c` type provides two constructors:
   
   | Method | Hardware Init | Use Case |
   |--------|--------------|----------|
   | `new()` | Yes (~50 register writes) | First-time setup, `enable_controller()` |
   | `from_initialized()` | No (0 register writes) | Per-operation wrappers |
   
   **Key insight**: Every I2C operation creates a temporary `Ast1060I2c` instance.
   Using `new()` for every operation causes ~50 redundant hardware writes per
   transfer. Using `from_initialized()` eliminates this overhead.
   
   ```rust
   // INEFFICIENT - re-initializes hardware on every write_read
   let mut i2c = Ast1060I2c::new(&ctrl, config).map_err(map_error)?;
   
   // EFFICIENT - assumes hardware pre-initialized by app main.rs
   let mut i2c = Ast1060I2c::from_initialized(&ctrl, config);
   ```
   
   **Requirements for `from_initialized()`**:
   - App's `main.rs` must initialize I2C hardware before kernel starts
   - Initialize global registers (I2CG0C, I2CG10)
   - Configure timing for each controller
   - Set up pin mux
   
   **When to use `new()`**:
   - `enable_controller()` - explicitly needs hardware init
   - `configure_timing()` - needs to apply new timing settings

3. **Error Mapping**: Map `i2c_core::I2cError` variants to `ResponseCode` using
   the exact same mapping as the original driver.

4. **API Compatibility**: The `i2c_core` API is 90%+ identical to `drv-ast1060-i2c`.
   Key differences:
   - `I2cController` in i2c_core has no `notification` field
   - `I2cController.controller` uses `aspeed_ddk::i2c_core::Controller` (newtype)
     instead of `drv_i2c_api::Controller` (enum)
   - Pin mux is NOT called by i2c_core (app pre-configures in main.rs)

### Phase 3: Validate with drv-openprot-i2c-server
- [ ] Update `drv-openprot-i2c-server/hardware.rs` to use refactored driver
- [ ] Build with `--features ast1060`
- [ ] Test master mode operations via IPC
- [ ] Test slave mode / MCTP integration
- [ ] Verify interrupt handling and notifications

### Phase 4: Deprecate Old Code
- [ ] Remove `transfer-work/drv/ast1060-i2c/src/*.rs` driver code
- [ ] Keep only the Hubris integration layer
- [ ] Update documentation

---

## File Changes Summary

### Files to Keep As-Is
- `drv-openprot-i2c-server/src/main.rs` - Vendor-agnostic, no changes
- `drv-openprot-i2c-server/src/hardware.rs` - Just add/verify ast1060 feature
- `drv-i2c-types/src/traits.rs` - I2cHardware trait definition

### Files to Refactor
- `drv-ast1060-i2c/src/server_driver.rs` → delegate to `aspeed-ddk::i2c_core`
- `drv-ast1060-i2c/src/lib.rs` → thin re-exports + peripheral ownership

### Files to Keep in Hubris (Platform-Specific)
- `drv-ast1060-i2c/src/pinmux.rs` - AST1060 pin configuration
- `drv-ast1060-i2c/src/mux/*.rs` - I2C multiplexer drivers (board-specific)

### Files That Become Single Source of Truth
- `aspeed-rust/src/i2c_core/*.rs` - All I2C driver logic lives here

---

## Benefits of This Architecture

| Benefit | Description |
|---------|-------------|
| **Single Source of Truth** | `i2c_core` is the only I2C implementation |
| **No Server Changes** | `drv-openprot-i2c-server/main.rs` stays untouched |
| **Compile-Time Dispatch** | Feature flags select vendor, zero runtime overhead |
| **Testability** | Core can be tested without Hubris |
| **Ecosystem Compatible** | `embedded-hal` traits for bare-metal use |
| **Clean Separation** | Platform code (pinmux) stays in Hubris |
| **Future Vendors** | Add STM32, LPC55, etc. without touching server |

---

## API Compatibility Notes

### embedded-hal Compatibility

The `i2c_core` module implements `embedded_hal::i2c::I2c`:

```rust
impl<'a> I2c<SevenBitAddress> for Ast1060I2c<'a> {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> { ... }
}
```

This allows using the driver with any embedded-hal compatible library.

### Hubris I2cHardware Compatibility

The Hubris wrapper implements `drv_i2c_types::I2cHardware`:

```rust
impl I2cHardware for HubrisI2cAdapter<'_> {
    type Error = ResponseCode;
    
    fn write_read(...) -> Result<usize, ResponseCode>;
    fn configure_slave(...) -> Result<(), ResponseCode>;
    // etc.
}
```

---

## Testing Strategy

### Integration Tests (QEMU)

Use the existing `test_qemu.py` framework to test the core driver:

```python
def test_i2c_master_write():
    """Test master write operation via QEMU I2C model"""
    # Start QEMU with I2C peripheral model
    # Exercise write operation
    # Verify I2C traffic on emulated bus
```

### Hubris Integration Tests

Test the full stack in Hubris environment:

```rust
// In test task
#[test]
fn test_hubris_i2c_write_read() {
    let i2c = I2C.get_task_id();
    let result = I2c::write_read(
        i2c,
        Controller::I2C0,
        0x50,
        &[0x00],
        &mut buffer,
    );
    assert!(result.is_ok());
}
```

---

## Future Optimizations

### 1. Slave Mode Pre-Initialization

**Current behavior:** Slave mode is configured at runtime via IPC:
1. Client calls `ConfigureSlaveAddress` → IPC to server
2. Server calls `driver.configure_slave_mode()`
3. Driver writes to slave address registers

**Optimization:** For MCTP/security applications where the slave address is known at design time, pre-configure slave mode in `main.rs` before the kernel starts.

```rust
// In app main.rs (before kernel starts)
fn configure_i2c_hardware(peripherals: &Peripherals) {
    configure_i2c_clocks(peripherals);
    configure_i2c_pins(peripherals);
    
    // Pre-configure slave mode for MCTP (address 0x0A)
    configure_i2c_slave(&peripherals.i2c2, 0x0A);
}

fn configure_i2c_slave(i2c: &ast1060_pac::i2c::RegisterBlock, slave_addr: u8) {
    // Set slave address register (I2CS00)
    i2c.i2cs00().write(|w| unsafe { 
        w.slave_addr_1().bits(slave_addr)
         .slave_addr_en().set_bit()
    });
    
    // Enable slave mode interrupts (I2CS10)
    i2c.i2cs10().write(|w| unsafe {
        w.bits(SLAVE_INTERRUPT_MASK)
    });
}
```

**Trade-offs:**

| Pre-init (main.rs) | Runtime (IPC) |
|-------------------|---------------|
| ✅ Faster startup | ✅ Flexible address |
| ✅ No IPC overhead | ✅ Can change at runtime |
| ✅ Simpler server code | ✅ Multi-tenant support |
| ❌ Fixed at compile time | ❌ Extra IPC round-trip |

**Recommendation:** For MCTP endpoints with fixed addresses, use pre-initialization. The `ConfigureSlaveAddress` IPC can become a no-op or verify the address matches the pre-configured value.

### 2. Interrupt-Driven Master Mode

**Current behavior:** Master mode busy-polls the status register until transfer completes.

**Optimization:** Use hardware interrupts + Hubris notifications for master transfers:
1. Start transfer, then `sys_recv()` with notification mask
2. Hardware IRQ fires on completion
3. Kernel delivers notification to driver task
4. Driver reads status and returns

**Impact:** Frees CPU during I2C transfers (~100-500 µs per byte at 100 kHz).

**Complexity:** Requires reworking the synchronous `I2cHardware` trait into an async model, which would be a significant API change.

### 3. DMA Support

**Current behavior:** CPU copies data to/from the 32-byte hardware buffer.

**Optimization:** Use DMA for transfers larger than buffer size.

**Impact:** Reduces CPU overhead for large transfers (>32 bytes).

**Note:** Most MCTP packets are small (<64 bytes), so the benefit may be limited for typical security firmware use cases.

---

## Open Questions

1. **aspeed-ddk location**: Should `aspeed-ddk` live in `aspeed-rust` repo or move to Hubris workspace?
   - **Recommendation**: Keep in `aspeed-rust` for bare-metal use; Hubris references via path or git dependency.

2. **Error conversion location**: Where should `impl From<I2cError> for ResponseCode` live?
   - **Option A**: In `drv-ast1060-i2c` (Hubris-specific)
   - **Option B**: In `aspeed-ddk` with optional `hubris` feature
   - **Recommendation**: Option A - keeps core driver free of Hubris types.

3. **I2C mux drivers**: Should PCA9545/PCA9548 drivers move to `i2c_core`?
   - **Recommendation**: Keep in Hubris - they require I2C transactions and topology is board-specific.

---

## References

- [drv-openprot-i2c-server](../../transfer-work/drv/openprot-i2c-server/) - Vendor-agnostic server
- [drv-i2c-types traits.rs](../../transfer-work/drv/i2c-types/src/traits.rs) - I2cHardware trait definition
- [VENDOR-INTEGRATION.md](../../transfer-work/drv/openprot-i2c-server/VENDOR-INTEGRATION.md) - Integration guide
- [AST1060 I2C Register Documentation](../docs/)
- [embedded-hal I2C traits](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/index.html)
- [MCTP over I2C Binding Specification](https://www.dmtf.org/sites/default/files/standards/documents/DSP0237_1.2.0.pdf)
