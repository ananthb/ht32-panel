//! LCD device communication via USB HID.

use crate::orientation::Orientation;
use crate::{Error, Result, LCD_PID, LCD_VID};
use hidapi::{HidApi, HidDevice};
use std::sync::Mutex;
use tracing::{debug, info};

use super::framebuffer::Framebuffer;
use super::protocol::{
    build_heartbeat_packet, build_orientation_packet, build_redraw_chunk, build_refresh_packet,
    CHUNK_COUNT, DATA_SIZE,
};

/// LCD device controller.
pub struct LcdDevice {
    device: Mutex<HidDevice>,
    current_orientation: Mutex<Orientation>,
}

/// The HID interface number used for LCD data transfer.
/// The device has multiple interfaces; interface 0 appears to be for display control.
/// Reference implementation uses interface 1.1 (config 1, interface 1).
const LCD_INTERFACE: i32 = 0;

impl LcdDevice {
    /// Opens the LCD device by VID:PID.
    ///
    /// The device has multiple HID interfaces. This function finds and opens
    /// the correct interface for display control (interface 2).
    pub fn open() -> Result<Self> {
        let api = HidApi::new()?;

        // Enumerate all devices to find the correct interface
        let devices: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == LCD_VID && d.product_id() == LCD_PID)
            .collect();

        if devices.is_empty() {
            return Err(Error::LcdNotFound);
        }

        // Log all found interfaces for debugging
        for dev in &devices {
            debug!(
                "Found HID device: path={:?}, interface={}",
                dev.path(),
                dev.interface_number()
            );
        }

        // Find the interface we need (interface 2 for display data)
        let device_info = devices
            .iter()
            .find(|d| d.interface_number() == LCD_INTERFACE)
            .or_else(|| devices.first()) // Fallback to first device if interface 2 not found
            .ok_or(Error::LcdNotFound)?;

        let device = device_info.open_device(&api).map_err(|e| {
            debug!("Failed to open device: {}", e);
            Error::LcdNotFound
        })?;

        info!(
            "LCD device opened (VID:{:04X} PID:{:04X}, interface={})",
            LCD_VID,
            LCD_PID,
            device_info.interface_number()
        );

        // Initial cooldown period - device needs time to initialize after opening
        // Reference implementation uses 1000ms delay before any commands
        debug!("Waiting for device initialization (1s cooldown)...");
        std::thread::sleep(std::time::Duration::from_millis(1000));

        Ok(Self {
            device: Mutex::new(device),
            current_orientation: Mutex::new(Orientation::default()),
        })
    }

    /// Opens a specific LCD device by path.
    pub fn open_path(path: &str) -> Result<Self> {
        let api = HidApi::new()?;

        let device = api
            .open_path(std::ffi::CString::new(path).unwrap().as_c_str())
            .map_err(|_| Error::LcdNotFound)?;

        info!("LCD device opened at path: {}", path);

        Ok(Self {
            device: Mutex::new(device),
            current_orientation: Mutex::new(Orientation::default()),
        })
    }

    /// Sets the display orientation.
    pub fn set_orientation(&self, orientation: Orientation) -> Result<()> {
        let packet = build_orientation_packet(orientation.is_portrait());

        let device = self.device.lock().unwrap();
        device.write(&packet)?;

        *self.current_orientation.lock().unwrap() = orientation;
        debug!("Set orientation to {}", orientation);

        Ok(())
    }

    /// Gets the current orientation.
    pub fn orientation(&self) -> Orientation {
        *self.current_orientation.lock().unwrap()
    }

    /// Sends a heartbeat with explicit time values.
    pub fn heartbeat_with_time(&self, hours: u8, minutes: u8, seconds: u8) -> Result<()> {
        let packet = build_heartbeat_packet(hours, minutes, seconds);

        let device = self.device.lock().unwrap();
        device.write(&packet)?;
        debug!("Heartbeat sent: {:02}:{:02}:{:02}", hours, minutes, seconds);

        Ok(())
    }

    /// Performs a full screen redraw.
    pub fn redraw(&self, framebuffer: &Framebuffer) -> Result<()> {
        let orientation = *self.current_orientation.lock().unwrap();
        let mut data = framebuffer.data().to_vec();

        // Apply software rotation if needed
        if orientation.needs_rotation() {
            Orientation::rotate_180(&mut data, framebuffer.width(), framebuffer.height());
        }

        let device = self.device.lock().unwrap();

        for chunk_idx in 0..CHUNK_COUNT {
            let offset = chunk_idx * (DATA_SIZE / 2);
            let packet = build_redraw_chunk(chunk_idx, &data, offset);
            device.write(&packet)?;
        }

        debug!("Full redraw completed ({} chunks)", CHUNK_COUNT);
        Ok(())
    }

    /// Performs a partial refresh of a rectangular region.
    pub fn refresh(&self, x: u16, y: u16, width: u8, height: u8, pixels: &[u16]) -> Result<()> {
        let orientation = *self.current_orientation.lock().unwrap();
        let mut data = pixels.to_vec();

        // Apply software rotation if needed
        if orientation.needs_rotation() {
            Orientation::rotate_180(&mut data, width as u16, height as u16);
        }

        let packet = build_refresh_packet(x, y, width, height, &data);

        let device = self.device.lock().unwrap();
        device.write(&packet)?;

        debug!("Partial refresh at ({}, {}) {}x{}", x, y, width, height);
        Ok(())
    }

    /// Clears the display to a solid color.
    pub fn clear(&self, color: u16) -> Result<()> {
        let mut fb = Framebuffer::new();
        fb.clear(color);
        self.redraw(&fb)
    }

    /// Sends a heartbeat to keep the device alive using system time.
    pub fn heartbeat(&self) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();

        // Simple time extraction (UTC)
        let hours = ((secs / 3600) % 24) as u8;
        let minutes = ((secs / 60) % 60) as u8;
        let seconds = (secs % 60) as u8;

        self.heartbeat_with_time(hours, minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hardware tests are skipped by default
    #[test]
    #[ignore]
    fn test_device_open() {
        let device = LcdDevice::open();
        assert!(device.is_ok());
    }
}
