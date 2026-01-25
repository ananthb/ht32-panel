//! Display orientation support.
//!
//! The hardware only supports two orientations (landscape 0x01 and portrait 0x02).
//! Upside-down variants are achieved through software rotation of the framebuffer.

use crate::{Error, Result, LCD_HEIGHT, LCD_WIDTH};
use std::str::FromStr;

/// Display orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Landscape mode (320x170), hardware native.
    #[default]
    Landscape,
    /// Portrait mode (170x320), hardware native.
    Portrait,
    /// Landscape upside-down (320x170), software rotated 180°.
    LandscapeUpsideDown,
    /// Portrait upside-down (170x320), software rotated 180°.
    PortraitUpsideDown,
}

impl Orientation {
    /// Returns the hardware orientation byte.
    pub fn hardware_byte(&self) -> u8 {
        match self {
            Orientation::Landscape | Orientation::LandscapeUpsideDown => 0x01,
            Orientation::Portrait | Orientation::PortraitUpsideDown => 0x02,
        }
    }

    /// Returns true if this orientation requires software rotation.
    pub fn needs_rotation(&self) -> bool {
        matches!(
            self,
            Orientation::LandscapeUpsideDown | Orientation::PortraitUpsideDown
        )
    }

    /// Returns true if this is a portrait orientation.
    pub fn is_portrait(&self) -> bool {
        matches!(
            self,
            Orientation::Portrait | Orientation::PortraitUpsideDown
        )
    }

    /// Returns the display dimensions for this orientation.
    pub fn dimensions(&self) -> (u16, u16) {
        if self.is_portrait() {
            (LCD_HEIGHT, LCD_WIDTH)
        } else {
            (LCD_WIDTH, LCD_HEIGHT)
        }
    }

    /// Rotate a buffer 180 degrees in place.
    pub fn rotate_180(buffer: &mut [u16], width: u16, height: u16) {
        let len = buffer.len();
        for i in 0..len / 2 {
            buffer.swap(i, len - 1 - i);
        }
        // Also need to swap bytes within each row for proper rotation
        let w = width as usize;
        let h = height as usize;
        for y in 0..h / 2 {
            for x in 0..w {
                let idx1 = y * w + x;
                let idx2 = (h - 1 - y) * w + (w - 1 - x);
                buffer.swap(idx1, idx2);
            }
        }
        // Handle middle row for odd height
        if h % 2 == 1 {
            let mid_y = h / 2;
            for x in 0..w / 2 {
                let idx1 = mid_y * w + x;
                let idx2 = mid_y * w + (w - 1 - x);
                buffer.swap(idx1, idx2);
            }
        }
    }
}

impl FromStr for Orientation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "landscape" => Ok(Orientation::Landscape),
            "portrait" => Ok(Orientation::Portrait),
            "landscape-upside-down" | "landscape_upside_down" => {
                Ok(Orientation::LandscapeUpsideDown)
            }
            "portrait-upside-down" | "portrait_upside_down" => Ok(Orientation::PortraitUpsideDown),
            _ => Err(Error::InvalidOrientation(s.to_string())),
        }
    }
}

impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Orientation::Landscape => write!(f, "landscape"),
            Orientation::Portrait => write!(f, "portrait"),
            Orientation::LandscapeUpsideDown => write!(f, "landscape-upside-down"),
            Orientation::PortraitUpsideDown => write!(f, "portrait-upside-down"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_byte() {
        assert_eq!(Orientation::Landscape.hardware_byte(), 0x01);
        assert_eq!(Orientation::Portrait.hardware_byte(), 0x02);
        assert_eq!(Orientation::LandscapeUpsideDown.hardware_byte(), 0x01);
        assert_eq!(Orientation::PortraitUpsideDown.hardware_byte(), 0x02);
    }

    #[test]
    fn test_needs_rotation() {
        assert!(!Orientation::Landscape.needs_rotation());
        assert!(!Orientation::Portrait.needs_rotation());
        assert!(Orientation::LandscapeUpsideDown.needs_rotation());
        assert!(Orientation::PortraitUpsideDown.needs_rotation());
    }

    #[test]
    fn test_dimensions() {
        assert_eq!(Orientation::Landscape.dimensions(), (320, 170));
        assert_eq!(Orientation::Portrait.dimensions(), (170, 320));
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "landscape".parse::<Orientation>().unwrap(),
            Orientation::Landscape
        );
        assert_eq!(
            "portrait".parse::<Orientation>().unwrap(),
            Orientation::Portrait
        );
        assert_eq!(
            "landscape-upside-down".parse::<Orientation>().unwrap(),
            Orientation::LandscapeUpsideDown
        );
    }
}
