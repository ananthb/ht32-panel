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

use crate::faces::{
    available_faces, available_themes, complication_names, complication_options,
    ComplicationOptionType,
};
use crate::state::AppState;

/// Face option for template rendering.
#[derive(Clone)]
struct FaceOption {
    id: String,
    display_name: String,
}

/// Theme option for template rendering.
#[derive(Clone)]
struct ThemeOption {
    id: String,
    display_name: String,
}
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
    faces: Vec<FaceOption>,
}

/// LED controls partial template.
#[derive(Template)]
#[template(path = "partials/led.html")]
struct LedTemplate {
    theme: u8,
    intensity: u8,
    speed: u8,
    error: Option<String>,
}

/// Theme partial template.
#[derive(Template)]
#[template(path = "partials/theme.html")]
struct ThemeTemplate {
    current: String,
    themes: Vec<ThemeOption>,
}

/// Preview partial template.
#[derive(Template)]
#[template(path = "partials/preview.html")]
struct PreviewTemplate {
    timestamp: u128,
}

/// Complication option choice for template.
struct ComplicationOptionChoice {
    value: String,
    label: String,
}

/// Range parameters for slider options.
struct ComplicationOptionRange {
    min: f32,
    max: f32,
    step: f32,
}

/// Complication option for template.
struct ComplicationOptionItem {
    id: String,
    name: String,
    current_value: String,
    is_range: bool,
    choices: Vec<ComplicationOptionChoice>,
    range: Option<ComplicationOptionRange>,
}

/// Complication item for template.
struct ComplicationItem {
    id: String,
    name: String,
    description: String,
    enabled: bool,
    options: Vec<ComplicationOptionItem>,
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
            "/complications",
            get(complications_get).post(complications_set),
        )
        .route("/complication-option", post(complication_option_set))
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
    let faces: Vec<FaceOption> = available_faces()
        .iter()
        .map(|f| FaceOption {
            id: f.id.to_string(),
            display_name: f.display_name.to_string(),
        })
        .collect();
    Html(FaceTemplate { current, faces }.render().unwrap())
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
    let faces: Vec<FaceOption> = available_faces()
        .iter()
        .map(|f| FaceOption {
            id: f.id.to_string(),
            display_name: f.display_name.to_string(),
        })
        .collect();
    Html(FaceTemplate { current, faces }.render().unwrap())
}

/// GET /led - LED controls partial
async fn led_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (theme, intensity, speed) = state.led_settings();
    Html(
        LedTemplate {
            theme,
            intensity,
            speed,
            error: None,
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

    let error = match state.set_led(theme, intensity, speed).await {
        Ok(()) => None,
        Err(e) => {
            tracing::error!("Failed to set LED: {}", e);
            Some(e.to_string())
        }
    };

    let (theme, intensity, speed) = state.led_settings();
    Html(
        LedTemplate {
            theme,
            intensity,
            speed,
            error,
        }
        .render()
        .unwrap(),
    )
}

/// GET /theme - Theme controls partial
async fn theme_get(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let current = state.theme_name();
    let themes: Vec<ThemeOption> = available_themes()
        .iter()
        .map(|t| ThemeOption {
            id: t.id.to_string(),
            display_name: t.display_name.to_string(),
        })
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
    let themes: Vec<ThemeOption> = available_themes()
        .iter()
        .map(|t| ThemeOption {
            id: t.id.to_string(),
            display_name: t.display_name.to_string(),
        })
        .collect();
    Html(ThemeTemplate { current, themes }.render().unwrap())
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
    let interfaces = state.list_network_interfaces();

    let complications: Vec<ComplicationItem> = available
        .into_iter()
        .map(|c| {
            let options: Vec<ComplicationOptionItem> = c
                .options
                .iter()
                .map(|opt| {
                    let current_value = state
                        .get_complication_option(&c.id, &opt.id)
                        .unwrap_or_else(|| opt.default_value.clone());

                    match &opt.option_type {
                        ComplicationOptionType::Choice(choices) => {
                            // For network interface, dynamically populate with available interfaces
                            let choice_list = if c.id == complication_names::NETWORK
                                && opt.id == complication_options::INTERFACE
                            {
                                let mut iface_choices = vec![ComplicationOptionChoice {
                                    value: "auto".to_string(),
                                    label: "Auto-detect".to_string(),
                                }];
                                for iface in &interfaces {
                                    iface_choices.push(ComplicationOptionChoice {
                                        value: iface.clone(),
                                        label: iface.clone(),
                                    });
                                }
                                iface_choices
                            } else {
                                choices
                                    .iter()
                                    .map(|ch| ComplicationOptionChoice {
                                        value: ch.value.clone(),
                                        label: ch.label.clone(),
                                    })
                                    .collect()
                            };
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: false,
                                choices: choice_list,
                                range: None,
                            }
                        }
                        ComplicationOptionType::Boolean => {
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: false,
                                choices: vec![
                                    ComplicationOptionChoice {
                                        value: "true".to_string(),
                                        label: "Yes".to_string(),
                                    },
                                    ComplicationOptionChoice {
                                        value: "false".to_string(),
                                        label: "No".to_string(),
                                    },
                                ],
                                range: None,
                            }
                        }
                        ComplicationOptionType::Range { min, max, step } => {
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: true,
                                choices: Vec::new(),
                                range: Some(ComplicationOptionRange {
                                    min: *min,
                                    max: *max,
                                    step: *step,
                                }),
                            }
                        }
                    }
                })
                .collect();

            ComplicationItem {
                enabled: enabled.contains(&c.id),
                id: c.id,
                name: c.name,
                description: c.description,
                options,
            }
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
    render_complications(&state)
}

/// Form data for complication option.
#[derive(Deserialize)]
struct ComplicationOptionForm {
    complication: String,
    option: String,
    value: String,
}

/// POST /complication-option - Set a complication option value
async fn complication_option_set(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ComplicationOptionForm>,
) -> impl IntoResponse {
    let _ = state.set_complication_option(&form.complication, &form.option, &form.value);

    // Re-render the complications list
    render_complications(&state)
}

/// Helper to render the complications template
fn render_complications(state: &Arc<AppState>) -> Html<String> {
    let face_name = state.face_name();
    let available = state.available_complications();
    let enabled_set = state.enabled_complications();
    let interfaces = state.list_network_interfaces();

    let complications: Vec<ComplicationItem> = available
        .into_iter()
        .map(|c| {
            let options: Vec<ComplicationOptionItem> = c
                .options
                .iter()
                .map(|opt| {
                    let current_value = state
                        .get_complication_option(&c.id, &opt.id)
                        .unwrap_or_else(|| opt.default_value.clone());

                    match &opt.option_type {
                        ComplicationOptionType::Choice(choices) => {
                            // For network interface, dynamically populate with available interfaces
                            let choice_list = if c.id == complication_names::NETWORK
                                && opt.id == complication_options::INTERFACE
                            {
                                let mut iface_choices = vec![ComplicationOptionChoice {
                                    value: "auto".to_string(),
                                    label: "Auto-detect".to_string(),
                                }];
                                for iface in &interfaces {
                                    iface_choices.push(ComplicationOptionChoice {
                                        value: iface.clone(),
                                        label: iface.clone(),
                                    });
                                }
                                iface_choices
                            } else {
                                choices
                                    .iter()
                                    .map(|ch| ComplicationOptionChoice {
                                        value: ch.value.clone(),
                                        label: ch.label.clone(),
                                    })
                                    .collect()
                            };
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: false,
                                choices: choice_list,
                                range: None,
                            }
                        }
                        ComplicationOptionType::Boolean => {
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: false,
                                choices: vec![
                                    ComplicationOptionChoice {
                                        value: "true".to_string(),
                                        label: "Yes".to_string(),
                                    },
                                    ComplicationOptionChoice {
                                        value: "false".to_string(),
                                        label: "No".to_string(),
                                    },
                                ],
                                range: None,
                            }
                        }
                        ComplicationOptionType::Range { min, max, step } => {
                            ComplicationOptionItem {
                                id: opt.id.clone(),
                                name: opt.name.clone(),
                                current_value,
                                is_range: true,
                                choices: Vec::new(),
                                range: Some(ComplicationOptionRange {
                                    min: *min,
                                    max: *max,
                                    step: *step,
                                }),
                            }
                        }
                    }
                })
                .collect();

            ComplicationItem {
                enabled: enabled_set.contains(&c.id),
                id: c.id,
                name: c.name,
                description: c.description,
                options,
            }
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
