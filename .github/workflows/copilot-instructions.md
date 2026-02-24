## Copilot Code Generation Guidelines
When generating code, always prioritize these patterns:


### no_std and Memory Allocation Guidelines

- This project is strictly **no_std** and **no_alloc** in production code
- All production paths must be allocation-free and compatible with bare-metal targets

#### Production Code Requirements

- **NEVER** use crates or features that require heap allocation in production code
- **DO NOT** use the `heapless` crate in production paths despite its name suggesting compatibility
  - *Rationale: While heapless is no_std, it still uses stack allocation with unpredictable growth patterns*
  - *Use fixed-size arrays and slices instead for predictable memory usage*
- **DO NOT** use any crate that depends on the `alloc` crate without feature gating
- **ALWAYS** use fixed-size arrays, slices, or static memory allocation
- **ALWAYS** design APIs to accept and return memory provided by the caller

#### Memory Management in Production Code

- Buffers must be pre-allocated by the caller and passed as slices
- Collection types must have fixed, compile-time sizes
- All data structures must have predictable, static memory footprints
- No dynamic memory growth patterns are allowed

#### Test Code Exceptions

- Test code (annotated with `#[cfg(test)]`) may use allocation if needed
- The `heapless` crate and other no_std compatible collections are permitted in tests
- Standard library features may be used in tests when the `std` feature is enabled
- Test helpers can use more ergonomic APIs that wouldn't be appropriate for production

#### Example: Production vs. Test Code

```rust
// Production code - strict no_alloc approach
pub fn process_data(data: &[u8], output: &mut [u8]) -> Result<usize, Error> {
    if output.len() < data.len() * 2 {
        return Err(Error::BufferTooSmall);
    }
    
    // Process data into output buffer
    // ...
    
    Ok(processed_bytes)
}

// Test code - can use more ergonomic approaches
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process_data() {
        // It's fine to use Vec in tests
        let input = vec![1, 2, 3, 4];
        let mut output = vec![0; 16];
        
        let result = process_data(&input, &mut output);
        assert!(result.is_ok());
        // ...
    }
    
    #[test]
    fn test_with_heapless() {
        // NOTE: Heapless is STRICTLY FORBIDDEN in production code (see guidelines above).
        // It is permitted here ONLY because this is test code.
        use heapless::Vec;
        input.extend_from_slice(&[1, 2, 3]).unwrap();
        
        let mut output = [0u8; 16];
        let result = process_data(&input, &mut output);
        assert!(result.is_ok());
        // ...
    }
}


### Error Handling
```rust
// ✅ GOOD: Always use Result/Option
fn parse_value(input: &str) -> Result<u32, ParseError> {
    input.parse().map_err(ParseError::InvalidFormat)
}

// ❌ BAD: Never use unwrap/expect in production code
fn parse_value(input: &str) -> u32 {
    input.parse().unwrap() // FORBIDDEN
}
```

### Array Access
```rust
// ✅ GOOD: Safe array access
if let Some(value) = array.get(index) {
    process_value(*value);
}

// ❌ BAD: Direct indexing can panic
let value = array[index]; // FORBIDDEN
```

### Hardware Register Access
```rust
// ✅ GOOD: Type-safe PAC usage
let reg_value = peripheral.register.read();
peripheral.register.write(|w| w.field().set_bit());

// ❌ BAD: Raw pointer access
unsafe {
    let reg_ptr = 0x4000_0000 as *mut u32; // FORBIDDEN
    *reg_ptr = value;
}
```

### Constants and Magic Numbers
```rust
// ✅ GOOD: Named constants with documentation
/// ECDSA operation timeout in microseconds
/// Based on hardware specification section 4.2.1
const ECDSA_TIMEOUT_US: u32 = 1000;

// ❌ BAD: Magic numbers
thread::sleep(Duration::from_micros(1000)); // FORBIDDEN
```

## Code Review Checklist

Use this checklist during code generation, local development, and code reviews:

- [ ] Code is completely panic-free (no unwrap/expect/panic/indexing) **except in test code**
- [ ] All fallible operations return Result or Option **except in test code**
- [ ] Integer operations use checked/saturating/wrapping methods where needed
- [ ] Array/slice access uses get() or pattern matching, not direct indexing **except in test code**
- [ ] Error cases are well documented and handled appropriately
- [ ] Tests verify error handling paths, not just happy paths
- [ ] Unsafe code blocks are documented with safety comments **including test code**
- [ ] Hardware register access uses proper volatile operations **including test code**
- [ ] Cryptographic operations use constant-time implementations where applicable
- [ ] Magic numbers are replaced with named constants (register offsets, bit masks, timeouts) **except in test code**
- [ ] All constants include documentation explaining their purpose and source
- [ ] All register access is hidden behind type-safe register access layer (PAC or safe wrapper) **including test code**
- [ ] No direct pointer arithmetic or raw memory access for hardware registers **including test code**


## Test Code Exceptions

Test code (annotated with `#[cfg(test)]` or in `tests/` modules) has relaxed requirements for developer ergonomics, but still maintains safety for hardware access:

### ✅ Allowed in Test Code Only

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_data() {
        // ✅ OK: unwrap() in tests for simplicity
        let result = parse_data("123").unwrap();
        assert_eq!(result, 123);
        
        // ✅ OK: Direct indexing in tests
        let data = vec![1, 2, 3, 4];
        assert_eq!(data[0], 1);
        
        // ✅ OK: expect() with descriptive messages in tests
        let config = Config::from_file("test.toml")
            .expect("test config file should exist");
            
        // ✅ OK: Magic numbers in test data
        let test_input = [0x01, 0x02, 0x03, 0x04];
        
        // ✅ OK: Vec allocation in tests
        let mut buffer = vec![0u8; 1024];
    }
}
```

### ❌ Still Forbidden in Test Code

```rust
#[cfg(test)] 
mod tests {
    use super::*;
    
    #[test]
    fn test_hardware_access() {
        // ❌ FORBIDDEN: Direct register access even in tests
        unsafe {
            write_volatile(0x4000_0000 as *mut u32, 0x1234);
        }
        
        // ✅ REQUIRED: Use safe wrappers even in tests
        let mut device = TestEcdsaController::new();
        device.enable_engine().unwrap();
    }
    
    #[test] 
    fn test_crypto() {
        // ❌ FORBIDDEN: Non-constant time crypto even in tests
        let mut key = [0u8; 32];
        for (i, byte) in secret_key.iter().enumerate() {
            if *byte == target_byte { // Timing attack possible
                key[i] = *byte;
            }
        }
        
        // ✅ REQUIRED: Use constant-time operations
        key.copy_from_slice(&secret_key);
    }
}
```

### Test Helper Patterns

```rust
#[cfg(test)]
mod test_helpers {
    use super::*;
    
    // ✅ OK: Test-specific helper that wraps unsafe operations
    pub struct TestEcdsaController {
        inner: EcdsaController,
    }
    
    impl TestEcdsaController {
        pub fn new() -> Self {
            // ✅ OK: unwrap() in test helper constructors
            let secure = unsafe { ast1060_pac::Secure::steal() };
            Self {
                inner: EcdsaController::new(secure).unwrap()
            }
        }
        
        pub fn enable_engine(&mut self) -> Result<(), EcdsaError> {
            // Still use proper error handling in test helpers
            self.inner.enable_engine()
        }
        
        pub fn force_state_for_testing(&mut self, state: u32) {
            // ✅ OK: Direct access for test setup
            self.inner.test_register_override(0x7c, state).unwrap();
        }
    }
    
    // ✅ OK: Test data with magic numbers
    pub const TEST_P384_PRIVATE_KEY: &[u8] = &[
        0x01, 0x02, 0x03, /* ... */
    ];
}
```

### Rationale for Test Exceptions

- **Ergonomics**: Tests should be easy to write and maintain
- **Clarity**: Test intent should be obvious without error handling noise  
- **Speed**: Test development shouldn't be slowed by production safety requirements
- **Hardware Safety**: Even tests must not bypass hardware safety mechanisms
- **Crypto Safety**: Even tests must not introduce timing attack vulnerabilities

## Quick Reference: Forbidden Patterns

**Note**: Test code (marked with `#[cfg(test)]`) may use unwrap(), expect(), direct indexing, and magic numbers for ergonomics. Hardware and crypto safety rules still apply to tests.

**Clarification**: Only panic-prone `unwrap` methods are forbidden. Safe methods like `unwrap_or()`, `unwrap_or_else()`, and `unwrap_or_default()` are acceptable in production code as they cannot panic.

| Forbidden Pattern | Required Alternative | Test Exception |
|-------------------|----------------------|----------------|
| `value.unwrap()` | `match value { Some(v) => v, None => return Err(...) }` | ✅ OK in tests |
| `result.expect("msg")` | `match result { Ok(v) => v, Err(e) => return Err(e.into()) }` | ✅ OK in tests |
| `collection[index]` | `collection.get(index).ok_or(Error::OutOfBounds)?` | ✅ OK in tests |
| `a + b` (integers) | `a.checked_add(b).ok_or(Error::Overflow)?` | ⚠️ Use sparingly |
| `ptr.read()` | `ptr.read_volatile()` (for MMIO) | ❌ Never allowed |
| `status & (1 << 20)` | `status & STATUS_COMPLETE_BIT` | ✅ OK in tests |
| `0x1234` (magic numbers) | `const REGISTER_OFFSET: u32 = 0x1234;` | ✅ OK in tests |
| `retry = 1000` (magic timeouts) | `const MAX_RETRIES: u32 = 1000; // Hardware spec: max 5ms` | ✅ OK in tests |
| `(BASE + offset) as *mut u32` | Use PAC register structs instead of raw pointers | ❌ Never allowed |
| `write_volatile(ptr, val)` | `device.register().write(\|w\| w.field().bits(val))` | ❌ Never allowed |
| `read_volatile(ptr)` | `device.register().read().field().bits()` | ❌ Never allowed |

## Security-Specific Guidelines

- **Timing attacks**: Use constant-time comparisons for secrets (subtle crate)
- **Zeroization**: Use `zeroize` crate for sensitive data cleanup (keys, passwords, etc.)
- **Memory safety**: Ensure sensitive data is properly zeroized after use
- **Hardware abstraction**: All register access must go through HAL traits
- **Error information**: Don't leak sensitive data in error messages
- **Register access**: All hardware registers must use type-safe access layers (PAC or safe wrappers)

## Type-Safe Register Access Requirements

All hardware register access must be hidden behind type-safe abstractions. Direct pointer arithmetic and raw memory access are forbidden.

### Peripheral Access Crate (PAC) Usage
```rust
// ❌ Forbidden - Direct register access
const CONTROL_REG: usize = 0x1000;
unsafe { write_volatile((BASE + CONTROL_REG) as *mut u32, value) };
let status = unsafe { read_volatile((BASE + STATUS_REG) as *mut u32) };

// ✅ Required - PAC register access
device.control_register().write(|w| w
    .enable().set_bit()
    .mode().bits(0b10)
    .timeout().bits(100)
);

let status = device.status_register().read();
if status.complete().bit_is_set() {
    // Handle completion
}
```

### Safe Register Wrapper (When PAC Unavailable)
```rust
// When PAC doesn't cover all registers, create safe wrappers
pub struct EcdsaController {
    // Use PAC for available registers
    secure: Secure,
    // Safe wrapper for missing registers
    control: RegisterRW<EcdsaControlRegister>,
    status: RegisterRO<EcdsaStatusRegister>,
}

impl EcdsaController {
    pub fn enable_engine(&mut self) -> Result<(), EcdsaError> {
        // Use PAC when available
        self.secure.secure0b4().write(|w| w.sec_boot_ecceng_enbl().set_bit());
        
        // Use safe wrapper for missing registers
        self.control.write(EcdsaControlRegister::new()
            .with_operation_mode(OperationMode::Verify)
            .with_curve_type(CurveType::P384)
        );
        
        Ok(())
    }
    
    pub fn wait_for_completion(&self) -> Result<EcdsaResult, EcdsaError> {
        loop {
            let status = self.status.read();
            if status.operation_complete() {
                return if status.operation_success() {
                    Ok(EcdsaResult::Success)
                } else {
                    Err(EcdsaError::InvalidSignature)
                };
            }
        }
    }
}

// Type-safe register definitions
#[derive(Clone, Copy)]
pub struct EcdsaControlRegister(u32);

impl EcdsaControlRegister {
    pub fn new() -> Self { Self(0) }
    
    pub fn with_operation_mode(mut self, mode: OperationMode) -> Self {
        self.0 = (self.0 & !0x3) | (mode as u32);
        self
    }
    
    pub fn with_curve_type(mut self, curve: CurveType) -> Self {
        self.0 = (self.0 & !0xC) | ((curve as u32) << 2);
        self
    }
}

#[repr(u32)]
pub enum OperationMode {
    Verify = 0,
    Sign = 1,
    KeyGen = 2,
}
```

### Memory-Mapped I/O Safety
```rust
// ❌ Forbidden - Raw MMIO
unsafe fn write_sram(base: *mut u8, offset: usize, data: &[u8]) {
    for (i, &byte) in data.iter().enumerate() {
        write_volatile(base.add(offset + i), byte);
    }
}

// ✅ Required - Safe MMIO wrapper
pub struct SramController {
    base: NonNull<u8>,
    size: usize,
}

impl SramController {
    pub fn write_region(&mut self, region: SramRegion, data: &[u8]) -> Result<(), SramError> {
        let offset = region.offset();
        let max_size = region.max_size();
        
        if data.len() > max_size {
            return Err(SramError::BufferTooLarge);
        }
        
        if offset + data.len() > self.size {
            return Err(SramError::OutOfBounds);
        }
        
        unsafe {
            for (i, &byte) in data.iter().enumerate() {
                write_volatile(self.base.as_ptr().add(offset + i), byte);
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SramRegion {
    PublicKeyX,   // Offset 0x2080, Max 48 bytes
    PublicKeyY,   // Offset 0x20C0, Max 48 bytes
    SignatureR,   // Offset 0x21C0, Max 48 bytes
    SignatureS,   // Offset 0x2200, Max 48 bytes
    MessageHash,  // Offset 0x2240, Max 48 bytes
}

impl SramRegion {
    fn offset(self) -> usize {
        match self {
            Self::PublicKeyX => 0x2080,
            Self::PublicKeyY => 0x20C0,
            Self::SignatureR => 0x21C0,
            Self::SignatureS => 0x2200,
            Self::MessageHash => 0x2240,
        }
    }
    
    fn max_size(self) -> usize {
        match self {
            Self::PublicKeyX | Self::PublicKeyY | 
            Self::SignatureR | Self::SignatureS | 
            Self::MessageHash => 48, // P384 size
        }
    }
}
```

## Magic Numbers and Constants

All magic numbers must be replaced with well-documented named constants:

### Hardware Register Access
```rust
// ❌ Forbidden - Direct register access
if status & (1 << 20) != 0 {
    let value = ptr.add(0x7c);
}

// ❌ Also Forbidden - Bypassing PAC
let base_ptr = (REGISTER_BASE + offset) as *mut u32;
unsafe { write_volatile(base_ptr, value) };

// ❌ Also Forbidden - Raw pointer arithmetic  
fn sec_wr(&self, offset: usize, val: u32) {
    unsafe { write_volatile(self.base.as_ptr().add(offset / 4), val) };
}

// ✅ Required - PAC register access
const STATUS_COMPLETE_BIT: u32 = 1 << 20;  // Hardware manual section 4.3.2

let status = device.status_register().read();
if status.operation_complete().bit_is_set() {
    device.control_register().write(|w| w
        .operation_mode().verify()
        .curve_type().p384()
        .enable().set_bit()
    );
}

// ✅ Alternative - Safe wrapper when PAC unavailable
device.write_control_register(ControlValue::new()
    .with_mode(OperationMode::Verify)
    .with_curve(CurveType::P384)
);
```

### Timeouts and Retry Limits
```rust
// ❌ Forbidden
let mut retry = 1000;
delay_ns(5000);

// ✅ Required
const MAX_OPERATION_RETRIES: u32 = 1000;  // Hardware spec: max 5ms at 200MHz
const POLL_INTERVAL_NS: u32 = 5000;       // Optimal polling interval per datasheet

let mut retry = MAX_OPERATION_RETRIES;
delay_ns(POLL_INTERVAL_NS);
```

### Cryptographic Parameters
```rust
// ❌ Forbidden  
let key_size = 32;
let signature_len = 64;

// ✅ Required
const P256_SCALAR_BYTES: usize = 32;      // NIST P-256 field element size
const P256_SIGNATURE_BYTES: usize = 64;   // r + s components, 32 bytes each

let key_size = P256_SCALAR_BYTES;
let signature_len = P256_SIGNATURE_BYTES;
```

### Documentation Requirements
Each constant must include:
- **Purpose**: What the value represents
- **Source**: Where the value comes from (datasheet section, RFC, etc.)
- **Units**: For timeouts, sizes, etc.
- **Constraints**: Valid ranges or special considerations

## Enforcement

To prevent violations of these guidelines, add these lint denials to your crate root:

```rust
// In lib.rs or main.rs - Apply rules only to production code
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::indexing_slicing))]
#![cfg_attr(not(test), warn(clippy::expect_used))]
```

For test modules, explicitly allow these patterns:

```rust
// In test module files (e.g., src/tests/mod.rs)
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]
```

For individual test functions that need these patterns:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[allow(clippy::unwrap_used, clippy::indexing_slicing)]
    fn test_with_ergonomic_patterns() {
        let data = vec![1, 2, 3, 4];
        let result = parse_data(&data).unwrap();
        assert_eq!(result[0], expected_value);
    }
}
```

These lints will:
- `clippy::unwrap_used` - Prevent all `.unwrap()` calls in production code
- `clippy::indexing_slicing` - Prevent direct array indexing like `array[index]`
- `clippy::expect_used` - Warn on `.expect()` usage (allowed in tests but discouraged in production)

**Key Benefits**:
- Production code is enforced to be panic-free
- Test code retains ergonomic patterns for developer productivity
- Compile-time safety guarantees for production paths
