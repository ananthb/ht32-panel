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

use crate::sensors::data::IpDisplayPreference;
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

/// Theme partial template.
#[derive(Template)]
#[template(path = "partials/theme.html")]
struct ThemeTemplate {
    current: String,
    themes: Vec<String>,
}

/// Network interface partial template.
#[derive(Template)]
#[template(path = "partials/network.html")]
struct NetworkTemplate {
    current: String,
    interfaces: Vec<String>,
    is_auto: bool,
}

/// IP display option for template.
struct IpDisplayOption {
    value: String,
    name: &'static str,
}

/// IP display preference partial template.
#[derive(Template)]
#[template(path = "partials/ip-display.html")]
struct IpDisplayTemplate {
    current: String,
    options: Vec<IpDisplayOption>,
}

/// Preview partial template.
#[derive(Template)]
#[template(path = "partials/preview.html")]
struct PreviewTemplate {
    timestamp: u128,
}

/// Complication item for template.
struct ComplicationItem {
    id: String,
    name: String,
    description: String,
    enabled: bool,
}

/// Complications partial template.
#[derive(Template)]
#[template(path = "partials/complications.html")]
struct ComplicationsTemplate {
    face_name: String,
    complications: Vec<ComplicationItem>,
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
        .route("/theme", get(theme_get).post(theme_set))
        .route(
            "/network-interface",
            get(network_interface_get).post(network_interface_set),
        )
        .route("/ip-display", get(ip_display_get).post(ip_display_set))
        .route("/complications", get(complications_get).post(complications_set))
        .route("/preview", get(preview_get))
        .route("/refresh-interval", post(refresh_interval_set))
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

/// GET /theme - Theme controls partial
async fn theme_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.theme_name();
    let themes: Vec<String> = state
        .available_themes()
        .iter()
        .map(|s| s.to_string())
        .collect();
    Html(ThemeTemplate { current, themes }.render().unwrap())
}

/// Form data for theme.
#[derive(Deserialize)]
struct ThemeForm {
    theme: String,
}

/// POST /theme - Set theme
async fn theme_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ThemeForm>,
) -> impl IntoResponse {
    let _ = state.set_theme(&form.theme);
    let current = state.theme_name();
    let themes: Vec<String> = state
        .available_themes()
        .iter()
        .map(|s| s.to_string())
        .collect();
    Html(ThemeTemplate { current, themes }.render().unwrap())
}

/// GET /network-interface - Network interface controls partial
async fn network_interface_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.network_interface_config();
    let interfaces = state.list_network_interfaces();
    let is_auto = state.network_interface().is_none();
    Html(
        NetworkTemplate {
            current,
            interfaces,
            is_auto,
        }
        .render()
        .unwrap(),
    )
}

/// Form data for network interface.
#[derive(Deserialize)]
struct NetworkInterfaceForm {
    interface: String,
}

/// POST /network-interface - Set network interface
async fn network_interface_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<NetworkInterfaceForm>,
) -> impl IntoResponse {
    let iface = if form.interface.eq_ignore_ascii_case("auto") || form.interface.is_empty() {
        None
    } else {
        Some(form.interface)
    };
    state.set_network_interface(iface);

    let current = state.network_interface_config();
    let interfaces = state.list_network_interfaces();
    let is_auto = state.network_interface().is_none();
    Html(
        NetworkTemplate {
            current,
            interfaces,
            is_auto,
        }
        .render()
        .unwrap(),
    )
}

/// GET /ip-display - IP display preference controls partial
async fn ip_display_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.ip_display().to_string();
    let options: Vec<IpDisplayOption> = IpDisplayPreference::all()
        .iter()
        .map(|p| IpDisplayOption {
            value: p.to_string(),
            name: p.display_name(),
        })
        .collect();
    Html(IpDisplayTemplate { current, options }.render().unwrap())
}

/// Form data for IP display preference.
#[derive(Deserialize)]
struct IpDisplayForm {
    preference: String,
}

/// POST /ip-display - Set IP display preference
async fn ip_display_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<IpDisplayForm>,
) -> impl IntoResponse {
    if let Ok(pref) = form.preference.parse::<IpDisplayPreference>() {
        state.set_ip_display(pref);
    }

    let current = state.ip_display().to_string();
    let options: Vec<IpDisplayOption> = IpDisplayPreference::all()
        .iter()
        .map(|p| IpDisplayOption {
            value: p.to_string(),
            name: p.display_name(),
        })
        .collect();
    Html(IpDisplayTemplate { current, options }.render().unwrap())
}

/// GET /preview - Preview image partial
async fn preview_get() -> impl IntoResponse {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    Html(PreviewTemplate { timestamp }.render().unwrap())
}

/// Form data for refresh interval (milliseconds).
#[derive(Deserialize)]
struct RefreshIntervalForm {
    interval: u32,
}

/// POST /refresh-interval - Set LCD refresh interval in milliseconds
async fn refresh_interval_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<RefreshIntervalForm>,
) -> impl IntoResponse {
    state.set_refresh_interval_ms(form.interval);
    StatusCode::OK
}

/// GET /complications - Complications controls partial
async fn complications_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let face_name = state.face_name();
    let available = state.available_complications();
    let enabled = state.enabled_complications();

    let complications: Vec<ComplicationItem> = available
        .into_iter()
        .map(|c| ComplicationItem {
            enabled: enabled.contains(&c.id),
            id: c.id,
            name: c.name,
            description: c.description,
        })
        .collect();

    Html(
        ComplicationsTemplate {
            face_name,
            complications,
        }
        .render()
        .unwrap(),
    )
}

/// Form data for complication toggle.
#[derive(Deserialize)]
struct ComplicationForm {
    complication: String,
    enabled: Option<String>,
}

/// POST /complications - Toggle a complication
async fn complications_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ComplicationForm>,
) -> impl IntoResponse {
    let enabled = form.enabled.as_deref() == Some("on");
    let _ = state.set_complication_enabled(&form.complication, enabled);

    // Re-render the complications list
    let face_name = state.face_name();
    let available = state.available_complications();
    let enabled_set = state.enabled_complications();

    let complications: Vec<ComplicationItem> = available
        .into_iter()
        .map(|c| ComplicationItem {
            enabled: enabled_set.contains(&c.id),
            id: c.id,
            name: c.name,
            description: c.description,
        })
        .collect();

    Html(
        ComplicationsTemplate {
            face_name,
            complications,
        }
        .render()
        .unwrap(),
    )
}
