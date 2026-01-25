//! LCD display module.
//!
//! Provides control over the 320x170 RGB565 LCD display via USB HID.

mod device;
mod protocol;

pub mod framebuffer;

pub use device::LcdDevice;
pub use framebuffer::{parse_hex_color, rgb565_to_rgb888, rgb888_to_rgb565, Framebuffer};
pub use protocol::{Command, SubCommand};
