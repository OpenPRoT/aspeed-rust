// Licensed under the Apache-2.0 license

use crate::hace_controller::{ContextCleanup, HaceController, HashAlgo, HACE_SG_EN};
use proposed_traits::mac::{Error, ErrorKind, ErrorType, MacAlgorithm, MacInit, MacOp};

// MacAlgorithm implementation for HashAlgo
impl MacAlgorithm for HashAlgo {
    const OUTPUT_BITS: usize = 512; // Maximum size for all variants
    type MacOutput = [u8; 64]; // Use the maximum size for all variants
    type Key = [u8; 64]; // Use the maximum size for all variants
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

impl MacAlgorithm for Sha1 {
    const OUTPUT_BITS: usize = 160;
    type MacOutput = [u8; 20];
    type Key = [u8; 64];
}

impl MacAlgorithm for Sha224 {
    const OUTPUT_BITS: usize = 224;
    type MacOutput = [u8; 28];
    type Key = [u8; 64];
}

impl MacAlgorithm for Sha256 {
    const OUTPUT_BITS: usize = 256;
    type MacOutput = [u8; 32];
    type Key = [u8; 32];
}

impl MacAlgorithm for Sha384 {
    const OUTPUT_BITS: usize = 384;
    type MacOutput = Digest48; // Use Digest48 for 384 bits
    type Key = [u8; 48];
}

impl MacAlgorithm for Sha512 {
    const OUTPUT_BITS: usize = 512;
    type MacOutput = Digest64; // Use Digest64 for 512 bits
    type Key = [u8; 64];
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

impl<A> MacInit<A> for HaceController
where
    A: MacAlgorithm + IntoHashAlgo,
    A::MacOutput: Default + AsMut<[u8]>,
    A::Key: AsRef<[u8]>,
{
    type OpContext<'a>
        = OpContextImpl<'a, A>
    where
        Self: 'a; // Define your OpContext type here

    fn init<'a>(&'a mut self, _algo: A, key: &A::Key) -> Result<Self::OpContext<'a>, Self::Error> {
        self.algo = A::to_hash_algo();
        self.ctx_mut().method = self.algo.hash_cmd();
        self.copy_iv_to_digest();
        self.ctx_mut().block_size = u32::try_from(self.algo.block_size()).unwrap();
        self.ctx_mut().bufcnt = 0;
        self.ctx_mut().digcnt = [0; 2];
        self.ctx_mut().buffer.fill(0);
        self.ctx_mut().digest.fill(0);
        self.ctx_mut().ipad.fill(0);
        self.ctx_mut().opad.fill(0);
        self.ctx_mut().key.fill(0);

        if key.as_ref().len() > self.ctx_mut().key.len() {
            // hash key if it is too long
            self.hash_key(key);
        } else {
            self.ctx_mut().key[..key.as_ref().len()].copy_from_slice(key.as_ref());
            self.ctx_mut().ipad[..key.as_ref().len()].copy_from_slice(key.as_ref());
            self.ctx_mut().opad[..key.as_ref().len()].copy_from_slice(key.as_ref());
            self.ctx_mut().key_len = u32::try_from(key.as_ref().len()).unwrap();
        }

        for i in 0..self.ctx_mut().block_size as usize {
            self.ctx_mut().ipad[i] ^= 0x36;
            self.ctx_mut().opad[i] ^= 0x5c;
        }

        Ok(OpContextImpl {
            controller: self,
            _phantom: core::marker::PhantomData,
        })
    }
}

pub struct OpContextImpl<'a, A: MacAlgorithm + IntoHashAlgo> {
    pub controller: &'a mut HaceController,
    _phantom: core::marker::PhantomData<A>,
}

#[derive(Debug)]
pub struct MacError(pub ErrorKind);

impl Error for MacError {
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

impl From<ErrorKind> for MacError {
    fn from(kind: ErrorKind) -> Self {
        MacError(kind)
    }
}

impl<A> ErrorType for OpContextImpl<'_, A>
where
    A: MacAlgorithm + IntoHashAlgo,
{
    type Error = MacError;
}

impl<A> MacOp for OpContextImpl<'_, A>
where
    A: MacAlgorithm + IntoHashAlgo,
    A::MacOutput: Default + AsMut<[u8]>,
{
    type Output = A::MacOutput;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        let ctrl: &mut HaceController = self.controller;
        let algo = ctrl.algo;
        let block_size = algo.block_size();
        let digest_size = algo.digest_size();
        let mut bufcnt: u32;

        {
            let ctx = ctrl.ctx_mut();
            ctx.digcnt[0] = block_size as u64;
            ctx.bufcnt =
                u32::try_from(block_size).map_err(|_| MacError(ErrorKind::InvalidInputLength))?;

            // H(ipad + input)
            let ipad = &ctx.ipad[..block_size];
            ctx.buffer[..algo.block_size()].copy_from_slice(ipad);
            ctx.buffer[algo.block_size()..(algo.block_size() + input.len())].copy_from_slice(input);
            ctx.digcnt[0] += input.len() as u64;
            ctx.bufcnt +=
                u32::try_from(input.len()).map_err(|_| MacError(ErrorKind::InvalidInputLength))?;
            ctx.method &= !HACE_SG_EN; // Disable SG mode for key hashing
        }

        ctrl.fill_padding(0);
        bufcnt = ctrl.ctx_mut().bufcnt;
        ctrl.copy_iv_to_digest();
        ctrl.start_hash_operation(bufcnt);
        let slice =
            unsafe { core::slice::from_raw_parts(ctrl.ctx_mut().digest.as_ptr(), digest_size) };

        // H(opad + H(opad + hash sum))
        {
            let ctx = ctrl.ctx_mut();
            ctx.digcnt[0] = block_size as u64 + digest_size as u64;
            ctx.bufcnt = u32::try_from(block_size + digest_size)
                .map_err(|_| MacError(ErrorKind::UpdateError))?;
            ctx.buffer[..block_size].copy_from_slice(&ctx.opad[..block_size]);
            ctx.buffer[block_size..(block_size + digest_size)].copy_from_slice(slice);
        }
        ctrl.fill_padding(0);
        bufcnt = ctrl.ctx_mut().bufcnt;
        ctrl.copy_iv_to_digest();
        ctrl.start_hash_operation(bufcnt);

        Ok(())
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        let digest_size = self.controller.algo.digest_size();
        let ctx = self.controller.ctx_mut();

        let slice = unsafe { core::slice::from_raw_parts(ctx.digest.as_ptr(), digest_size) };

        let mut output = A::MacOutput::default();
        output.as_mut()[..digest_size].copy_from_slice(slice);

        self.controller.cleanup_context();

        Ok(output) // Return the final output
    }
}
