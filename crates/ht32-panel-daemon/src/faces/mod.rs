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
use std::collections::{HashMap, HashSet};

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
    pub fn from_preset(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "hacker" => Self {
                // Matrix-like green on black
                primary: 0x00FF00,   // Bright green
                secondary: 0x00AA00, // Darker green
                text: 0x00FF00,      // Green text
                background: 0x000000,
            },
            "solarized-light" | "solarized_light" => Self {
                // Solarized Light
                primary: 0x268BD2,    // Blue
                secondary: 0x859900,  // Green
                text: 0x657B83,       // Base00 (dark gray for light bg)
                background: 0xFDF6E3, // Base3
            },
            "solarized-dark" | "solarized_dark" => Self {
                // Solarized Dark
                primary: 0x268BD2,    // Blue
                secondary: 0x859900,  // Green
                text: 0x839496,       // Base0 (light gray for dark bg)
                background: 0x002B36, // Base03
            },
            "nord" => Self {
                // Nord
                primary: 0x88C0D0,    // Nord8 (frost cyan)
                secondary: 0x81A1C1,  // Nord9 (frost blue)
                text: 0xECEFF4,       // Nord6 (snow storm white)
                background: 0x2E3440, // Nord0
            },
            "tokyonight" | "tokyo-night" | "tokyo_night" => Self {
                // Tokyo Night
                primary: 0x7AA2F7,   // Blue
                secondary: 0xBB9AF7, // Magenta
                text: 0xC0CAF5,      // Foreground
                background: 0x1A1B26,
            },
            _ => Self {
                // Default - cyan/coral on dark blue-gray
                primary: 0x00DDDD,
                secondary: 0xFF6B6B,
                text: 0xFFFFFF,
                background: 0x1A1A2E,
            },
        }
    }
}

/// Returns a list of available theme preset names.
pub fn available_themes() -> Vec<&'static str> {
    vec![
        "default",
        "hacker",
        "solarized-light",
        "solarized-dark",
        "nord",
        "tokyonight",
    ]
}

/// A complication is an optional display element that can be enabled or disabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Complication {
    /// Unique identifier for this complication.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this complication shows.
    pub description: String,
    /// Whether this complication is enabled by default.
    pub default_enabled: bool,
}

impl Complication {
    /// Creates a new complication.
    pub fn new(id: &str, name: &str, description: &str, default_enabled: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            default_enabled,
        }
    }
}

/// Standard complication IDs used across faces.
pub mod complications {
    pub const NETWORK: &str = "network";
    pub const DISK_IO: &str = "disk_io";
    pub const CPU_TEMP: &str = "cpu_temp";
    pub const IP_ADDRESS: &str = "ip_address";
    pub const UPTIME: &str = "uptime";
    pub const HOSTNAME: &str = "hostname";
    pub const TIME: &str = "time";
    pub const CPU: &str = "cpu";
    pub const RAM: &str = "ram";
}

/// Set of enabled complications for rendering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnabledComplications {
    /// Map of face name to set of enabled complication IDs.
    #[serde(default)]
    face_complications: HashMap<String, HashSet<String>>,
}

impl EnabledComplications {
    /// Creates a new empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a complication is enabled for a face.
    /// If the face has no explicit settings, returns the complication's default.
    pub fn is_enabled(&self, face: &str, complication_id: &str, default: bool) -> bool {
        if let Some(enabled) = self.face_complications.get(face) {
            enabled.contains(complication_id)
        } else {
            default
        }
    }

    /// Sets whether a complication is enabled for a face.
    pub fn set_enabled(&mut self, face: &str, complication_id: &str, enabled: bool) {
        let face_set = self
            .face_complications
            .entry(face.to_string())
            .or_default();
        if enabled {
            face_set.insert(complication_id.to_string());
        } else {
            face_set.remove(complication_id);
        }
    }

    /// Initializes complications for a face from its defaults.
    pub fn init_from_defaults(&mut self, face: &dyn Face) {
        let face_name = face.name();
        if !self.face_complications.contains_key(face_name) {
            let mut enabled = HashSet::new();
            for comp in face.available_complications() {
                if comp.default_enabled {
                    enabled.insert(comp.id.clone());
                }
            }
            self.face_complications.insert(face_name.to_string(), enabled);
        }
    }

    /// Gets all enabled complication IDs for a face.
    pub fn get_enabled(&self, face: &str) -> HashSet<String> {
        self.face_complications
            .get(face)
            .cloned()
            .unwrap_or_default()
    }

    /// Sets all enabled complications for a face at once.
    pub fn set_all(&mut self, face: &str, enabled: HashSet<String>) {
        self.face_complications.insert(face.to_string(), enabled);
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
