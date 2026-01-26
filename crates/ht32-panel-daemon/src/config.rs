//! Configuration management.

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Web server configuration
    #[serde(default)]
    pub web: WebConfig,

    /// D-Bus configuration
    #[serde(default)]
    pub dbus: DbusConfig,

    /// Path to theme configuration
    #[serde(default = "default_theme")]
    pub theme: String,

    /// State directory for persisting runtime state
    #[serde(default = "default_state_dir")]
    pub state_dir: String,

    /// Render loop poll interval in milliseconds
    #[serde(default = "default_poll")]
    pub poll: u64,

    /// Display refresh rate in milliseconds
    #[serde(default = "default_refresh")]
    pub refresh: u64,

    /// Heartbeat interval in milliseconds
    #[serde(default = "default_heartbeat")]
    pub heartbeat: u64,

    /// LCD configuration
    #[serde(default)]
    pub lcd: LcdConfig,

    /// LED configuration
    #[serde(default)]
    pub led: LedConfig,

    /// Canvas configuration
    #[serde(default)]
    pub canvas: CanvasConfig,

    /// Display configuration
    #[serde(default)]
    pub display: DisplayConfig,
}

/// Web server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Whether to enable the web server
    #[serde(default)]
    pub enable: bool,

    /// Server listen address (e.g., "0.0.0.0:8686")
    #[serde(default = "default_listen")]
    pub listen: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enable: false,
            listen: default_listen(),
        }
    }
}

/// D-Bus bus type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DbusBusType {
    /// Automatically detect: try session bus first, fall back to system bus.
    #[default]
    Auto,
    /// Use the session bus (for user services).
    Session,
    /// Use the system bus (for system services).
    System,
}

/// D-Bus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbusConfig {
    /// Which D-Bus bus to use.
    #[serde(default)]
    pub bus: DbusBusType,
}

impl Default for DbusConfig {
    fn default() -> Self {
        Self {
            bus: DbusBusType::Auto,
        }
    }
}

/// LCD device configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LcdConfig {
    /// Device path or "auto" for auto-detection
    #[serde(default = "default_lcd_device")]
    pub device: String,
}

/// LED device configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedConfig {
    /// Serial port path
    #[serde(default = "default_led_device")]
    pub device: String,

    /// Current theme (1-5)
    #[serde(default = "default_led_theme")]
    pub theme: u8,

    /// Intensity (1-5)
    #[serde(default = "default_led_value")]
    pub intensity: u8,

    /// Speed (1-5)
    #[serde(default = "default_led_value")]
    pub speed: u8,
}

impl Default for LedConfig {
    fn default() -> Self {
        Self {
            device: default_led_device(),
            theme: default_led_theme(),
            intensity: default_led_value(),
            speed: default_led_value(),
        }
    }
}

/// Canvas configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    /// Canvas width
    #[serde(default = "default_width")]
    pub width: u32,

    /// Canvas height
    #[serde(default = "default_height")]
    pub height: u32,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
        }
    }
}

/// Display configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Network interface to monitor (e.g., "eth0", "wlan0")
    #[serde(default)]
    pub network_interface: Option<String>,
}

// Default value functions
fn default_listen() -> String {
    "[::1]:8686".to_string()
}

fn default_theme() -> String {
    "themes/default.toml".to_string()
}

fn default_state_dir() -> String {
    // Check STATE_DIRECTORY first (set by systemd when StateDirectory= is configured)
    // Then fall back to XDG state directory or /var/lib
    if let Ok(state_dir) = std::env::var("STATE_DIRECTORY") {
        state_dir
    } else if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        format!("{}/ht32-panel", state_home)
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.local/state/ht32-panel", home)
    } else {
        "/var/lib/ht32-panel".to_string()
    }
}

fn default_poll() -> u64 {
    500
}

fn default_refresh() -> u64 {
    1600
}

fn default_heartbeat() -> u64 {
    60000
}

fn default_lcd_device() -> String {
    "auto".to_string()
}

fn default_led_device() -> String {
    "/dev/ttyUSB0".to_string()
}

fn default_led_theme() -> u8 {
    2 // Breathing
}

fn default_led_value() -> u8 {
    3
}

fn default_width() -> u32 {
    320
}

fn default_height() -> u32 {
    170
}

impl Config {
    /// Loads configuration from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read configuration file")?;
        let config: Config = toml::from_str(&content).context("Failed to parse configuration")?;
        Ok(config)
    }

    /// Saves configuration to a TOML file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;
        std::fs::write(path.as_ref(), content).context("Failed to write configuration file")?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            web: WebConfig::default(),
            dbus: DbusConfig::default(),
            theme: default_theme(),
            state_dir: default_state_dir(),
            poll: default_poll(),
            refresh: default_refresh(),
            heartbeat: default_heartbeat(),
            lcd: LcdConfig::default(),
            led: LedConfig::default(),
            canvas: CanvasConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}
