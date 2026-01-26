//! RGB565 framebuffer for LCD display.

use crate::{Error, Result, LCD_HEIGHT, LCD_WIDTH};

/// Total pixel count for the display.
pub const PIXEL_COUNT: usize = LCD_WIDTH as usize * LCD_HEIGHT as usize;

/// RGB565 framebuffer for the 320x170 display.
#[derive(Clone)]
pub struct Framebuffer {
    /// Pixel data in RGB565 format.
    data: Vec<u16>,
    /// Width of the framebuffer.
    width: u16,
    /// Height of the framebuffer.
    height: u16,
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Framebuffer {
    /// Creates a new framebuffer initialized to black.
    pub fn new() -> Self {
        Self {
            data: vec![0; PIXEL_COUNT],
            width: LCD_WIDTH,
            height: LCD_HEIGHT,
        }
    }

    /// Creates a framebuffer with custom dimensions.
    pub fn with_dimensions(width: u16, height: u16) -> Self {
        let size = width as usize * height as usize;
        Self {
            data: vec![0; size],
            width,
            height,
        }
    }

    /// Returns the width of the framebuffer.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Returns the height of the framebuffer.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Resizes the framebuffer to new dimensions.
    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            let size = width as usize * height as usize;
            self.data = vec![0; size];
            self.width = width;
            self.height = height;
        }
    }

    /// Returns a reference to the raw pixel data.
    pub fn data(&self) -> &[u16] {
        &self.data
    }

    /// Returns a mutable reference to the raw pixel data.
    pub fn data_mut(&mut self) -> &mut [u16] {
        &mut self.data
    }

    /// Clears the framebuffer to a solid color.
    pub fn clear(&mut self, color: u16) {
        self.data.fill(color);
    }

    /// Sets a pixel at the given coordinates.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) {
        if x < self.width && y < self.height {
            let idx = y as usize * self.width as usize + x as usize;
            self.data[idx] = color;
        }
    }

    /// Gets a pixel at the given coordinates.
    pub fn get_pixel(&self, x: u16, y: u16) -> Option<u16> {
        if x < self.width && y < self.height {
            let idx = y as usize * self.width as usize + x as usize;
            Some(self.data[idx])
        } else {
            None
        }
    }

    /// Fills a rectangle with a solid color.
    pub fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: u16) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Copies pixel data from an RGB565 slice.
    pub fn copy_from_rgb565(&mut self, data: &[u16]) -> Result<()> {
        if data.len() != self.data.len() {
            return Err(Error::FramebufferSize {
                expected: self.data.len(),
                actual: data.len(),
            });
        }
        self.data.copy_from_slice(data);
        Ok(())
    }

    /// Copies pixel data from an RGBA8 slice, converting to RGB565.
    pub fn copy_from_rgba8(&mut self, data: &[u8]) -> Result<()> {
        let expected_len = self.data.len() * 4;
        if data.len() != expected_len {
            return Err(Error::FramebufferSize {
                expected: expected_len,
                actual: data.len(),
            });
        }

        for (i, chunk) in data.chunks_exact(4).enumerate() {
            self.data[i] = rgb888_to_rgb565(chunk[0], chunk[1], chunk[2]);
        }
        Ok(())
    }

    /// Copies pixel data from an RGB8 slice, converting to RGB565.
    pub fn copy_from_rgb8(&mut self, data: &[u8]) -> Result<()> {
        let expected_len = self.data.len() * 3;
        if data.len() != expected_len {
            return Err(Error::FramebufferSize {
                expected: expected_len,
                actual: data.len(),
            });
        }

        for (i, chunk) in data.chunks_exact(3).enumerate() {
            self.data[i] = rgb888_to_rgb565(chunk[0], chunk[1], chunk[2]);
        }
        Ok(())
    }

    /// Extracts a rectangular region as a new pixel vector.
    pub fn extract_region(&self, x: u16, y: u16, width: u16, height: u16) -> Vec<u16> {
        let mut region = Vec::with_capacity(width as usize * height as usize);
        for dy in 0..height {
            for dx in 0..width {
                let px = x + dx;
                let py = y + dy;
                if px < self.width && py < self.height {
                    let idx = py as usize * self.width as usize + px as usize;
                    region.push(self.data[idx]);
                } else {
                    region.push(0);
                }
            }
        }
        region
    }

    /// Rotates the framebuffer 180 degrees in place.
    pub fn rotate_180(&mut self) {
        let len = self.data.len();
        for i in 0..len / 2 {
            self.data.swap(i, len - 1 - i);
        }
    }

    /// Converts the framebuffer to RGBA8 bytes for PNG encoding.
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.data.len() * 4);
        for &pixel in &self.data {
            let (r, g, b) = rgb565_to_rgb888(pixel);
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(255);
        }
        rgba
    }
}

/// Converts RGB888 to RGB565.
#[inline]
pub fn rgb888_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r >> 3) as u16;
    let g6 = (g >> 2) as u16;
    let b5 = (b >> 3) as u16;
    (r5 << 11) | (g6 << 5) | b5
}

/// Converts RGB565 to RGB888.
#[inline]
pub fn rgb565_to_rgb888(pixel: u16) -> (u8, u8, u8) {
    let r = ((pixel >> 11) & 0x1F) as u8;
    let g = ((pixel >> 5) & 0x3F) as u8;
    let b = (pixel & 0x1F) as u8;
    // Expand to 8-bit
    let r8 = (r << 3) | (r >> 2);
    let g8 = (g << 2) | (g >> 4);
    let b8 = (b << 3) | (b >> 2);
    (r8, g8, b8)
}

/// Parses a hex color string to RGB565.
pub fn parse_hex_color(hex: &str) -> Option<u16> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(rgb888_to_rgb565(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb565_conversion() {
        // Pure red
        let red = rgb888_to_rgb565(255, 0, 0);
        assert_eq!(red, 0xF800);

        // Pure green
        let green = rgb888_to_rgb565(0, 255, 0);
        assert_eq!(green, 0x07E0);

        // Pure blue
        let blue = rgb888_to_rgb565(0, 0, 255);
        assert_eq!(blue, 0x001F);

        // White
        let white = rgb888_to_rgb565(255, 255, 255);
        assert_eq!(white, 0xFFFF);

        // Black
        let black = rgb888_to_rgb565(0, 0, 0);
        assert_eq!(black, 0x0000);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#FF0000"), Some(0xF800));
        assert_eq!(parse_hex_color("00FF00"), Some(0x07E0));
        assert_eq!(parse_hex_color("#000000"), Some(0x0000));
        assert_eq!(parse_hex_color("#FFFFFF"), Some(0xFFFF));
        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_framebuffer_ops() {
        let mut fb = Framebuffer::new();
        assert_eq!(fb.width(), 320);
        assert_eq!(fb.height(), 170);

        fb.set_pixel(10, 20, 0xF800);
        assert_eq!(fb.get_pixel(10, 20), Some(0xF800));

        fb.clear(0xFFFF);
        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF));
    }
}
