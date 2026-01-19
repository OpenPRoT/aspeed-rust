// Licensed under the Apache-2.0 license

use crate::{
    common::Logger,
    otp::{
        common::{
            AspeedChipVersion, AspeedOtpRegion, OtpError, ProtectionStatus, SessionInfo,
            StrapStatus,
        },
        OtpController, OTP_MEM_LIMIT,
    },
};
use proposed_traits::otp::{
    OtpIdentification, OtpMemory, OtpMemoryLayout, OtpProtection, OtpRegions, OtpSession,
};

impl<L: Logger> OtpMemory<u32> for OtpController<L> {
    ///
    /// Read 1 DWORD
    ///
    fn read(&self, address: usize) -> Result<u32, Self::Error> {
        let mut buffer: [u32; 1] = [0];
        self.read_region(AspeedOtpRegion::Data, address, &mut buffer)?;
        Ok(buffer[0])
    }
    fn write(&mut self, address: usize, data: u32) -> Result<(), Self::Error> {
        let ignore_mask: [u32; 2] = [0, 0xffff_ffff];
        let mut buffer: [u32; 2] = [0, 0];
        let mut otp_data: [u32; 2] = [0, 0];
        if self.locked {
            return Err(OtpError::MemoryLocked);
        }
        if address > OTP_MEM_LIMIT as usize {
            return Err(OtpError::InvalidAddress);
        }
        otp_data[0] = self.read(address)?;
        buffer[0] = data;
        self.otp_prog_verify_2dw(
            u32::try_from(address).unwrap(),
            &otp_data,
            &buffer,
            &ignore_mask,
        )
    }

    fn lock(&mut self) -> Result<(), Self::Error> {
        self.otp_lock_mem()
    }

    fn is_locked(&self) -> bool {
        self.is_otp_locked()
    }
}

impl<L: Logger> OtpSession for OtpController<L> {
    type SessionInfo = SessionInfo;
    /// Session information type returned when establishing a session
    /// Establish an OTP session with hardware access
    ///
    /// # Returns
    /// - `Ok(SessionInfo)`: Session established successfully
    /// - `Err(Self::Error)`: Failed to establish session
    fn begin_session(&mut self) -> Result<Self::SessionInfo, Self::Error> {
        if self.session_active {
            return Err(OtpError::NoSession);
        }
        let ver_bytes = self.get_tool_verion();
        let mut session_info = SessionInfo {
            chip_version: AspeedChipVersion::Ast1060A2,
            version_name: *b"AST1060A2\0",
            protection_status: ProtectionStatus {
                memory_locked: false,
                key_protected: false,
                strap_protected: false,
                config_protected: false,
                user_ecc_protected: false,
                security_protected: false,
                security_size: 0,
            },
            tool_version: [0; 32],
            software_revision: 0x1234_5678,
            key_count: 5,
        };
        if ver_bytes.len() > session_info.tool_version.len() {
            return Err(OtpError::BoundaryError);
        }
        self.update_prot_info(&mut session_info);
        session_info.chip_version = self.chip_version();
        session_info.tool_version[..ver_bytes.len()].copy_from_slice(ver_bytes);
        session_info.key_count = u32::from(self.get_key_count());

        self.session_active = true;

        Ok(session_info)
    }

    /// Terminate the OTP session and release resources
    ///
    /// # Returns
    /// - `Ok(())`: Session terminated successfully
    /// - `Err(Self::Error)`: Failed to terminate session properly
    fn end_session(&mut self) -> Result<(), Self::Error> {
        if !self.session_active {
            return Err(OtpError::NoSession);
        }
        self.session_active = false;
        Ok(())
    }

    /// Check if a session is currently active
    fn is_session_active(&self) -> bool {
        self.session_active
    }
}

impl<L: Logger> OtpRegions<u32> for OtpController<L> {
    /// Region identifier type
    type Region = AspeedOtpRegion;

    /// Read data from a specific OTP region
    ///
    /// # Parameters
    /// - `region`: The region to read from
    /// - `offset`: Offset within the region. For strap, it's `strap_bit_offset`
    /// - `buffer`: Buffer to store read data
    ///
    /// # Returns
    /// - `Ok(())`: Data read successfully
    /// - `Err(Self::Error)`: Read operation failed
    #[allow(clippy::needless_range_loop)]
    fn read_region(
        &self,
        region: Self::Region,
        offset: usize,
        buffer: &mut [u32],
    ) -> Result<(), Self::Error> {
        match region {
            AspeedOtpRegion::Data => self.aspeed_otp_read_data(offset, buffer),
            AspeedOtpRegion::Configuration => {
                self.aspeed_otp_read_conf(u32::try_from(offset).unwrap(), buffer)
            }
            AspeedOtpRegion::Strap => {
                if buffer.len() < 2 {
                    return Err(OtpError::InvalidBufSize);
                }
                let mut strap_status: [StrapStatus; 64] = [StrapStatus {
                    value: false,
                    protected: false,
                    options: [0; 7],
                    remaining_writes: 6,
                    writable_option: 0xff,
                }; 64];
                match self.otp_strap_status(&mut strap_status) {
                    Ok(()) => {
                        buffer[0] = 0;
                        for i in 0usize..32 {
                            buffer[0] |= u32::from(strap_status[i].value) << i;
                        }
                        buffer[1] = 0;
                        for i in 32usize..64 {
                            buffer[1] |= u32::from(strap_status[i].value) << (i - 32);
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            AspeedOtpRegion::ScuProtection => self.aspeed_otp_read_scuprot(offset, buffer),
        }
    }

    /// Write data to a specific OTP region
    ///
    /// # Parameters
    /// - `region`: The region to write to
    /// - `offset`: Offset within the region. For strap, it's `strap_bit_offset`
    /// - `data`: Data to write
    ///
    /// # Returns
    /// - `Ok(())`: Data written successfully
    /// - `Err(Self::Error)`: Write operation failed
    fn write_region(
        &mut self,
        region: Self::Region,
        offset: usize,
        data: &[u32],
    ) -> Result<(), Self::Error> {
        match region {
            AspeedOtpRegion::Data => self.otp_prog_data(offset, data),
            AspeedOtpRegion::Configuration => self.otp_prog_conf(offset, data),
            AspeedOtpRegion::Strap => self.otp_prog_strap(offset, data),
            AspeedOtpRegion::ScuProtection => self.otp_prog_scu_protect(offset, data),
        }
    }
    /// Get the capacity of a specific region
    ///
    /// # Parameters
    /// - `region`: The region to query
    ///
    /// # Returns
    /// The capacity of the region in elements of type T
    fn region_capacity(&self, region: Self::Region) -> usize {
        self.region_capacity(region)
    }
    /// Get the alignment requirement for a specific region
    ///
    /// # Parameters
    /// - `region`: The region to query
    ///
    /// # Returns
    /// The alignment requirement in bytes
    fn region_alignment(&self, region: Self::Region) -> usize {
        self.region_alignment(region)
    }
}

impl<L: Logger> OtpProtection for OtpController<L> {
    type Region = AspeedOtpRegion;

    /// Check if a specific region is protected
    ///
    /// # Parameters
    /// - `region`: The region to check
    ///
    /// # Returns
    /// - `Ok(bool)`: Protection status (true = protected)
    /// - `Err(Self::Error)`: Failed to check protection status
    fn is_region_protected(&self, region: Self::Region) -> Result<bool, Self::Error> {
        self.is_region_protected(region)
    }

    /// Enable protection for a specific region
    ///
    /// # Parameters
    /// - `region`: The region to protect
    ///
    /// # Returns
    /// - `Ok(())`: Protection enabled successfully
    /// - `Err(Self::Error)`: Failed to enable protection
    fn enable_region_protection(&mut self, region: Self::Region) -> Result<(), Self::Error> {
        self.enable_region_protection(region)
    }

    /// Check if the entire memory is globally locked
    ///
    /// # Returns
    /// - `Ok(bool)`: Lock status (true = locked)
    /// - `Err(Self::Error)`: Failed to check lock status
    fn is_globally_locked(&self) -> Result<bool, Self::Error> {
        Ok(self.is_otp_locked())
    }

    /// Enable global memory lock (typically irreversible)
    ///
    /// This operation permanently locks all OTP regions and usually cannot be undone.
    /// Use with extreme caution.
    ///
    /// # Returns
    /// - `Ok(())`: Global lock enabled successfully
    /// - `Err(Self::Error)`: Failed to enable global lock
    fn enable_global_lock(&mut self) -> Result<(), Self::Error> {
        self.otp_lock_mem()
    }
}

impl<L: Logger> OtpIdentification for OtpController<L> {
    /// Chip version or identifier type
    type ChipVersion = AspeedChipVersion;

    /// Get the chip version or hardware identifier
    ///
    /// # Returns
    /// - `Ok(ChipVersion)`: Hardware version information
    /// - `Err(Self::Error)`: Failed to read chip identification
    fn get_chip_version(&self) -> Result<Self::ChipVersion, Self::Error> {
        Ok(self.chip_version())
    }

    /// Check if a specific feature is supported by this chip version
    ///
    /// # Parameters
    /// - `feature`: Feature identifier to check
    ///
    /// # Returns
    /// - `Ok(bool)`: Feature support status (true = supported)
    /// - `Err(Self::Error)`: Failed to check feature support
    fn is_feature_supported(&self, feature: &str) -> Result<bool, Self::Error> {
        self.is_feature_supported(feature)
    }
}

impl<L: Logger> OtpMemoryLayout for OtpController<L> {
    /// Region identifier type
    type Region = AspeedOtpRegion;

    /// Get the total memory capacity in bytes
    fn total_capacity(&self) -> usize {
        self.total_capacity()
    }

    /// Get the minimum alignment requirement for write operations
    ///
    /// # Returns
    /// Alignment requirement in bytes (e.g., 1, 4, 8)
    fn write_alignment(&self) -> usize {
        4
    }

    /// Get the size of the minimum programmable unit
    ///
    /// # Returns
    /// Size in bytes of the smallest unit that can be programmed independently
    fn programming_granularity(&self) -> usize {
        4
    }

    /// List all available memory regions
    ///
    /// Returns an iterator over available regions. The exact collection type
    /// depends on the implementation (could be array, slice, or heap-allocated).
    ///
    /// # Returns
    /// - `Ok(regions)`: Iterator over available regions
    /// - `Err(Self::Error)`: Failed to enumerate regions
    fn list_regions(&self) -> Result<&[Self::Region], Self::Error> {
        self.list_regions()
    }

    /// Get detailed information about a specific region
    ///
    /// # Parameters
    /// - `region`: The region to query
    ///
    /// # Returns
    /// - `Ok((start_addr, size, alignment))`: Region details
    /// - `Err(Self::Error)`: Failed to get region information
    fn get_region_info(&self, region: Self::Region) -> Result<(usize, usize, usize), Self::Error> {
        self.get_region_info(region)
    }
}
