// Licensed under the Apache-2.0 license

use ast1060_pac::Secure;
use core::ptr::{read_volatile, write_bytes, write_volatile, NonNull};
use embedded_hal::delay::DelayNs;
use proposed_traits::common::{
    Endian, ErrorKind as CommonErrorKind, ErrorType as CommonErrorType, FromBytes,
    SerdeError as CommonSerdeError, ToBytes,
};
use proposed_traits::rsa::{
    Error, ErrorKind, ErrorType as RsaErrorType, PaddingMode, RsaKeyGen, RsaKeys, RsaMessage,
    RsaSign, RsaSignature, RsaSize, RsaVerify,
};

const RSA_SRAM_BASE: usize = 0x7900_0000; // SBC base address

const SRAM_DST_EXPONENT: usize = 0x0;
const SRAM_DST_MODULUS: usize = 0x400;
const SRAM_DST_DATA: usize = 0x800;
const SRAM_DST_RESULT: usize = 0x1400;
const SRAM_SIZE: usize = 0x1800; // SRAM size for RSA operations

const RSA_MAX_LEN: usize = 0x400;

#[derive(Debug)]
pub enum RsaDriverError {
    InvalidLength,
    HardwareError,
    VerificationFailed,
}

impl Error for RsaDriverError {
    fn kind(&self) -> ErrorKind {
        match self {
            RsaDriverError::InvalidLength => ErrorKind::InvalidLength,
            RsaDriverError::HardwareError => ErrorKind::SignError,
            RsaDriverError::VerificationFailed => ErrorKind::VerifyError,
        }
    }
}

pub struct RsaPrivateKey<'a> {
    pub m: &'a [u8],
    pub d: &'a [u8],
    pub m_bits: u32,
    pub d_bits: u32,
}

pub struct RsaPublicKey<'a> {
    pub m: &'a [u8],
    pub e: &'a [u8],
    pub m_bits: u32,
    pub e_bits: u32,
}

pub struct RsaSignatureData {
    pub data: [u8; 512],
    pub len: usize,
}

pub struct RsaDigest {
    pub data: [u8; 64],
    pub len: usize,
}

#[derive(Debug)]
pub enum SignatureSerdeError {
    BufferTooSmall,
    Unsupported,
}

impl CommonSerdeError for SignatureSerdeError {
    fn kind(&self) -> CommonErrorKind {
        match self {
            SignatureSerdeError::BufferTooSmall => CommonErrorKind::SourceBufferTooSmall,
            SignatureSerdeError::Unsupported => CommonErrorKind::NotSupported,
        }
    }
}

impl CommonErrorType for RsaDigest {
    type Error = SignatureSerdeError;
}

impl ToBytes for RsaDigest {
    fn to_bytes(&self, dest: &mut [u8], _endian: Endian) -> Result<(), Self::Error> {
        if dest.len() < self.len {
            return Err(SignatureSerdeError::BufferTooSmall);
        }

        for (i, dest_item) in dest.iter_mut().enumerate().take(self.len) {
            *dest_item = self.data[self.len - i - 1];
        }

        Ok(())
    }
}

impl FromBytes for RsaDigest {
    fn from_bytes(bytes: &[u8], _endian: Endian) -> Result<Self, Self::Error> {
        if bytes.len() > 64 {
            return Err(SignatureSerdeError::BufferTooSmall);
        }

        let mut data = [0u8; 64];
        for (i, b) in bytes.iter().rev().enumerate() {
            data[i] = *b;
        }

        Ok(Self {
            data,
            len: bytes.len(),
        })
    }
}

impl ToBytes for RsaSignatureData {
    fn to_bytes(&self, dest: &mut [u8], _endian: Endian) -> Result<(), Self::Error> {
        if dest.len() < self.len {
            return Err(SignatureSerdeError::BufferTooSmall);
        }

        for (i, dest_item) in dest.iter_mut().enumerate().take(self.len) {
            *dest_item = self.data[self.len - i - 1];
        }

        Ok(())
    }
}

impl FromBytes for RsaSignatureData {
    fn from_bytes(bytes: &[u8], _endian: Endian) -> Result<Self, Self::Error> {
        if bytes.len() > 512 {
            return Err(SignatureSerdeError::BufferTooSmall);
        }
        let mut data = [0u8; 512];
        for (i, b) in bytes.iter().rev().enumerate() {
            data[i] = *b;
        }
        Ok(Self {
            data,
            len: bytes.len(),
        })
    }
}

impl CommonErrorType for RsaSignatureData {
    type Error = SignatureSerdeError;
}

#[derive(Debug)]
pub enum PaddingError {
    OutputTooSmall,
    UnsupportedDigest,
}

pub struct AspeedRsa<'a, D: DelayNs> {
    pub secure: &'a Secure,
    sram_base: NonNull<u8>,
    delay: D,
}

impl<'a, D: DelayNs> AspeedRsa<'a, D> {
    pub fn new(secure: &'a Secure, delay: D) -> Self {
        Self {
            secure,
            sram_base: unsafe { NonNull::new_unchecked(RSA_SRAM_BASE as *mut u8) },
            delay,
        }
    }

    pub fn pkcs1_v1_5_pad_inplace(digest: &[u8], out: &mut [u8]) -> Result<usize, PaddingError> {
        const DER_SHA256: &[u8] = &[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x05, 0x00, 0x04, 0x20,
        ];
        const DER_SHA384: &[u8] = &[
            0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x02, 0x05, 0x00, 0x04, 0x30,
        ];
        const DER_SHA512: &[u8] = &[
            0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x03, 0x05, 0x00, 0x04, 0x40,
        ];

        let (der, hash_len) = match digest.len() {
            32 => (DER_SHA256, 32),
            48 => (DER_SHA384, 48),
            64 => (DER_SHA512, 64),
            _ => return Err(PaddingError::UnsupportedDigest), // unsupported digest size
        };

        let t_len = der.len() + hash_len;
        let out_len = out.len();

        if out_len < t_len + 11 {
            return Err(PaddingError::OutputTooSmall);
        }

        let mut idx = 0;

        out[idx] = 0x00;
        idx += 1;
        out[idx] = 0x01;
        idx += 1;

        let ps_len = out_len - t_len - 3;
        for i in 0..ps_len {
            out[idx + i] = 0xff;
        }
        idx += ps_len;

        out[idx] = 0x00;
        idx += 1;

        out[idx..idx + der.len()].copy_from_slice(der);
        idx += der.len();

        out[idx..idx + digest.len()].copy_from_slice(digest);
        idx += digest.len();

        Ok(idx)
    }

    pub fn aspeed_rsa_trigger(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        m: &[u8],
        e_or_d: &[u8],
        m_bits: u32,
        ed_bits: u32,
    ) -> Result<usize, RsaDriverError> {
        if input.len() > 512 {
            return Err(RsaDriverError::InvalidLength);
        }

        unsafe {
            for i in 0..SRAM_SIZE {
                write_volatile(self.sram_base.as_ptr().add(i), 0);
            }

            for (i, byte) in e_or_d.iter().rev().enumerate() {
                // print byte
                write_volatile(self.sram_base.as_ptr().add(SRAM_DST_EXPONENT + i), *byte);
            }

            for (i, byte) in m.iter().rev().enumerate() {
                write_volatile(self.sram_base.as_ptr().add(SRAM_DST_MODULUS + i), *byte);
            }

            for (i, byte) in input.iter().rev().enumerate() {
                write_volatile(self.sram_base.as_ptr().add(SRAM_DST_DATA + i), *byte);
            }

            let key_len = (ed_bits << 16) | m_bits;
            self.secure.secure0b0().write(|w| w.bits(key_len));
            self.secure
                .secure0bc()
                .write(|w| w.sec_boot_rsaeng_trigger_reg().set_bit());
            self.secure
                .secure0bc()
                .write(|w| w.sec_boot_rsaeng_trigger_reg().clear_bit());

            let mut retry = 10000;
            loop {
                if self.secure.secure014().read().rsaeng_sts().bit_is_set() {
                    break;
                }
                retry -= 1;
                if retry == 0 {
                    return Err(RsaDriverError::HardwareError);
                }
                self.delay.delay_ns(10000);
            }

            let mut out_len = 0;
            let mut leading_zero = true;
            let mut i = 0;

            for j in (0..RSA_MAX_LEN).rev() {
                let byte = read_volatile(self.sram_base.as_ptr().add(SRAM_DST_RESULT + j));
                if byte == 0 && leading_zero {
                    continue;
                }
                leading_zero = false;
                if i < output.len() {
                    output[i] = byte as u8;
                    i += 1;
                }
                out_len += 1;
            }

            write_bytes(self.sram_base.as_ptr(), 0, SRAM_SIZE);

            Ok(out_len)
        }
    }
}

impl<D: DelayNs> RsaErrorType for AspeedRsa<'_, D> {
    type Error = RsaDriverError;
}

impl<D: DelayNs> RsaMessage for AspeedRsa<'_, D> {
    type Message = RsaDigest;
}

impl<'a, D: DelayNs> RsaKeys for AspeedRsa<'a, D> {
    type PrivateKey = RsaPrivateKey<'a>;
    type PublicKey = RsaPublicKey<'a>;
}

impl<D: DelayNs> RsaSignature for AspeedRsa<'_, D> {
    type Signature = RsaSignatureData;
}

impl<D: DelayNs> RsaKeyGen for AspeedRsa<'_, D> {
    fn generate_keys(_bits: RsaSize) -> Result<(Self::PrivateKey, Self::PublicKey), Self::Error> {
        // Not supported by hardware
        Err(RsaDriverError::HardwareError)
    }
}

impl<D: DelayNs> RsaSign for AspeedRsa<'_, D> {
    /// Performs RSA signature generation using PKCS#1 v1.5 padding and a private key.
    ///
    /// This function pads the input message digest using PKCS#1 v1.5, triggers the RSA engine
    /// with the private key components (modulus and private exponent), and writes the resulting
    /// signature into the output buffer.
    ///
    /// If the RSA engine returns a signature shorter than the modulus length (e.g. leading zeros
    /// are omitted), this function will right-align the signature and pad the high bytes with zeros
    /// to ensure a consistent `m_len`-byte big-endian representation.
    ///
    /// # Parameters
    /// - `private_key`: RSA private key containing modulus `m`, exponent `d`, and bit lengths
    /// - `message`: Pre-hashed message (digest) to be signed
    /// - `_padding_mode`: Currently ignored; only PKCS#1 v1.5 is supported
    ///
    /// # Returns
    /// A `RsaSignatureData` struct containing the fixed-length signature output
    /// and its length (always equal to the modulus length in bytes).
    ///
    /// # Errors
    /// Returns `RsaDriverError::InvalidLength` if the message or padding is malformed,
    /// or if the hardware RSA engine fails.
    fn sign(
        &mut self,
        private_key: &Self::PrivateKey,
        message: Self::Message,
        _padding_mode: PaddingMode,
    ) -> Result<Self::Signature, Self::Error> {
        let mut output = [0u8; 512];

        let m_len = private_key.m_bits.div_ceil(8) as usize;
        let d_len = private_key.d_bits.div_ceil(8) as usize;
        let input_len = message.len;

        let input = &message.data[..input_len];
        let m = &private_key.m[..m_len];
        let d = &private_key.d[..d_len];

        let mut padded_input = [0u8; 512];
        let padded_len = Self::pkcs1_v1_5_pad_inplace(input, &mut padded_input[..m_len])
            .map_err(|_e| RsaDriverError::InvalidLength)?;
        let len = self.aspeed_rsa_trigger(
            &padded_input[..padded_len],
            &mut output,
            m,
            d,
            private_key.m_bits,
            private_key.d_bits,
        )?;

        if len < m_len {
            // Hardware output is shorter than modulus length.
            // RSA signatures are represented as big-endian integers,
            // so if the hardware omits leading zeros (e.g., starts with 0x00),
            // we must right-align the result and pad the high bytes with 0.
            //
            // [    padding    |      actual data     ]
            // [0 .. m_len-len | m_len-len .. m_len   ]
            output.copy_within(0..len, m_len - len);
            output[..m_len - len].fill(0);
        }

        Ok(RsaSignatureData {
            data: output,
            len: m_len,
        })
    }
}

impl<D: DelayNs> RsaVerify for AspeedRsa<'_, D> {
    /// Verifies an RSA signature using the provided public key and digest.
    ///
    /// This function performs RSA public-key decryption (i.e., modular exponentiation)
    /// on the input signature using the public modulus `m` and exponent `e`, then
    /// compares the result against the expected digest (`message`).
    ///
    /// The verification is successful if the tail end of the decrypted output
    /// matches the provided digest byte-for-byte. This assumes that the signature
    /// follows the PKCS#1 v1.5 padding convention.
    ///
    /// # Arguments
    /// - `public_key`: RSA public key, including modulus and exponent
    /// - `message`: Digest value expected to be embedded in the signature
    /// - `_padding_mode`: Padding mode placeholder (currently unused)
    /// - `signature`: The RSA signature to be verified
    ///
    /// # Returns
    /// - `Ok(signature)` if verification succeeds
    /// - `Err(RsaDriverError::VerificationFailed)` if digest mismatch
    ///
    /// # Notes
    /// - The implementation uses a fixed-size internal buffer (512 bytes) for output.
    fn verify(
        &mut self,
        public_key: &Self::PublicKey,
        message: Self::Message,
        _padding_mode: PaddingMode,
        signature: &Self::Signature,
    ) -> Result<Self::Signature, Self::Error> {
        let mut output = [0u8; 512];

        let input_len = signature.len;
        let e_len = public_key.e_bits.div_ceil(8) as usize;
        let m_len = public_key.m_bits.div_ceil(8) as usize;

        let input = &signature.data[..input_len];
        let m = &public_key.m[..m_len];
        let e = &public_key.e[..e_len];

        let len = self.aspeed_rsa_trigger(
            input,
            &mut output,
            m,
            e,
            public_key.m_bits,
            public_key.e_bits,
        )?;

        if output[len.saturating_sub(message.len)..len] == message.data[..message.len] {
            Ok(RsaSignatureData {
                data: signature.data,
                len,
            })
        } else {
            Err(RsaDriverError::VerificationFailed)
        }
    }
}
