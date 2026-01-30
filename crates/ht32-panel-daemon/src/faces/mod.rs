//! Face system for pre-configured layouts.
//!
//! Faces display system information in different styles and colours.
//! Each face supports configurable complications that allow users to
//! enable or disable specific display elements.

#![allow(dead_code)]

mod arcs;
mod ascii;
mod digits;
mod professional;

pub use arcs::ArcsFace;
pub use ascii::AsciiFace;
pub use digits::DigitsFace;
pub use professional::ProfessionalFace;

use crate::rendering::Canvas;
use crate::sensors::data::SystemData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Color theme for face rendering.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Primary color (used for highlights, interface names) - RGB888
    pub primary: u32,
    /// Secondary color (used for accents) - RGB888
    pub secondary: u32,
    /// Main text color - RGB888
    pub text: u32,
    /// Background color - RGB888
    pub background: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_preset("default")
    }
}

impl Theme {
    /// Creates a theme from a preset name.
    /// All themes are designed for good contrast ratios (WCAG AA compliant).
    pub fn from_preset(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "hacker" => Self {
                // Matrix-like green on black - high contrast
                primary: 0x00FF00,   // Bright green
                secondary: 0x00DD00, // Slightly darker green
                text: 0x00FF00,      // Green text
                background: 0x000000,
            },
            "ember" | "fire" => Self {
                // Red/orange warm theme
                primary: 0xFF6B35,   // Bright orange
                secondary: 0xFF4444, // Red
                text: 0xFFEEDD,      // Warm white
                background: 0x1A0A00,
            },
            "solarized-light" | "solarized_light" => Self {
                // Solarized Light - adjusted for better contrast
                primary: 0x268BD2,    // Blue
                secondary: 0x859900,  // Green
                text: 0x073642,       // Base02 (darker for better contrast)
                background: 0xFDF6E3, // Base3
            },
            "solarized-dark" | "solarized_dark" => Self {
                // Solarized Dark - adjusted for better contrast
                primary: 0x268BD2,    // Blue
                secondary: 0x2AA198,  // Cyan (more visible)
                text: 0xEEE8D5,       // Base2 (brighter for better contrast)
                background: 0x002B36, // Base03
            },
            "nord" => Self {
                // Nord - already good contrast
                primary: 0x88C0D0,    // Nord8 (frost cyan)
                secondary: 0x81A1C1,  // Nord9 (frost blue)
                text: 0xECEFF4,       // Nord6 (snow storm white)
                background: 0x2E3440, // Nord0
            },
            "tokyonight" | "tokyo-night" | "tokyo_night" => Self {
                // Tokyo Night - brightened text
                primary: 0x7AA2F7,   // Blue
                secondary: 0xBB9AF7, // Magenta
                text: 0xE0E0FF,      // Brighter foreground
                background: 0x1A1B26,
            },
            // Unknown theme - fall back to nord
            _ => Self::from_preset("nord"),
        }
    }
}

/// Returns a list of available theme preset names.
pub fn available_themes() -> Vec<&'static str> {
    vec![
        "ember",
        "hacker",
        "nord",
        "solarized-dark",
        "solarized-light",
        "tokyonight",
    ]
}

/// Type of complication option value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplicationOptionType {
    /// A choice from a list of values.
    Choice(Vec<ComplicationChoice>),
    /// A boolean toggle.
    Boolean,
}

/// A choice value for a complication option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComplicationChoice {
    /// The stored value.
    pub value: String,
    /// Human-readable label.
    pub label: String,
}

impl ComplicationChoice {
    /// Creates a new choice.
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
        }
    }
}

/// An option that can be configured for a complication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComplicationOption {
    /// Unique identifier for this option.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this option controls.
    pub description: String,
    /// The type of this option (choice or boolean).
    pub option_type: ComplicationOptionType,
    /// Default value for this option.
    pub default_value: String,
}

impl ComplicationOption {
    /// Creates a new choice-based option.
    pub fn choice(
        id: &str,
        name: &str,
        description: &str,
        choices: Vec<ComplicationChoice>,
        default: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            option_type: ComplicationOptionType::Choice(choices),
            default_value: default.to_string(),
        }
    }
}

/// A complication is an optional display element that can be enabled or disabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Complication {
    /// Unique identifier for this complication.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this complication shows.
    pub description: String,
    /// Whether this complication is enabled by default.
    pub default_enabled: bool,
    /// Configuration options for this complication.
    #[serde(default)]
    pub options: Vec<ComplicationOption>,
}

impl Complication {
    /// Creates a new complication without options.
    pub fn new(id: &str, name: &str, description: &str, default_enabled: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            default_enabled,
            options: Vec::new(),
        }
    }

    /// Creates a new complication with options.
    pub fn with_options(
        id: &str,
        name: &str,
        description: &str,
        default_enabled: bool,
        options: Vec<ComplicationOption>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            default_enabled,
            options,
        }
    }
}

/// Standard complication IDs used across faces.
pub mod complications {
    pub const TIME: &str = "time";
    pub const DATE: &str = "date";
    pub const NETWORK: &str = "network";
    pub const DISK_IO: &str = "disk_io";
    pub const CPU_TEMP: &str = "cpu_temp";
    pub const IP_ADDRESS: &str = "ip_address";
}

/// Standard complication option IDs.
pub mod complication_options {
    pub const TIME_FORMAT: &str = "format";
    pub const DATE_FORMAT: &str = "format";
    pub const IP_TYPE: &str = "ip_type";
    pub const INTERFACE: &str = "interface";
}

/// Time format options.
pub mod time_formats {
    pub const DIGITAL_24H: &str = "digital-24h";
    pub const DIGITAL_12H: &str = "digital-12h";
    pub const ANALOGUE: &str = "analogue";
}

/// Date format options.
pub mod date_formats {
    pub const ISO: &str = "iso"; // 2024-01-15
    pub const US: &str = "us"; // 01/15/2024
    pub const EU: &str = "eu"; // 15/01/2024
    pub const SHORT: &str = "short"; // Jan 15
    pub const LONG: &str = "long"; // January 15, 2024
    pub const WEEKDAY: &str = "weekday"; // Mon, Jan 15
}

/// Configuration for a single complication instance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComplicationConfig {
    /// Whether this complication is enabled.
    pub enabled: bool,
    /// Option values for this complication.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// Set of enabled complications with their configurations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnabledComplications {
    /// Map of face name to map of complication ID to configuration.
    #[serde(default)]
    face_complications: HashMap<String, HashMap<String, ComplicationConfig>>,
}

impl EnabledComplications {
    /// Creates a new empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a complication is enabled for a face.
    /// If the face has no explicit settings, returns the complication's default.
    pub fn is_enabled(&self, face: &str, complication_id: &str, default: bool) -> bool {
        if let Some(configs) = self.face_complications.get(face) {
            configs
                .get(complication_id)
                .map(|c| c.enabled)
                .unwrap_or(default)
        } else {
            default
        }
    }

    /// Sets whether a complication is enabled for a face.
    pub fn set_enabled(&mut self, face: &str, complication_id: &str, enabled: bool) {
        let face_map = self.face_complications.entry(face.to_string()).or_default();
        let config = face_map.entry(complication_id.to_string()).or_default();
        config.enabled = enabled;
    }

    /// Initializes complications for a face from its defaults.
    pub fn init_from_defaults(&mut self, face: &dyn Face) {
        let face_name = face.name();
        if !self.face_complications.contains_key(face_name) {
            let mut configs = HashMap::new();
            for comp in face.available_complications() {
                let mut config = ComplicationConfig {
                    enabled: comp.default_enabled,
                    options: HashMap::new(),
                };
                // Initialize options with defaults
                for opt in &comp.options {
                    config
                        .options
                        .insert(opt.id.clone(), opt.default_value.clone());
                }
                configs.insert(comp.id.clone(), config);
            }
            self.face_complications
                .insert(face_name.to_string(), configs);
        }
    }

    /// Gets all enabled complication IDs for a face.
    pub fn get_enabled(&self, face: &str) -> std::collections::HashSet<String> {
        self.face_complications
            .get(face)
            .map(|configs| {
                configs
                    .iter()
                    .filter(|(_, c)| c.enabled)
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets an option value for a complication.
    pub fn get_option(
        &self,
        face: &str,
        complication_id: &str,
        option_id: &str,
    ) -> Option<&String> {
        self.face_complications
            .get(face)
            .and_then(|configs| configs.get(complication_id))
            .and_then(|config| config.options.get(option_id))
    }

    /// Sets an option value for a complication.
    pub fn set_option(
        &mut self,
        face: &str,
        complication_id: &str,
        option_id: &str,
        value: String,
    ) {
        let face_map = self.face_complications.entry(face.to_string()).or_default();
        let config = face_map.entry(complication_id.to_string()).or_default();
        config.options.insert(option_id.to_string(), value);
    }

    /// Gets the full configuration for a complication.
    pub fn get_config(&self, face: &str, complication_id: &str) -> Option<&ComplicationConfig> {
        self.face_complications
            .get(face)
            .and_then(|configs| configs.get(complication_id))
    }
}

/// Trait for display faces.
pub trait Face: Send + Sync {
    /// Returns the name of the face.
    fn name(&self) -> &str;

    /// Returns the list of available complications for this face.
    fn available_complications(&self) -> Vec<Complication>;

    /// Renders the face onto the canvas using current system data, theme,
    /// and enabled complications.
    fn render(
        &self,
        canvas: &mut Canvas,
        data: &SystemData,
        theme: &Theme,
        complications: &EnabledComplications,
    );
}

/// Creates a face by name.
pub fn create_face(name: &str) -> Option<Box<dyn Face>> {
    match name.to_lowercase().as_str() {
        "arcs" => Some(Box::new(ArcsFace::new())),
        "ascii" => Some(Box::new(AsciiFace::new())),
        "digits" => Some(Box::new(DigitsFace::new())),
        "professional" => Some(Box::new(ProfessionalFace::new())),
        _ => None,
    }
}

/// Returns a list of available face names.
pub fn available_faces() -> Vec<&'static str> {
    vec!["arcs", "ascii", "digits", "professional"]
}

/// Returns available complications for a face by name.
pub fn face_complications(name: &str) -> Vec<Complication> {
    create_face(name)
        .map(|f| f.available_complications())
        .unwrap_or_default()
}
