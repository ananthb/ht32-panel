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
use crate::faces::{self, EnabledComplications, Face, Theme};
use crate::rendering::Canvas;
use crate::sensors::{
    data::{IpDisplayPreference, SystemData},
    CpuSensor, DiskSensor, MemorySensor, NetworkSensor, Sensor, SystemInfo, TemperatureSensor,
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

    /// Color theme preset name.
    #[serde(default = "default_theme")]
    pub theme: String,

    /// LED theme (1=rainbow, 2=breathing, 3=colors, 4=off, 5=auto).
    #[serde(default = "default_led_theme")]
    pub led_theme: u8,

    /// LED intensity (1-5).
    #[serde(default = "default_led_value")]
    pub led_intensity: u8,

    /// LED speed (1-5).
    #[serde(default = "default_led_value")]
    pub led_speed: u8,

    /// Refresh interval in milliseconds (1500-10000).
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_ms: u32,

    /// Network interface to monitor (None = auto-detect).
    #[serde(default)]
    pub network_interface: Option<String>,

    /// IP address display preference.
    #[serde(default = "default_ip_display")]
    pub ip_display: String,

    /// Enabled complications per face.
    #[serde(default)]
    pub complications: EnabledComplications,
}

fn default_ip_display() -> String {
    "ipv6-gua".to_string()
}

fn default_face() -> String {
    "professional".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_led_theme() -> u8 {
    2 // Breathing
}

fn default_led_value() -> u8 {
    3
}

fn default_refresh_interval() -> u32 {
    2000 // 2 second default
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            face: default_face(),
            orientation: "landscape".to_string(),
            theme: default_theme(),
            led_theme: default_led_theme(),
            led_intensity: default_led_value(),
            led_speed: default_led_value(),
            refresh_interval_ms: default_refresh_interval(),
            network_interface: None,
            ip_display: default_ip_display(),
            complications: EnabledComplications::new(),
        }
    }
}

/// Sensors collection for sampling system data.
struct Sensors {
    cpu: CpuSensor,
    temperature: TemperatureSensor,
    memory: MemorySensor,
    network: NetworkSensor,
    disk: DiskSensor,
    system: SystemInfo,
}

impl Sensors {
    fn new(network_interface: &str) -> Self {
        Self {
            cpu: CpuSensor::new(),
            temperature: TemperatureSensor::new(),
            memory: MemorySensor::new(),
            network: NetworkSensor::new(network_interface),
            disk: DiskSensor::auto(),
            system: SystemInfo::new(),
        }
    }

    fn new_auto() -> Self {
        Self {
            cpu: CpuSensor::new(),
            temperature: TemperatureSensor::new(),
            memory: MemorySensor::new(),
            network: NetworkSensor::auto(),
            disk: DiskSensor::auto(),
            system: SystemInfo::new(),
        }
    }

    fn sample(&mut self, ip_preference: IpDisplayPreference) -> SystemData {
        // Sample all sensors
        let cpu_percent = self.cpu.sample();
        let _ = self.temperature.sample(); // Updates internal state
        let cpu_temp = self.temperature.temperature();
        let ram_percent = self.memory.sample();
        let _ = self.network.sample(); // Updates internal state
        let _ = self.disk.sample(); // Updates internal state

        // Get the IP address based on preference
        let display_ip = match ip_preference {
            IpDisplayPreference::Ipv6Gua => self.network.ipv6_gua(),
            IpDisplayPreference::Ipv6Lla => self.network.ipv6_lla(),
            IpDisplayPreference::Ipv6Ula => self.network.ipv6_ula(),
            IpDisplayPreference::Ipv4 => self.network.ipv4_address(),
        };

        SystemData {
            hostname: self.system.hostname(),
            time: self.system.time(),
            uptime: self.system.uptime(),
            cpu_percent,
            cpu_temp,
            ram_percent,
            disk_read_rate: self.disk.read_rate(),
            disk_write_rate: self.disk.write_rate(),
            disk_history: self.disk.history().clone(),
            net_interface: self.network.interface_name().to_string(),
            net_rx_rate: self.network.rx_rate(),
            net_tx_rate: self.network.tx_rate(),
            net_history: self.network.history().clone(),
            display_ip,
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

    /// Current color theme name
    theme_name: RwLock<String>,

    /// Refresh interval in milliseconds (1500-10000)
    refresh_interval_ms: RwLock<u32>,

    /// Network interface to monitor (None = auto-detect)
    network_interface: RwLock<Option<String>>,

    /// IP address display preference
    ip_display: RwLock<IpDisplayPreference>,

    /// Enabled complications per face
    complications: RwLock<EnabledComplications>,
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
                "Unknown face '{}', falling back to 'professional'",
                settings.face
            );
            faces::create_face("professional").unwrap()
        });
        info!("Using display face: {}", face.name());

        // Load theme and set canvas background
        let theme = Theme::from_preset(&settings.theme);
        canvas.set_background(theme.background);

        info!("Display orientation: {}", orientation);
        info!("Theme: {}", settings.theme);

        // Parse IP display preference
        let ip_display: IpDisplayPreference = settings
            .ip_display
            .parse()
            .unwrap_or(IpDisplayPreference::Ipv6Gua);

        // Initialize complications from settings
        let mut complications = settings.complications.clone();
        complications.init_from_defaults(face.as_ref());

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
            theme_name: RwLock::new(settings.theme),
            refresh_interval_ms: RwLock::new(settings.refresh_interval_ms),
            network_interface: RwLock::new(network_interface),
            ip_display: RwLock::new(ip_display),
            complications: RwLock::new(complications),
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
            theme: self.theme_name.read().unwrap().clone(),
            led_theme: *self.led_theme.read().unwrap(),
            led_intensity: *self.led_intensity.read().unwrap(),
            led_speed: *self.led_speed.read().unwrap(),
            refresh_interval_ms: *self.refresh_interval_ms.read().unwrap(),
            network_interface: self.network_interface.read().unwrap().clone(),
            ip_display: self.ip_display.read().unwrap().to_string(),
            complications: self.complications.read().unwrap().clone(),
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

    /// Returns true if the web UI is enabled.
    pub fn is_web_enabled(&self) -> bool {
        self.config.read().unwrap().web.enable
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

    /// Gets the current refresh interval in milliseconds.
    pub fn refresh_interval_ms(&self) -> u32 {
        *self.refresh_interval_ms.read().unwrap()
    }

    /// Sets the refresh interval in milliseconds (clamped to 1500-10000).
    pub fn set_refresh_interval_ms(&self, ms: u32) {
        let clamped = ms.clamp(1500, 10000);
        *self.refresh_interval_ms.write().unwrap() = clamped;
        self.save_display_settings();
        info!("Refresh interval set to {}ms", clamped);
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
        let ip_preference = *self.ip_display.read().unwrap();
        sensors.sample(ip_preference)
    }

    /// Renders a frame and updates the display.
    pub async fn render_frame(&self) -> Result<()> {
        // Always sample sensors and render the face (faces update every frame)
        let system_data = self.sample_sensors();

        // Get theme from current preset
        let theme = Theme::from_preset(&self.theme_name.read().unwrap());

        {
            // Get canvas and render face
            let mut canvas = self.canvas.write().unwrap();
            let face = self.face.read().unwrap();
            let complications = self.complications.read().unwrap();

            // Clear and render face
            canvas.clear();
            face.render(&mut canvas, &system_data, &theme, &complications);
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
            // Initialize complications from defaults for this face
            {
                let mut complications = self.complications.write().unwrap();
                complications.init_from_defaults(new_face.as_ref());
            }
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

    /// Gets available complications for the current face.
    pub fn available_complications(&self) -> Vec<faces::Complication> {
        self.face.read().unwrap().available_complications()
    }

    /// Gets enabled complications for the current face.
    pub fn enabled_complications(&self) -> std::collections::HashSet<String> {
        let face_name = self.face.read().unwrap().name().to_string();
        self.complications.read().unwrap().get_enabled(&face_name)
    }

    /// Sets whether a complication is enabled for the current face.
    pub fn set_complication_enabled(&self, complication_id: &str, enabled: bool) -> Result<()> {
        let face_name = self.face.read().unwrap().name().to_string();
        let available: Vec<_> = self.face.read().unwrap().available_complications();

        // Validate complication exists for this face
        if !available.iter().any(|c| c.id == complication_id) {
            return Err(anyhow::anyhow!(
                "Unknown complication '{}' for face '{}'",
                complication_id,
                face_name
            ));
        }

        self.complications
            .write()
            .unwrap()
            .set_enabled(&face_name, complication_id, enabled);
        self.save_display_settings();
        self.request_redraw();

        info!(
            "Complication '{}' {} for face '{}'",
            complication_id,
            if enabled { "enabled" } else { "disabled" },
            face_name
        );
        Ok(())
    }

    /// Gets the current theme name.
    pub fn theme_name(&self) -> String {
        self.theme_name.read().unwrap().clone()
    }

    /// Sets the theme by name.
    pub fn set_theme(&self, name: &str) -> Result<()> {
        // Validate theme exists
        if !faces::available_themes().contains(&name) {
            return Err(anyhow::anyhow!("Unknown theme: {}", name));
        }

        *self.theme_name.write().unwrap() = name.to_string();

        // Update canvas background
        let theme = Theme::from_preset(name);
        self.canvas
            .write()
            .unwrap()
            .set_background(theme.background);

        *self.needs_redraw.write().unwrap() = true;
        self.save_display_settings();
        info!("Theme set to: {}", name);
        Ok(())
    }

    /// Returns a list of available theme names.
    pub fn available_themes(&self) -> Vec<&'static str> {
        faces::available_themes()
    }

    /// Gets the current display settings as a struct.
    pub fn display_settings(&self) -> DisplaySettings {
        DisplaySettings {
            face: self.face.read().unwrap().name().to_string(),
            orientation: self.orientation.read().unwrap().to_string(),
            theme: self.theme_name.read().unwrap().clone(),
            led_theme: *self.led_theme.read().unwrap(),
            led_intensity: *self.led_intensity.read().unwrap(),
            led_speed: *self.led_speed.read().unwrap(),
            refresh_interval_ms: *self.refresh_interval_ms.read().unwrap(),
            network_interface: self.network_interface.read().unwrap().clone(),
            ip_display: self.ip_display.read().unwrap().to_string(),
        }
    }

    /// Gets the current IP display preference.
    pub fn ip_display(&self) -> IpDisplayPreference {
        *self.ip_display.read().unwrap()
    }

    /// Sets the IP display preference.
    pub fn set_ip_display(&self, preference: IpDisplayPreference) {
        *self.ip_display.write().unwrap() = preference;
        self.save_display_settings();
        info!("IP display preference set to: {}", preference);
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
