# Copilot Instructions for aspeed-ddk

## Project Overview
aspeed-ddk is a Rust-based driver development kit for ASPEED SoCs, focusing on no_std environments and efficient resource usage.

## Pull Request Review Checklist

- [ ] Code is completely panic-free (no unwrap/expect/panic/indexing)
- [ ] All fallible operations return Result or Option
- [ ] Integer operations use checked/saturating/wrapping methods where needed
- [ ] Array/slice access uses get() or pattern matching, not direct indexing
- [ ] Error cases are well documented and handled appropriately
- [ ] Tests verify error handling paths, not just happy paths

## Quick Reference: Forbidden Patterns

| Forbidden Pattern | Required Alternative |
|-------------------|----------------------|
| `value.unwrap()` | `match value { Some(v) => v, None => return Err(...) }` |
| `result.expect("msg")` | `match result { Ok(v) => v, Err(e) => return Err(e.into()) }` |
| `collection[index]` | `collection.get(index).ok_or(Error::OutOfBounds)?` |


## Code Style

### no_std and Memory Allocation Guidelines

- This project is strictly **no_std** and **no_alloc** in production code
- All production paths must be allocation-free and compatible with bare-metal targets

#### Production Code Requirements

- **NEVER** use crates or features that require heap allocation in production code
- **DO NOT** use the `heapless` crate in production paths despite its name suggesting compatibility
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
        // Heapless is also fine in tests
        use heapless::Vec;
        let mut input: Vec<u8, 8> = Vec::new();
        input.extend_from_slice(&[1, 2, 3]).unwrap();
        
        let mut output = [0u8; 16];
        let result = process_data(&input, &mut output);
        assert!(result.is_ok());
        // ...
    }
}


### Unsafe Code

- Minimize unsafe code; isolate in well-documented functions
- Document all safety preconditions in unsafe functions
- Add safety comments explaining why unsafe is necessary
- Unit test unsafe code thoroughly

## Common Patterns

### Static vs. Dynamic Dispatch

This project strongly prefers static dispatch over dynamic dispatch to optimize for binary size, performance, and no_std compatibility.

#### Static Dispatch Requirements

- Use generic parameters instead of trait objects (`dyn Trait`) whenever possible
- Leverage impl Trait in return positions rather than Box<dyn Trait>
- Monomorphize code at compile time rather than using virtual dispatch at runtime
- Avoid heap allocations associated with typical dyn Trait usage


## Workflows

### Pre-commit
cargo xtask precommit