//! D-Bus interface implementation using zbus.
//!
//! Provides the `org.ht32panel.Daemon1` interface.

use std::sync::Arc;

use ht32_panel_hw::{lcd::parse_hex_color, Orientation};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use zbus::{interface, Connection};

use crate::config::DbusBusType;
use crate::state::AppState;

/// D-Bus signal types for state change notifications.
#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum DaemonSignals {
    /// Orientation was changed.
    OrientationChanged,
    /// LED settings changed.
    LedChanged,
    /// Display settings (theme, face, etc.) changed.
    DisplaySettingsChanged,
    /// Complication option changed.
    ComplicationOptionChanged,
}

/// D-Bus interface implementation for the HT32 Panel Daemon.
pub struct Daemon1Interface {
    state: Arc<AppState>,
    signal_tx: broadcast::Sender<DaemonSignals>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

impl Daemon1Interface {
    /// Creates a new D-Bus interface.
    pub fn new(
        state: Arc<AppState>,
        signal_tx: broadcast::Sender<DaemonSignals>,
        shutdown_tx: tokio::sync::mpsc::Sender<()>,
    ) -> Self {
        Self {
            state,
            signal_tx,
            shutdown_tx,
        }
    }
}

#[interface(name = "org.ht32panel.Daemon1")]
impl Daemon1Interface {
    /// Sets the display orientation.
    async fn set_orientation(&self, orientation: &str) -> zbus::fdo::Result<()> {
        let orientation: Orientation = orientation
            .parse()
            .map_err(|_| zbus::fdo::Error::InvalidArgs("Invalid orientation".to_string()))?;

        self.state
            .set_orientation(orientation)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        // Emit signal
        let _ = self.signal_tx.send(DaemonSignals::OrientationChanged);

        debug!("D-Bus: SetOrientation({})", orientation);
        Ok(())
    }

    /// Gets the current orientation.
    fn get_orientation(&self) -> String {
        self.state.orientation().to_string()
    }

    /// Clears the display to a solid color.
    fn clear_display(&self, color: &str) -> zbus::fdo::Result<()> {
        let color_u16 = parse_hex_color(color)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("Invalid color format".to_string()))?;

        self.state
            .clear_display(color_u16)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        debug!("D-Bus: ClearDisplay({})", color);
        Ok(())
    }

    /// Sets the display face.
    fn set_face(&self, face: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_face(face)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;

        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: SetFace({})", face);
        Ok(())
    }

    /// Gets the current face name.
    fn get_face(&self) -> String {
        self.state.face_name()
    }

    /// Gets the current color theme name.
    fn get_theme(&self) -> String {
        self.state.theme_name()
    }

    /// Sets the color theme by name.
    fn set_theme(&self, name: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_theme(name)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;

        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: SetTheme({})", name);
        Ok(())
    }

    /// Lists available color themes (IDs only, for backwards compatibility).
    fn list_themes(&self) -> Vec<String> {
        self.state
            .available_themes()
            .iter()
            .map(|t| t.id.to_string())
            .collect()
    }

    /// Lists available color themes with display names.
    /// Returns JSON-encoded theme data.
    fn list_themes_detailed(&self) -> Vec<String> {
        self.state
            .available_themes()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "display_name": t.display_name
                })
                .to_string()
            })
            .collect()
    }

    /// Lists available faces (IDs only).
    fn list_face_ids(&self) -> Vec<String> {
        crate::faces::available_faces()
            .iter()
            .map(|f| f.id.to_string())
            .collect()
    }

    /// Lists available faces with display names.
    /// Returns JSON-encoded face data.
    fn list_faces(&self) -> Vec<String> {
        crate::faces::available_faces()
            .iter()
            .map(|f| {
                serde_json::json!({
                    "id": f.id,
                    "display_name": f.display_name
                })
                .to_string()
            })
            .collect()
    }

    /// Returns the current framebuffer as PNG data.
    fn get_screen_png(&self) -> zbus::fdo::Result<Vec<u8>> {
        self.state
            .get_screen_png()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Sets LED parameters.
    async fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> zbus::fdo::Result<()> {
        // Validate parameters
        if !(1..=5).contains(&theme) {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Theme must be 1-5".to_string(),
            ));
        }
        if !(1..=5).contains(&intensity) {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Intensity must be 1-5".to_string(),
            ));
        }
        if !(1..=5).contains(&speed) {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Speed must be 1-5".to_string(),
            ));
        }

        self.state
            .set_led(theme, intensity, speed)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        // Emit signal
        let _ = self.signal_tx.send(DaemonSignals::LedChanged);

        debug!("D-Bus: SetLed({}, {}, {})", theme, intensity, speed);
        Ok(())
    }

    /// Turns off LEDs.
    async fn led_off(&self) -> zbus::fdo::Result<()> {
        self.state
            .led_off()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        // Emit signal
        let _ = self.signal_tx.send(DaemonSignals::LedChanged);

        debug!("D-Bus: LedOff");
        Ok(())
    }

    /// Gets current LED settings as (theme, intensity, speed).
    fn get_led_settings(&self) -> (u8, u8, u8) {
        self.state.led_settings()
    }

    /// Shuts down the daemon.
    async fn quit(&self) -> zbus::fdo::Result<()> {
        info!("D-Bus: Quit requested");
        self.shutdown_tx
            .send(())
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(())
    }

    // Properties

    /// Whether the LCD device is connected.
    #[zbus(property)]
    fn connected(&self) -> bool {
        self.state.is_lcd_connected()
    }

    /// Whether the web UI is enabled.
    #[zbus(property)]
    fn web_enabled(&self) -> bool {
        self.state.is_web_enabled()
    }

    /// Current display orientation.
    #[zbus(property)]
    fn orientation(&self) -> String {
        self.state.orientation().to_string()
    }

    /// Current LED theme (1-5).
    #[zbus(property)]
    fn led_theme(&self) -> u8 {
        self.state.led_settings().0
    }

    /// Current LED intensity (1-5).
    #[zbus(property)]
    fn led_intensity(&self) -> u8 {
        self.state.led_settings().1
    }

    /// Current LED speed (1-5).
    #[zbus(property)]
    fn led_speed(&self) -> u8 {
        self.state.led_settings().2
    }

    /// Current color theme name.
    #[zbus(property)]
    fn theme(&self) -> String {
        self.state.theme_name()
    }

    /// Current display face name.
    #[zbus(property)]
    fn face(&self) -> String {
        self.state.face_name()
    }

    /// Lists all available network interfaces.
    fn list_network_interfaces(&self) -> Vec<String> {
        self.state.list_network_interfaces()
    }

    /// Lists available complications for the current face.
    /// Returns a list of (id, name, description, enabled) tuples.
    fn list_complications(&self) -> Vec<(String, String, String, bool)> {
        let available = self.state.available_complications();
        let enabled = self.state.enabled_complications();
        available
            .into_iter()
            .map(|c| {
                let is_enabled = enabled.contains(&c.id);
                (c.id, c.name, c.description, is_enabled)
            })
            .collect()
    }

    /// Lists available complications with full details including options.
    /// Returns JSON-encoded complication data.
    fn list_complications_detailed(&self) -> Vec<String> {
        let available = self.state.available_complications();
        let enabled = self.state.enabled_complications();
        let face_name = self.state.face_name();

        available
            .into_iter()
            .map(|c| {
                let is_enabled = enabled.contains(&c.id);
                // Get current option values
                let options: Vec<serde_json::Value> = c.options.iter().map(|opt| {
                    let current_value = self.state.get_complication_option(&c.id, &opt.id)
                        .unwrap_or_else(|| opt.default_value.clone());

                    match &opt.option_type {
                        crate::faces::ComplicationOptionType::Choice(choices) => {
                            // For network interface, dynamically get available interfaces
                            let choice_list: Vec<serde_json::Value> = if c.id == "network" && opt.id == "interface" {
                                let mut ifaces: Vec<serde_json::Value> = vec![
                                    serde_json::json!({"value": "auto", "label": "Auto-detect"})
                                ];
                                for iface in self.state.list_network_interfaces() {
                                    ifaces.push(serde_json::json!({"value": iface, "label": iface}));
                                }
                                ifaces
                            } else {
                                choices.iter().map(|ch| {
                                    serde_json::json!({"value": ch.value, "label": ch.label})
                                }).collect()
                            };
                            serde_json::json!({
                                "id": opt.id,
                                "name": opt.name,
                                "description": opt.description,
                                "current_value": current_value,
                                "type": "choice",
                                "choices": choice_list
                            })
                        }
                        crate::faces::ComplicationOptionType::Boolean => {
                            serde_json::json!({
                                "id": opt.id,
                                "name": opt.name,
                                "description": opt.description,
                                "current_value": current_value,
                                "type": "boolean",
                                "choices": [
                                    {"value": "true", "label": "Yes"},
                                    {"value": "false", "label": "No"}
                                ]
                            })
                        }
                        crate::faces::ComplicationOptionType::Range { min, max, step } => {
                            serde_json::json!({
                                "id": opt.id,
                                "name": opt.name,
                                "description": opt.description,
                                "current_value": current_value,
                                "type": "range",
                                "min": min,
                                "max": max,
                                "step": step
                            })
                        }
                    }
                }).collect();

                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "description": c.description,
                    "enabled": is_enabled,
                    "options": options,
                    "face": face_name
                }).to_string()
            })
            .collect()
    }

    /// Gets enabled complications for the current face.
    fn get_enabled_complications(&self) -> Vec<String> {
        self.state.enabled_complications().into_iter().collect()
    }

    /// Enables a complication for the current face.
    fn enable_complication(&self, complication_id: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_complication_enabled(complication_id, true)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: EnableComplication({})", complication_id);
        Ok(())
    }

    /// Disables a complication for the current face.
    fn disable_complication(&self, complication_id: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_complication_enabled(complication_id, false)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: DisableComplication({})", complication_id);
        Ok(())
    }

    /// Gets a complication option value.
    fn get_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
    ) -> zbus::fdo::Result<String> {
        self.state
            .get_complication_option(complication_id, option_id)
            .ok_or_else(|| {
                zbus::fdo::Error::InvalidArgs(format!(
                    "Unknown option '{}' for complication '{}'",
                    option_id, complication_id
                ))
            })
    }

    /// Sets a complication option value.
    fn set_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
        value: &str,
    ) -> zbus::fdo::Result<()> {
        self.state
            .set_complication_option(complication_id, option_id, value)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let _ = self
            .signal_tx
            .send(DaemonSignals::ComplicationOptionChanged);
        debug!(
            "D-Bus: SetComplicationOption({}, {}, {})",
            complication_id, option_id, value
        );
        Ok(())
    }
}

/// Connects to the appropriate D-Bus bus based on configuration.
async fn connect_to_bus(bus_type: DbusBusType) -> anyhow::Result<(Connection, &'static str)> {
    match bus_type {
        DbusBusType::Session => {
            let conn = Connection::session()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect to session bus: {}", e))?;
            Ok((conn, "session"))
        }
        DbusBusType::System => {
            let conn = Connection::system()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect to system bus: {}", e))?;
            Ok((conn, "system"))
        }
        DbusBusType::Auto => {
            // Try session bus first, fall back to system bus
            match Connection::session().await {
                Ok(conn) => Ok((conn, "session")),
                Err(session_err) => {
                    warn!(
                        "Session bus unavailable ({}), trying system bus",
                        session_err
                    );
                    let conn = Connection::system().await.map_err(|system_err| {
                        anyhow::anyhow!(
                            "Failed to connect to any D-Bus: session={}, system={}",
                            session_err,
                            system_err
                        )
                    })?;
                    Ok((conn, "system"))
                }
            }
        }
    }
}

/// Runs the D-Bus server.
pub async fn run_dbus_server(
    state: Arc<AppState>,
    signal_tx: broadcast::Sender<DaemonSignals>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
    bus_type: DbusBusType,
) -> anyhow::Result<Connection> {
    let interface = Daemon1Interface::new(state, signal_tx, shutdown_tx);

    let (connection, bus_name) = connect_to_bus(bus_type).await?;

    connection
        .object_server()
        .at("/org/ht32panel/Daemon", interface)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to register object: {}", e))?;

    connection
        .request_name("org.ht32panel.Daemon")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to request bus name: {}", e))?;

    info!(
        "D-Bus service registered at org.ht32panel.Daemon on {} bus",
        bus_name
    );
    Ok(connection)
}
