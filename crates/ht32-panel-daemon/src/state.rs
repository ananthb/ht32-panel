//! Application state management.

use anyhow::Result;
use ht32_panel_hw::{
    lcd::{Framebuffer, LcdDevice},
    led::{LedDevice, LedTheme},
    Orientation,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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

    /// Refresh interval in milliseconds (500-10000).
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u32,

    /// Network interface to monitor (legacy - migrated to complications).
    #[serde(default, skip_serializing)]
    pub network_interface: Option<String>,

    /// IP address display preference (legacy - migrated to complications).
    #[serde(default, skip_serializing)]
    pub ip_display: Option<String>,

    /// Enabled complications per face.
    #[serde(default)]
    pub complications: EnabledComplications,
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
    2500 // 2.5s default
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
            refresh_interval: default_refresh_interval(),
            network_interface: None,
            ip_display: None,
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
        let cpu_percent = self.cpu.sample();
        let _ = self.temperature.sample();
        let cpu_temp = self.temperature.temperature();
        let ram_percent = self.memory.sample();
        let _ = self.network.sample();
        let _ = self.disk.sample();

        let display_ip = match ip_preference {
            IpDisplayPreference::Ipv6Gua => self.network.ipv6_gua(),
            IpDisplayPreference::Ipv6Lla => self.network.ipv6_lla(),
            IpDisplayPreference::Ipv6Ula => self.network.ipv6_ula(),
            IpDisplayPreference::Ipv4 => self.network.ipv4_address(),
        };

        let (hour, minute, day, month, year, day_of_week, _) = self.system.time_components();

        SystemData {
            hostname: self.system.hostname(),
            time: self.system.time(),
            hour,
            minute,
            day,
            month,
            year,
            day_of_week,
            uptime: self.system.uptime(),
            cpu_percent,
            cpu_temp,
            ram_percent,
            disk_read_rate: self.disk.read_rate(),
            disk_write_rate: self.disk.write_rate(),
            disk_history: self.disk.history().clone(),
            disk_read_history: self.disk.read_history().clone(),
            disk_write_history: self.disk.write_history().clone(),
            net_interface: self.network.interface_name().to_string(),
            net_rx_rate: self.network.rx_rate(),
            net_tx_rate: self.network.tx_rate(),
            net_history: self.network.history().clone(),
            net_rx_history: self.network.rx_history().clone(),
            net_tx_history: self.network.tx_history().clone(),
            display_ip,
        }
    }
}

/// Display-related state (face, orientation, theme, complications).
struct DisplayState {
    orientation: Orientation,
    face: Box<dyn Face>,
    theme_name: String,
    refresh_interval: u32,
    complications: EnabledComplications,
    needs_redraw: bool,
}

/// LED-related state.
struct LedState {
    theme: u8,
    intensity: u8,
    speed: u8,
    needs_update: bool,
}

/// Render pipeline state (canvas, framebuffer, PNG cache).
struct RenderState {
    canvas: Canvas,
    framebuffer: Framebuffer,
    cached_png: Option<Vec<u8>>,
}

/// Minimum interval between disk writes for display settings.
const SAVE_DEBOUNCE_SECS: u64 = 5;

/// Shared application state.
pub struct AppState {
    /// Configuration (immutable after init)
    config: Config,

    /// State directory for persisting runtime state
    state_dir: PathBuf,

    /// LCD device (mutable Option for reconnection)
    lcd: Mutex<Option<LcdDevice>>,

    /// Timestamp of last LCD reconnection attempt
    last_lcd_reconnect: Mutex<std::time::Instant>,

    /// LED device path
    led_device_path: String,

    /// Display state
    display: RwLock<DisplayState>,

    /// LED state
    led: RwLock<LedState>,

    /// Render pipeline
    render: RwLock<RenderState>,

    /// System sensors
    sensors: Mutex<Sensors>,

    /// Save debouncing: set when a save is needed
    save_pending: AtomicBool,

    /// Timestamp of last save
    last_save: Mutex<std::time::Instant>,
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
                if let Err(e) = device.heartbeat() {
                    warn!("Failed to send initial heartbeat: {}", e);
                }
                if let Err(e) = device.set_orientation(Orientation::Landscape) {
                    warn!("Failed to set initial orientation: {}", e);
                }
                info!("LCD device opened successfully");
                Some(device)
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

        // Load face from settings
        let face = faces::create_face(&settings.face).unwrap_or_else(|| {
            warn!(
                "Unknown face '{}', falling back to 'professional'",
                settings.face
            );
            faces::create_face("professional").unwrap()
        });
        info!("Using display face: {}", face.name());

        // Initialize complications from settings and migrate legacy settings
        let mut complications = settings.complications.clone();
        complications.init_from_defaults(face.as_ref());

        if let Some(ref ip_display) = settings.ip_display {
            let face_name = face.name();
            complications.set_option(
                face_name,
                faces::complication_names::IP_ADDRESS,
                faces::complication_options::IP_TYPE,
                ip_display.clone(),
            );
            info!("Migrated legacy ip_display setting: {}", ip_display);
        }

        if let Some(ref network_interface) = settings.network_interface {
            let face_name = face.name();
            complications.set_option(
                face_name,
                faces::complication_names::NETWORK,
                faces::complication_options::INTERFACE,
                network_interface.clone(),
            );
            info!(
                "Migrated legacy network_interface setting: {}",
                network_interface
            );
        }

        let network_interface_value = complications
            .get_option(
                face.name(),
                faces::complication_names::NETWORK,
                faces::complication_options::INTERFACE,
            )
            .cloned();

        let sensors = match network_interface_value.as_ref() {
            Some(iface) if iface != "auto" && !iface.is_empty() => Sensors::new(iface),
            _ => Sensors::new_auto(),
        };

        let theme = Theme::from_preset(&settings.theme);
        canvas.set_background(theme.background);

        info!("State directory: {:?}", state_dir);
        info!("Display orientation: {}", orientation);
        info!("Theme: {}", settings.theme);

        let now = std::time::Instant::now();

        let app_state = Self {
            led_device_path: config.devices.led.clone(),
            config,
            state_dir,
            lcd: Mutex::new(lcd),
            last_lcd_reconnect: Mutex::new(now),
            display: RwLock::new(DisplayState {
                orientation,
                face,
                theme_name: settings.theme,
                refresh_interval: settings.refresh_interval,
                complications,
                needs_redraw: true,
            }),
            led: RwLock::new(LedState {
                theme: settings.led_theme,
                intensity: settings.led_intensity,
                speed: settings.led_speed,
                needs_update: true,
            }),
            render: RwLock::new(RenderState {
                canvas,
                framebuffer,
                cached_png: None,
            }),
            sensors: Mutex::new(sensors),
            save_pending: AtomicBool::new(false),
            last_save: Mutex::new(now),
        };

        // Save initial state so the file always exists
        app_state.flush_display_settings();

        Ok(app_state)
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

    /// Marks that display settings need to be saved (debounced).
    fn save_display_settings(&self) {
        self.save_pending.store(true, Ordering::Relaxed);
    }

    /// Flushes display settings to disk immediately.
    fn flush_display_settings(&self) {
        self.save_pending.store(false, Ordering::Relaxed);
        *self.last_save.lock().unwrap() = std::time::Instant::now();

        let display = self.display.read().unwrap();
        let led = self.led.read().unwrap();

        let settings = DisplaySettings {
            face: display.face.name().to_string(),
            orientation: display.orientation.to_string(),
            theme: display.theme_name.clone(),
            led_theme: led.theme,
            led_intensity: led.intensity,
            led_speed: led.speed,
            refresh_interval: display.refresh_interval,
            network_interface: None,
            ip_display: None,
            complications: display.complications.clone(),
        };

        // Drop locks before disk I/O
        drop(display);
        drop(led);

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

    /// Flushes display settings if a save is pending and enough time has elapsed.
    /// Called from the render loop.
    pub fn maybe_flush_settings(&self) {
        if self.save_pending.load(Ordering::Relaxed) {
            let elapsed = self.last_save.lock().unwrap().elapsed();
            if elapsed >= std::time::Duration::from_secs(SAVE_DEBOUNCE_SECS) {
                self.flush_display_settings();
            }
        }
    }

    /// Returns the configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Gets the current orientation.
    pub fn orientation(&self) -> Orientation {
        self.display.read().unwrap().orientation
    }

    /// Returns true if the LCD device is connected.
    pub fn is_lcd_connected(&self) -> bool {
        self.lcd.lock().unwrap().is_some()
    }

    /// Returns true if the web UI is enabled.
    pub fn is_web_enabled(&self) -> bool {
        self.config.web.enable
    }

    /// Attempts to reconnect the LCD device if it's disconnected.
    /// Returns true if connected (either already or newly).
    fn try_lcd_reconnect(&self) -> bool {
        let mut lcd = self.lcd.lock().unwrap();
        if lcd.is_some() {
            return true;
        }

        // Only attempt reconnection every 30 seconds
        let mut last_attempt = self.last_lcd_reconnect.lock().unwrap();
        if last_attempt.elapsed() < std::time::Duration::from_secs(30) {
            return false;
        }
        *last_attempt = std::time::Instant::now();
        drop(last_attempt);

        match LcdDevice::open() {
            Ok(device) => {
                if let Err(e) = device.heartbeat() {
                    warn!("Reconnected LCD but heartbeat failed: {}", e);
                }
                if let Err(e) = device.set_orientation(Orientation::Landscape) {
                    warn!("Reconnected LCD but orientation set failed: {}", e);
                }
                info!("LCD device reconnected successfully");
                *lcd = Some(device);
                true
            }
            Err(_) => false,
        }
    }

    /// Sets the display orientation.
    pub fn set_orientation(&self, orientation: Orientation) -> Result<()> {
        // Always keep hardware in landscape mode
        {
            let lcd = self.lcd.lock().unwrap();
            if let Some(ref device) = *lcd {
                device.set_orientation(Orientation::Landscape)?;
            }
        }

        let (width, height) = orientation.dimensions();
        {
            let mut display = self.display.write().unwrap();
            display.orientation = orientation;
            display.needs_redraw = true;
        }
        {
            let mut render = self.render.write().unwrap();
            render.canvas.resize(width as u32, height as u32);
            render.canvas.clear();
            render.framebuffer.resize(320, 170);
            render.framebuffer.clear(0);
            render.cached_png = None;
        }

        self.save_display_settings();
        info!("Orientation set to: {}", orientation);
        Ok(())
    }

    /// Gets the current refresh interval in milliseconds.
    pub fn refresh_interval_ms(&self) -> u32 {
        self.display.read().unwrap().refresh_interval
    }

    /// Gets the current LED settings.
    pub fn led_settings(&self) -> (u8, u8, u8) {
        let led = self.led.read().unwrap();
        (led.theme, led.intensity, led.speed)
    }

    /// Sets the LED theme and parameters.
    pub async fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> Result<()> {
        {
            let mut led = self.led.write().unwrap();
            led.theme = theme;
            led.intensity = intensity;
            led.speed = speed;
        }
        self.save_display_settings();

        let led = LedDevice::new(&self.led_device_path);
        let led_theme = LedTheme::from_byte(theme)?;
        if let Err(e) = led.set_theme(led_theme, intensity, speed).await {
            warn!(
                "Failed to send LED command to {}: {}",
                self.led_device_path, e
            );
            return Err(e.into());
        }

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
        {
            let mut state = self.led.write().unwrap();
            state.theme = 4; // Off
        }
        self.save_display_settings();
        info!("LED turned off");
        Ok(())
    }

    /// Sends a heartbeat to the LCD device.
    pub fn send_heartbeat(&self) -> Result<()> {
        let lcd = self.lcd.lock().unwrap();
        if let Some(ref device) = *lcd {
            device.heartbeat()?;
            debug!("Heartbeat sent");
        }
        Ok(())
    }

    /// Samples all sensors and returns the current system data.
    fn sample_sensors(&self) -> SystemData {
        let mut sensors = self.sensors.lock().unwrap();
        let ip_preference = self.get_ip_display_from_complications();
        sensors.sample(ip_preference)
    }

    /// Gets the IP display preference from complications.
    fn get_ip_display_from_complications(&self) -> IpDisplayPreference {
        let display = self.display.read().unwrap();
        let face_name = display.face.name().to_string();
        display
            .complications
            .get_option(
                &face_name,
                faces::complication_names::IP_ADDRESS,
                faces::complication_options::IP_TYPE,
            )
            .and_then(|s| s.parse().ok())
            .unwrap_or(IpDisplayPreference::Ipv6Gua)
    }

    /// Renders a frame and updates the display.
    pub async fn render_frame(&self) -> Result<()> {
        let system_data = self.sample_sensors();

        // Render face to canvas
        {
            let display = self.display.read().unwrap();
            let theme = Theme::from_preset(&display.theme_name);
            let mut render = self.render.write().unwrap();

            render.canvas.clear();
            display.face.render(
                &mut render.canvas,
                &system_data,
                &theme,
                &display.complications,
            );

            // Invalidate PNG cache
            render.cached_png = None;
        }

        // Transform canvas to framebuffer and send to LCD
        {
            let orientation = self.display.read().unwrap().orientation;
            let mut render = self.render.write().unwrap();
            Self::render_to_framebuffer(&mut render, orientation)?;

            // Try to reconnect LCD if disconnected, then send
            let lcd = self.lcd.lock().unwrap();
            if let Some(ref device) = *lcd {
                device.redraw(&render.framebuffer)?;
            } else {
                drop(lcd);
                // Try reconnection (will rate-limit internally)
                if self.try_lcd_reconnect() {
                    let lcd = self.lcd.lock().unwrap();
                    if let Some(ref device) = *lcd {
                        device.redraw(&render.framebuffer)?;
                    }
                }
            }
        }

        // Handle LED updates
        let needs_led = self.led.read().unwrap().needs_update;
        if needs_led {
            let (theme, intensity, speed) = self.led_settings();
            if let Err(e) = self.set_led(theme, intensity, speed).await {
                warn!("LED update failed: {}", e);
            }
            self.led.write().unwrap().needs_update = false;
        }

        // Flush settings if debounce timer has elapsed
        self.maybe_flush_settings();

        Ok(())
    }

    /// Renders canvas to framebuffer with orientation transformation.
    fn render_to_framebuffer(render: &mut RenderState, orientation: Orientation) -> Result<()> {
        use ht32_panel_hw::lcd::rgb888_to_rgb565;

        let pixels = render.canvas.pixmap_pixels();
        let fb_data = render.framebuffer.data_mut();
        let (cw, ch) = render.canvas.dimensions();

        match orientation {
            Orientation::Landscape => {
                for (i, pixel) in pixels.iter().enumerate() {
                    if i < fb_data.len() {
                        fb_data[i] = rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                    }
                }
            }
            Orientation::LandscapeUpsideDown => {
                let len = fb_data.len();
                for (i, pixel) in pixels.iter().enumerate() {
                    if i < len {
                        fb_data[len - 1 - i] =
                            rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
                    }
                }
            }
            Orientation::Portrait => {
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
        self.display.write().unwrap().needs_redraw = true;
    }

    /// Returns the current canvas as PNG bytes (cached).
    pub fn get_screen_png(&self) -> Result<Vec<u8>> {
        // Check cache first
        {
            let render = self.render.read().unwrap();
            if let Some(ref cached) = render.cached_png {
                return Ok(cached.clone());
            }
        }

        // Generate PNG and cache it
        let mut render = self.render.write().unwrap();
        // Double-check after acquiring write lock
        if let Some(ref cached) = render.cached_png {
            return Ok(cached.clone());
        }

        let (width, height) = render.canvas.dimensions();
        let rgba = render.canvas.pixels();

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(rgba)?;
        }

        render.cached_png = Some(png_data.clone());
        Ok(png_data)
    }

    /// Clears the display to a color.
    pub fn clear_display(&self, color: u16) -> Result<()> {
        {
            let mut render = self.render.write().unwrap();
            render.framebuffer.clear(color);
            render.cached_png = None;
        }
        self.force_redraw();
        Ok(())
    }

    /// Sets the display face.
    pub fn set_face(&self, name: &str) -> Result<()> {
        if let Some(new_face) = faces::create_face(name) {
            let mut display = self.display.write().unwrap();
            display.complications.init_from_defaults(new_face.as_ref());
            display.face = new_face;
            display.needs_redraw = true;
            drop(display);
            self.save_display_settings();
            info!("Display face changed to: {}", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unknown face: {}", name))
        }
    }

    /// Gets the current face name.
    pub fn face_name(&self) -> String {
        self.display.read().unwrap().face.name().to_string()
    }

    /// Gets available complications for the current face.
    pub fn available_complications(&self) -> Vec<faces::Complication> {
        self.display.read().unwrap().face.available_complications()
    }

    /// Gets enabled complications for the current face.
    pub fn enabled_complications(&self) -> std::collections::HashSet<String> {
        let display = self.display.read().unwrap();
        let face_name = display.face.name().to_string();
        display.complications.get_enabled(&face_name)
    }

    /// Sets whether a complication is enabled for the current face.
    pub fn set_complication_enabled(&self, complication_id: &str, enabled: bool) -> Result<()> {
        let mut display = self.display.write().unwrap();
        let face_name = display.face.name().to_string();
        let available = display.face.available_complications();

        if !available.iter().any(|c| c.id == complication_id) {
            return Err(anyhow::anyhow!(
                "Unknown complication '{}' for face '{}'",
                complication_id,
                face_name
            ));
        }

        display
            .complications
            .set_enabled(&face_name, complication_id, enabled);
        display.needs_redraw = true;
        drop(display);

        self.save_display_settings();
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
        self.display.read().unwrap().theme_name.clone()
    }

    /// Sets the theme by name.
    pub fn set_theme(&self, name: &str) -> Result<()> {
        if !faces::available_themes().iter().any(|t| t.id == name) {
            return Err(anyhow::anyhow!("Unknown theme: {}", name));
        }

        {
            let mut display = self.display.write().unwrap();
            display.theme_name = name.to_string();
            display.needs_redraw = true;
        }

        let theme = Theme::from_preset(name);
        {
            let mut render = self.render.write().unwrap();
            render.canvas.set_background(theme.background);
            render.cached_png = None;
        }

        self.save_display_settings();
        info!("Theme set to: {}", name);
        Ok(())
    }

    /// Returns a list of available themes.
    pub fn available_themes(&self) -> Vec<faces::ThemeInfo> {
        faces::available_themes()
    }

    /// Lists all available network interfaces.
    pub fn list_network_interfaces(&self) -> Vec<String> {
        NetworkSensor::list_interfaces()
    }

    /// Gets a complication option value.
    pub fn get_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
    ) -> Option<String> {
        let display = self.display.read().unwrap();
        let face_name = display.face.name().to_string();
        display
            .complications
            .get_option(&face_name, complication_id, option_id)
            .cloned()
    }

    /// Sets a complication option value.
    pub fn set_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let mut display = self.display.write().unwrap();
        let face_name = display.face.name().to_string();
        let available = display.face.available_complications();

        let complication = available
            .iter()
            .find(|c| c.id == complication_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown complication '{}' for face '{}'",
                    complication_id,
                    face_name
                )
            })?;

        let option = complication
            .options
            .iter()
            .find(|o| o.id == option_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown option '{}' for complication '{}'",
                    option_id,
                    complication_id
                )
            })?;

        if let faces::ComplicationOptionType::Choice(choices) = &option.option_type {
            if !choices.iter().any(|c| c.value == value) {
                if complication_id == faces::complication_names::NETWORK
                    && option_id == faces::complication_options::INTERFACE
                {
                    let interfaces = NetworkSensor::list_interfaces();
                    if value != "auto" && !interfaces.contains(&value.to_string()) {
                        return Err(anyhow::anyhow!(
                            "Unknown interface '{}'. Available: auto, {:?}",
                            value,
                            interfaces
                        ));
                    }
                } else {
                    let valid_values: Vec<_> = choices.iter().map(|c| c.value.as_str()).collect();
                    return Err(anyhow::anyhow!(
                        "Invalid value '{}' for option '{}'. Valid values: {:?}",
                        value,
                        option_id,
                        valid_values
                    ));
                }
            }
        }

        display
            .complications
            .set_option(&face_name, complication_id, option_id, value.to_string());
        display.needs_redraw = true;
        drop(display);

        // Special handling for network interface changes
        if complication_id == faces::complication_names::NETWORK
            && option_id == faces::complication_options::INTERFACE
        {
            let mut sensors = self.sensors.lock().unwrap();
            if value == "auto" || value.is_empty() {
                sensors.network.set_auto();
            } else {
                sensors.network.set_interface(value);
            }
        }

        self.save_display_settings();
        info!(
            "Complication option '{}.{}' set to '{}' for face '{}'",
            complication_id,
            option_id,
            value,
            self.face_name()
        );
        Ok(())
    }
}
