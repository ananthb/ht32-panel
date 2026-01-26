//! HTMX web UI module.

use askama::Template;
use axum::{
    extract::{Form, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::rendering::WidgetRect;
use crate::state::AppState;
use ht32_panel_hw::Orientation;

/// Main index page template.
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

/// Status partial template.
#[derive(Template)]
#[template(path = "partials/status.html")]
struct StatusTemplate {
    connected: bool,
}

/// Orientation partial template.
#[derive(Template)]
#[template(path = "partials/orientation.html")]
struct OrientationTemplate {
    current: String,
}

/// Face partial template.
#[derive(Template)]
#[template(path = "partials/face.html")]
struct FaceTemplate {
    current: String,
}

/// LED controls partial template.
#[derive(Template)]
#[template(path = "partials/led.html")]
struct LedTemplate {
    theme: u8,
    intensity: u8,
    speed: u8,
}

/// Colors partial template.
#[derive(Template)]
#[template(path = "partials/colors.html")]
struct ColorsTemplate {
    background_color: String,
    foreground_color: String,
}

/// Background image partial template.
#[derive(Template)]
#[template(path = "partials/background.html")]
struct BackgroundTemplate {
    background_image: String,
}

/// Preview partial template.
#[derive(Template)]
#[template(path = "partials/preview.html")]
struct PreviewTemplate {
    timestamp: u128,
}

/// Widget info for template.
struct WidgetInfo {
    id: u32,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Widgets partial template.
#[derive(Template)]
#[template(path = "partials/widgets.html")]
struct WidgetsTemplate {
    widgets: Vec<WidgetInfo>,
}

/// Creates the web router with all routes.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Main page
        .route("/", get(index))
        // LCD preview image
        .route("/lcd.png", get(lcd_png))
        // Partials for HTMX
        .route("/status", get(status))
        .route("/orientation", get(orientation_get).post(orientation_set))
        .route("/face", get(face_get).post(face_set))
        .route("/led", get(led_get).post(led_set))
        .route("/colors", get(colors_get).post(colors_set))
        .route("/background", get(background_get).post(background_set))
        .route("/background/clear", post(background_clear))
        .route("/preview", get(preview_get))
        .route("/refresh-rate", post(refresh_rate_set))
        .route("/widgets", get(widgets_get))
        .route("/widgets/add", post(widget_add))
        .route("/widgets/delete", post(widget_delete))
        // State
        .with_state(state)
}

/// GET / - Main page
async fn index() -> impl IntoResponse {
    Html(IndexTemplate.render().unwrap())
}

/// GET /lcd.png - LCD framebuffer as PNG
async fn lcd_png(State(state): State<Arc<AppState>>) -> Response {
    match state.get_screen_png() {
        Ok(png_data) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            png_data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate PNG: {}", e),
        )
            .into_response(),
    }
}

/// GET /status - Connection status partial
async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let connected = state.is_lcd_connected();
    Html(StatusTemplate { connected }.render().unwrap())
}

/// GET /orientation - Orientation controls partial
async fn orientation_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.orientation().to_string();
    Html(OrientationTemplate { current }.render().unwrap())
}

/// Form data for orientation.
#[derive(Deserialize)]
struct OrientationForm {
    orientation: String,
}

/// POST /orientation - Set orientation
async fn orientation_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<OrientationForm>,
) -> impl IntoResponse {
    if let Ok(orientation) = form.orientation.parse::<Orientation>() {
        let _ = state.set_orientation(orientation);
    }
    let current = state.orientation().to_string();
    Html(OrientationTemplate { current }.render().unwrap())
}

/// GET /face - Face controls partial
async fn face_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.face_name();
    Html(FaceTemplate { current }.render().unwrap())
}

/// Form data for face.
#[derive(Deserialize)]
struct FaceForm {
    face: String,
}

/// POST /face - Set face
async fn face_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<FaceForm>,
) -> impl IntoResponse {
    let _ = state.set_face(&form.face);
    let current = state.face_name();
    Html(FaceTemplate { current }.render().unwrap())
}

/// GET /led - LED controls partial
async fn led_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (theme, intensity, speed) = state.led_settings();
    Html(
        LedTemplate {
            theme,
            intensity,
            speed,
        }
        .render()
        .unwrap(),
    )
}

/// Form data for LED settings.
#[derive(Deserialize)]
struct LedForm {
    theme: u8,
    #[serde(default = "default_led")]
    intensity: u8,
    #[serde(default = "default_led")]
    speed: u8,
}

fn default_led() -> u8 {
    3
}

/// POST /led - Set LED settings
async fn led_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LedForm>,
) -> impl IntoResponse {
    let theme = form.theme.clamp(1, 5);
    let intensity = form.intensity.clamp(1, 5);
    let speed = form.speed.clamp(1, 5);

    let _ = state.set_led(theme, intensity, speed).await;

    let (theme, intensity, speed) = state.led_settings();
    Html(
        LedTemplate {
            theme,
            intensity,
            speed,
        }
        .render()
        .unwrap(),
    )
}

/// GET /colors - Color controls partial
async fn colors_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let background_color = format!("#{:06X}", state.background_color());
    let foreground_color = format!("#{:06X}", state.foreground_color());
    Html(
        ColorsTemplate {
            background_color,
            foreground_color,
        }
        .render()
        .unwrap(),
    )
}

/// Form data for colors.
#[derive(Deserialize)]
struct ColorsForm {
    background_color: String,
    foreground_color: String,
}

/// POST /colors - Set colors
async fn colors_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ColorsForm>,
) -> impl IntoResponse {
    let _ = state.set_background_color_hex(&form.background_color);
    let _ = state.set_foreground_color_hex(&form.foreground_color);

    let background_color = format!("#{:06X}", state.background_color());
    let foreground_color = format!("#{:06X}", state.foreground_color());
    Html(
        ColorsTemplate {
            background_color,
            foreground_color,
        }
        .render()
        .unwrap(),
    )
}

/// GET /background - Background image controls partial
async fn background_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let background_image = state
        .background_image()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Html(BackgroundTemplate { background_image }.render().unwrap())
}

/// Form data for background image.
#[derive(Deserialize)]
struct BackgroundForm {
    background_image: String,
}

/// POST /background - Set background image
async fn background_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<BackgroundForm>,
) -> impl IntoResponse {
    let path = if form.background_image.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(&form.background_image))
    };
    state.set_background_image(path);

    let background_image = state
        .background_image()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    Html(BackgroundTemplate { background_image }.render().unwrap())
}

/// POST /background/clear - Clear background image
async fn background_clear(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.set_background_image(None);
    let background_image = String::new();
    Html(BackgroundTemplate { background_image }.render().unwrap())
}

/// GET /preview - Preview image partial
async fn preview_get() -> impl IntoResponse {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    Html(PreviewTemplate { timestamp }.render().unwrap())
}

/// Form data for refresh rate.
#[derive(Deserialize)]
struct RefreshRateForm {
    rate: u32,
}

/// POST /refresh-rate - Set LCD refresh rate
async fn refresh_rate_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RefreshRateForm>,
) -> impl IntoResponse {
    state.set_refresh_rate_secs(form.rate);
    StatusCode::OK
}

/// GET /widgets - Widgets list partial
async fn widgets_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let widgets = state.read_canvas(|canvas| {
        canvas
            .widgets()
            .iter()
            .map(|w| WidgetInfo {
                id: w.id,
                name: w.widget_type.clone(),
                x: w.rect.x,
                y: w.rect.y,
                width: w.rect.width,
                height: w.rect.height,
            })
            .collect()
    });
    Html(WidgetsTemplate { widgets }.render().unwrap())
}

/// Form data for adding widget.
#[derive(Deserialize)]
struct AddWidgetForm {
    name: String,
}

/// POST /widgets/add - Add a widget
async fn widget_add(
    State(state): State<Arc<AppState>>,
    Form(form): Form<AddWidgetForm>,
) -> impl IntoResponse {
    state.with_canvas(|canvas| {
        canvas.add_widget(
            &form.name,
            WidgetRect {
                x: 10,
                y: 10,
                width: 50,
                height: 50,
            },
        );
    });

    let widgets = state.read_canvas(|canvas| {
        canvas
            .widgets()
            .iter()
            .map(|w| WidgetInfo {
                id: w.id,
                name: w.widget_type.clone(),
                x: w.rect.x,
                y: w.rect.y,
                width: w.rect.width,
                height: w.rect.height,
            })
            .collect()
    });
    Html(WidgetsTemplate { widgets }.render().unwrap())
}

/// Form data for deleting widget.
#[derive(Deserialize)]
struct DeleteWidgetForm {
    id: u32,
}

/// POST /widgets/delete - Delete a widget
async fn widget_delete(
    State(state): State<Arc<AppState>>,
    Form(form): Form<DeleteWidgetForm>,
) -> impl IntoResponse {
    state.with_canvas(|canvas| {
        canvas.remove_widget(form.id);
    });

    let widgets = state.read_canvas(|canvas| {
        canvas
            .widgets()
            .iter()
            .map(|w| WidgetInfo {
                id: w.id,
                name: w.widget_type.clone(),
                x: w.rect.x,
                y: w.rect.y,
                width: w.rect.width,
                height: w.rect.height,
            })
            .collect()
    });
    Html(WidgetsTemplate { widgets }.render().unwrap())
}
