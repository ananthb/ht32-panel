//! HT32 Panel Hardware Library
//!
//! Provides hardware abstraction for LCD and LED control on HT32-based
//! mini PC display panels (AceMagic, Agni, and other whitelabeled devices).

pub mod error;
pub mod lcd;
pub mod led;
pub mod orientation;

pub use error::{Error, Result};
pub use lcd::{Framebuffer, LcdDevice};
pub use led::{LedDevice, LedTheme};
pub use orientation::Orientation;

/// LCD display dimensions
pub const LCD_WIDTH: u16 = 320;
pub const LCD_HEIGHT: u16 = 170;

/// USB VID:PID for the LCD device
pub const LCD_VID: u16 = 0x04D9;
pub const LCD_PID: u16 = 0xFD01;
