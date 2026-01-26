//! Canvas for rendering widgets to framebuffer.

use anyhow::Result;
use ht32_panel_hw::lcd::framebuffer::{rgb888_to_rgb565, Framebuffer};
use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

use super::text::TextRenderer;

/// Widget rectangle.
#[derive(Debug, Clone)]
pub struct WidgetRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Widget instance.
#[derive(Debug, Clone)]
pub struct Widget {
    pub id: u32,
    pub widget_type: String,
    pub rect: WidgetRect,
    pub value: String,
    pub color: u32,
    pub background: Option<u32>,
}

/// Canvas for rendering.
pub struct Canvas {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    widgets: Vec<Widget>,
    next_widget_id: u32,
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
            widgets: Vec::new(),
            next_widget_id: 1,
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

    /// Returns a reference to the widgets.
    pub fn widgets(&self) -> &[Widget] {
        &self.widgets
    }

    /// Adds a widget to the canvas.
    pub fn add_widget(&mut self, widget_type: &str, rect: WidgetRect) -> Widget {
        let widget = Widget {
            id: self.next_widget_id,
            widget_type: widget_type.to_string(),
            rect,
            value: String::new(),
            color: 0xFFFFFF,
            background: None,
        };
        self.next_widget_id += 1;
        self.widgets.push(widget.clone());
        widget
    }

    /// Updates a widget's rectangle.
    pub fn update_widget_rect(&mut self, id: u32, rect: WidgetRect) -> bool {
        if let Some(widget) = self.widgets.iter_mut().find(|w| w.id == id) {
            widget.rect = rect;
            true
        } else {
            false
        }
    }

    /// Updates a widget's value.
    pub fn update_widget_value(&mut self, id: u32, value: &str) -> bool {
        if let Some(widget) = self.widgets.iter_mut().find(|w| w.id == id) {
            widget.value = value.to_string();
            true
        } else {
            false
        }
    }

    /// Removes a widget.
    pub fn remove_widget(&mut self, id: u32) -> bool {
        let len_before = self.widgets.len();
        self.widgets.retain(|w| w.id != id);
        self.widgets.len() != len_before
    }

    /// Draws a filled rectangle.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
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

    /// Draws text at the specified position.
    ///
    /// # Arguments
    /// * `x` - X position (left edge of text)
    /// * `y` - Y position (top edge of text)
    /// * `text` - The text to render
    /// * `size` - Font size in pixels
    /// * `color` - RGB888 color (0xRRGGBB)
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, size: f32, color: u32) {
        self.text_renderer
            .draw_text(&mut self.pixmap, x, y, text, size, color);
    }

    /// Returns the width of text when rendered at the specified size.
    pub fn text_width(&self, text: &str, size: f32) -> i32 {
        self.text_renderer.text_width(text, size)
    }

    /// Returns the line height for the specified font size.
    pub fn line_height(&self, size: f32) -> i32 {
        self.text_renderer.line_height(size)
    }

    /// Renders all widgets to the internal pixmap.
    pub fn render(&mut self) {
        self.clear();

        // Collect widget draw operations to avoid borrow issues
        let draw_ops: Vec<_> = self
            .widgets
            .iter()
            .map(|w| {
                (
                    w.rect.x,
                    w.rect.y,
                    w.rect.width,
                    w.rect.height,
                    w.color,
                    w.background,
                )
            })
            .collect();

        for (x, y, width, height, color, background) in draw_ops {
            // Draw widget background if set
            if let Some(bg) = background {
                self.fill_rect(x, y, width, height, bg);
            }

            // Draw widget border (debug frame)
            self.draw_rect_outline(x, y, width, height, color);
        }
    }

    /// Draws a rectangle outline.
    fn draw_rect_outline(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(r, g, b, 1.0).unwrap());

        let x = x as f32;
        let y = y as f32;
        let w = width as f32;
        let h = height as f32;

        // Top
        if let Some(rect) = Rect::from_xywh(x, y, w, 1.0) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
        }
        // Bottom
        if let Some(rect) = Rect::from_xywh(x, y + h - 1.0, w, 1.0) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
        }
        // Left
        if let Some(rect) = Rect::from_xywh(x, y, 1.0, h) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
        }
        // Right
        if let Some(rect) = Rect::from_xywh(x + w - 1.0, y, 1.0, h) {
            self.pixmap
                .fill_rect(rect, &paint, Transform::identity(), None);
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

    #[test]
    fn test_widget_management() {
        let mut canvas = Canvas::new(320, 170);

        let widget = canvas.add_widget(
            "text",
            WidgetRect {
                x: 10,
                y: 10,
                width: 100,
                height: 50,
            },
        );
        assert_eq!(widget.id, 1);
        assert_eq!(canvas.widgets().len(), 1);

        let updated = canvas.update_widget_rect(
            1,
            WidgetRect {
                x: 20,
                y: 20,
                width: 100,
                height: 50,
            },
        );
        assert!(updated);

        let removed = canvas.remove_widget(1);
        assert!(removed);
        assert_eq!(canvas.widgets().len(), 0);
    }
}
