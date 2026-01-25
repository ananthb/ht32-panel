//! Application state management.

#![allow(dead_code, unused_imports)]

use anyhow::{Context, Result};
use ht32_panel_hw::{
    lcd::{Framebuffer, LcdDevice},
    led::{LedDevice, LedTheme},
    Orientation,
};
use std::sync::{Mutex, RwLock};
use tracing::{debug, info};

use crate::config::Config;
use crate::rendering::Canvas;

/// Shared application state.
pub struct AppState {
    /// Configuration
    config: RwLock<Config>,

    /// LCD device (optional - may not be present)
    lcd: Option<Mutex<LcdDevice>>,

    /// LED device path
    led_device_path: String,

    /// Current orientation
    orientation: RwLock<Orientation>,

    /// Render canvas
    canvas: RwLock<Canvas>,

    /// Output framebuffer
    framebuffer: RwLock<Framebuffer>,

    /// Flag indicating a redraw is needed
    needs_redraw: RwLock<bool>,

    /// Current LED settings
    led_theme: RwLock<u8>,
    led_intensity: RwLock<u8>,
    led_speed: RwLock<u8>,

    /// Flag indicating LED update is needed
    needs_led_update: RwLock<bool>,
}

impl AppState {
    /// Creates a new application state.
    pub fn new(config: Config) -> Result<Self> {
        // Try to open LCD device
        let lcd = match LcdDevice::open() {
            Ok(device) => {
                info!("LCD device opened successfully");
                Some(Mutex::new(device))
            }
            Err(e) => {
                tracing::warn!("LCD device not found: {}. Running in headless mode.", e);
                None
            }
        };

        let canvas = Canvas::new(config.canvas.width, config.canvas.height);
        let framebuffer = Framebuffer::new();

        Ok(Self {
            led_device_path: config.led.device.clone(),
            led_theme: RwLock::new(config.led.theme),
            led_intensity: RwLock::new(config.led.intensity),
            led_speed: RwLock::new(config.led.speed),
            config: RwLock::new(config),
            lcd,
            orientation: RwLock::new(Orientation::default()),
            canvas: RwLock::new(canvas),
            framebuffer: RwLock::new(framebuffer),
            needs_redraw: RwLock::new(true),
            needs_led_update: RwLock::new(true),
        })
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    /// Updates the configuration.
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut Config),
    {
        let mut config = self.config.write().unwrap();
        f(&mut config);
    }

    /// Gets the current orientation.
    pub fn orientation(&self) -> Orientation {
        *self.orientation.read().unwrap()
    }

    /// Returns true if the LCD device is connected.
    pub fn is_lcd_connected(&self) -> bool {
        self.lcd.is_some()
    }

    /// Sets the display orientation.
    pub fn set_orientation(&self, orientation: Orientation) -> Result<()> {
        if let Some(ref lcd) = self.lcd {
            let device = lcd.lock().unwrap();
            device.set_orientation(orientation)?;
        }
        *self.orientation.write().unwrap() = orientation;
        *self.needs_redraw.write().unwrap() = true;
        info!("Orientation set to: {}", orientation);
        Ok(())
    }

    /// Gets the current LED settings.
    pub fn led_settings(&self) -> (u8, u8, u8) {
        (
            *self.led_theme.read().unwrap(),
            *self.led_intensity.read().unwrap(),
            *self.led_speed.read().unwrap(),
        )
    }

    /// Sets the LED theme and parameters.
    pub async fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> Result<()> {
        let led = LedDevice::new(&self.led_device_path);
        let led_theme = LedTheme::from_byte(theme)?;
        led.set_theme(led_theme, intensity, speed).await?;

        *self.led_theme.write().unwrap() = theme;
        *self.led_intensity.write().unwrap() = intensity;
        *self.led_speed.write().unwrap() = speed;

        info!(
            "LED set to theme {} (intensity: {}, speed: {})",
            theme, intensity, speed
        );
        Ok(())
    }

    /// Turns off the LEDs.
    pub async fn led_off(&self) -> Result<()> {
        let led = LedDevice::new(&self.led_device_path);
        led.set_off().await?;
        *self.led_theme.write().unwrap() = 4; // Off
        info!("LED turned off");
        Ok(())
    }

    /// Sends a heartbeat to the LCD device.
    pub fn send_heartbeat(&self) -> Result<()> {
        if let Some(ref lcd) = self.lcd {
            let device = lcd.lock().unwrap();
            device.heartbeat()?;
            debug!("Heartbeat sent");
        }
        Ok(())
    }

    /// Renders a frame and updates the display.
    pub async fn render_frame(&self) -> Result<()> {
        let needs_redraw = *self.needs_redraw.read().unwrap();

        if needs_redraw {
            // Get canvas and render to framebuffer
            let canvas = self.canvas.read().unwrap();
            let mut framebuffer = self.framebuffer.write().unwrap();

            canvas.render_to_framebuffer(&mut framebuffer)?;

            // Send to LCD
            if let Some(ref lcd) = self.lcd {
                let device = lcd.lock().unwrap();
                device.redraw(&framebuffer)?;
            }

            *self.needs_redraw.write().unwrap() = false;
            debug!("Frame rendered");
        }

        // Handle LED updates
        let needs_led = *self.needs_led_update.read().unwrap();
        if needs_led {
            let (theme, intensity, speed) = self.led_settings();
            if let Err(e) = self.set_led(theme, intensity, speed).await {
                tracing::warn!("LED update failed: {}", e);
            }
            *self.needs_led_update.write().unwrap() = false;
        }

        Ok(())
    }

    /// Triggers a full redraw on the next frame.
    pub fn force_redraw(&self) {
        *self.needs_redraw.write().unwrap() = true;
    }

    /// Returns the current framebuffer as PNG bytes.
    pub fn get_screen_png(&self) -> Result<Vec<u8>> {
        let fb = self.framebuffer.read().unwrap();
        let rgba = fb.to_rgba8();

        let mut png_data = Vec::new();
        {
            let mut encoder =
                png::Encoder::new(&mut png_data, fb.width() as u32, fb.height() as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&rgba)?;
        }

        Ok(png_data)
    }

    /// Clears the display to a color.
    pub fn clear_display(&self, color: u16) -> Result<()> {
        {
            let mut fb = self.framebuffer.write().unwrap();
            fb.clear(color);
        }
        self.force_redraw();
        Ok(())
    }

    /// Gets a mutable reference to the canvas.
    pub fn with_canvas<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Canvas) -> R,
    {
        let mut canvas = self.canvas.write().unwrap();
        let result = f(&mut canvas);
        *self.needs_redraw.write().unwrap() = true;
        result
    }

    /// Gets a read reference to the canvas.
    pub fn read_canvas<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Canvas) -> R,
    {
        let canvas = self.canvas.read().unwrap();
        f(&canvas)
    }
}
