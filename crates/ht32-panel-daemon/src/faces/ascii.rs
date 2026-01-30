//! ASCII text-only face with ASCII art graphs.
//!
//! Portrait layout (135x240):
//! ```text
//! endeavour         18:45
//! Up: 5d 12h 34m    2025-01-31
//! IP:
//! 192.168.1.100
//! Temp:                  45°C
//! CPU: 45%
//! [########...............]
//! RAM: 67%
//! [##########..............]
//! DSK:             R:12M W:5M
//! [_._.-=+*##*+=-._.____..]
//! NET:           D:1.2M U:0.8M
//! [__..--==++**##**++==..]
//! ```
//!
//! Landscape layout (320x170):
//! ```text
//! endeavour               18:45
//! Up: 5d 12h 34m
//! IP: 192.168.1.100
//! CPU [########........] 45%
//! RAM [##########......] 67%
//! DSK  R:12M W:5M
//! [_._.-=+*##*+=-._.____..]
//! NET  D:1.2M U:0.8M
//! [__..--==++**##**++==..]
//! ```

use super::{
    complication_names, complication_options, complications, date_formats, draw_mini_analog_clock,
    time_formats, Complication, EnabledComplications, Face, Theme,
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
            dim: dim_color(theme.text, theme.background, 0.7), // Higher for better contrast
            bar_bg: dim_color(theme.primary, theme.background, 0.2),
            bar_disk: dim_color(theme.primary, theme.secondary, 0.5),
            bar_net: theme.secondary,
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

/// Creates an ASCII sparkline from historical data.
/// Uses ASCII characters to represent different heights:
/// `_` (lowest), `.`, `-`, `=`, `+`, `*`, `#` (highest)
fn ascii_sparkline(data: &std::collections::VecDeque<f64>, max_value: f64, width: usize) -> String {
    const CHARS: [char; 7] = ['_', '.', '-', '=', '+', '*', '#'];

    if data.is_empty() || max_value <= 0.0 {
        return "_".repeat(width);
    }

    // Sample data to fit width
    let num_points = data.len();
    let mut result = String::with_capacity(width);

    for i in 0..width {
        // Map output position to data index
        let data_idx = if width <= num_points {
            // More data than width: sample from recent data
            num_points - width + i
        } else {
            // Less data than width: stretch or pad
            (i * num_points) / width
        };

        let value = data.get(data_idx).copied().unwrap_or(0.0);
        let normalized = (value / max_value).clamp(0.0, 1.0);
        let level = (normalized * (CHARS.len() - 1) as f64).round() as usize;
        result.push(CHARS[level.min(CHARS.len() - 1)]);
    }

    result
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
        complications: &EnabledComplications,
    ) {
        let colors = FaceColors::from_theme(theme);
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 6;
        let mut y = 4; // Start near top
        let bar_chars = if portrait { 10 } else { 16 };

        let is_enabled = |id: &str| complications.is_enabled(self.name(), id, true);

        // Get time format option
        let time_format = complications
            .get_option(
                self.name(),
                complication_names::TIME,
                complication_options::TIME_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(time_formats::DIGITAL_24H);

        // Get date format option
        let date_format = complications
            .get_option(
                self.name(),
                complication_names::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::ISO);

        if portrait {
            // Portrait layout - labels on separate lines, wider graphs
            let line_height = canvas.line_height(FONT_SMALL);
            let section_spacing = 6; // Extra spacing between label/value pairs
                                     // Calculate bar width to fill most of the line (leave margin on each side)
            let bar_width = ((width as i32 - margin * 2) / 7).max(12) as usize; // ~7 pixels per char

            // Hostname (always shown)
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complication_names::TIME) {
                if time_format == time_formats::ANALOGUE {
                    // Draw small analog clock on the right
                    let clock_radius = 10_u32;
                    let clock_cx = width as i32 - margin - clock_radius as i32;
                    let clock_cy = y + clock_radius as i32;
                    draw_mini_analog_clock(
                        canvas,
                        clock_cx,
                        clock_cy,
                        clock_radius,
                        data.hour,
                        data.minute,
                        colors.highlight,
                        colors.text,
                    );
                } else {
                    let time_str = data.format_time(time_format);
                    let time_width = canvas.text_width(&time_str, FONT_LARGE);
                    canvas.draw_text(
                        width as i32 - margin - time_width,
                        y,
                        &time_str,
                        FONT_LARGE,
                        colors.text,
                    );
                }
            }
            y += canvas.line_height(FONT_LARGE) + 1;

            // Complication: Date (right-aligned)
            if is_enabled(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_SMALL,
                        colors.dim,
                    );
                }
            }
            y += line_height; // Skip line for date
            y += line_height; // Extra line before Up

            // Up: on its own line (two lines below date)
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += line_height + section_spacing;

            // IP: on its own line
            if is_enabled(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    canvas.draw_text(margin, y, "IP:", FONT_SMALL, colors.dim);
                    y += line_height;
                    // IP value on next line, possibly split for IPv6
                    let max_width = width as i32 - margin * 2;
                    let ip_width = canvas.text_width(ip, FONT_SMALL);
                    if ip_width > max_width && ip.contains(':') {
                        let mid = ip.len() / 2;
                        let split_pos = ip[..mid].rfind(':').map(|p| p + 1).unwrap_or(mid);
                        let (first, second) = ip.split_at(split_pos);
                        canvas.draw_text(margin, y, first, FONT_SMALL, colors.text);
                        y += line_height;
                        canvas.draw_text(margin, y, second, FONT_SMALL, colors.text);
                    } else {
                        canvas.draw_text(margin, y, ip, FONT_SMALL, colors.text);
                    }
                    y += line_height + section_spacing;
                }
            }

            // Temp: on its own line
            if is_enabled(complication_names::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    canvas.draw_text(margin, y, "Temp:", FONT_SMALL, colors.dim);
                    let temp_val = format!("{:.0}°C", temp);
                    let temp_w = canvas.text_width(&temp_val, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - temp_w,
                        y,
                        &temp_val,
                        FONT_SMALL,
                        colors.text,
                    );
                    y += line_height + section_spacing;
                }
            }

            // CPU: label line, then bar on next line
            let cpu_label = format!("CPU: {:2.0}%", data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_label, FONT_SMALL, colors.dim);
            y += line_height;
            let cpu_bar = ascii_bar(data.cpu_percent, bar_width);
            canvas.draw_text(margin, y, &cpu_bar, FONT_SMALL, colors.text);
            y += line_height + section_spacing;

            // RAM: label line, then bar on next line
            let ram_label = format!("RAM: {:2.0}%", data.ram_percent);
            canvas.draw_text(margin, y, &ram_label, FONT_SMALL, colors.dim);
            y += line_height;
            let ram_bar = ascii_bar(data.ram_percent, bar_width);
            canvas.draw_text(margin, y, &ram_bar, FONT_SMALL, colors.text);
            y += line_height + section_spacing;

            // DSK: label line, then sparkline on next line
            if is_enabled(complication_names::DISK_IO) {
                let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
                let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
                canvas.draw_text(margin, y, "DSK:", FONT_SMALL, colors.dim);
                let disk_rates = format!("R:{} W:{}", disk_r, disk_w);
                let disk_rates_w = canvas.text_width(&disk_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - disk_rates_w,
                    y,
                    &disk_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
                let sparkline = ascii_sparkline(
                    &data.disk_history,
                    SystemData::compute_graph_scale(&data.disk_history),
                    bar_width,
                );
                canvas.draw_text(
                    margin,
                    y,
                    &format!("[{}]", sparkline),
                    FONT_SMALL,
                    colors.bar_disk,
                );
                y += line_height + section_spacing;
            }

            // NET: label line, then sparkline on next line
            if is_enabled(complication_names::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
                canvas.draw_text(margin, y, "NET:", FONT_SMALL, colors.dim);
                let net_rates = format!("D:{} U:{}", net_rx, net_tx);
                let net_rates_w = canvas.text_width(&net_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - net_rates_w,
                    y,
                    &net_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
                let sparkline = ascii_sparkline(
                    &data.net_history,
                    SystemData::compute_graph_scale(&data.net_history),
                    bar_width,
                );
                canvas.draw_text(
                    margin,
                    y,
                    &format!("[{}]", sparkline),
                    FONT_SMALL,
                    colors.bar_net,
                );
            }
        } else {
            // Landscape layout
            // Hostname (always shown)
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complication_names::TIME) {
                if time_format == time_formats::ANALOGUE {
                    // Draw small analog clock on the right
                    let clock_radius = 10_u32;
                    let clock_cx = width as i32 - margin - clock_radius as i32;
                    let clock_cy = y + clock_radius as i32;
                    draw_mini_analog_clock(
                        canvas,
                        clock_cx,
                        clock_cy,
                        clock_radius,
                        data.hour,
                        data.minute,
                        colors.highlight,
                        colors.text,
                    );
                } else {
                    let time_str = data.format_time(time_format);
                    let time_width = canvas.text_width(&time_str, FONT_LARGE);
                    canvas.draw_text(
                        width as i32 - margin - time_width,
                        y,
                        &time_str,
                        FONT_LARGE,
                        colors.text,
                    );
                }
            }
            y += canvas.line_height(FONT_LARGE) + 1;

            // Complication: Date (right-aligned, under time)
            if is_enabled(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_NORMAL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_NORMAL,
                        colors.dim,
                    );
                }
            }

            // Base element: Uptime (always shown, same line as date on left)
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, colors.dim);
            y += canvas.line_height(FONT_NORMAL) + 1;

            // Complication: IP address
            if is_enabled(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    canvas.draw_text(margin, y, &format!("IP: {}", ip), FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL) + 4;
                } else {
                    y += 4;
                }
            }

            // Base element: CPU bar with optional temperature (always shown)
            let cpu_bar = ascii_bar(data.cpu_percent, bar_chars);
            let cpu_text = if is_enabled(complication_names::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    format!("CPU {} {:3.0}%  {:.0}°C", cpu_bar, data.cpu_percent, temp)
                } else {
                    format!("CPU {} {:3.0}%", cpu_bar, data.cpu_percent)
                }
            } else {
                format!("CPU {} {:3.0}%", cpu_bar, data.cpu_percent)
            };
            canvas.draw_text(margin, y, &cpu_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 1;

            // Base element: RAM bar (always shown)
            let ram_bar = ascii_bar(data.ram_percent, bar_chars);
            let ram_text = format!("RAM {} {:3.0}%", ram_bar, data.ram_percent);
            canvas.draw_text(margin, y, &ram_text, FONT_NORMAL, colors.text);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // Complication: Disk I/O
            if is_enabled(complication_names::DISK_IO) {
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
                y += canvas.line_height(FONT_NORMAL);
                let sparkline = ascii_sparkline(
                    &data.disk_history,
                    SystemData::compute_graph_scale(&data.disk_history),
                    bar_chars + 20,
                );
                canvas.draw_text(
                    margin,
                    y,
                    &format!("[{}]", sparkline),
                    FONT_NORMAL,
                    colors.bar_disk,
                );
                y += canvas.line_height(FONT_NORMAL) + 2;
            }

            // Complication: Network
            if is_enabled(complication_names::NETWORK) {
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
                y += canvas.line_height(FONT_NORMAL);
                let sparkline = ascii_sparkline(
                    &data.net_history,
                    SystemData::compute_graph_scale(&data.net_history),
                    bar_chars + 20,
                );
                canvas.draw_text(
                    margin,
                    y,
                    &format!("[{}]", sparkline),
                    FONT_NORMAL,
                    colors.bar_net,
                );
            }
        }
        let _ = y;
    }
}
