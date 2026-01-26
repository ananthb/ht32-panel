//! LED strip device communication via serial port.

use crate::{Error, Result};
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use tokio_serial::{DataBits, Parity, SerialPortBuilderExt, StopBits};
use tracing::{debug, info};

/// LED signature byte.
const SIGNATURE_BYTE: u8 = 0xFA;

/// LED baud rate.
const BAUD_RATE: u32 = 10000;

/// LED theme options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LedTheme {
    /// Rainbow cycling effect.
    Rainbow = 0x01,
    /// Breathing/pulsing effect.
    #[default]
    Breathing = 0x02,
    /// Solid colors cycling.
    Colors = 0x03,
    /// LEDs off.
    Off = 0x04,
    /// Automatic mode.
    Auto = 0x05,
}

impl LedTheme {
    /// Converts a byte value to LedTheme.
    pub fn from_byte(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(LedTheme::Rainbow),
            0x02 => Ok(LedTheme::Breathing),
            0x03 => Ok(LedTheme::Colors),
            0x04 => Ok(LedTheme::Off),
            0x05 => Ok(LedTheme::Auto),
            _ => Err(Error::InvalidTheme(value)),
        }
    }
}

impl FromStr for LedTheme {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "rainbow" => Ok(LedTheme::Rainbow),
            "breathing" => Ok(LedTheme::Breathing),
            "colors" => Ok(LedTheme::Colors),
            "off" => Ok(LedTheme::Off),
            "auto" => Ok(LedTheme::Auto),
            _ => Err(Error::InvalidTheme(0)),
        }
    }
}

impl std::fmt::Display for LedTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedTheme::Rainbow => write!(f, "rainbow"),
            LedTheme::Breathing => write!(f, "breathing"),
            LedTheme::Colors => write!(f, "colors"),
            LedTheme::Off => write!(f, "off"),
            LedTheme::Auto => write!(f, "auto"),
        }
    }
}

/// LED strip device controller.
pub struct LedDevice {
    port_path: String,
}

impl LedDevice {
    /// Creates a new LED device controller.
    pub fn new(port_path: &str) -> Self {
        Self {
            port_path: port_path.to_string(),
        }
    }

    /// Opens the serial port and sends data.
    async fn send_packet(&self, packet: [u8; 5]) -> Result<()> {
        let mut port = tokio_serial::new(&self.port_path, BAUD_RATE)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .open_native_async()
            .map_err(|e| {
                // Check if the error is due to device not existing
                if let tokio_serial::ErrorKind::Io(kind) = &e.kind {
                    if *kind == std::io::ErrorKind::NotFound
                        || *kind == std::io::ErrorKind::PermissionDenied
                    {
                        // Check if file actually exists
                        if !std::path::Path::new(&self.port_path).exists() {
                            return Error::LedNotFound(self.port_path.clone());
                        }
                    }
                }
                Error::Serial(e)
            })?;

        debug!("Sending LED packet to {}: {:02X?}", self.port_path, packet);

        // Write all bytes at once, then flush
        port.write_all(&packet).await?;
        port.flush().await?;

        debug!("LED packet sent successfully");
        Ok(())
    }

    /// Fixes the intensity/speed value (inversion: value = 6 - input).
    fn fix_value(value: u8) -> Result<u8> {
        if !(1..=5).contains(&value) {
            return Err(Error::InvalidLedValue(value));
        }
        Ok(6 - value)
    }

    /// Calculates checksum for the packet.
    fn checksum(packet: &[u8; 4]) -> u8 {
        packet.iter().fold(0u8, |acc, &x| acc.wrapping_add(x))
    }

    /// Builds a 5-byte LED packet.
    fn build_packet(theme: LedTheme, intensity: u8, speed: u8) -> Result<[u8; 5]> {
        let fixed_intensity = Self::fix_value(intensity)?;
        let fixed_speed = Self::fix_value(speed)?;

        let base = [SIGNATURE_BYTE, theme as u8, fixed_intensity, fixed_speed];
        let checksum = Self::checksum(&base);

        Ok([base[0], base[1], base[2], base[3], checksum])
    }

    /// Sets the LED theme with intensity and speed.
    pub async fn set_theme(&self, theme: LedTheme, intensity: u8, speed: u8) -> Result<()> {
        let packet = Self::build_packet(theme, intensity, speed)?;
        self.send_packet(packet).await?;
        info!(
            "LED set to {} (intensity: {}, speed: {})",
            theme, intensity, speed
        );
        Ok(())
    }

    /// Sets rainbow effect.
    pub async fn set_rainbow(&self, intensity: u8, speed: u8) -> Result<()> {
        self.set_theme(LedTheme::Rainbow, intensity, speed).await
    }

    /// Sets breathing effect.
    pub async fn set_breathing(&self, intensity: u8, speed: u8) -> Result<()> {
        self.set_theme(LedTheme::Breathing, intensity, speed).await
    }

    /// Sets colors effect.
    pub async fn set_colors(&self, intensity: u8, speed: u8) -> Result<()> {
        self.set_theme(LedTheme::Colors, intensity, speed).await
    }

    /// Sets auto mode.
    pub async fn set_auto(&self, intensity: u8, speed: u8) -> Result<()> {
        self.set_theme(LedTheme::Auto, intensity, speed).await
    }

    /// Turns off the LEDs.
    pub async fn set_off(&self) -> Result<()> {
        // Off uses fixed values of 5 (which become 1 after inversion)
        let packet = [SIGNATURE_BYTE, LedTheme::Off as u8, 0x05, 0x05, 0x00];
        let checksum = Self::checksum(&[packet[0], packet[1], packet[2], packet[3]]);
        let packet_with_checksum = [packet[0], packet[1], packet[2], packet[3], checksum];
        self.send_packet(packet_with_checksum).await?;
        info!("LED turned off");
        Ok(())
    }

    /// Returns the port path.
    pub fn port_path(&self) -> &str {
        &self.port_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_value() {
        assert_eq!(LedDevice::fix_value(1).unwrap(), 5);
        assert_eq!(LedDevice::fix_value(3).unwrap(), 3);
        assert_eq!(LedDevice::fix_value(5).unwrap(), 1);
        assert!(LedDevice::fix_value(0).is_err());
        assert!(LedDevice::fix_value(6).is_err());
    }

    #[test]
    fn test_checksum() {
        let packet = [0xFA, 0x01, 0x03, 0x03];
        let sum = LedDevice::checksum(&packet);
        // 0xFA + 0x01 + 0x03 + 0x03 = 0x101, wraps to 0x01
        assert_eq!(sum, 0x01);
    }

    #[test]
    fn test_build_packet() {
        let packet = LedDevice::build_packet(LedTheme::Rainbow, 3, 3).unwrap();
        assert_eq!(packet[0], 0xFA);
        assert_eq!(packet[1], 0x01); // Rainbow
        assert_eq!(packet[2], 0x03); // 6 - 3 = 3
        assert_eq!(packet[3], 0x03); // 6 - 3 = 3
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!("rainbow".parse::<LedTheme>().unwrap(), LedTheme::Rainbow);
        assert_eq!(
            "breathing".parse::<LedTheme>().unwrap(),
            LedTheme::Breathing
        );
        assert_eq!("off".parse::<LedTheme>().unwrap(), LedTheme::Off);
    }
}
