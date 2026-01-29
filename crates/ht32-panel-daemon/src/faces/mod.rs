//! Face system for pre-configured layouts.
//!
//! Faces display system information in different styles and colours.

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

/// Trait for display faces.
pub trait Face: Send + Sync {
    /// Returns the name of the face.
    fn name(&self) -> &str;

    /// Renders the face onto the canvas using current system data and theme.
    fn render(&self, canvas: &mut Canvas, data: &SystemData, theme: &Theme);
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
