# ht32-panel-client

D-Bus client library for communicating with the HT32 Panel daemon.

## Usage

```rust
use ht32_panel_client::DaemonClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = DaemonClient::connect().await?;

    // Display
    client.set_orientation("landscape").await?;
    let orient = client.get_orientation().await?;
    client.set_face("clock").await?;
    client.set_theme("catppuccin").await?;
    client.clear_display("#000000").await?;

    // LEDs (theme, intensity, speed: each 1-5)
    client.set_led(2, 3, 3).await?;
    let (theme, intensity, speed) = client.get_led_settings().await?;
    client.led_off().await?;

    // Queries
    let faces = client.list_faces().await?;
    let themes = client.list_themes().await?;
    let png_data = client.get_screen_png().await?;
    let connected = client.is_connected().await?;

    // Complications
    let complications = client.list_complications().await?;
    client.enable_complication("weather").await?;
    client.set_complication_option("weather", "unit", "celsius").await?;

    // Shutdown
    client.quit().await?;

    Ok(())
}
```

Use `DaemonClient::connect_with_bus(BusType::Session)` or `BusType::System` to target a specific bus. The default `connect()` tries session first, then falls back to system.

## D-Bus Interface

Connects to `org.ht32panel.Daemon1` on either the system or session bus.

## License

AGPL-3.0-or-later
