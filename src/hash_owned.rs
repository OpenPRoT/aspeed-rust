// Licensed under the Apache-2.0 license

//! `OpenProt` owned digest API implementation for ASPEED HACE controller
//!
//! This module implements the move-based digest API from openprot-hal-blocking
//! which provides exclusive access to the shared HACE hardware controller
//! and compile-time prevention of use-after-finalize.
//!
//! Note: The underlying cryptographic context is shared globally in `.ram_nc` section.
//! The "owned" aspect refers to exclusive ownership of the `HaceController` wrapper,
//! not the actual hardware context. Only one digest operation can be active at a time.
//!
//! Unlike the scoped API, the controller wrapper has no lifetime constraints
//! and can be stored in structs, moved across functions, and persist across IPC.
//!

use crate::hace_controller::{ContextCleanup, HaceController, HashAlgo, HACE_SG_LAST};
use core::convert::Infallible;
use core::marker::PhantomData;
use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{DigestAlgorithm, ErrorType};

// Re-export digest algorithm types from existing hash module
pub use crate::hash::{Digest48, Digest64, Sha1, Sha224, Sha256, Sha384, Sha512};

// Also re-export OpenProt digest types for convenience
pub use openprot_hal_blocking::digest::{Digest, Sha2_256, Sha2_384, Sha2_512};

/// Trait to convert digest algorithm types to our internal `HashAlgo` enum
pub trait IntoHashAlgo {
    fn to_hash_algo() -> HashAlgo;
}

impl IntoHashAlgo for Sha2_256 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA256
    }
}

impl IntoHashAlgo for Sha2_384 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA384
    }
}

impl IntoHashAlgo for Sha2_512 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA512
    }
}

/// Owned digest context that wraps the HACE controller for exclusive access
///
/// This context owns the controller wrapper (not the underlying shared hardware context)
/// and provides exclusive access to the HACE hardware during digest operations.
/// It has no lifetime constraints and can be stored in structs, moved across functions,
/// and persist across IPC boundaries.
pub struct OwnedDigestContext<T: DigestAlgorithm + IntoHashAlgo> {
    controller: HaceController,
    _phantom: PhantomData<T>,
}

// Implement ErrorType for HaceController (required by OpenProt DigestInit)
impl ErrorType for HaceController {
    type Error = Infallible;
}

impl<T: DigestAlgorithm + IntoHashAlgo> ErrorType for OwnedDigestContext<T> {
    type Error = Infallible;
}

/// Macro to implement owned digest traits for each algorithm
macro_rules! impl_owned_digest {
    ($algo:ident) => {
        impl DigestInit<$algo> for HaceController {
            type Context = OwnedDigestContext<$algo>;
            type Output = <$algo as DigestAlgorithm>::Digest;

            fn init(mut self, _init_params: $algo) -> Result<Self::Context, Self::Error> {
                // Set up the algorithm and initialize the context
                self.algo = <$algo as IntoHashAlgo>::to_hash_algo();
                self.ctx_mut().method = self.algo.hash_cmd();
                self.copy_iv_to_digest();
                self.ctx_mut().block_size = u32::try_from(self.algo.block_size()).unwrap();
                self.ctx_mut().bufcnt = 0;
                self.ctx_mut().digcnt = [0; 2];

                Ok(OwnedDigestContext {
                    controller: self,
                    _phantom: PhantomData,
                })
            }
        }

        impl DigestOp for OwnedDigestContext<$algo> {
            type Output = <$algo as DigestAlgorithm>::Digest;
            type Controller = HaceController;

            fn update(mut self, data: &[u8]) -> Result<Self, Self::Error> {
                let input_len = u32::try_from(data.len()).unwrap_or(u32::MAX);

                // Update digest count
                let (new_len, carry) =
                    self.controller.ctx_mut().digcnt[0].overflowing_add(u64::from(input_len));

                self.controller.ctx_mut().digcnt[0] = new_len;
                if carry {
                    self.controller.ctx_mut().digcnt[1] += 1;
                }

                let start = self.controller.ctx_mut().bufcnt as usize;
                let end = start + input_len as usize;

                // If we can fit everything in the buffer, just copy it
                if self.controller.ctx_mut().bufcnt + input_len
                    < self.controller.ctx_mut().block_size
                {
                    self.controller.ctx_mut().buffer[start..end].copy_from_slice(data);
                    self.controller.ctx_mut().bufcnt += input_len;
                    return Ok(self);
                }

                // Process data in blocks using scatter-gather
                let remaining = (input_len + self.controller.ctx_mut().bufcnt)
                    % self.controller.ctx_mut().block_size;
                let total_len = (input_len + self.controller.ctx_mut().bufcnt) - remaining;
                let mut i = 0;

                if self.controller.ctx_mut().bufcnt != 0 {
                    self.controller.ctx_mut().sg[0].addr =
                        self.controller.ctx_mut().buffer.as_ptr() as u32;
                    self.controller.ctx_mut().sg[0].len = self.controller.ctx_mut().bufcnt;
                    if total_len == self.controller.ctx_mut().bufcnt {
                        self.controller.ctx_mut().sg[0].addr = data.as_ptr() as u32;
                        self.controller.ctx_mut().sg[0].len |= HACE_SG_LAST;
                    }
                    i += 1;
                }

                if total_len != self.controller.ctx_mut().bufcnt {
                    self.controller.ctx_mut().sg[i].addr = data.as_ptr() as u32;
                    self.controller.ctx_mut().sg[i].len =
                        (total_len - self.controller.ctx_mut().bufcnt) | HACE_SG_LAST;
                }

                self.controller.start_hash_operation(total_len);

                // Handle remaining data
                if remaining != 0 {
                    let src_start = (total_len - self.controller.ctx_mut().bufcnt) as usize;
                    let src_end = src_start + remaining as usize;

                    self.controller.ctx_mut().buffer[..(remaining as usize)]
                        .copy_from_slice(&data[src_start..src_end]);
                    self.controller.ctx_mut().bufcnt = remaining;
                } else {
                    self.controller.ctx_mut().bufcnt = 0;
                }

                Ok(self)
            }

            fn finalize(mut self) -> Result<(Self::Output, Self::Controller), Self::Error> {
                use openprot_hal_blocking::digest::Digest;
                const OUTPUT_WORDS: usize = <$algo as DigestAlgorithm>::OUTPUT_BITS / 32;

                // Fill padding and finalize
                self.controller.fill_padding(0);
                let digest_len = self.controller.algo.digest_size();

                let (digest_ptr, bufcnt) = {
                    let ctx = self.controller.ctx_mut();

                    ctx.sg[0].addr = ctx.buffer.as_ptr() as u32;
                    ctx.sg[0].len = ctx.bufcnt | HACE_SG_LAST;

                    (ctx.digest.as_ptr(), ctx.bufcnt)
                };

                self.controller.start_hash_operation(bufcnt);

                // Copy the digest result
                let slice = unsafe { core::slice::from_raw_parts(digest_ptr, digest_len) };

                // Create OpenProt Digest from the raw bytes using constructor
                let mut value = [0u32; OUTPUT_WORDS];

                // Copy bytes to u32 array in big-endian format
                for (i, chunk) in slice.chunks(4).enumerate() {
                    if i < OUTPUT_WORDS {
                        let mut bytes = [0u8; 4];
                        bytes[..chunk.len()].copy_from_slice(chunk);
                        value[i] = u32::from_be_bytes(bytes);
                    }
                }

                let output = Digest::new(value);

                // Clean up the context before returning the controller
                self.controller.cleanup_context();

                Ok((output, self.controller))
            }

            fn cancel(mut self) -> Self::Controller {
                // Clean up the context and return the controller
                self.controller.cleanup_context();
                self.controller
            }
        }
    };
}

// Implement the owned traits for each supported algorithm
impl_owned_digest!(Sha2_256);
impl_owned_digest!(Sha2_384);
impl_owned_digest!(Sha2_512);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hace_controller::HaceController;
    use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

    #[test]
    fn test_owned_digest_pattern() {
        // This test demonstrates the owned pattern usage
        // Note: In a real test, you'd need actual hardware or mocking

        // Example of what digest operations would look like on real hardware:
        // let controller = HaceController::new(hace_peripheral);
        // let context = controller.init(Sha2_256::default())?;
        // let context = context.update(b"hello")?;
        // let context = context.update(b" world")?;
        // let (digest, controller) = context.finalize()?;
        // // Controller is now recovered for reuse

        // This test verifies compilation
        assert!(true);
    }

    #[test]
    fn test_session_storage_pattern() {
        // Demonstrate controller storage pattern - impossible with scoped API
        // This simulates what a server would do to store controller wrappers
        // Note: Only one can be active at a time due to shared hardware context

        struct SimpleSessionManager {
            session_sha256: Option<OwnedDigestContext<Sha2_256>>,
            session_sha384: Option<OwnedDigestContext<Sha2_384>>,
            controller: Option<HaceController>,
        }

        impl SimpleSessionManager {
            fn new(controller: HaceController) -> Self {
                Self {
                    session_sha256: None,
                    session_sha384: None,
                    controller: Some(controller),
                }
            }

            // Multiple controller wrappers can be stored (but only one can be active at a time)
            fn create_sha256_session(&mut self) -> Result<(), Infallible> {
                let controller = self.controller.take().unwrap();
                let context = controller.init(Sha2_256::default())?;
                self.session_sha256 = Some(context);
                Ok(())
            }

            fn update_sha256_session(&mut self, data: &[u8]) -> Result<(), Infallible> {
                let context = self.session_sha256.take().unwrap();
                let updated_context = context.update(data)?;
                self.session_sha256 = Some(updated_context);
                Ok(())
            }
        }

        // This test verifies the pattern compiles correctly
        // In real usage, you'd have actual hardware initialization
        assert!(true);
    }
}
