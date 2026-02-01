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

    /// State directory for persisting runtime state
    #[serde(default = "default_state_dir")]
    pub state_dir: String,

    /// Display refresh rate in Hz
    #[serde(default = "default_refresh_rate")]
    pub refresh_rate: u64,

    /// Heartbeat interval in milliseconds
    #[serde(default = "default_heartbeat")]
    pub heartbeat: u64,

    /// Device configuration
    #[serde(default)]
    pub devices: DevicesConfig,

    /// Canvas configuration
    #[serde(default)]
    pub canvas: CanvasConfig,
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

/// Device configuration for LCD and LED hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicesConfig {
    /// LCD device path or "auto" for auto-detection
    #[serde(default = "default_lcd_device")]
    pub lcd: String,

    /// LED serial port path
    #[serde(default = "default_led_device")]
    pub led: String,
}

impl Default for DevicesConfig {
    fn default() -> Self {
        Self {
            lcd: default_lcd_device(),
            led: default_led_device(),
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

// Default value functions
fn default_listen() -> String {
    "[::1]:8686".to_string()
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

fn default_refresh_rate() -> u64 {
    2
}

fn default_heartbeat() -> u64 {
    1000
}

fn default_lcd_device() -> String {
    "auto".to_string()
}

fn default_led_device() -> String {
    "/dev/ttyUSB0".to_string()
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
            state_dir: default_state_dir(),
            refresh_rate: default_refresh_rate(),
            heartbeat: default_heartbeat(),
            devices: DevicesConfig::default(),
            canvas: CanvasConfig::default(),
        }
    }
}
