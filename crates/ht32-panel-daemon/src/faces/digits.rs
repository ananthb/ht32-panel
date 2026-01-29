//! Digits face inspired by Casio digital watches.
//!
//! Features a retro LCD aesthetic with large time display and
//! segmented areas for system metrics.

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

/// Derive colors from theme for the digits face.
struct FaceColors {
    /// LCD segment "on" color
    segment_on: u32,
    /// LCD segment "off" color (ghost segments)
    segment_off: u32,
    /// Label text color
    label: u32,
    /// Divider line color
    divider: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            segment_on: theme.primary,
            segment_off: dim_color(theme.primary, theme.background, 0.15),
            label: dim_color(theme.text, theme.background, 0.6),
            divider: dim_color(theme.primary, theme.background, 0.3),
        }
    }
}

/// Font sizes.
const FONT_TIME: f32 = 32.0;
const FONT_LARGE: f32 = 20.0;
const FONT_SMALL: f32 = 11.0;

/// A Casio-inspired digital watch face.
pub struct DigitsFace;

impl DigitsFace {
    /// Creates a new digits face.
    pub fn new() -> Self {
        Self
    }

    /// Draws a horizontal divider line.
    fn draw_divider(canvas: &mut Canvas, y: i32, width: u32, margin: i32, color: u32) {
        canvas.fill_rect(margin, y, width - (margin * 2) as u32, 1, color);
    }

    /// Draws a labeled value in the segmented LCD style.
    fn draw_segment_value(
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        label: &str,
        value: &str,
        label_color: u32,
        value_color: u32,
    ) {
        canvas.draw_text(x, y, label, FONT_SMALL, label_color);
        canvas.draw_text(x, y + 10, value, FONT_LARGE, value_color);
    }
}

impl Default for DigitsFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for DigitsFace {
    fn name(&self) -> &str {
        "digits"
    }

    fn render(&self, canvas: &mut Canvas, data: &SystemData, theme: &Theme) {
        let colors = FaceColors::from_theme(theme);
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 6;
        let mut y = margin;

        if portrait {
            // Portrait layout - stacked vertically
            // Large time at top
            let time_width = canvas.text_width(&data.time, FONT_TIME);
            let time_x = (width as i32 - time_width) / 2;
            canvas.draw_text(time_x, y, &data.time, FONT_TIME, colors.segment_on);
            y += canvas.line_height(FONT_TIME) + 2;

            // Hostname below time
            let host_width = canvas.text_width(&data.hostname, FONT_SMALL);
            let host_x = (width as i32 - host_width) / 2;
            canvas.draw_text(host_x, y, &data.hostname, FONT_SMALL, colors.label);
            y += canvas.line_height(FONT_SMALL) + 4;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // CPU and RAM side by side
            let col_width = (width as i32 - margin * 3) / 2;
            Self::draw_segment_value(
                canvas,
                margin,
                y,
                "CPU",
                &format!("{:2.0}%", data.cpu_percent),
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + col_width + margin,
                y,
                "RAM",
                &format!("{:2.0}%", data.ram_percent),
                colors.label,
                colors.segment_on,
            );
            y += 32;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // Disk I/O
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            Self::draw_segment_value(
                canvas,
                margin,
                y,
                "DSK R",
                &disk_r,
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + col_width + margin,
                y,
                "DSK W",
                &disk_w,
                colors.label,
                colors.segment_on,
            );
            y += 32;

            // Network I/O
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
            Self::draw_segment_value(
                canvas,
                margin,
                y,
                "NET \u{2193}",
                &net_rx,
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + col_width + margin,
                y,
                "NET \u{2191}",
                &net_tx,
                colors.label,
                colors.segment_on,
            );
            y += 32;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // Uptime and IP at bottom
            canvas.draw_text(margin, y, &format!("UP {}", data.uptime), FONT_SMALL, colors.label);
            y += canvas.line_height(FONT_SMALL) + 2;

            if let Some(ref ip) = data.display_ip {
                // Truncate if too long
                let max_chars = if width < 150 { 20 } else { 30 };
                let ip_display = if ip.len() > max_chars {
                    format!("{}...", &ip[..max_chars - 3])
                } else {
                    ip.clone()
                };
                canvas.draw_text(margin, y, &ip_display, FONT_SMALL, colors.label);
            }
        } else {
            // Landscape layout - more horizontal space
            // Large time on left, hostname on right
            canvas.draw_text(margin, y, &data.time, FONT_TIME, colors.segment_on);
            let host_width = canvas.text_width(&data.hostname, FONT_SMALL);
            canvas.draw_text(
                width as i32 - margin - host_width,
                y + 8,
                &data.hostname,
                FONT_SMALL,
                colors.label,
            );

            // Uptime below hostname
            let uptime_text = format!("UP {}", data.uptime);
            let uptime_width = canvas.text_width(&uptime_text, FONT_SMALL);
            canvas.draw_text(
                width as i32 - margin - uptime_width,
                y + 22,
                &uptime_text,
                FONT_SMALL,
                colors.label,
            );
            y += canvas.line_height(FONT_TIME) + 6;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 8;

            // Four columns: CPU, RAM, TEMP, IP
            let col_width = (width as i32 - margin * 5) / 4;

            // Row 1: CPU, RAM, Temp (if available), empty or IP
            Self::draw_segment_value(
                canvas,
                margin,
                y,
                "CPU",
                &format!("{:2.0}%", data.cpu_percent),
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + col_width + margin,
                y,
                "RAM",
                &format!("{:2.0}%", data.ram_percent),
                colors.label,
                colors.segment_on,
            );
            if let Some(temp) = data.cpu_temp {
                Self::draw_segment_value(
                    canvas,
                    margin + (col_width + margin) * 2,
                    y,
                    "TEMP",
                    &format!("{:.0}°", temp),
                    colors.label,
                    colors.segment_on,
                );
            }
            y += 34;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 8;

            // Row 2: Disk R, Disk W, Net Down, Net Up
            let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
            let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
            let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
            let net_tx = SystemData::format_rate_compact(data.net_tx_rate);

            Self::draw_segment_value(
                canvas,
                margin,
                y,
                "DSK R",
                &disk_r,
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + col_width + margin,
                y,
                "DSK W",
                &disk_w,
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + (col_width + margin) * 2,
                y,
                "NET \u{2193}",
                &net_rx,
                colors.label,
                colors.segment_on,
            );
            Self::draw_segment_value(
                canvas,
                margin + (col_width + margin) * 3,
                y,
                "NET \u{2191}",
                &net_tx,
                colors.label,
                colors.segment_on,
            );
            y += 34;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // IP at bottom
            if let Some(ref ip) = data.display_ip {
                canvas.draw_text(margin, y, ip, FONT_SMALL, colors.label);
            }
        }
    }
}
