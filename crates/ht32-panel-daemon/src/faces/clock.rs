//! Clock face displaying a clean analog clock.
//!
//! A minimalist watch face focused on time display with optional
//! date and hostname complications.

use std::f32::consts::PI;

use super::{
    complication_names, complication_options, complications, date_formats, Complication,
    EnabledComplications, Face, Theme,
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

/// Derive colors from theme for the clock face.
struct FaceColors {
    /// Clock face outline and markers
    outline: u32,
    /// Hour hand color
    hour_hand: u32,
    /// Minute hand color
    minute_hand: u32,
    /// Center dot color
    center: u32,
    /// Text color (for complications)
    text: u32,
    /// Dimmed text color
    dim: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            outline: theme.primary,
            hour_hand: theme.text,
            minute_hand: theme.text,
            center: theme.primary,
            text: theme.text,
            dim: dim_color(theme.text, theme.background, 0.7),
        }
    }
}

/// Font sizes.
const FONT_LARGE: f32 = 32.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// A minimalist analog clock face.
pub struct ClockFace;

/// Display options for the clock face.
struct ClockLayout {
    show_hostname: bool,
    show_date: bool,
    hostname: String,
    date: Option<String>,
}

impl ClockFace {
    /// Creates a new clock face.
    pub fn new() -> Self {
        Self
    }

    /// Draws centered text and returns its height.
    fn draw_centered_text(
        canvas: &mut Canvas,
        y: i32,
        text: &str,
        font_size: f32,
        color: u32,
    ) -> i32 {
        let (width, _) = canvas.dimensions();
        let text_width = canvas.text_width(text, font_size);
        let x = (width as i32 - text_width) / 2;
        canvas.draw_text(x, y, text, font_size, color);
        canvas.line_height(font_size)
    }

    /// Draws a digital time display with optional hostname and date.
    fn draw_digital_time(
        canvas: &mut Canvas,
        hour: u8,
        minute: u8,
        layout: &ClockLayout,
        colors: &FaceColors,
    ) {
        let (_, height) = canvas.dimensions();
        let time_str = format!("{:02}:{:02}", hour, minute);

        // Calculate total height needed
        let time_height = canvas.line_height(FONT_LARGE);
        let mut total_height = time_height;
        if layout.show_hostname {
            total_height += canvas.line_height(FONT_SMALL) + 4;
        }
        if layout.show_date && layout.date.is_some() {
            total_height += canvas.line_height(FONT_NORMAL) + 4;
        }

        let mut y = (height as i32 - total_height) / 2;

        if layout.show_hostname {
            let h = Self::draw_centered_text(canvas, y, &layout.hostname, FONT_SMALL, colors.dim);
            y += h + 4;
        }

        let h = Self::draw_centered_text(canvas, y, &time_str, FONT_LARGE, colors.text);
        y += h + 4;

        if layout.show_date {
            if let Some(date) = &layout.date {
                Self::draw_centered_text(canvas, y, date, FONT_NORMAL, colors.dim);
            }
        }
    }

    /// Draws an analog clock with optional hostname above and date below.
    fn draw_analog_clock(
        canvas: &mut Canvas,
        hour: u8,
        minute: u8,
        layout: &ClockLayout,
        colors: &FaceColors,
    ) {
        let (width, height) = canvas.dimensions();
        let margin = 10;

        // Calculate space for complications
        let top_space = if layout.show_hostname { 18 } else { 0 };
        let bottom_space = if layout.show_date { 20 } else { 0 };

        // Calculate clock size and position
        let available_height = height as i32 - margin * 2 - top_space - bottom_space;
        let available_width = width as i32 - margin * 2;
        let radius = (available_height.min(available_width) / 2) as u32;
        let cx = width as i32 / 2;
        let cy = top_space + margin + radius as i32;

        // Draw hostname above
        if layout.show_hostname {
            Self::draw_centered_text(canvas, margin / 2, &layout.hostname, FONT_SMALL, colors.dim);
        }

        // Draw clock face
        Self::draw_clock_face(canvas, cx, cy, radius, hour, minute, colors);

        // Draw date below
        if layout.show_date {
            if let Some(date) = &layout.date {
                let date_y = cy + radius as i32 + 8;
                Self::draw_centered_text(canvas, date_y, date, FONT_NORMAL, colors.text);
            }
        }
    }

    /// Draws the analog clock face (circle, markers, and hands).
    fn draw_clock_face(
        canvas: &mut Canvas,
        cx: i32,
        cy: i32,
        radius: u32,
        hour: u8,
        minute: u8,
        colors: &FaceColors,
    ) {
        let radius_f = radius as f32;

        // Draw clock face outline
        canvas.draw_arc(cx, cy, radius, 0.0, 2.0 * PI, 2.0, colors.outline);

        // Draw hour markers
        for i in 0..12 {
            let angle = (i as f32) * PI / 6.0 - PI / 2.0;
            let inner_r = radius_f * 0.85;
            let outer_r = radius_f * 0.95;

            let x1 = cx as f32 + inner_r * angle.cos();
            let y1 = cy as f32 + inner_r * angle.sin();
            let x2 = cx as f32 + outer_r * angle.cos();
            let y2 = cy as f32 + outer_r * angle.sin();

            let stroke = if i % 3 == 0 { 3.0 } else { 1.5 };
            canvas.draw_line(x1 as i32, y1 as i32, x2 as i32, y2 as i32, stroke, colors.outline);
        }

        // Calculate hand angles (12 o'clock = -PI/2)
        let minute_angle = (minute as f32) * PI / 30.0 - PI / 2.0;
        let hour_angle = ((hour % 12) as f32 + minute as f32 / 60.0) * PI / 6.0 - PI / 2.0;

        // Draw hour hand (shorter, thicker)
        let hour_length = radius_f * 0.5;
        let hour_x = cx as f32 + hour_length * hour_angle.cos();
        let hour_y = cy as f32 + hour_length * hour_angle.sin();
        canvas.draw_line(cx, cy, hour_x as i32, hour_y as i32, 4.0, colors.hour_hand);

        // Draw minute hand (longer, thinner)
        let minute_length = radius_f * 0.75;
        let minute_x = cx as f32 + minute_length * minute_angle.cos();
        let minute_y = cy as f32 + minute_length * minute_angle.sin();
        canvas.draw_line(cx, cy, minute_x as i32, minute_y as i32, 2.5, colors.minute_hand);

        // Draw center dot
        canvas.fill_circle(cx, cy, 4, colors.center);
    }
}

impl Default for ClockFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for ClockFace {
    fn name(&self) -> &str {
        "clock"
    }

    fn available_complications(&self) -> Vec<Complication> {
        vec![
            complications::hostname(false),
            complications::digital_time(false),
            complications::date(false, date_formats::SHORT),
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
        let is_on = |id: &str| comp.is_enabled(self.name(), id, false);

        // Get date format option
        let date_format = comp
            .get_option(
                self.name(),
                complication_names::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::SHORT);

        // Build layout options
        let layout = ClockLayout {
            show_hostname: is_on("hostname"),
            show_date: is_on(complication_names::DATE),
            hostname: data.hostname.clone(),
            date: data.format_date(date_format),
        };

        if is_on("digital_time") {
            Self::draw_digital_time(canvas, data.hour, data.minute, &layout, &colors);
        } else {
            Self::draw_analog_clock(canvas, data.hour, data.minute, &layout, &colors);
        }
    }
}
