// Licensed under the Apache-2.0 license

use crate::hace_controller::{ContextCleanup, HaceController, HashAlgo, HACE_SG_LAST};
use proposed_traits::digest::{DigestAlgorithm, DigestInit, DigestOp, Error, ErrorKind, ErrorType};

// DigestAlgorithm implementation for HashAlgo
impl DigestAlgorithm for HashAlgo {
    const OUTPUT_BITS: usize = 512; // Maximum size for all variants
    type DigestOutput = [u8; 64]; // Use the maximum size for all variants
}

pub trait IntoHashAlgo {
    fn to_hash_algo() -> HashAlgo;
}

pub struct Digest48(pub [u8; 48]);

impl Default for Digest48 {
    fn default() -> Self {
        Digest48([0u8; 48])
    }
}

impl AsRef<[u8]> for Digest48 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Digest48 {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

pub struct Digest64(pub [u8; 64]);
impl Default for Digest64 {
    fn default() -> Self {
        Digest64([0u8; 64])
    }
}

impl AsRef<[u8]> for Digest64 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Digest64 {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

pub struct Sha1;
pub struct Sha224;
pub struct Sha256;
pub struct Sha384;
pub struct Sha512;

impl DigestAlgorithm for Sha1 {
    const OUTPUT_BITS: usize = 160;
    type DigestOutput = [u8; 20];
}

impl DigestAlgorithm for Sha224 {
    const OUTPUT_BITS: usize = 224;
    type DigestOutput = [u8; 28];
}

impl DigestAlgorithm for Sha256 {
    const OUTPUT_BITS: usize = 256;
    type DigestOutput = [u8; 32];
}

impl DigestAlgorithm for Sha384 {
    const OUTPUT_BITS: usize = 384;
    type DigestOutput = Digest48; // Use Digest48 for 384 bits
}

impl DigestAlgorithm for Sha512 {
    const OUTPUT_BITS: usize = 512;
    type DigestOutput = Digest64; // Use Digest64 for 512 bits
}

impl Default for Sha256 {
    fn default() -> Self {
        Sha256
    }
}

impl Default for Sha384 {
    fn default() -> Self {
        Sha384
    }
}

impl Default for Sha512 {
    fn default() -> Self {
        Sha512
    }
}

impl IntoHashAlgo for Sha256 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA256
    }
}

impl IntoHashAlgo for Sha384 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA384
    }
}

impl IntoHashAlgo for Sha512 {
    fn to_hash_algo() -> HashAlgo {
        HashAlgo::SHA512
    }
}

impl<A> DigestInit<A> for HaceController
where
    A: DigestAlgorithm + IntoHashAlgo,
    A::DigestOutput: Default + AsMut<[u8]>,
{
    type OpContext<'a>
        = OpContextImpl<'a, A>
    where
        Self: 'a; // Define your OpContext type here

    fn init(&mut self, _algo: A) -> Result<Self::OpContext<'_>, Self::Error> {
        self.algo = A::to_hash_algo();
        self.ctx_mut().method = self.algo.hash_cmd();
        self.copy_iv_to_digest();
        self.ctx_mut().block_size = u32::try_from(self.algo.block_size()).unwrap();
        self.ctx_mut().bufcnt = 0;
        self.ctx_mut().digcnt = [0; 2];

        Ok(OpContextImpl {
            controller: self,
            _phantom: core::marker::PhantomData,
        })
    }
}

pub struct OpContextImpl<'a, A: DigestAlgorithm + IntoHashAlgo> {
    pub controller: &'a mut HaceController,
    _phantom: core::marker::PhantomData<A>,
}

#[derive(Debug)]
pub struct HashError(pub ErrorKind);

impl Error for HashError {
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

impl From<ErrorKind> for HashError {
    fn from(kind: ErrorKind) -> Self {
        HashError(kind)
    }
}

impl<A> ErrorType for OpContextImpl<'_, A>
where
    A: DigestAlgorithm + IntoHashAlgo,
{
    type Error = HashError;
}

impl<A> DigestOp for OpContextImpl<'_, A>
where
    A: DigestAlgorithm + IntoHashAlgo,
    A::DigestOutput: Default + AsMut<[u8]>,
{
    type Output = A::DigestOutput;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        let input_len = u32::try_from(input.len()).map_err(|_| ErrorKind::InvalidInputLength)?;

        let (new_len, carry) =
            self.controller.ctx_mut().digcnt[0].overflowing_add(u64::from(input_len));

        self.controller.ctx_mut().digcnt[0] = new_len;
        if carry {
            self.controller.ctx_mut().digcnt[1] += 1;
        }

        let start = self.controller.ctx_mut().bufcnt as usize;
        let end = start + input_len as usize;
        if self.controller.ctx_mut().bufcnt + input_len < self.controller.ctx_mut().block_size {
            self.controller.ctx_mut().buffer[start..end].copy_from_slice(input);
            self.controller.ctx_mut().bufcnt += input_len;
            return Ok(());
        }

        let remaining =
            (input_len + self.controller.ctx_mut().bufcnt) % self.controller.ctx_mut().block_size;
        let total_len = (input_len + self.controller.ctx_mut().bufcnt) - remaining;
        let mut i = 0;

        if self.controller.ctx_mut().bufcnt != 0 {
            self.controller.ctx_mut().sg[0].addr = self.controller.ctx_mut().buffer.as_ptr() as u32;
            self.controller.ctx_mut().sg[0].len = self.controller.ctx_mut().bufcnt;
            if total_len == self.controller.ctx_mut().bufcnt {
                self.controller.ctx_mut().sg[0].addr = input.as_ptr() as u32;
                self.controller.ctx_mut().sg[0].len |= HACE_SG_LAST;
            }
            i += 1;
        }

        if total_len != self.controller.ctx_mut().bufcnt {
            self.controller.ctx_mut().sg[i].addr = input.as_ptr() as u32;
            self.controller.ctx_mut().sg[i].len =
                (total_len - self.controller.ctx_mut().bufcnt) | HACE_SG_LAST;
        }

        self.controller.start_hash_operation(total_len);

        if remaining != 0 {
            let src_start = (total_len - self.controller.ctx_mut().bufcnt) as usize;
            let src_end = src_start + remaining as usize;

            self.controller.ctx_mut().buffer[..(remaining as usize)]
                .copy_from_slice(&input[src_start..src_end]);
            self.controller.ctx_mut().bufcnt = remaining;
        }
        Ok(())
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        self.controller.fill_padding(0);
        let digest_len = self.controller.algo.digest_size();

        let (digest_ptr, bufcnt) = {
            let ctx = self.controller.ctx_mut();

            ctx.sg[0].addr = ctx.buffer.as_ptr() as u32;
            ctx.sg[0].len = ctx.bufcnt | HACE_SG_LAST;

            (ctx.digest.as_ptr(), ctx.bufcnt)
        };

        self.controller.start_hash_operation(bufcnt);

        let slice = unsafe { core::slice::from_raw_parts(digest_ptr, digest_len) };

        let mut output = A::DigestOutput::default();
        output.as_mut()[..digest_len].copy_from_slice(slice);

        self.controller.cleanup_context();

        Ok(output) // Return the final output
    }
}
