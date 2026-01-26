//! Detailed face with progress bars.
//!
//! Layout (320x170):
//! ```text
//! endeavour               18:45:32
//! Uptime: 5d 12h 34m
//!
//! CPU [████████░░░░░░░░] 45%
//! RAM [██████████░░░░░░] 67%
//!
//! Disk  R: 12 MB/s   W: 5 MB/s
//! Net   ↓: 1.2 MB/s  ↑: 0.8 MB/s
//! ```

use super::Face;
use crate::rendering::Canvas;
use crate::sensors::data::SystemData;

/// Colors for the detailed face.
const COLOR_WHITE: u32 = 0xFFFFFF;
const COLOR_GRAY: u32 = 0xAAAAAA;
const COLOR_CYAN: u32 = 0x00DDDD;
const COLOR_GREEN: u32 = 0x00AA00;
const COLOR_BLUE: u32 = 0x0066CC;
const COLOR_BAR_BG: u32 = 0x333333;

/// Font sizes.
const FONT_LARGE: f32 = 16.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// Progress bar dimensions.
const BAR_WIDTH: u32 = 160;
const BAR_HEIGHT: u32 = 12;

/// A detailed face with progress bars.
pub struct DetailedFace;

impl DetailedFace {
    /// Creates a new detailed face.
    pub fn new() -> Self {
        Self
    }

    /// Draws a progress bar.
    fn draw_progress_bar(
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        percent: f64,
        fill_color: u32,
    ) {
        // Draw background
        canvas.fill_rect(x, y, width, height, COLOR_BAR_BG);

        // Draw filled portion
        let fill_width = ((width as f64 * (percent / 100.0)) as u32).min(width);
        if fill_width > 0 {
            canvas.fill_rect(x, y, fill_width, height, fill_color);
        }
    }
}

impl Default for DetailedFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for DetailedFace {
    fn name(&self) -> &str {
        "detailed"
    }

    fn render(&self, canvas: &mut Canvas, data: &SystemData) {
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 8;
        let mut y = margin;

        if portrait {
            // Portrait layout - narrower bars, stacked text
            let bar_width = (width as i32 - margin * 2 - 70) as u32;
            let bar_x = margin + 35;

            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, COLOR_CYAN);
            y += canvas.line_height(FONT_LARGE) + 2;

            canvas.draw_text(margin, y, &data.time, FONT_LARGE, COLOR_WHITE);
            y += canvas.line_height(FONT_LARGE) + 2;

            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, COLOR_GRAY);
            y += canvas.line_height(FONT_NORMAL) + 6;

            // CPU bar
            canvas.draw_text(margin, y, "CPU", FONT_SMALL, COLOR_WHITE);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                bar_width,
                BAR_HEIGHT,
                data.cpu_percent,
                COLOR_GREEN,
            );
            let cpu_pct = format!("{:2.0}%", data.cpu_percent);
            canvas.draw_text(
                bar_x + bar_width as i32 + 4,
                y,
                &cpu_pct,
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 4;

            // RAM bar
            canvas.draw_text(margin, y, "RAM", FONT_SMALL, COLOR_WHITE);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                bar_width,
                BAR_HEIGHT,
                data.ram_percent,
                COLOR_BLUE,
            );
            let ram_pct = format!("{:2.0}%", data.ram_percent);
            canvas.draw_text(
                bar_x + bar_width as i32 + 4,
                y,
                &ram_pct,
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 6;

            // Disk I/O - split lines
            let disk_r = SystemData::format_rate(data.disk_read_rate);
            let disk_w = SystemData::format_rate(data.disk_write_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("Disk R: {}", disk_r),
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 2;
            canvas.draw_text(
                margin,
                y,
                &format!("Disk W: {}", disk_w),
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 4;

            // Network I/O - split lines
            let net_rx = SystemData::format_rate(data.net_rx_rate);
            let net_tx = SystemData::format_rate(data.net_tx_rate);
            canvas.draw_text(
                margin,
                y,
                &format!("Net \u{2193}: {}", net_rx),
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 2;
            canvas.draw_text(
                margin,
                y,
                &format!("Net \u{2191}: {}", net_tx),
                FONT_SMALL,
                COLOR_WHITE,
            );
        } else {
            // Landscape layout
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

            let uptime_text = format!("Uptime: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, COLOR_GRAY);
            y += canvas.line_height(FONT_NORMAL) + 8;

            let bar_x = margin + 35;
            canvas.draw_text(margin, y, "CPU", FONT_SMALL, COLOR_WHITE);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                BAR_WIDTH,
                BAR_HEIGHT,
                data.cpu_percent,
                COLOR_GREEN,
            );
            let cpu_percent = format!("{:3.0}%", data.cpu_percent);
            canvas.draw_text(
                bar_x + BAR_WIDTH as i32 + 6,
                y,
                &cpu_percent,
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 4;

            canvas.draw_text(margin, y, "RAM", FONT_SMALL, COLOR_WHITE);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                BAR_WIDTH,
                BAR_HEIGHT,
                data.ram_percent,
                COLOR_BLUE,
            );
            let ram_percent = format!("{:3.0}%", data.ram_percent);
            canvas.draw_text(
                bar_x + BAR_WIDTH as i32 + 6,
                y,
                &ram_percent,
                FONT_SMALL,
                COLOR_WHITE,
            );
            y += canvas.line_height(FONT_SMALL) + 8;

            let disk_r = SystemData::format_rate(data.disk_read_rate);
            let disk_w = SystemData::format_rate(data.disk_write_rate);
            let disk_text = format!("Disk  R: {}  W: {}", disk_r, disk_w);
            canvas.draw_text(margin, y, &disk_text, FONT_SMALL, COLOR_WHITE);
            y += canvas.line_height(FONT_SMALL) + 4;

            let net_rx = SystemData::format_rate(data.net_rx_rate);
            let net_tx = SystemData::format_rate(data.net_tx_rate);
            let net_text = format!("Net   \u{2193}: {}  \u{2191}: {}", net_rx, net_tx);
            canvas.draw_text(margin, y, &net_text, FONT_SMALL, COLOR_WHITE);
        }
    }
}
