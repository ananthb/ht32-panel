//! Digits face inspired by Casio digital watches.
//!
//! Features a retro LCD aesthetic with large time display and
//! segmented areas for system metrics.

use super::{
    complication_names, complication_options, complications, date_formats, time_formats,
    Complication, EnabledComplications, Face, Theme,
};
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
            segment_off: dim_color(theme.primary, theme.background, 0.2),
            label: dim_color(theme.text, theme.background, 0.7), // Higher for better contrast
            divider: dim_color(theme.primary, theme.background, 0.35),
        }
    }
}

/// Font sizes.
const FONT_TIME: f32 = 32.0;
const FONT_LARGE: f32 = 20.0;
const FONT_MEDIUM: f32 = 14.0;
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

    fn available_complications(&self) -> Vec<Complication> {
        vec![
            complications::time(true),
            complications::date(true, date_formats::ISO),
            complications::ip_address(true),
            complications::network(true),
            complications::disk_io(true),
            complications::cpu_temp(true),
        ]
    }

    fn render(
        &self,
        canvas: &mut Canvas,
        data: &SystemData,
        theme: &Theme,
        comp: &EnabledComplications,
    ) {
        let colors = FaceColors::from_theme(theme);
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 6;
        let mut y = margin;

        let is_on = |id: &str| comp.is_enabled(self.name(), id, true);

        // Get time format option
        let time_format = comp
            .get_option(
                self.name(),
                complication_names::TIME,
                complication_options::TIME_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(time_formats::DIGITAL_24H);

        // Get date format option
        let date_format = comp
            .get_option(
                self.name(),
                complication_names::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::ISO);

        if portrait {
            // Portrait layout
            // Complication: Time
            if is_on(complication_names::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                let time_width = canvas.text_width(&time_str, FONT_TIME);
                let time_x = (width as i32 - time_width) / 2;
                canvas.draw_text(time_x, y, &time_str, FONT_TIME, colors.segment_on);
                y += canvas.line_height(FONT_TIME) + 2;
            }

            // Complication: Date (centered, if not hidden)
            if is_on(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_MEDIUM);
                    let date_x = (width as i32 - date_width) / 2;
                    canvas.draw_text(date_x, y, &date_str, FONT_MEDIUM, colors.label);
                    y += canvas.line_height(FONT_MEDIUM) + 2;
                }
            }

            // Base element: Hostname (always shown)
            let host_width = canvas.text_width(&data.hostname, FONT_MEDIUM);
            let host_x = (width as i32 - host_width) / 2;
            canvas.draw_text(host_x, y, &data.hostname, FONT_MEDIUM, colors.label);
            y += canvas.line_height(FONT_MEDIUM) + 4;

            let col_width = (width as i32 - margin * 3) / 2;

            // Base elements: CPU and RAM (always shown)
            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;
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

            // Complication: Disk I/O
            if is_on(complication_names::DISK_IO) {
                Self::draw_divider(canvas, y, width, margin, colors.divider);
                y += 6;
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
            }

            // Complication: Network
            if is_on(complication_names::NETWORK) {
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
            }

            // Base element: Uptime and complication: IP address
            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // Base element: Uptime (always shown)
            canvas.draw_text(
                margin,
                y,
                &format!("UP {}", data.uptime),
                FONT_SMALL,
                colors.label,
            );
            y += canvas.line_height(FONT_SMALL) + 2;

            // Complication: IP address
            if is_on(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    let max_chars = if width < 150 { 20 } else { 30 };
                    let ip_display = if ip.len() > max_chars {
                        format!("{}...", &ip[..max_chars - 3])
                    } else {
                        ip.clone()
                    };
                    canvas.draw_text(margin, y, &ip_display, FONT_MEDIUM, colors.label);
                }
            }
        } else {
            // Landscape layout
            // Complication: Time
            if is_on(complication_names::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                canvas.draw_text(margin, y, &time_str, FONT_TIME, colors.segment_on);
            }

            // Complication: Date (right side, below hostname if shown)
            let host_width = canvas.text_width(&data.hostname, FONT_MEDIUM);
            canvas.draw_text(
                width as i32 - margin - host_width,
                y + 8,
                &data.hostname,
                FONT_MEDIUM,
                colors.label,
            );

            if is_on(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_MEDIUM);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y + 24,
                        &date_str,
                        FONT_MEDIUM,
                        colors.label,
                    );
                } else {
                    let uptime_text = format!("UP {}", data.uptime);
                    let uptime_width = canvas.text_width(&uptime_text, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - uptime_width,
                        y + 22,
                        &uptime_text,
                        FONT_SMALL,
                        colors.label,
                    );
                }
            } else {
                let uptime_text = format!("UP {}", data.uptime);
                let uptime_width = canvas.text_width(&uptime_text, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - uptime_width,
                    y + 22,
                    &uptime_text,
                    FONT_SMALL,
                    colors.label,
                );
            }
            y += canvas.line_height(FONT_TIME) + 6;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 8;

            let col_width = (width as i32 - margin * 5) / 4;

            // Row 1: CPU (base), RAM (base), Temp (complication)
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
            // Complication: CPU temperature
            if is_on(complication_names::CPU_TEMP) {
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
            }
            y += 34;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 8;

            // Row 2: Disk R, Disk W (complication), Net Down, Net Up (complication)
            if is_on(complication_names::DISK_IO) {
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
            }
            if is_on(complication_names::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
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
            }
            y += 34;

            Self::draw_divider(canvas, y, width, margin, colors.divider);
            y += 6;

            // Complication: IP address
            if is_on(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    canvas.draw_text(margin, y, ip, FONT_MEDIUM, colors.label);
                }
            }
        }
        let _ = y;
    }
}
