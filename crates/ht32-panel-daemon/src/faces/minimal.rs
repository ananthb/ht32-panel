//! Minimal text-only face.
//!
//! Layout (320x170):
//! ```text
//! endeavour               18:45
//! Up: 5d 12h 34m
//!
//! CPU: 45%    RAM: 67%
//! Disk R/W: 12/5 MB/s
//! Net: 1.2/0.8 MB/s
//! enp2s0: 2001:db8::1
//!         192.168.1.100
//! ```

use super::Face;
use crate::rendering::Canvas;
use crate::sensors::data::SystemData;

/// Colors for the minimal face.
const COLOR_WHITE: u32 = 0xFFFFFF;
const COLOR_GRAY: u32 = 0xAAAAAA;
const COLOR_CYAN: u32 = 0x00DDDD;

/// Font sizes.
const FONT_LARGE: f32 = 16.0;
const FONT_NORMAL: f32 = 14.0;

/// A minimal text-only face.
pub struct MinimalFace;

impl MinimalFace {
    /// Creates a new minimal face.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MinimalFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for MinimalFace {
    fn name(&self) -> &str {
        "minimal"
    }

    fn render(&self, canvas: &mut Canvas, data: &SystemData) {
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 8;
        let mut y = margin;

        if portrait {
            // Portrait layout - stack vertically
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, COLOR_CYAN);
            y += canvas.line_height(FONT_LARGE) + 2;

            canvas.draw_text(margin, y, &data.time, FONT_LARGE, COLOR_WHITE);
            y += canvas.line_height(FONT_LARGE) + 2;

            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, COLOR_GRAY);
            y += canvas.line_height(FONT_NORMAL) + 8;

            let cpu_text = format!("CPU: {:3.0}%", data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_text, FONT_NORMAL, COLOR_WHITE);
            y += canvas.line_height(FONT_NORMAL) + 2;

            let ram_text = format!("RAM: {:3.0}%", data.ram_percent);
            canvas.draw_text(margin, y, &ram_text, FONT_NORMAL, COLOR_WHITE);
            y += canvas.line_height(FONT_NORMAL) + 6;

            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("Disk R: {}", disk_r),
                FONT_NORMAL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_NORMAL) + 2;
            canvas.draw_text(
                margin,
                y,
                &format!("Disk W: {}", disk_w),
                FONT_NORMAL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_NORMAL) + 6;

            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("Net \u{2193}: {}", net_rx),
                FONT_NORMAL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_NORMAL) + 2;
            canvas.draw_text(
                margin,
                y,
                &format!("Net \u{2191}: {}", net_tx),
                FONT_NORMAL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_NORMAL) + 6;

            // Network interface and IPs (indented under Net:)
            let indent = margin + 24;
            canvas.draw_text(indent, y, &data.net_interface, FONT_NORMAL, COLOR_CYAN);
            y += canvas.line_height(FONT_NORMAL) + 2;

            if let Some(ref ipv6) = data.ipv6_address {
                canvas.draw_text(indent, y, ipv6, FONT_NORMAL, COLOR_GRAY);
                y += canvas.line_height(FONT_NORMAL) + 2;
            }
            if let Some(ref ipv4) = data.ipv4_address {
                canvas.draw_text(indent, y, ipv4, FONT_NORMAL, COLOR_GRAY);
            }
        } else {
            // Landscape layout - side by side
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, COLOR_CYAN);
            let time_width = canvas.text_width(&data.time, FONT_LARGE);
            canvas.draw_text(
                width as i32 - margin - time_width,
                y,
                &data.time,
                FONT_LARGE,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_LARGE) + 2;

            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, COLOR_GRAY);
            y += canvas.line_height(FONT_NORMAL) + 10;

            let cpu_text = format!("CPU: {:3.0}%", data.cpu_percent);
            let ram_text = format!("RAM: {:3.0}%", data.ram_percent);
            canvas.draw_text(margin, y, &cpu_text, FONT_NORMAL, COLOR_WHITE);
            canvas.draw_text(margin + 100, y, &ram_text, FONT_NORMAL, COLOR_WHITE);
            y += canvas.line_height(FONT_NORMAL) + 4;

            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            let disk_text = format!("Disk R/W: {}/{}", disk_r, disk_w);
            canvas.draw_text(margin, y, &disk_text, FONT_NORMAL, COLOR_WHITE);
            y += canvas.line_height(FONT_NORMAL) + 4;

            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            let net_text = format!("Net \u{2193}{} \u{2191}{}", net_rx, net_tx);
            canvas.draw_text(margin, y, &net_text, FONT_NORMAL, COLOR_WHITE);
            y += canvas.line_height(FONT_NORMAL) + 4;

            // Network interface and IPs (indented under Net:)
            let indent = margin + 24;
            canvas.draw_text(indent, y, &data.net_interface, FONT_NORMAL, COLOR_CYAN);
            y += canvas.line_height(FONT_NORMAL) + 2;

            if let Some(ref ipv6) = data.ipv6_address {
                canvas.draw_text(indent, y, ipv6, FONT_NORMAL, COLOR_GRAY);
                y += canvas.line_height(FONT_NORMAL) + 2;
            }
            if let Some(ref ipv4) = data.ipv4_address {
                canvas.draw_text(indent, y, ipv4, FONT_NORMAL, COLOR_GRAY);
            }
        }
    }
}
