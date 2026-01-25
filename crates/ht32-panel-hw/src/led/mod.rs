//! LED strip module.
//!
//! Provides control over the LED strip via serial (CH340).

mod device;

pub use device::{LedDevice, LedTheme};
