//! Face system for pre-configured layouts.
//!
//! Faces display system information in different styles and colours.
//! Each face supports configurable complications that allow users to
//! enable or disable specific display elements.

#![allow(dead_code)]

mod arcs;
mod ascii;
mod clock;
mod digits;
mod professional;

pub use arcs::ArcsFace;
pub use ascii::AsciiFace;
pub use clock::ClockFace;
pub use digits::DigitsFace;
pub use professional::ProfessionalFace;

use crate::rendering::Canvas;
use crate::sensors::data::SystemData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::PI;

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
                // Solarized Light
                primary: 0x268BD2,    // Blue
                secondary: 0x859900,  // Green
                text: 0x073642,       // Base02 (darker for better contrast)
                background: 0xFDF6E3, // Base3
            },
            "solarized-dark" | "solarized_dark" => Self {
                // Solarized Dark
                primary: 0x268BD2,    // Blue
                secondary: 0x2AA198,  // Cyan (more visible)
                text: 0xEEE8D5,       // Base2 (brighter for better contrast)
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
                text: 0xE0E0FF,      // Brighter foreground
                background: 0x1A1B26,
            },
            // Unknown theme - fall back to nord
            _ => Self::from_preset("nord"),
        }
    }
}

/// Lighten a color by blending it towards white.
fn lighten_color(color: u32, factor: f32) -> u32 {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;

    let r_light = r + (255.0 - r) * factor;
    let g_light = g + (255.0 - g) * factor;
    let b_light = b + (255.0 - b) * factor;

    ((r_light as u32) << 16) | ((g_light as u32) << 8) | (b_light as u32)
}

/// Draws a small analog clock for use in complications.
///
/// This is a minimal clock face with hands, suitable for
/// displaying as a small complication within larger faces.
#[allow(clippy::too_many_arguments)]
pub fn draw_mini_analog_clock(
    canvas: &mut Canvas,
    cx: i32,
    cy: i32,
    radius: u32,
    hour: u8,
    minute: u8,
    primary_color: u32,
    hand_color: u32,
) {
    let radius_f = radius as f32;

    // Draw clock face outline
    canvas.draw_arc(cx, cy, radius, 0.0, 2.0 * PI, 1.5, primary_color);

    // Calculate hand angles (12 o'clock = -PI/2)
    let minute_angle = (minute as f32) * PI / 30.0 - PI / 2.0;
    let hour_angle = ((hour % 12) as f32 + minute as f32 / 60.0) * PI / 6.0 - PI / 2.0;

    // Draw hour hand (shorter, thicker, original color)
    let hour_length = radius_f * 0.5;
    let hour_x = cx as f32 + hour_length * hour_angle.cos();
    let hour_y = cy as f32 + hour_length * hour_angle.sin();
    canvas.draw_line(cx, cy, hour_x as i32, hour_y as i32, 2.5, hand_color);

    // Draw minute hand (longer, thinner, lighter color)
    let minute_color = lighten_color(hand_color, 0.4);
    let minute_length = radius_f * 0.7;
    let minute_x = cx as f32 + minute_length * minute_angle.cos();
    let minute_y = cy as f32 + minute_length * minute_angle.sin();
    canvas.draw_line(cx, cy, minute_x as i32, minute_y as i32, 1.5, minute_color);

    // Draw center dot
    canvas.fill_circle(cx, cy, 2, primary_color);
}

/// Information about an available theme.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Internal identifier used for setting the theme.
    pub id: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
}

/// Returns a list of available theme presets with display names.
pub fn available_themes() -> Vec<ThemeInfo> {
    vec![
        ThemeInfo {
            id: "ember",
            display_name: "Ember",
        },
        ThemeInfo {
            id: "hacker",
            display_name: "Hacker",
        },
        ThemeInfo {
            id: "nord",
            display_name: "Nord",
        },
        ThemeInfo {
            id: "solarized-dark",
            display_name: "Solarized Dark",
        },
        ThemeInfo {
            id: "solarized-light",
            display_name: "Solarized Light",
        },
        ThemeInfo {
            id: "tokyonight",
            display_name: "Tokyo Night",
        },
    ]
}

/// Type of complication option value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplicationOptionType {
    /// A choice from a list of values.
    Choice(Vec<ComplicationChoice>),
    /// A boolean toggle.
    Boolean,
    /// A numeric range (slider). Values are stored as strings but represent f32.
    Range {
        /// Minimum value.
        min: f32,
        /// Maximum value.
        max: f32,
        /// Step increment.
        step: f32,
    },
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Creates a new range-based option (slider).
    pub fn range(
        id: &str,
        name: &str,
        description: &str,
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            option_type: ComplicationOptionType::Range { min, max, step },
            default_value: default.to_string(),
        }
    }
}

/// A complication is an optional display element that can be enabled or disabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Complication IDs used across faces.
pub mod complication_names {
    pub const TIME: &str = "time";
    pub const DATE: &str = "date";
    pub const NETWORK: &str = "network";
    pub const DISK_IO: &str = "disk_io";
    pub const CPU_TEMP: &str = "cpu_temp";
    pub const IP_ADDRESS: &str = "ip_address";
}

/// Complication option IDs.
pub mod complication_options {
    pub const TIME_FORMAT: &str = "format";
    pub const DATE_FORMAT: &str = "format";
    pub const IP_TYPE: &str = "ip_type";
    pub const INTERFACE: &str = "interface";
    pub const SIZE: &str = "size";
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

/// Pre-built complications used across faces.
pub mod complications {
    use super::*;

    /// Time complication with format options.
    pub fn time(default_enabled: bool) -> Complication {
        Complication::with_options(
            complication_names::TIME,
            "Time",
            "Display the current time",
            default_enabled,
            vec![ComplicationOption::choice(
                complication_options::TIME_FORMAT,
                "Format",
                "Time display format",
                vec![
                    ComplicationChoice::new(time_formats::DIGITAL_24H, "Digital (24h)"),
                    ComplicationChoice::new(time_formats::DIGITAL_12H, "Digital (12h)"),
                    ComplicationChoice::new(time_formats::ANALOGUE, "Analogue"),
                ],
                time_formats::DIGITAL_24H,
            )],
        )
    }

    /// Date complication with format options.
    pub fn date(default_enabled: bool, default_format: &str) -> Complication {
        Complication::with_options(
            complication_names::DATE,
            "Date",
            "Display the current date",
            default_enabled,
            vec![ComplicationOption::choice(
                complication_options::DATE_FORMAT,
                "Format",
                "Date display format",
                vec![
                    ComplicationChoice::new(date_formats::ISO, "ISO (2024-01-15)"),
                    ComplicationChoice::new(date_formats::US, "US (01/15/2024)"),
                    ComplicationChoice::new(date_formats::EU, "EU (15/01/2024)"),
                    ComplicationChoice::new(date_formats::SHORT, "Short (Jan 15)"),
                    ComplicationChoice::new(date_formats::LONG, "Long (January 15, 2024)"),
                    ComplicationChoice::new(date_formats::WEEKDAY, "Weekday (Mon, Jan 15)"),
                ],
                default_format,
            )],
        )
    }

    /// IP address complication with type options.
    pub fn ip_address(default_enabled: bool) -> Complication {
        Complication::with_options(
            complication_names::IP_ADDRESS,
            "IP Address",
            "Display network IP address",
            default_enabled,
            vec![ComplicationOption::choice(
                complication_options::IP_TYPE,
                "IP Type",
                "Type of IP address to display",
                vec![
                    ComplicationChoice::new("ipv6-gua", "IPv6 Global"),
                    ComplicationChoice::new("ipv6-lla", "IPv6 Link-Local"),
                    ComplicationChoice::new("ipv6-ula", "IPv6 ULA"),
                    ComplicationChoice::new("ipv4", "IPv4"),
                ],
                "ipv6-gua",
            )],
        )
    }

    /// Network activity complication with interface options.
    pub fn network(default_enabled: bool) -> Complication {
        Complication::with_options(
            complication_names::NETWORK,
            "Network",
            "Display network activity graph",
            default_enabled,
            vec![ComplicationOption::choice(
                complication_options::INTERFACE,
                "Interface",
                "Network interface to monitor",
                vec![ComplicationChoice::new("auto", "Auto-detect")],
                "auto",
            )],
        )
    }

    /// Disk I/O complication.
    pub fn disk_io(default_enabled: bool) -> Complication {
        Complication::new(
            complication_names::DISK_IO,
            "Disk I/O",
            "Display disk read/write activity graph",
            default_enabled,
        )
    }

    /// CPU temperature complication.
    pub fn cpu_temp(default_enabled: bool) -> Complication {
        Complication::new(
            complication_names::CPU_TEMP,
            "CPU Temperature",
            "Display CPU temperature",
            default_enabled,
        )
    }

    /// Hostname complication.
    pub fn hostname(default_enabled: bool) -> Complication {
        Complication::new(
            "hostname",
            "Hostname",
            "Display the system hostname",
            default_enabled,
        )
    }

    /// Digital time complication (replaces analog clock).
    pub fn digital_time(default_enabled: bool) -> Complication {
        Complication::with_options(
            "digital_time",
            "Digital Time",
            "Display the current time in digital format",
            default_enabled,
            vec![ComplicationOption::range(
                complication_options::SIZE,
                "Size",
                "Size of the digital clock display",
                32.0, // min (current default)
                96.0, // max (large enough to fill screen)
                4.0,  // step
                32.0, // default
            )],
        )
    }
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
        "clock" => Some(Box::new(ClockFace::new())),
        "digits" => Some(Box::new(DigitsFace::new())),
        "professional" => Some(Box::new(ProfessionalFace::new())),
        _ => None,
    }
}

/// Information about an available face.
#[derive(Debug, Clone)]
pub struct FaceInfo {
    /// Internal identifier used for setting the face.
    pub id: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
}

/// Returns a list of available faces with display names.
pub fn available_faces() -> Vec<FaceInfo> {
    vec![
        FaceInfo {
            id: "arcs",
            display_name: "Arcs",
        },
        FaceInfo {
            id: "ascii",
            display_name: "ASCII",
        },
        FaceInfo {
            id: "clock",
            display_name: "Clock",
        },
        FaceInfo {
            id: "digits",
            display_name: "Digits",
        },
        FaceInfo {
            id: "professional",
            display_name: "Professional",
        },
    ]
}

/// Returns available complications for a face by name.
pub fn face_complications(name: &str) -> Vec<Complication> {
    create_face(name)
        .map(|f| f.available_complications())
        .unwrap_or_default()
}
