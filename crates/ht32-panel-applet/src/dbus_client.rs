//! D-Bus client for communicating with the HT32 Panel Daemon.

use anyhow::{Context, Result};
use zbus::{proxy, Connection};

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
    pub async fn connect() -> Result<Self> {
        let connection = Connection::session()
            .await
            .context("Failed to connect to session bus")?;

        let proxy = Daemon1Proxy::new(&connection)
            .await
            .context("Failed to create D-Bus proxy")?;

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

    /// Sets LED parameters.
    pub async fn set_led(&self, theme: u8, intensity: u8, speed: u8) -> Result<()> {
        self.proxy
            .set_led(theme, intensity, speed)
            .await
            .context("Failed to set LED via D-Bus")
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
