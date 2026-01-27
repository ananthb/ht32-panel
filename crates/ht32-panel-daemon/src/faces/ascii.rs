//! ASCII text-only face with ASCII art graphs.
//!
//! Layout (320x170):
//! ```text
//! endeavour               18:45
//! Up: 5d 12h 34m
//!
//! CPU [########........] 45%
//! RAM [##########......] 67%
//! Disk[####............]  R:12M W:5M
//! Net [######..........]  D:1.2M U:0.8M
//!
//! enp2s0: 192.168.1.100
//! ```

use super::{Face, Theme};
use crate::rendering::Canvas;
use crate::sensors::data::SystemData;

/// Dim a color by mixing it toward the background.
fn dim_color(color: u32, background: u32, factor: f32) -> u32 {
    let r1 = ((color >> 16) & 0xFF) as f32;
    let g1 = ((color >> 8) & 0xFF) as f32;
    let b1 = (color & 0xFF) as f32;
    let r2 = ((background >> 16) & 0xFF) as f32;
    let g2 = ((background >> 8) & 0xFF) as f32;
    let b2 = (background & 0xFF) as f32;

    let r = (r1 * factor + r2 * (1.0 - factor)) as u32;
    let g = (g1 * factor + g2 * (1.0 - factor)) as u32;
    let b = (b1 * factor + b2 * (1.0 - factor)) as u32;

    (r << 16) | (g << 8) | b
}

/// Derive colors from theme for the ASCII face.
struct FaceColors {
    /// Primary highlight color (hostname, interface name)
    highlight: u32,
    /// Main text color
    text: u32,
    /// Dimmed text color (uptime, IPs)
    dim: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            highlight: theme.primary,
            text: 0xFFFFFF,
            dim: dim_color(0xFFFFFF, theme.background, 0.5),
        }
    }
}

/// Font sizes.
const FONT_LARGE: f32 = 16.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// Creates an ASCII progress bar string.
/// Returns something like "[########........]"
fn ascii_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("[{}{}]", "#".repeat(filled), ".".repeat(empty))
}

/// A text-only ASCII face.
pub struct AsciiFace;

impl AsciiFace {
    /// Creates a new ASCII face.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AsciiFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for AsciiFace {
    fn name(&self) -> &str {
        "ascii"
    }

    fn render(&self, canvas: &mut Canvas, data: &SystemData, theme: &Theme) {
        let colors = FaceColors::from_theme(theme);
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 8;
        let mut y = margin;

        // Bar width in characters
        let bar_chars = if portrait { 10 } else { 16 };

        if portrait {
            // Portrait layout - stack vertically
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);
            y += canvas.line_height(FONT_LARGE) + 2;

            canvas.draw_text(margin, y, &data.time, FONT_LARGE, colors.text);
            y += canvas.line_height(FONT_LARGE) + 2;

            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += canvas.line_height(FONT_SMALL) + 6;

            // CPU with ASCII bar
            let cpu_bar = ascii_bar(data.cpu_percent, bar_chars);
            let cpu_text = format!("CPU {} {:2.0}%", cpu_bar, data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_text, FONT_SMALL, colors.text);
            y += canvas.line_height(FONT_SMALL) + 2;

            // RAM with ASCII bar
            let ram_bar = ascii_bar(data.ram_percent, bar_chars);
            let ram_text = format!("RAM {} {:2.0}%", ram_bar, data.ram_percent);
            canvas.draw_text(margin, y, &ram_text, FONT_SMALL, colors.text);
            y += canvas.line_height(FONT_SMALL) + 4;

            // Disk I/O with ASCII bar (use combined rate for bar)
            let disk_total = data.disk_read_rate + data.disk_write_rate;
            let disk_percent = (disk_total / 10_000_000.0 * 100.0).min(100.0); // Scale: 10MB/s = 100%
            let disk_bar = ascii_bar(disk_percent, bar_chars);
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("DSK {}", disk_bar),
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 2;
            canvas.draw_text(
                margin + 16,
                y,
                &format!("R:{} W:{}", disk_r, disk_w),
                FONT_SMALL,
                colors.dim,
            );
            y += canvas.line_height(FONT_SMALL) + 4;

            // Network I/O with ASCII bar
            let net_total = data.net_rx_rate + data.net_tx_rate;
            let net_percent = (net_total / 10_000_000.0 * 100.0).min(100.0); // Scale: 10MB/s = 100%
            let net_bar = ascii_bar(net_percent, bar_chars);
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("NET {}", net_bar),
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 2;
            canvas.draw_text(
                margin + 16,
                y,
                &format!("D:{} U:{}", net_rx, net_tx),
                FONT_SMALL,
                colors.dim,
            );
            y += canvas.line_height(FONT_SMALL) + 6;

            // Network interface and IPs
            canvas.draw_text(margin, y, &data.net_interface, FONT_SMALL, colors.highlight);
            y += canvas.line_height(FONT_SMALL) + 2;

            if let Some(ref ipv4) = data.ipv4_address {
                canvas.draw_text(margin, y, ipv4, FONT_SMALL, colors.dim);
                y += canvas.line_height(FONT_SMALL) + 2;
            }
            if let Some(ref ipv6) = data.ipv6_address {
                canvas.draw_text(margin, y, ipv6, FONT_SMALL, colors.dim);
            }
        } else {
            // Landscape layout - side by side where possible
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);
            let time_width = canvas.text_width(&data.time, FONT_LARGE);
            canvas.draw_text(
                width as i32 - margin - time_width,
                y,
                &data.time,
                FONT_LARGE,
                colors.text,
            );
            y += canvas.line_height(FONT_LARGE) + 2;

            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, colors.dim);
            y += canvas.line_height(FONT_NORMAL) + 8;

            // CPU with ASCII bar
            let cpu_bar = ascii_bar(data.cpu_percent, bar_chars);
            let cpu_text = format!("CPU {} {:3.0}%", cpu_bar, data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // RAM with ASCII bar
            let ram_bar = ascii_bar(data.ram_percent, bar_chars);
            let ram_text = format!("RAM {} {:3.0}%", ram_bar, data.ram_percent);
            canvas.draw_text(margin, y, &ram_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 4;

            // Disk I/O with ASCII bar
            let disk_total = data.disk_read_rate + data.disk_write_rate;
            let disk_percent = (disk_total / 10_000_000.0 * 100.0).min(100.0);
            let disk_bar = ascii_bar(disk_percent, bar_chars);
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            let disk_text = format!("DSK {}  R:{} W:{}", disk_bar, disk_r, disk_w);
            canvas.draw_text(margin, y, &disk_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // Network I/O with ASCII bar
            let net_total = data.net_rx_rate + data.net_tx_rate;
            let net_percent = (net_total / 10_000_000.0 * 100.0).min(100.0);
            let net_bar = ascii_bar(net_percent, bar_chars);
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            let net_text = format!("NET {}  D:{} U:{}", net_bar, net_rx, net_tx);
            canvas.draw_text(margin, y, &net_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 6;

            // Network interface and IP on one line
            let mut net_info = data.net_interface.clone();
            if let Some(ref ipv4) = data.ipv4_address {
                net_info.push_str(": ");
                net_info.push_str(ipv4);
            }
            canvas.draw_text(margin, y, &net_info, FONT_NORMAL, colors.highlight);
            y += canvas.line_height(FONT_NORMAL) + 2;

            if let Some(ref ipv6) = data.ipv6_address {
                canvas.draw_text(margin, y, ipv6, FONT_SMALL, colors.dim);
            }
        }
    }
}
