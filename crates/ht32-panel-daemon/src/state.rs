//! Application state management.

#![allow(dead_code, unused_imports)]

use anyhow::{Context, Result};
use ht32_panel_hw::{
    lcd::{Framebuffer, LcdDevice},
    led::{LedDevice, LedTheme},
    Orientation,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::faces::{self, Face};
use crate::rendering::Canvas;
use crate::sensors::{
    data::SystemData, CpuSensor, DiskSensor, MemorySensor, NetworkSensor, Sensor, SystemInfo,
};

/// Display settings persisted to state directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    /// Current face name.
    #[serde(default = "default_face")]
    pub face: String,

    /// Display orientation.
    #[serde(default)]
    pub orientation: String,

    /// Background color (RGB888 hex string, e.g., "#000000").
    #[serde(default = "default_bg_color")]
    pub background_color: String,

    /// Foreground/text color (RGB888 hex string, e.g., "#FFFFFF").
    #[serde(default = "default_fg_color")]
    pub foreground_color: String,

    /// Optional background image path.
    #[serde(default)]
    pub background_image: Option<String>,

    /// LED theme (1=rainbow, 2=breathing, 3=colors, 4=off, 5=auto).
    #[serde(default = "default_led_theme")]
    pub led_theme: u8,

    /// LED intensity (1-5).
    #[serde(default = "default_led_value")]
    pub led_intensity: u8,

    /// LED speed (1-5).
    #[serde(default = "default_led_value")]
    pub led_speed: u8,

    /// Refresh rate in seconds (2-60).
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate_secs: u32,

    /// Network interface to monitor (None = auto-detect).
    #[serde(default)]
    pub network_interface: Option<String>,
}

fn default_face() -> String {
    "detailed".to_string()
}

fn default_bg_color() -> String {
    "#000000".to_string()
}

fn default_fg_color() -> String {
    "#FFFFFF".to_string()
}

fn default_led_theme() -> u8 {
    2 // Breathing
}

fn default_led_value() -> u8 {
    3
}

fn default_refresh_rate() -> u32 {
    2
}

/// Parse a hex color string (e.g., "#FFFFFF" or "FFFFFF") to RGB888.
fn parse_hex_color(hex: &str) -> Option<u32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            face: default_face(),
            orientation: "landscape".to_string(),
            background_color: default_bg_color(),
            foreground_color: default_fg_color(),
            background_image: None,
            led_theme: default_led_theme(),
            led_intensity: default_led_value(),
            led_speed: default_led_value(),
            refresh_rate_secs: default_refresh_rate(),
            network_interface: None,
        }
    }
}

/// Sensors collection for sampling system data.
struct Sensors {
    cpu: CpuSensor,
    memory: MemorySensor,
    network: NetworkSensor,
    disk: DiskSensor,
    system: SystemInfo,
}

impl Sensors {
    fn new(network_interface: &str) -> Self {
        Self {
            cpu: CpuSensor::new(),
            memory: MemorySensor::new(),
            network: NetworkSensor::new(network_interface),
            disk: DiskSensor::auto(),
            system: SystemInfo::new(),
        }
    }

    fn new_auto() -> Self {
        Self {
            cpu: CpuSensor::new(),
            memory: MemorySensor::new(),
            network: NetworkSensor::auto(),
            disk: DiskSensor::auto(),
            system: SystemInfo::new(),
        }
    }

    fn sample(&mut self) -> SystemData {
        // Sample all sensors
        let cpu_percent = self.cpu.sample();
        let ram_percent = self.memory.sample();
        let _ = self.network.sample(); // Updates internal state
        let _ = self.disk.sample(); // Updates internal state

        SystemData {
            hostname: self.system.hostname(),
            time: self.system.time(),
            uptime: self.system.uptime(),
            cpu_percent,
            ram_percent,
            disk_read_rate: self.disk.read_rate(),
            disk_write_rate: self.disk.write_rate(),
            net_interface: self.network.interface_name().to_string(),
            net_rx_rate: self.network.rx_rate(),
            net_tx_rate: self.network.tx_rate(),
            ipv6_address: self.network.ipv6_address(),
            ipv4_address: self.network.ipv4_address(),
        }
    }
}

/// Shared application state.
pub struct AppState {
    /// Configuration
    config: RwLock<Config>,

    /// State directory for persisting runtime state
    state_dir: PathBuf,

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

    /// System sensors
    sensors: Mutex<Sensors>,

    /// Current display face
    face: RwLock<Box<dyn Face>>,

    /// Background color (RGB888)
    background_color: RwLock<u32>,

    /// Foreground color (RGB888)
    foreground_color: RwLock<u32>,

    /// Background image path (optional)
    background_image: RwLock<Option<PathBuf>>,

    /// Refresh rate in seconds (2-60)
    refresh_rate_secs: RwLock<u32>,

    /// Network interface to monitor (None = auto-detect)
    network_interface: RwLock<Option<String>>,
}

impl AppState {
    /// Creates a new application state.
    pub fn new(config: Config) -> Result<Self> {
        // Setup state directory
        let state_dir = PathBuf::from(&config.state_dir);
        if let Err(e) = std::fs::create_dir_all(&state_dir) {
            warn!("Failed to create state directory {:?}: {}", state_dir, e);
        }

        // Load display settings from state
        let settings = Self::load_display_settings(&state_dir);

        // Parse orientation from settings
        let orientation: Orientation = settings.orientation.parse().unwrap_or_default();

        // Try to open LCD device
        let lcd = match LcdDevice::open() {
            Ok(device) => {
                // Send initial heartbeat to wake up the device
                if let Err(e) = device.heartbeat() {
                    warn!("Failed to send initial heartbeat: {}", e);
                }
                // Always use hardware landscape mode - orientation is handled in software
                if let Err(e) = device.set_orientation(Orientation::Landscape) {
                    warn!("Failed to set initial orientation: {}", e);
                }
                info!("LCD device opened successfully");
                Some(Mutex::new(device))
            }
            Err(e) => {
                warn!("LCD device not found: {}. Running in headless mode.", e);
                None
            }
        };

        // Create canvas with dimensions based on saved orientation
        let (canvas_w, canvas_h) = orientation.dimensions();
        let mut canvas = Canvas::new(canvas_w as u32, canvas_h as u32);
        let framebuffer = Framebuffer::new();

        // Initialize sensors - use saved settings or auto-detect
        let network_interface = settings.network_interface.clone();
        let sensors = match network_interface.as_ref() {
            Some(iface) => Sensors::new(iface),
            None => Sensors::new_auto(),
        };

        // Load face from settings
        let face = faces::create_face(&settings.face).unwrap_or_else(|| {
            warn!(
                "Unknown face '{}', falling back to 'detailed'",
                settings.face
            );
            faces::create_face("detailed").unwrap()
        });
        info!("Using display face: {}", face.name());

        // Parse colors
        let bg_color = parse_hex_color(&settings.background_color).unwrap_or(0x000000);
        let fg_color = parse_hex_color(&settings.foreground_color).unwrap_or(0xFFFFFF);

        // Set canvas background
        canvas.set_background(bg_color);

        // Parse background image path
        let bg_image = settings.background_image.map(PathBuf::from);

        info!("Display orientation: {}", orientation);
        info!("Background color: #{:06X}", bg_color);
        info!("Foreground color: #{:06X}", fg_color);

        Ok(Self {
            led_device_path: config.devices.led.clone(),
            led_theme: RwLock::new(settings.led_theme),
            led_intensity: RwLock::new(settings.led_intensity),
            led_speed: RwLock::new(settings.led_speed),
            state_dir,
            config: RwLock::new(config),
            lcd,
            orientation: RwLock::new(orientation),
            canvas: RwLock::new(canvas),
            framebuffer: RwLock::new(framebuffer),
            needs_redraw: RwLock::new(true),
            needs_led_update: RwLock::new(true),
            sensors: Mutex::new(sensors),
            face: RwLock::new(face),
            background_color: RwLock::new(bg_color),
            foreground_color: RwLock::new(fg_color),
            background_image: RwLock::new(bg_image),
            refresh_rate_secs: RwLock::new(settings.refresh_rate_secs),
            network_interface: RwLock::new(network_interface),
        })
    }

    /// Loads display settings from state directory.
    fn load_display_settings(state_dir: &Path) -> DisplaySettings {
        let settings_file = state_dir.join("display.toml");
        if let Ok(content) = std::fs::read_to_string(&settings_file) {
            if let Ok(settings) = toml::from_str(&content) {
                return settings;
            }
        }
        DisplaySettings::default()
    }

    /// Saves display settings to state directory.
    fn save_display_settings(&self) {
        let settings = DisplaySettings {
            face: self.face.read().unwrap().name().to_string(),
            orientation: self.orientation.read().unwrap().to_string(),
            background_color: format!("#{:06X}", *self.background_color.read().unwrap()),
            foreground_color: format!("#{:06X}", *self.foreground_color.read().unwrap()),
            background_image: self
                .background_image
                .read()
                .unwrap()
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            led_theme: *self.led_theme.read().unwrap(),
            led_intensity: *self.led_intensity.read().unwrap(),
            led_speed: *self.led_speed.read().unwrap(),
            refresh_rate_secs: *self.refresh_rate_secs.read().unwrap(),
            network_interface: self.network_interface.read().unwrap().clone(),
        };

        let settings_file = self.state_dir.join("display.toml");
        match toml::to_string_pretty(&settings) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&settings_file, content) {
                    warn!("Failed to save display settings: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to serialize display settings: {}", e);
            }
        }
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
        // Always keep hardware in landscape mode - we handle orientation in software
        if let Some(ref lcd) = self.lcd {
            let device = lcd.lock().unwrap();
            // Use hardware landscape mode always - portrait is handled via software rotation
            device.set_orientation(Orientation::Landscape)?;
        }
        *self.orientation.write().unwrap() = orientation;

        // Resize canvas for the logical orientation (faces render to this)
        let (width, height) = orientation.dimensions();
        {
            let mut canvas = self.canvas.write().unwrap();
            canvas.resize(width as u32, height as u32);
            canvas.clear(); // Clear to avoid stale content
        }
        // Keep framebuffer at hardware native size (320x170) - we transform canvas into it
        {
            let mut fb = self.framebuffer.write().unwrap();
            fb.resize(320, 170);
            fb.clear(0); // Clear to black
        }

        *self.needs_redraw.write().unwrap() = true;
        self.save_display_settings();
        info!("Orientation set to: {}", orientation);
        Ok(())
    }

    /// Gets the current refresh rate in seconds.
    pub fn refresh_rate_secs(&self) -> u32 {
        *self.refresh_rate_secs.read().unwrap()
    }

    /// Sets the refresh rate in seconds (clamped to 2-60).
    pub fn set_refresh_rate_secs(&self, secs: u32) {
        let clamped = secs.clamp(2, 60);
        *self.refresh_rate_secs.write().unwrap() = clamped;
        self.save_display_settings();
        info!("Refresh rate set to {}s", clamped);
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

        self.save_display_settings();
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
        self.save_display_settings();
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

    /// Samples all sensors and returns the current system data.
    fn sample_sensors(&self) -> SystemData {
        let mut sensors = self.sensors.lock().unwrap();
        sensors.sample()
    }

    /// Renders a frame and updates the display.
    pub async fn render_frame(&self) -> Result<()> {
        // Always sample sensors and render the face (faces update every frame)
        let system_data = self.sample_sensors();

        {
            // Get canvas and render face
            let mut canvas = self.canvas.write().unwrap();
            let face = self.face.read().unwrap();

            // Clear and render face
            canvas.clear();
            face.render(&mut canvas, &system_data);
        }

        // Render canvas to framebuffer with orientation transformation
        {
            let canvas = self.canvas.read().unwrap();
            let mut framebuffer = self.framebuffer.write().unwrap();
            let orientation = *self.orientation.read().unwrap();

            // Transform canvas to framebuffer based on orientation
            self.render_with_orientation(&canvas, &mut framebuffer, orientation)?;

            // Send to LCD
            if let Some(ref lcd) = self.lcd {
                let device = lcd.lock().unwrap();
                device.redraw(&framebuffer)?;
            }
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

    /// Renders canvas to framebuffer with orientation transformation.
    fn render_with_orientation(
        &self,
        canvas: &Canvas,
        framebuffer: &mut Framebuffer,
        orientation: Orientation,
    ) -> Result<()> {
        use ht32_panel_hw::lcd::rgb888_to_rgb565;

        let pixels = canvas.pixmap_pixels();
        let fb_data = framebuffer.data_mut();
        let (cw, ch) = canvas.dimensions();

        match orientation {
            Orientation::Landscape => {
                // Direct copy - canvas is 320x170, framebuffer is 320x170
                for (i, pixel) in pixels.iter().enumerate() {
                    if i < fb_data.len() {
                        fb_data[i] = rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                    }
                }
            }
            Orientation::LandscapeUpsideDown => {
                // Copy reversed (180° rotation)
                let len = fb_data.len();
                for (i, pixel) in pixels.iter().enumerate() {
                    if i < len {
                        fb_data[len - 1 - i] =
                            rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                    }
                }
            }
            Orientation::Portrait => {
                // Canvas is 170x320, rotate 90° CW to get 320x170
                // For each pixel at (x, y) in canvas, place at (ch - 1 - y, x) in framebuffer
                for y in 0..ch {
                    for x in 0..cw {
                        let src_idx = (y * cw + x) as usize;
                        let dst_x = ch - 1 - y;
                        let dst_y = x;
                        let dst_idx = (dst_y * 320 + dst_x) as usize;
                        if src_idx < pixels.len() && dst_idx < fb_data.len() {
                            let pixel = &pixels[src_idx];
                            fb_data[dst_idx] =
                                rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                        }
                    }
                }
            }
            Orientation::PortraitUpsideDown => {
                // Canvas is 170x320, rotate 90° CCW to get 320x170
                // For each pixel at (x, y) in canvas, place at (y, cw - 1 - x) in framebuffer
                for y in 0..ch {
                    for x in 0..cw {
                        let src_idx = (y * cw + x) as usize;
                        let dst_x = y;
                        let dst_y = cw - 1 - x;
                        let dst_idx = (dst_y * 320 + dst_x) as usize;
                        if src_idx < pixels.len() && dst_idx < fb_data.len() {
                            let pixel = &pixels[src_idx];
                            fb_data[dst_idx] =
                                rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Triggers a full redraw on the next frame.
    pub fn force_redraw(&self) {
        *self.needs_redraw.write().unwrap() = true;
    }

    /// Returns the current canvas as PNG bytes.
    /// This shows the logical orientation (portrait/landscape) as seen by the user.
    pub fn get_screen_png(&self) -> Result<Vec<u8>> {
        let canvas = self.canvas.read().unwrap();
        let (width, height) = canvas.dimensions();
        let rgba = canvas.pixels();

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(rgba)?;
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

    /// Sets the display face.
    pub fn set_face(&self, name: &str) -> Result<()> {
        if let Some(new_face) = faces::create_face(name) {
            *self.face.write().unwrap() = new_face;
            self.save_display_settings();
            info!("Display face changed to: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unknown face: {}", name))
        }
    }

    /// Gets the current face name.
    pub fn face_name(&self) -> String {
        self.face.read().unwrap().name().to_string()
    }

    /// Gets the background color (RGB888).
    pub fn background_color(&self) -> u32 {
        *self.background_color.read().unwrap()
    }

    /// Sets the background color (RGB888).
    pub fn set_background_color(&self, color: u32) {
        *self.background_color.write().unwrap() = color;
        self.canvas.write().unwrap().set_background(color);
        *self.needs_redraw.write().unwrap() = true;
        self.save_display_settings();
        info!("Background color set to #{:06X}", color);
    }

    /// Sets the background color from a hex string (e.g., "#FFFFFF").
    pub fn set_background_color_hex(&self, hex: &str) -> Result<()> {
        let color =
            parse_hex_color(hex).ok_or_else(|| anyhow::anyhow!("Invalid hex color: {}", hex))?;
        self.set_background_color(color);
        Ok(())
    }

    /// Gets the foreground/text color (RGB888).
    pub fn foreground_color(&self) -> u32 {
        *self.foreground_color.read().unwrap()
    }

    /// Sets the foreground/text color (RGB888).
    pub fn set_foreground_color(&self, color: u32) {
        *self.foreground_color.write().unwrap() = color;
        *self.needs_redraw.write().unwrap() = true;
        self.save_display_settings();
        info!("Foreground color set to #{:06X}", color);
    }

    /// Sets the foreground color from a hex string (e.g., "#FFFFFF").
    pub fn set_foreground_color_hex(&self, hex: &str) -> Result<()> {
        let color =
            parse_hex_color(hex).ok_or_else(|| anyhow::anyhow!("Invalid hex color: {}", hex))?;
        self.set_foreground_color(color);
        Ok(())
    }

    /// Gets the background image path (if any).
    pub fn background_image(&self) -> Option<PathBuf> {
        self.background_image.read().unwrap().clone()
    }

    /// Sets the background image path.
    pub fn set_background_image(&self, path: Option<PathBuf>) {
        *self.background_image.write().unwrap() = path.clone();
        *self.needs_redraw.write().unwrap() = true;
        self.save_display_settings();
        match path {
            Some(p) => info!("Background image set to {:?}", p),
            None => info!("Background image cleared"),
        }
    }

    /// Gets the current display settings as a struct.
    pub fn display_settings(&self) -> DisplaySettings {
        DisplaySettings {
            face: self.face.read().unwrap().name().to_string(),
            orientation: self.orientation.read().unwrap().to_string(),
            background_color: format!("#{:06X}", *self.background_color.read().unwrap()),
            foreground_color: format!("#{:06X}", *self.foreground_color.read().unwrap()),
            background_image: self
                .background_image
                .read()
                .unwrap()
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            led_theme: *self.led_theme.read().unwrap(),
            led_intensity: *self.led_intensity.read().unwrap(),
            led_speed: *self.led_speed.read().unwrap(),
            refresh_rate_secs: *self.refresh_rate_secs.read().unwrap(),
            network_interface: self.network_interface.read().unwrap().clone(),
        }
    }

    /// Gets the current network interface (None if auto-detected).
    pub fn network_interface(&self) -> Option<String> {
        self.network_interface.read().unwrap().clone()
    }

    /// Gets the currently active network interface name (resolved from auto if needed).
    pub fn network_interface_config(&self) -> String {
        let sensors = self.sensors.lock().unwrap();
        sensors.network.interface_name().to_string()
    }

    /// Sets the network interface to monitor.
    /// Pass None to enable auto-detection.
    pub fn set_network_interface(&self, interface: Option<String>) {
        *self.network_interface.write().unwrap() = interface.clone();

        // Update the sensor
        let mut sensors = self.sensors.lock().unwrap();
        match interface {
            Some(ref iface) => sensors.network.set_interface(iface),
            None => sensors.network.set_auto(),
        }

        self.save_display_settings();
    }

    /// Lists all available network interfaces.
    pub fn list_network_interfaces(&self) -> Vec<String> {
        NetworkSensor::list_interfaces()
    }
}
