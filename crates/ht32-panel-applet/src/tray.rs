//! System tray implementation using StatusNotifierItem (SNI).

use ksni::{menu::*, Tray, TrayService};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// LED theme options: (display name, theme byte value)
const LED_THEMES: &[(&str, u8)] = &[
    ("Rainbow", 1),
    ("Breathing", 2),
    ("Colors", 3),
    ("Off", 4),
    ("Auto", 5),
];

/// Orientation options: (display name, orientation string)
const ORIENTATIONS: &[(&str, &str)] = &[
    ("Landscape", "landscape"),
    ("Portrait", "portrait"),
    ("Landscape (Upside Down)", "landscape-upside-down"),
    ("Portrait (Upside Down)", "portrait-upside-down"),
];

/// Commands that can be sent from tray callbacks to the async worker.
#[derive(Debug, Clone)]
pub enum TrayCommand {
    SetLedTheme(u8),
    SetOrientation(String),
    QuitDaemon,
}

/// Shared state for the tray applet.
pub struct TrayState {
    pub connected: bool,
    pub led_theme: u8,
    pub led_intensity: u8,
    pub led_speed: u8,
    pub orientation: String,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            connected: false,
            led_theme: 2, // Breathing
            led_intensity: 3,
            led_speed: 3,
            orientation: "landscape".to_string(),
        }
    }
}

/// The HT32 Panel tray icon.
pub struct HT32PanelTray {
    state: Arc<Mutex<TrayState>>,
    command_tx: mpsc::UnboundedSender<TrayCommand>,
}

impl HT32PanelTray {
    /// Creates a new tray icon instance.
    pub fn new(
        state: Arc<Mutex<TrayState>>,
        command_tx: mpsc::UnboundedSender<TrayCommand>,
    ) -> Self {
        Self { state, command_tx }
    }

    fn set_led_theme(&mut self, index: usize) {
        if let Some((_, theme)) = LED_THEMES.get(index) {
            if let Err(e) = self.command_tx.send(TrayCommand::SetLedTheme(*theme)) {
                debug!("Failed to send LED theme command: {}", e);
            }
            // Update local state immediately for UI feedback
            if let Ok(mut s) = self.state.lock() {
                s.led_theme = *theme;
            }
        }
    }

    fn set_orientation(&mut self, index: usize) {
        if let Some((_, orientation)) = ORIENTATIONS.get(index) {
            if let Err(e) = self
                .command_tx
                .send(TrayCommand::SetOrientation(orientation.to_string()))
            {
                debug!("Failed to send orientation command: {}", e);
            }
            // Update local state immediately for UI feedback
            if let Ok(mut s) = self.state.lock() {
                s.orientation = orientation.to_string();
            }
        }
    }

    fn quit_daemon(&self) {
        if let Err(e) = self.command_tx.send(TrayCommand::QuitDaemon) {
            debug!("Failed to send quit command: {}", e);
        }
    }

    fn open_web_ui(&self) {
        if let Err(e) = open::that("http://localhost:8686") {
            warn!("Failed to open web UI: {}", e);
        }
    }
}

impl Tray for HT32PanelTray {
    fn id(&self) -> String {
        "ht32-panel-applet".to_string()
    }

    fn title(&self) -> String {
        "HT32 Panel".to_string()
    }

    fn icon_name(&self) -> String {
        let state = self.state.lock().unwrap();
        if state.connected {
            "display-brightness-symbolic".to_string()
        } else {
            "display-brightness-off-symbolic".to_string()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = self.state.lock().unwrap();
        let current_theme = state.led_theme;
        let current_orientation = state.orientation.clone();
        drop(state);

        // Find current LED theme index
        let led_selected = LED_THEMES
            .iter()
            .position(|(_, t)| *t == current_theme)
            .unwrap_or(0);

        // Find current orientation index
        let orientation_selected = ORIENTATIONS
            .iter()
            .position(|(_, o)| *o == current_orientation)
            .unwrap_or(0);

        // Create LED theme radio items
        let led_options: Vec<RadioItem> = LED_THEMES
            .iter()
            .map(|(name, _)| RadioItem {
                label: name.to_string(),
                ..Default::default()
            })
            .collect();

        // Create orientation radio items
        let orientation_options: Vec<RadioItem> = ORIENTATIONS
            .iter()
            .map(|(name, _)| RadioItem {
                label: name.to_string(),
                ..Default::default()
            })
            .collect();

        vec![
            SubMenu {
                label: "LED Theme".to_string(),
                submenu: vec![RadioGroup {
                    selected: led_selected,
                    select: Box::new(|tray: &mut Self, index| {
                        tray.set_led_theme(index);
                    }),
                    options: led_options,
                }
                .into()],
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Orientation".to_string(),
                submenu: vec![RadioGroup {
                    selected: orientation_selected,
                    select: Box::new(|tray: &mut Self, index| {
                        tray.set_orientation(index);
                    }),
                    options: orientation_options,
                }
                .into()],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Open Web UI".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    tray.open_web_ui();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit Daemon".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    tray.quit_daemon();
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Creates the tray service and command receiver.
pub fn create_tray(
    state: Arc<Mutex<TrayState>>,
) -> anyhow::Result<(
    TrayService<HT32PanelTray>,
    mpsc::UnboundedReceiver<TrayCommand>,
)> {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let tray = HT32PanelTray::new(state, command_tx);
    let service = TrayService::new(tray);
    Ok((service, command_rx))
}
