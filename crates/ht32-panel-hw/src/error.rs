//! Error types for the HT32 Panel hardware library.

use thiserror::Error;

/// Result type alias using our Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when interacting with the hardware.
#[derive(Error, Debug)]
pub enum Error {
    /// LCD device not found or could not be opened.
    #[error("LCD device not found (VID:PID 04D9:FD01)")]
    LcdNotFound,

    /// LED device not found or could not be opened.
    #[error("LED device not found at {0}")]
    LedNotFound(String),

    /// USB HID communication error.
    #[error("USB HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    /// Serial port communication error.
    #[error("Serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),

    /// Serial I/O error.
    #[error("Serial I/O error: {0}")]
    SerialIo(#[from] std::io::Error),

    /// Invalid orientation value.
    #[error("Invalid orientation: {0}")]
    InvalidOrientation(String),

    /// Invalid LED theme value.
    #[error("Invalid LED theme: {0}")]
    InvalidTheme(u8),

    /// Invalid LED intensity or speed value.
    #[error("Invalid LED value (must be 1-5): {0}")]
    InvalidLedValue(u8),

    /// Framebuffer size mismatch.
    #[error("Framebuffer size mismatch: expected {expected}, got {actual}")]
    FramebufferSize { expected: usize, actual: usize },

    /// Image processing error.
    #[error("Image error: {0}")]
    Image(String),
}
