//! Face system for pre-configured widget layouts.
//!
//! Faces are like watch faces - pre-configured layouts of widgets
//! that display system information in different styles.

#![allow(dead_code)]

mod detailed;
mod minimal;

pub use detailed::DetailedFace;
pub use minimal::MinimalFace;

use crate::rendering::Canvas;
use crate::sensors::data::SystemData;

/// Trait for display faces.
pub trait Face: Send + Sync {
    /// Returns the name of the face.
    fn name(&self) -> &str;

    /// Renders the face onto the canvas using current system data.
    fn render(&self, canvas: &mut Canvas, data: &SystemData);
}

/// Creates a face by name.
pub fn create_face(name: &str) -> Option<Box<dyn Face>> {
    match name.to_lowercase().as_str() {
        "minimal" => Some(Box::new(MinimalFace::new())),
        "detailed" => Some(Box::new(DetailedFace::new())),
        _ => None,
    }
}

/// Returns a list of available face names.
pub fn available_faces() -> Vec<&'static str> {
    vec!["minimal", "detailed"]
}
