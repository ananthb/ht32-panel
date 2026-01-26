//! Text rendering using fontdue.

use fontdue::{Font, FontSettings};
use tiny_skia::Pixmap;

/// Embedded DejaVu Sans Mono font.
const FONT_DATA: &[u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");

/// Text renderer using fontdue for rasterization.
pub struct TextRenderer {
    font: Font,
}

impl TextRenderer {
    /// Creates a new text renderer with the embedded font.
    pub fn new() -> Self {
        let font = Font::from_bytes(FONT_DATA, FontSettings::default())
            .expect("Failed to load embedded font");
        Self { font }
    }

    /// Draws text onto a pixmap at the specified position.
    ///
    /// # Arguments
    /// * `pixmap` - The pixmap to draw onto
    /// * `x` - X position (left edge of text)
    /// * `y` - Y position (top edge of text)
    /// * `text` - The text to render
    /// * `size` - Font size in pixels
    /// * `color` - RGB888 color (0xRRGGBB)
    pub fn draw_text(
        &self,
        pixmap: &mut Pixmap,
        x: i32,
        y: i32,
        text: &str,
        size: f32,
        color: u32,
    ) {
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;

        let mut cursor_x = x;

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);

            // Draw the glyph bitmap
            for glyph_y in 0..metrics.height {
                for glyph_x in 0..metrics.width {
                    let coverage = bitmap[glyph_y * metrics.width + glyph_x];
                    if coverage > 0 {
                        let px = cursor_x + metrics.xmin + glyph_x as i32;
                        let py = y
                            + (size as i32 - metrics.ymin - metrics.height as i32)
                            + glyph_y as i32;

                        if px >= 0
                            && py >= 0
                            && (px as u32) < pixmap.width()
                            && (py as u32) < pixmap.height()
                        {
                            let idx = (py as u32 * pixmap.width() + px as u32) as usize * 4;
                            let data = pixmap.data_mut();

                            // Alpha blend the glyph
                            let alpha = coverage as f32 / 255.0;
                            let inv_alpha = 1.0 - alpha;

                            data[idx] = (r as f32 * alpha + data[idx] as f32 * inv_alpha) as u8;
                            data[idx + 1] =
                                (g as f32 * alpha + data[idx + 1] as f32 * inv_alpha) as u8;
                            data[idx + 2] =
                                (b as f32 * alpha + data[idx + 2] as f32 * inv_alpha) as u8;
                            data[idx + 3] = 255; // Full opacity
                        }
                    }
                }
            }

            cursor_x += metrics.advance_width as i32;
        }
    }

    /// Returns the width of text when rendered at the specified size.
    pub fn text_width(&self, text: &str, size: f32) -> i32 {
        text.chars()
            .map(|ch| {
                let (metrics, _) = self.font.rasterize(ch, size);
                metrics.advance_width as i32
            })
            .sum()
    }

    /// Returns the line height for the specified font size.
    pub fn line_height(&self, size: f32) -> i32 {
        // fontdue doesn't provide line metrics directly, approximate
        (size * 1.2) as i32
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_renderer_creation() {
        let renderer = TextRenderer::new();
        let width = renderer.text_width("Hello", 16.0);
        assert!(width > 0);
    }

    #[test]
    fn test_draw_text() {
        let renderer = TextRenderer::new();
        let mut pixmap = Pixmap::new(100, 50).unwrap();
        renderer.draw_text(&mut pixmap, 10, 10, "Test", 14.0, 0xFFFFFF);
        // Just verify no panic
    }
}
