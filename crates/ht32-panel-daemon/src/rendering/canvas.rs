//! Canvas for rendering to framebuffer.

use anyhow::Result;
use ht32_panel_hw::lcd::framebuffer::{rgb888_to_rgb565, Framebuffer};
use std::collections::VecDeque;
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use super::text::TextRenderer;

/// Brightens a color by the given factor.
fn brighten_color(color: u32, factor: f32) -> u32 {
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >> 8) & 0xFF) as f32;
    let b = (color & 0xFF) as f32;

    let r = ((r * factor).min(255.0)) as u32;
    let g = ((g * factor).min(255.0)) as u32;
    let b = ((b * factor).min(255.0)) as u32;

    (r << 16) | (g << 8) | b
}

/// Canvas for rendering.
pub struct Canvas {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    background_color: u32,
    text_renderer: TextRenderer,
}

impl Canvas {
    /// Creates a new canvas.
    pub fn new(width: u32, height: u32) -> Self {
        let pixmap = Pixmap::new(width, height).expect("Failed to create pixmap");

        Self {
            width,
            height,
            pixmap,
            background_color: 0x000000, // Black
            text_renderer: TextRenderer::new(),
        }
    }

    /// Returns the canvas dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Resizes the canvas to new dimensions.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.pixmap = Pixmap::new(width, height).expect("Failed to create pixmap");
        }
    }

    /// Sets the background color.
    pub fn set_background(&mut self, color: u32) {
        self.background_color = color;
    }

    /// Clears the canvas.
    pub fn clear(&mut self) {
        let r = ((self.background_color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((self.background_color >> 8) & 0xFF) as f32 / 255.0;
        let b = (self.background_color & 0xFF) as f32 / 255.0;
        self.pixmap.fill(Color::from_rgba(r, g, b, 1.0).unwrap());
    }

    /// Draws a filled rectangle.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        debug_assert!(
            x >= 0 && y >= 0,
            "fill_rect: negative coordinates ({}, {})",
            x,
            y
        );
        debug_assert!(
            x + width as i32 <= self.width as i32,
            "fill_rect: x overflow ({} + {} > {})",
            x,
            width,
            self.width
        );
        debug_assert!(
            y + height as i32 <= self.height as i32,
            "fill_rect: y overflow ({} + {} > {})",
            y,
            height,
            self.height
        );

        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());

        if let Some(rect) = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
        }
    }

    /// Draws a filled circle.
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: u32, color: u32) {
        debug_assert!(
            cx - radius as i32 >= 0,
            "fill_circle: left edge off screen ({} - {} < 0)",
            cx,
            radius
        );
        debug_assert!(
            cy - radius as i32 >= 0,
            "fill_circle: top edge off screen ({} - {} < 0)",
            cy,
            radius
        );
        debug_assert!(
            cx + radius as i32 <= self.width as i32,
            "fill_circle: right edge off screen ({} + {} > {})",
            cx,
            radius,
            self.width
        );
        debug_assert!(
            cy + radius as i32 <= self.height as i32,
            "fill_circle: bottom edge off screen ({} + {} > {})",
            cy,
            radius,
            self.height
        );

        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
        paint.anti_alias = true;

        let mut pb = PathBuilder::new();
        pb.push_circle(cx as f32, cy as f32, radius as f32);
        if let Some(path) = pb.finish() {
            self.pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    /// Draws a line between two points.
    ///
    /// # Arguments
    /// * `x1`, `y1` - Start point
    /// * `x2`, `y2` - End point
    /// * `stroke_width` - Width of the line
    /// * `color` - RGB888 color
    #[allow(clippy::too_many_arguments)]
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, stroke_width: f32, color: u32) {
        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
        paint.anti_alias = true;

        let stroke = Stroke {
            width: stroke_width,
            line_cap: tiny_skia::LineCap::Round,
            ..Default::default()
        };

        let mut pb = PathBuilder::new();
        pb.move_to(x1 as f32, y1 as f32);
        pb.line_to(x2 as f32, y2 as f32);

        if let Some(path) = pb.finish() {
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Draws an arc (unfilled, stroke only).
    ///
    /// # Arguments
    /// * `cx`, `cy` - Center of the arc
    /// * `radius` - Radius of the arc
    /// * `start_angle` - Start angle in radians (0 = right, PI/2 = down)
    /// * `end_angle` - End angle in radians
    /// * `stroke_width` - Width of the arc stroke
    /// * `color` - RGB888 color
    #[allow(clippy::too_many_arguments)]
    pub fn draw_arc(
        &mut self,
        cx: i32,
        cy: i32,
        radius: u32,
        start_angle: f32,
        end_angle: f32,
        stroke_width: f32,
        color: u32,
    ) {
        let total_radius = radius as i32 + (stroke_width / 2.0).ceil() as i32;
        debug_assert!(
            cx - total_radius >= 0,
            "draw_arc: left edge off screen ({} - {} < 0)",
            cx,
            total_radius
        );
        debug_assert!(
            cy - total_radius >= 0,
            "draw_arc: top edge off screen ({} - {} < 0)",
            cy,
            total_radius
        );
        debug_assert!(
            cx + total_radius <= self.width as i32,
            "draw_arc: right edge off screen ({} + {} > {})",
            cx,
            total_radius,
            self.width
        );
        debug_assert!(
            cy + total_radius <= self.height as i32,
            "draw_arc: bottom edge off screen ({} + {} > {})",
            cy,
            total_radius,
            self.height
        );

        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
        paint.anti_alias = true;

        let stroke = Stroke {
            width: stroke_width,
            ..Default::default()
        };

        // Build arc path using line segments (tiny_skia doesn't have native arc)
        let mut pb = PathBuilder::new();
        let segments = 64;
        let angle_span = end_angle - start_angle;
        let cx_f = cx as f32;
        let cy_f = cy as f32;
        let radius_f = radius as f32;

        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = start_angle + t * angle_span;
            let px = cx_f + radius_f * angle.cos();
            let py = cy_f + radius_f * angle.sin();
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }

        if let Some(path) = pb.finish() {
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    /// Draws text at the specified position.
    ///
    /// # Arguments
    /// * `x` - X position (left edge of text)
    /// * `y` - Y position (top edge of text)
    /// * `text` - The text to render
    /// * `size` - Font size in pixels
    /// * `color` - RGB888 color (0xRRGGBB)
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, size: f32, color: u32) {
        let text_width = self.text_renderer.text_width(text, size);
        let text_height = self.text_renderer.line_height(size);
        debug_assert!(
            x >= 0 && y >= 0,
            "draw_text: negative coordinates ({}, {}) for '{}'",
            x,
            y,
            text
        );
        debug_assert!(
            x + text_width <= self.width as i32,
            "draw_text: text extends past right edge ({} + {} > {}) for '{}'",
            x,
            text_width,
            self.width,
            text
        );
        debug_assert!(
            y + text_height <= self.height as i32,
            "draw_text: text extends past bottom edge ({} + {} > {}) for '{}'",
            y,
            text_height,
            self.height,
            text
        );

        self.text_renderer
            .draw_text(&mut self.pixmap, x, y, text, size, color);
    }

    /// Draws text with horizontal scaling.
    ///
    /// # Arguments
    /// * `x` - X position (left edge of text)
    /// * `y` - Y position (top edge of text)
    /// * `text` - The text to render
    /// * `size` - Font size in pixels
    /// * `color` - RGB888 color (0xRRGGBB)
    /// * `x_scale` - Horizontal scale factor (< 1.0 makes text narrower)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_scaled(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        size: f32,
        color: u32,
        x_scale: f32,
    ) {
        let text_width = self.text_renderer.text_width_scaled(text, size, x_scale);
        let text_height = self.text_renderer.line_height(size);
        debug_assert!(
            x >= 0 && y >= 0,
            "draw_text_scaled: negative coordinates ({}, {}) for '{}'",
            x,
            y,
            text
        );
        debug_assert!(
            x + text_width <= self.width as i32,
            "draw_text_scaled: text extends past right edge ({} + {} > {}) for '{}'",
            x,
            text_width,
            self.width,
            text
        );
        debug_assert!(
            y + text_height <= self.height as i32,
            "draw_text_scaled: text extends past bottom edge ({} + {} > {}) for '{}'",
            y,
            text_height,
            self.height,
            text
        );

        self.text_renderer
            .draw_text_scaled(&mut self.pixmap, x, y, text, size, color, x_scale);
    }

    /// Returns the width of text when rendered at the specified size.
    pub fn text_width(&self, text: &str, size: f32) -> i32 {
        self.text_renderer.text_width(text, size)
    }

    /// Returns the width of text when rendered with horizontal scaling.
    pub fn text_width_scaled(&self, text: &str, size: f32, x_scale: f32) -> i32 {
        self.text_renderer.text_width_scaled(text, size, x_scale)
    }

    /// Returns the line height for the specified font size.
    pub fn line_height(&self, size: f32) -> i32 {
        self.text_renderer.line_height(size)
    }

    /// Draws a scrolling line graph from historical data.
    ///
    /// # Arguments
    /// * `x` - X position (left edge)
    /// * `y` - Y position (top edge)
    /// * `width` - Width of the graph area
    /// * `height` - Height of the graph area
    /// * `data` - Historical data points (oldest first, newest last)
    /// * `max_value` - Maximum value for scaling (values above this are clamped)
    /// * `line_color` - Color for the line/bars
    /// * `bg_color` - Background color for the graph area
    #[allow(clippy::too_many_arguments)]
    pub fn draw_graph(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        data: &VecDeque<f64>,
        max_value: f64,
        line_color: u32,
        bg_color: u32,
    ) {
        debug_assert!(
            x >= 0 && y >= 0,
            "draw_graph: negative coordinates ({}, {})",
            x,
            y
        );
        debug_assert!(
            x + width as i32 <= self.width as i32,
            "draw_graph: x overflow ({} + {} > {})",
            x,
            width,
            self.width
        );
        debug_assert!(
            y + height as i32 <= self.height as i32,
            "draw_graph: y overflow ({} + {} > {})",
            y,
            height,
            self.height
        );

        // Draw background - use internal fill to avoid duplicate bounds check
        let r = ((bg_color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((bg_color >> 8) & 0xFF) as f32 / 255.0;
        let b = (bg_color & 0xFF) as f32 / 255.0;
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());
        if let Some(rect) = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
        }

        if data.is_empty() || max_value <= 0.0 {
            return;
        }

        // Compute highlight colors for high values
        let high_color = brighten_color(line_color, 1.4); // 95-99%: brighter
        let max_color = 0xFFFFFF; // 100%: white

        let num_points = data.len();
        let bar_width = (width as f64 / num_points as f64).max(1.0);

        // Draw bars from left to right (oldest to newest)
        for (i, &value) in data.iter().enumerate() {
            let normalized = (value / max_value).min(1.0);
            let bar_height = (normalized * height as f64) as u32;

            if bar_height > 0 {
                let bar_x = x + (i as f64 * bar_width) as i32;
                let bar_y = y + (height - bar_height) as i32;

                // Choose color based on how close to max
                let color = if normalized >= 1.0 {
                    max_color
                } else if normalized >= 0.95 {
                    high_color
                } else {
                    line_color
                };

                self.fill_rect(bar_x, bar_y, bar_width.ceil() as u32, bar_height, color);
            }
        }
    }

    /// Renders the canvas to a framebuffer.
    pub fn render_to_framebuffer(&self, fb: &mut Framebuffer) -> Result<()> {
        let pixels = self.pixmap.pixels();
        let data = fb.data_mut();

        for (i, pixel) in pixels.iter().enumerate() {
            if i < data.len() {
                data[i] = rgb888_to_rgb565(pixel.red(), pixel.green(), pixel.blue());
            }
        }

        Ok(())
    }

    /// Returns the raw RGBA pixels.
    pub fn pixels(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Returns the pixmap pixels as color values.
    pub fn pixmap_pixels(&self) -> &[tiny_skia::PremultipliedColorU8] {
        self.pixmap.pixels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_creation() {
        let canvas = Canvas::new(320, 170);
        assert_eq!(canvas.dimensions(), (320, 170));
    }
}
