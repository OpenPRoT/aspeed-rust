// Licensed under the Apache-2.0 license

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum I2cSpeed {
    Standard = 100_000,
    Fast = 400_000,
    FastPlus = 1_000_000,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum I2cXferMode {
    DmaMode,
    BuffMode,
    ByteMode,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum I2cSEvent {
    SlaveRdReq,
    SlaveWrReq,
    SlaveRdProc,
    SlaveWrRecvd,
    SlaveStop,
}

pub struct TimingConfig {
    pub manual_scl_high: u8,
    pub manual_scl_low: u8,
    pub manual_sda_hold: u8,
    pub clk_src: u32,
}
pub struct I2cConfig {
    pub xfer_mode: I2cXferMode,
    pub multi_master: bool,
    pub smbus_timeout: bool,
    pub smbus_alert: bool,
    pub timing_config: TimingConfig,
    pub speed: I2cSpeed,
}
pub struct I2cConfigBuilder {
    xfer_mode: I2cXferMode,
    multi_master: bool,
    smbus_timeout: bool,
    smbus_alert: bool,
    timing_config: Option<TimingConfig>,
    speed: I2cSpeed,
}
impl Default for I2cConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl I2cConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            xfer_mode: I2cXferMode::ByteMode,
            multi_master: false,
            smbus_alert: false,
            smbus_timeout: false,
            timing_config: None,
            speed: I2cSpeed::Standard,
        }
    }
    #[must_use]
    pub fn xfer_mode(mut self, mode: I2cXferMode) -> Self {
        self.xfer_mode = mode;
        self
    }
    #[must_use]
    pub fn multi_master(mut self, enabled: bool) -> Self {
        self.multi_master = enabled;
        self
    }
    #[must_use]
    pub fn smbus_alert(mut self, enabled: bool) -> Self {
        self.smbus_alert = enabled;
        self
    }
    #[must_use]
    pub fn smbus_timeout(mut self, enabled: bool) -> Self {
        self.smbus_timeout = enabled;
        self
    }
    #[must_use]
    pub fn speed(mut self, speed: I2cSpeed) -> Self {
        self.speed = speed;
        self
    }
    #[must_use]
    pub fn timing_config(mut self, config: TimingConfig) -> Self {
        self.timing_config = Some(config);
        self
    }
    #[must_use]
    pub fn build(self) -> I2cConfig {
        I2cConfig {
            xfer_mode: self.xfer_mode,
            multi_master: self.multi_master,
            smbus_timeout: self.smbus_timeout,
            smbus_alert: self.smbus_alert,
            timing_config: self.timing_config.unwrap_or(TimingConfig {
                manual_scl_high: 0,
                manual_scl_low: 0,
                manual_sda_hold: 0,
                clk_src: 0,
            }),
            speed: self.speed,
        }
    }
}
