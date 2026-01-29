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
    /// Graph background
    bar_bg: u32,
    /// Disk graph fill color
    bar_disk: u32,
    /// Network graph fill color
    bar_net: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            highlight: theme.primary,
            text: theme.text,
            dim: dim_color(theme.text, theme.background, 0.5),
            bar_bg: dim_color(theme.primary, theme.background, 0.15),
            bar_disk: dim_color(theme.primary, theme.secondary, 0.5),
            bar_net: theme.secondary,
        }
    }
}

/// Font sizes.
const FONT_LARGE: f32 = 16.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// Graph dimensions.
const GRAPH_HEIGHT: u32 = 14;

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
            // Hostname on left, time on top right
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

            // Uptime
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += canvas.line_height(FONT_SMALL) + 2;

            // IP address under uptime (hyphenate IPv6 if too long)
            if let Some(ref ip) = data.display_ip {
                let max_width = width as i32 - margin * 2;
                let ip_width = canvas.text_width(ip, FONT_SMALL);
                if ip_width > max_width && ip.contains(':') {
                    // IPv6 address - split at a colon near the middle
                    let mid = ip.len() / 2;
                    let split_pos = ip[..mid].rfind(':').map(|p| p + 1).unwrap_or(mid);
                    let (first, second) = ip.split_at(split_pos);
                    canvas.draw_text(margin, y, first, FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL);
                    canvas.draw_text(margin, y, second, FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL) + 2;
                } else {
                    canvas.draw_text(margin, y, ip, FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL) + 2;
                }
            }

            // CPU temperature (portrait: below IP)
            if let Some(temp) = data.cpu_temp {
                let temp_text = format!("Temp: {:.0}°C", temp);
                canvas.draw_text(margin, y, &temp_text, FONT_SMALL, colors.dim);
                y += canvas.line_height(FONT_SMALL) + 4;
            } else {
                y += 4; // Keep spacing consistent
            }

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

            // Disk I/O graph - counters on left side
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("DSK R:{} W:{}", disk_r, disk_w),
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 1;
            canvas.draw_graph(
                margin,
                y,
                width - (margin * 2) as u32,
                GRAPH_HEIGHT,
                &data.disk_history,
                SystemData::compute_graph_scale(&data.disk_history),
                colors.bar_disk,
                colors.bar_bg,
            );
            y += GRAPH_HEIGHT as i32 + 4;

            // Network I/O graph - counters on left side
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("NET D:{} U:{}", net_rx, net_tx),
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 1;
            canvas.draw_graph(
                margin,
                y,
                width - (margin * 2) as u32,
                GRAPH_HEIGHT,
                &data.net_history,
                SystemData::compute_graph_scale(&data.net_history),
                colors.bar_net,
                colors.bar_bg,
            );
        } else {
            // Landscape layout - hostname on left, time on top right
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

            // Uptime
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, colors.dim);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // IP address on its own line under uptime
            if let Some(ref ip) = data.display_ip {
                canvas.draw_text(margin, y, ip, FONT_SMALL, colors.dim);
                y += canvas.line_height(FONT_SMALL) + 6;
            } else {
                y += 6;
            }

            // CPU with ASCII bar and temperature
            let cpu_bar = ascii_bar(data.cpu_percent, bar_chars);
            let cpu_text = if let Some(temp) = data.cpu_temp {
                format!("CPU {} {:3.0}%  {:.0}°C", cpu_bar, data.cpu_percent, temp)
            } else {
                format!("CPU {} {:3.0}%", cpu_bar, data.cpu_percent)
            };
            canvas.draw_text(margin, y, &cpu_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // RAM with ASCII bar
            let ram_bar = ascii_bar(data.ram_percent, bar_chars);
            let ram_text = format!("RAM {} {:3.0}%", ram_bar, data.ram_percent);
            canvas.draw_text(margin, y, &ram_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 4;

            // Disk I/O graph
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            canvas.draw_text(margin, y, "DSK", FONT_NORMAL, colors.text);
            canvas.draw_text(
                margin + 40,
                y,
                &format!("R:{} W:{}", disk_r, disk_w),
                FONT_NORMAL,
                colors.dim,
            );
            y += canvas.line_height(FONT_NORMAL) + 1;
            canvas.draw_graph(
                margin,
                y,
                width - (margin * 2) as u32,
                GRAPH_HEIGHT,
                &data.disk_history,
                SystemData::compute_graph_scale(&data.disk_history),
                colors.bar_disk,
                colors.bar_bg,
            );
            y += GRAPH_HEIGHT as i32 + 3;

            // Network I/O graph
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            canvas.draw_text(margin, y, "NET", FONT_NORMAL, colors.text);
            canvas.draw_text(
                margin + 40,
                y,
                &format!("D:{} U:{}", net_rx, net_tx),
                FONT_NORMAL,
                colors.dim,
            );
            y += canvas.line_height(FONT_NORMAL) + 1;
            canvas.draw_graph(
                margin,
                y,
                width - (margin * 2) as u32,
                GRAPH_HEIGHT,
                &data.net_history,
                SystemData::compute_graph_scale(&data.net_history),
                colors.bar_net,
                colors.bar_bg,
            );
        }
    }
}
