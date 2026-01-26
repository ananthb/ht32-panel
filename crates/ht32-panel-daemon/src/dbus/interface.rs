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
pub enum DaemonSignals {
    /// Orientation was changed.
    OrientationChanged,
    /// LED settings changed.
    LedChanged,
    /// Display settings (colors, background image, etc.) changed.
    DisplaySettingsChanged,
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

        debug!("D-Bus: SetFace({})", face);
        Ok(())
    }

    /// Gets the current face name.
    fn get_face(&self) -> String {
        self.state.face_name()
    }

    /// Gets the background color as a hex string (e.g., "#000000").
    fn get_background_color(&self) -> String {
        format!("#{:06X}", self.state.background_color())
    }

    /// Sets the background color from a hex string (e.g., "#000000").
    fn set_background_color(&self, color: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_background_color_hex(color)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;

        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: SetBackgroundColor({})", color);
        Ok(())
    }

    /// Gets the foreground/text color as a hex string (e.g., "#FFFFFF").
    fn get_foreground_color(&self) -> String {
        format!("#{:06X}", self.state.foreground_color())
    }

    /// Sets the foreground/text color from a hex string (e.g., "#FFFFFF").
    fn set_foreground_color(&self, color: &str) -> zbus::fdo::Result<()> {
        self.state
            .set_foreground_color_hex(color)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;

        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: SetForegroundColor({})", color);
        Ok(())
    }

    /// Gets the background image path (empty string if none).
    fn get_background_image(&self) -> String {
        self.state
            .background_image()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Sets the background image path.
    fn set_background_image(&self, path: &str) -> zbus::fdo::Result<()> {
        let bg_path = if path.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(path))
        };
        self.state.set_background_image(bg_path);

        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: SetBackgroundImage({})", path);
        Ok(())
    }

    /// Clears the background image.
    fn clear_background_image(&self) {
        self.state.set_background_image(None);
        let _ = self.signal_tx.send(DaemonSignals::DisplaySettingsChanged);
        debug!("D-Bus: ClearBackgroundImage");
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

    /// Current background color (hex string).
    #[zbus(property)]
    fn background_color(&self) -> String {
        format!("#{:06X}", self.state.background_color())
    }

    /// Current foreground/text color (hex string).
    #[zbus(property)]
    fn foreground_color(&self) -> String {
        format!("#{:06X}", self.state.foreground_color())
    }

    /// Current background image path (empty if none).
    #[zbus(property)]
    fn background_image(&self) -> String {
        self.state
            .background_image()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Current display face name.
    #[zbus(property)]
    fn face(&self) -> String {
        self.state.face_name()
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
