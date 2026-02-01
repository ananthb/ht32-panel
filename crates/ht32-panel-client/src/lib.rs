//! D-Bus client library for communicating with the HT32 Panel Daemon.
//!
//! This crate provides a unified client for both CLI and applet use cases.

use anyhow::{Context, Result};
use tracing::debug;
use zbus::{proxy, Connection};

/// D-Bus bus type selection.
#[derive(Debug, Clone, Copy, Default)]
pub enum BusType {
    /// Session bus (user session).
    Session,
    /// System bus (system-wide).
    System,
    /// Try session first, fall back to system.
    #[default]
    Auto,
}

/// D-Bus proxy for the HT32 Panel Daemon.
#[proxy(
    interface = "org.ht32panel.Daemon1",
    default_service = "org.ht32panel.Daemon",
    default_path = "/org/ht32panel/Daemon"
)]
trait Daemon1 {
    /// Sets the display orientation.
    fn set_orientation(&self, orientation: &str) -> zbus::Result<()>;

    /// Gets the current orientation.
    fn get_orientation(&self) -> zbus::Result<String>;

    /// Clears the display to a solid color.
    fn clear_display(&self, color: &str) -> zbus::Result<()>;

    /// Sets the display face.
    fn set_face(&self, face: &str) -> zbus::Result<()>;

    /// Gets the current face name.
    fn get_face(&self) -> zbus::Result<String>;

    /// Sets LED parameters.
    fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> zbus::Result<()>;

    /// Turns off LEDs.
    fn led_off(&self) -> zbus::Result<()>;

    /// Gets current LED settings as (theme, intensity, speed).
    fn get_led_settings(&self) -> zbus::Result<(u8, u8, u8)>;

    /// Gets the current color theme name.
    fn get_theme(&self) -> zbus::Result<String>;

    /// Sets the color theme by name.
    fn set_theme(&self, name: &str) -> zbus::Result<()>;

    /// Lists available color themes (IDs only).
    fn list_themes(&self) -> zbus::Result<Vec<String>>;

    /// Lists available color themes with display names (JSON-encoded).
    fn list_themes_detailed(&self) -> zbus::Result<Vec<String>>;

    /// Lists available faces (IDs only).
    fn list_face_ids(&self) -> zbus::Result<Vec<String>>;

    /// Lists available faces with display names (JSON-encoded).
    fn list_faces(&self) -> zbus::Result<Vec<String>>;

    /// Lists all available network interfaces.
    fn list_network_interfaces(&self) -> zbus::Result<Vec<String>>;

    /// Lists available complications for the current face.
    /// Returns (id, name, description, enabled) tuples.
    fn list_complications(&self) -> zbus::Result<Vec<(String, String, String, bool)>>;

    /// Lists available complications with full details including options.
    /// Returns JSON-encoded complication data.
    fn list_complications_detailed(&self) -> zbus::Result<Vec<String>>;

    /// Gets enabled complications for the current face.
    fn get_enabled_complications(&self) -> zbus::Result<Vec<String>>;

    /// Enables a complication for the current face.
    fn enable_complication(&self, complication_id: &str) -> zbus::Result<()>;

    /// Disables a complication for the current face.
    fn disable_complication(&self, complication_id: &str) -> zbus::Result<()>;

    /// Gets a complication option value.
    fn get_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
    ) -> zbus::Result<String>;

    /// Sets a complication option value.
    fn set_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
        value: &str,
    ) -> zbus::Result<()>;

    /// Returns the current framebuffer as PNG data.
    fn get_screen_png(&self) -> zbus::Result<Vec<u8>>;

    /// Shuts down the daemon.
    fn quit(&self) -> zbus::Result<()>;

    /// Whether the LCD device is connected.
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

    /// Whether the web UI is enabled.
    #[zbus(property)]
    fn web_enabled(&self) -> zbus::Result<bool>;

    /// Current display orientation.
    #[zbus(property)]
    fn orientation(&self) -> zbus::Result<String>;

    /// Current LED theme (1-5).
    #[zbus(property)]
    fn led_theme(&self) -> zbus::Result<u8>;

    /// Current LED intensity (1-5).
    #[zbus(property)]
    fn led_intensity(&self) -> zbus::Result<u8>;

    /// Current LED speed (1-5).
    #[zbus(property)]
    fn led_speed(&self) -> zbus::Result<u8>;

    /// Current display face name.
    #[zbus(property)]
    fn face(&self) -> zbus::Result<String>;
}

/// D-Bus client wrapper for the daemon.
pub struct DaemonClient {
    proxy: Daemon1Proxy<'static>,
}

impl DaemonClient {
    /// Attempts to connect to the daemon via D-Bus with auto bus detection.
    ///
    /// Tries session bus first, falls back to system bus.
    pub async fn connect() -> Result<Self> {
        Self::connect_with_bus(BusType::Auto).await
    }

    /// Attempts to connect to the daemon via D-Bus with specified bus type.
    pub async fn connect_with_bus(bus_type: BusType) -> Result<Self> {
        let connection = match bus_type {
            BusType::Session => {
                debug!("Connecting to session bus");
                Connection::session()
                    .await
                    .context("Failed to connect to session bus")?
            }
            BusType::System => {
                debug!("Connecting to system bus");
                Connection::system()
                    .await
                    .context("Failed to connect to system bus")?
            }
            BusType::Auto => {
                // Try session bus first, but verify the service exists
                if let Ok(conn) = Connection::session().await {
                    debug!("Connected to session bus, checking for daemon service");
                    if Self::service_exists(&conn).await {
                        debug!("Found daemon on session bus");
                        conn
                    } else {
                        debug!("Daemon not on session bus, trying system bus");
                        let sys_conn = Connection::system()
                            .await
                            .context("Failed to connect to system bus")?;
                        if Self::service_exists(&sys_conn).await {
                            debug!("Found daemon on system bus");
                            sys_conn
                        } else {
                            // Neither bus has the service, return session for better error
                            anyhow::bail!("Daemon service not found on session or system bus. Is ht32paneld running?")
                        }
                    }
                } else {
                    debug!("Session bus unavailable, trying system bus");
                    Connection::system()
                        .await
                        .context("Failed to connect to any D-Bus")?
                }
            }
        };

        let proxy = Daemon1Proxy::new(&connection)
            .await
            .context("Failed to create D-Bus proxy")?;

        Ok(Self { proxy })
    }

    /// Checks if the daemon service exists on the given connection.
    async fn service_exists(conn: &Connection) -> bool {
        use zbus::fdo::DBusProxy;
        if let Ok(dbus_proxy) = DBusProxy::new(conn).await {
            dbus_proxy
                .name_has_owner("org.ht32panel.Daemon".try_into().unwrap())
                .await
                .unwrap_or(false)
        } else {
            false
        }
    }

    /// Sets the display orientation.
    pub async fn set_orientation(&self, orientation: &str) -> Result<()> {
        self.proxy
            .set_orientation(orientation)
            .await
            .context("Failed to set orientation via D-Bus")
    }

    /// Gets the current orientation.
    pub async fn get_orientation(&self) -> Result<String> {
        self.proxy
            .get_orientation()
            .await
            .context("Failed to get orientation via D-Bus")
    }

    /// Clears the display to a solid color.
    pub async fn clear_display(&self, color: &str) -> Result<()> {
        self.proxy
            .clear_display(color)
            .await
            .context("Failed to clear display via D-Bus")
    }

    /// Sets the display face.
    pub async fn set_face(&self, face: &str) -> Result<()> {
        self.proxy
            .set_face(face)
            .await
            .context("Failed to set face via D-Bus")
    }

    /// Gets the current face name.
    pub async fn get_face(&self) -> Result<String> {
        self.proxy
            .get_face()
            .await
            .context("Failed to get face via D-Bus")
    }

    /// Sets LED parameters.
    pub async fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> Result<()> {
        self.proxy
            .set_led(theme, intensity, speed)
            .await
            .context("Failed to set LED via D-Bus")
    }

    /// Turns off LEDs.
    pub async fn led_off(&self) -> Result<()> {
        self.proxy
            .led_off()
            .await
            .context("Failed to turn off LED via D-Bus")
    }

    /// Gets current LED settings.
    pub async fn get_led_settings(&self) -> Result<(u8, u8, u8)> {
        self.proxy
            .get_led_settings()
            .await
            .context("Failed to get LED settings via D-Bus")
    }

    /// Gets the current color theme name.
    pub async fn get_theme(&self) -> Result<String> {
        self.proxy
            .get_theme()
            .await
            .context("Failed to get theme via D-Bus")
    }

    /// Sets the color theme by name.
    pub async fn set_theme(&self, name: &str) -> Result<()> {
        self.proxy
            .set_theme(name)
            .await
            .context("Failed to set theme via D-Bus")
    }

    /// Lists available color themes (IDs only).
    pub async fn list_themes(&self) -> Result<Vec<String>> {
        self.proxy
            .list_themes()
            .await
            .context("Failed to list themes via D-Bus")
    }

    /// Lists available color themes with display names (JSON-encoded).
    pub async fn list_themes_detailed(&self) -> Result<Vec<String>> {
        self.proxy
            .list_themes_detailed()
            .await
            .context("Failed to list themes detailed via D-Bus")
    }

    /// Lists available faces (IDs only).
    pub async fn list_face_ids(&self) -> Result<Vec<String>> {
        self.proxy
            .list_face_ids()
            .await
            .context("Failed to list face IDs via D-Bus")
    }

    /// Lists available faces with display names (JSON-encoded).
    pub async fn list_faces(&self) -> Result<Vec<String>> {
        self.proxy
            .list_faces()
            .await
            .context("Failed to list faces via D-Bus")
    }

    /// Gets the refresh interval in milliseconds.
    /// Lists available network interfaces.
    pub async fn list_network_interfaces(&self) -> Result<Vec<String>> {
        self.proxy
            .list_network_interfaces()
            .await
            .context("Failed to list network interfaces via D-Bus")
    }

    /// Gets the screen as PNG data.
    pub async fn get_screen_png(&self) -> Result<Vec<u8>> {
        self.proxy
            .get_screen_png()
            .await
            .context("Failed to get screen PNG via D-Bus")
    }

    /// Shuts down the daemon.
    pub async fn quit(&self) -> Result<()> {
        self.proxy
            .quit()
            .await
            .context("Failed to quit daemon via D-Bus")
    }

    /// Checks if the LCD is connected.
    pub async fn is_connected(&self) -> Result<bool> {
        self.proxy
            .connected()
            .await
            .context("Failed to get connection status via D-Bus")
    }

    /// Checks if the web UI is enabled.
    pub async fn is_web_enabled(&self) -> Result<bool> {
        self.proxy
            .web_enabled()
            .await
            .context("Failed to get web enabled status via D-Bus")
    }

    /// Lists complications for the current face.
    /// Returns (id, name, description, enabled) tuples.
    pub async fn list_complications(&self) -> Result<Vec<(String, String, String, bool)>> {
        self.proxy
            .list_complications()
            .await
            .context("Failed to list complications via D-Bus")
    }

    /// Lists complications with full details including options.
    /// Returns JSON-encoded complication data.
    pub async fn list_complications_detailed(&self) -> Result<Vec<String>> {
        self.proxy
            .list_complications_detailed()
            .await
            .context("Failed to list complications detailed via D-Bus")
    }

    /// Gets enabled complications for the current face.
    pub async fn get_enabled_complications(&self) -> Result<Vec<String>> {
        self.proxy
            .get_enabled_complications()
            .await
            .context("Failed to get enabled complications via D-Bus")
    }

    /// Enables a complication for the current face.
    pub async fn enable_complication(&self, complication_id: &str) -> Result<()> {
        self.proxy
            .enable_complication(complication_id)
            .await
            .context("Failed to enable complication via D-Bus")
    }

    /// Disables a complication for the current face.
    pub async fn disable_complication(&self, complication_id: &str) -> Result<()> {
        self.proxy
            .disable_complication(complication_id)
            .await
            .context("Failed to disable complication via D-Bus")
    }

    /// Gets a complication option value.
    pub async fn get_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
    ) -> Result<String> {
        self.proxy
            .get_complication_option(complication_id, option_id)
            .await
            .context("Failed to get complication option via D-Bus")
    }

    /// Sets a complication option value.
    pub async fn set_complication_option(
        &self,
        complication_id: &str,
        option_id: &str,
        value: &str,
    ) -> Result<()> {
        self.proxy
            .set_complication_option(complication_id, option_id, value)
            .await
            .context("Failed to set complication option via D-Bus")
    }
}
