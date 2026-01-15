# Feedback: Drawbacks of Instance-Based Generic Types for Task-Based Frameworks

**Status:** Analysis  
**Author:** OpenPRoT Team  
**Date:** January 2026

## Overview

This document analyzes the integration challenges when using the `aspeed-rust` I2C driver's generic type pattern (`Ast1060I2c<I2C: Instance, ...>`) within a microkernel task-based framework like OpenPRoT. The driver uses Rust's type system to encode peripheral identity at compile time, which creates friction with dynamic dispatch requirements.

## The Instance Pattern

The `aspeed-rust` driver uses a trait + macro pattern to represent each I2C peripheral:

```rust
// Trait defines per-peripheral behavior
pub trait Instance {
    fn ptr() -> *const ast1060_pac::i2c::RegisterBlock;
    fn buff_ptr() -> *const ast1060_pac::i2cbuff::RegisterBlock;
    const BUS_NUM: u8;
}

// Macro generates impl for each peripheral
macro_rules! macro_i2c {
    ($I2cx: ident, $I2cbuffx: ident, $x: literal) => {
        impl Instance for ast1060_pac::$I2cx {
            fn ptr() -> *const ast1060_pac::i2c::RegisterBlock { ... }
            fn buff_ptr() -> *const ast1060_pac::i2cbuff::RegisterBlock { ... }
            const BUS_NUM: u8 = $x;
        // ...
    }
}
```

**Conflicts with framework:**
- Framework may want to allocate buffers per-task, not globally
- `I2C_TOTAL = 4` hardcoded, but AST1060 has 14 controllers
- Static `mut` requires `unsafe` and prevents safe concurrent access
- No support for task-isolated buffer pools

### 4. Lifetime Complexity

**Problem:** The driver carries multiple lifetime parameters:

```rust
pub struct Ast1060I2c<'a, I2C: Instance, I2CT: I2CTarget, L: Logger> {
    pub mdma_buf: &'a mut DmaBuffer<ASPEED_I2C_DMA_SIZE>,
    pub sdma_buf: &'a mut DmaBuffer<I2C_SLAVE_BUF_SIZE>,
    pub i2c_data: I2cData<'a, I2CT>,
    // ...
}
```

**Impact:**
- Lifetime `'a` ties driver to buffer lifetime
- Cannot easily store in task-owned structures with `'static` requirement
- Complicates adapter implementations that need owned storage

### 5. Target Mode Type Parameter Infection

**Problem:** The driver is generic over the I2C target handler:

```rust
pub struct Ast1060I2c<'a, I2C: Instance, I2CT: I2CTarget, L: Logger> {
    pub slave_target: Option<&'a mut I2CT>,
    // ...
}
```

**Impact:**
- Even controller-only use requires specifying `I2CT` type
- Different target handlers = different driver types
- Prevents storing heterogeneous target configurations

### 6. Logger Type Parameter

**Problem:** The logger is also a type parameter:

```rust
pub struct Ast1060I2c<'a, I2C: Instance, I2CT: I2CTarget, L: Logger>
```

**Impact:**
- Different loggers = different driver types
- Cannot mix debug/release logging configurations at runtime
- Adds another dimension of type variation


## Comparison: What the Framework Needs

| Framework Requirement | Instance Pattern | Ideal Pattern |
|-----------------------|------------------|---------------|
| Single collection for all controllers | ❌ Different types | ✅ Same type, runtime dispatch |
| Runtime controller selection | ❌ Compile-time only | ✅ Index-based dispatch |
| Minimal code size | ❌ 14× monomorphization | ✅ Single implementation |
| Task-isolated buffers | ❌ Global static allocation | ✅ Per-instance buffers |
| Simple adapter implementation | ❌ Type erasure required | ✅ Direct trait impl |

## Conclusion

The `Instance` trait pattern in `aspeed-rust` is a common embedded Rust idiom for type-safe peripheral access, but it creates significant friction when integrating with task-based frameworks that require:

1. **Runtime dispatch** by controller index
2. **Homogeneous collections** of drivers
3. **Minimal code size** for embedded targets
4. **Task-isolated state** management

Refactoring to use runtime instance selection or implementing framework traits directly would significantly reduce integration complexity while preserving the driver's functionality.

## Glossary

- **Type erasure**: A technique where concrete type information is hidden behind a uniform interface (for example, trait objects or enums) so that values of different concrete types can be handled through a single, common type at runtime.
- **Dynamic dispatch**: Method call resolution performed at runtime through an indirection mechanism (for example, a vtable for `dyn Trait`), allowing different concrete implementations to be selected via a shared interface.
- **Static dispatch / monomorphization**: Compile-time specialization of generic functions or types for each concrete type parameter, producing separate code paths per instantiation (great for inlining and performance, but can increase code size).
- **Homogeneous collection**: A collection whose elements all share the same concrete type (for example, an array of a single driver type), which allows simple iteration and indexing without extra type erasure or adapter layers.
- **Runtime dispatch by index**: Selecting a controller or peripheral at runtime using an integer index (for example, `controller_id: u8`) rather than encoding the choice in the type parameter list, which is necessary for task-based frameworks that route requests dynamically.
