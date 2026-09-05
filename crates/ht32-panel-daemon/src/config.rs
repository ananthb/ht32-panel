//! Configuration management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, warn};

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

    /// Display refresh interval in milliseconds (500-10000)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,

    /// Heartbeat interval in milliseconds
    #[serde(default = "default_heartbeat")]
    pub heartbeat: u64,

    /// Consecutive LCD write failures before declaring a disconnect.
    #[serde(default = "default_lcd_failure_threshold")]
    pub lcd_failure_threshold: u32,

    /// Minimum interval between LCD reopen attempts (ms).
    #[serde(default = "default_lcd_reconnect_interval_ms")]
    pub lcd_reconnect_interval_ms: u64,

    /// Dark-time before exiting for systemd to relaunch (ms); 0 disables.
    #[serde(default = "default_lcd_exit_after_ms")]
    pub lcd_exit_after_ms: u64,

    /// Throttled error-log cadence (ms).
    #[serde(default = "default_lcd_error_log_interval_ms")]
    pub lcd_error_log_interval_ms: u64,

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

fn default_refresh_interval() -> u64 {
    2500
}

fn default_heartbeat() -> u64 {
    1000
}

fn default_lcd_failure_threshold() -> u32 {
    10
}

fn default_lcd_reconnect_interval_ms() -> u64 {
    5_000
}

fn default_lcd_exit_after_ms() -> u64 {
    300_000
}

fn default_lcd_error_log_interval_ms() -> u64 {
    60_000
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

/// Keys that belong at the top level of the file (above any `[section]`).
const TOP_LEVEL_KEYS: &[&str] = &[
    "state_dir",
    "refresh_interval",
    "heartbeat",
    "lcd_failure_threshold",
    "lcd_reconnect_interval_ms",
    "lcd_exit_after_ms",
    "lcd_error_log_interval_ms",
];

/// Each `[section]` and the keys it accepts.
const TABLE_KEYS: &[(&str, &[&str])] = &[
    ("web", &["enable", "listen"]),
    ("dbus", &["bus"]),
    ("devices", &["lcd", "led"]),
    ("canvas", &["width", "height"]),
];

/// Finds keys the daemon will silently ignore.
///
/// TOML is section-scoped: a top-level key written *below* a `[section]` header
/// is parsed as a member of that section, and serde then drops it because the
/// section's struct has no such field. The setting looks present in the file but
/// has no effect. This walks the parsed document and reports every ignored key,
/// calling out that specific mistake so the fix is obvious.
fn unknown_key_warnings(doc: &toml::Value) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(root) = doc.as_table() else {
        return warnings;
    };

    for (key, value) in root {
        if let Some(table) = value.as_table() {
            // A known section: check the keys inside it.
            if let Some((_, allowed)) = TABLE_KEYS.iter().find(|(name, _)| name == key) {
                for inner in table.keys() {
                    if allowed.contains(&inner.as_str()) {
                        continue;
                    }
                    if TOP_LEVEL_KEYS.contains(&inner.as_str()) {
                        warnings.push(format!(
                            "`{inner}` is set under [{key}] and is being IGNORED: it is a \
                             top-level setting, so it must appear ABOVE the first [section] \
                             header in the file"
                        ));
                    } else {
                        warnings.push(format!("unknown key `{inner}` under [{key}] is ignored"));
                    }
                }
            } else {
                warnings.push(format!("unknown section [{key}] is ignored"));
            }
        } else if !TOP_LEVEL_KEYS.contains(&key.as_str()) {
            warnings.push(format!("unknown top-level key `{key}` is ignored"));
        }
    }

    warnings.sort();
    warnings
}

impl Config {
    /// Loads configuration from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read configuration file")?;

        // Warn (don't fail) about settings that parse but will never be applied,
        // so an upgrade never dies on a stale config file.
        match toml::from_str::<toml::Value>(&content) {
            Ok(doc) => {
                for w in unknown_key_warnings(&doc) {
                    warn!("Config: {}", w);
                }
            }
            Err(e) => debug!("Could not pre-scan config for unknown keys: {}", e),
        }

        let config: Config = toml::from_str(&content).context("Failed to parse configuration")?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            web: WebConfig::default(),
            dbus: DbusConfig::default(),
            state_dir: default_state_dir(),
            refresh_interval: default_refresh_interval(),
            heartbeat: default_heartbeat(),
            lcd_failure_threshold: default_lcd_failure_threshold(),
            lcd_reconnect_interval_ms: default_lcd_reconnect_interval_ms(),
            lcd_exit_after_ms: default_lcd_exit_after_ms(),
            lcd_error_log_interval_ms: default_lcd_error_log_interval_ms(),
            devices: DevicesConfig::default(),
            canvas: CanvasConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped example must keep top-level keys above the first [section],
    /// otherwise TOML scopes them into that section and they are silently
    /// dropped. This is the exact defect that made `heartbeat` un-settable.
    #[test]
    fn shipped_default_config_applies_its_own_values() {
        let text = include_str!("../../../config/default.toml");
        let cfg: Config = toml::from_str(text).expect("default.toml must parse");

        assert_eq!(cfg.refresh_interval, 2500);
        assert_eq!(cfg.heartbeat, 1000);
        assert_eq!(cfg.canvas.width, 320);
        assert_eq!(cfg.devices.lcd, "auto");

        let doc: toml::Value = toml::from_str(text).unwrap();
        assert!(
            unknown_key_warnings(&doc).is_empty(),
            "shipped default.toml has ignored keys: {:?}",
            unknown_key_warnings(&doc)
        );
    }

    /// A non-default value in the file must actually reach the daemon.
    #[test]
    fn top_level_heartbeat_is_honoured() {
        let cfg: Config = toml::from_str("heartbeat = 30000\n[web]\nenable = false\n").unwrap();
        assert_eq!(cfg.heartbeat, 30000);
    }

    /// Raja's report: `heartbeat` written under [web] parses fine but is dropped.
    /// We cannot make it apply (that would change TOML semantics) but we must say so.
    #[test]
    fn heartbeat_misplaced_under_web_is_reported() {
        let text = "[web]\nenable = false\nheartbeat = 30000\n";
        let cfg: Config = toml::from_str(text).unwrap();
        assert_eq!(cfg.heartbeat, 1000, "misplaced key must not apply");

        let doc: toml::Value = toml::from_str(text).unwrap();
        let warnings = unknown_key_warnings(&doc);
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("heartbeat") && warnings[0].contains("IGNORED"),
            "unhelpful warning: {}",
            warnings[0]
        );
    }

    #[test]
    fn unknown_keys_and_sections_are_reported() {
        let doc: toml::Value =
            toml::from_str("bogus = 1\n[web]\ntypo = true\n[nope]\nx = 1\n").unwrap();
        let warnings = unknown_key_warnings(&doc);
        assert_eq!(warnings.len(), 3, "got: {warnings:?}");
    }

    #[test]
    fn resilience_defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.lcd_failure_threshold, 10);
        assert_eq!(c.lcd_reconnect_interval_ms, 5_000);
        assert_eq!(c.lcd_exit_after_ms, 300_000);
        assert_eq!(c.lcd_error_log_interval_ms, 60_000);
    }
}
