//! D-Bus client for communicating with the HT32 Panel Daemon.

use anyhow::{Context, Result};
use tracing::warn;
use zbus::{proxy, Connection};

use crate::BusType;

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

    /// Shuts down the daemon.
    fn quit(&self) -> zbus::Result<()>;

    /// Whether the LCD device is connected.
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

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
}

/// D-Bus client wrapper for the daemon.
pub struct DaemonClient {
    proxy: Daemon1Proxy<'static>,
}

impl DaemonClient {
    /// Attempts to connect to the daemon via D-Bus.
    pub async fn connect(bus_type: BusType) -> Result<Self> {
        match bus_type {
            BusType::Session => Self::connect_to_bus(Connection::session().await?).await,
            BusType::System => Self::connect_to_bus(Connection::system().await?).await,
            BusType::Auto => {
                // Try session bus first, fall back to system bus
                if let Ok(session_conn) = Connection::session().await {
                    if let Ok(client) = Self::connect_to_bus(session_conn).await {
                        return Ok(client);
                    }
                    warn!("Daemon not found on session bus, trying system bus");
                }
                let system_conn = Connection::system()
                    .await
                    .context("Failed to connect to any D-Bus")?;
                Self::connect_to_bus(system_conn)
                    .await
                    .context("Daemon not found on session or system bus")
            }
        }
    }

    /// Connect to the daemon on a specific bus connection.
    async fn connect_to_bus(connection: Connection) -> Result<Self> {
        let proxy = Daemon1Proxy::new(&connection)
            .await
            .context("Failed to create D-Bus proxy")?;

        // Verify daemon is running by checking a property
        proxy.connected().await.context("Daemon not responding")?;

        Ok(Self { proxy })
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
}
