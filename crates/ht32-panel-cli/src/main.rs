//! HT32 Panel Control Tool
//!
//! CLI for controlling the HT32 Panel daemon via D-Bus.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ht32_panel_client::{BusType, DaemonClient};
use tracing_subscriber::EnvFilter;

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
enum CliBusType {
    /// Try session bus first, fall back to system bus
    #[default]
    Auto,
    /// Use session bus (for user services)
    Session,
    /// Use system bus (for system services)
    System,
}

impl From<CliBusType> for BusType {
    fn from(bus: CliBusType) -> Self {
        match bus {
            CliBusType::Auto => BusType::Auto,
            CliBusType::Session => BusType::Session,
            CliBusType::System => BusType::System,
        }
    }
}

#[derive(Parser)]
#[command(name = "ht32panelctl")]
#[command(about = "Control tool for HT32 Panel daemon")]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// D-Bus bus type to use
    #[arg(long, default_value = "auto", value_enum)]
    bus: CliBusType,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// LCD display commands
    Lcd {
        #[command(subcommand)]
        action: LcdCommands,
    },
    /// LED strip commands
    Led {
        #[command(subcommand)]
        action: LedCommands,
    },
    /// Color theme settings
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
    /// Network interface settings
    Network {
        #[command(subcommand)]
        action: NetworkCommands,
    },
    /// Face complications settings
    Complication {
        #[command(subcommand)]
        action: ComplicationCommands,
    },
    /// Save a screenshot of the display
    Screenshot {
        /// Output file path (default: screenshot.png)
        #[arg(default_value = "screenshot.png")]
        output: String,
    },
    /// Daemon control commands
    Daemon {
        #[command(subcommand)]
        action: DaemonCommands,
    },
}

#[derive(Subcommand)]
enum LcdCommands {
    /// Set display orientation
    Orientation {
        /// Orientation: landscape, portrait, landscape-upside-down, portrait-upside-down
        orientation: String,
    },
    /// Clear the display to a solid color
    Clear {
        /// Color in hex format (e.g., #FF0000 for red)
        #[arg(long, default_value = "#000000")]
        color: String,
    },
    /// Set or show the current face
    Face {
        /// Face name: ascii, professional (omit to show current)
        face: Option<String>,
    },
    /// Set or show the refresh interval
    Refresh {
        /// Refresh interval in milliseconds (1500-10000, omit to show current)
        milliseconds: Option<u32>,
    },
    /// Show device information
    Info,
}

#[derive(Subcommand)]
enum LedCommands {
    /// Set LED theme
    Set {
        /// Theme: rainbow, breathing, colors, off, auto
        theme: String,

        /// Intensity (1-5)
        #[arg(long, default_value = "3")]
        intensity: u8,

        /// Speed (1-5)
        #[arg(long, default_value = "3")]
        speed: u8,
    },
    /// Turn off LEDs
    Off,
    /// Show current LED settings
    Status,
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Check if daemon is running
    Status,
    /// Request daemon shutdown
    Quit,
}

#[derive(Subcommand)]
enum ThemeCommands {
    /// Show current theme
    Show,
    /// Set theme by name
    Set {
        /// Theme name (default, hacker, solarized-light, solarized-dark, nord, tokyonight)
        name: String,
    },
    /// List available themes
    List,
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// Show current network interface
    Show,
    /// Set network interface to monitor
    Set {
        /// Interface name (e.g., eth0, wlan0) or "auto" for auto-detection
        interface: String,
    },
    /// List available network interfaces
    List,
}

#[derive(Subcommand)]
enum ComplicationCommands {
    /// List available complications for the current face
    List,
    /// Enable a complication
    Enable {
        /// Complication ID (e.g., network, disk_io, cpu_temp)
        id: String,
    },
    /// Disable a complication
    Disable {
        /// Complication ID (e.g., network, disk_io, cpu_temp)
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("warn")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Connect to daemon
    let client = DaemonClient::connect_with_bus(cli.bus.into())
        .await
        .context("Failed to connect to daemon. Is ht32paneld running?")?;

    match cli.command {
        Commands::Lcd { action } => handle_lcd(action, &client).await,
        Commands::Led { action } => handle_led(action, &client).await,
        Commands::Theme { action } => handle_theme(action, &client).await,
        Commands::Network { action } => handle_network(action, &client).await,
        Commands::Complication { action } => handle_complication(action, &client).await,
        Commands::Screenshot { output } => handle_screenshot(&output, &client).await,
        Commands::Daemon { action } => handle_daemon(action, &client).await,
    }
}

async fn handle_lcd(action: LcdCommands, client: &DaemonClient) -> Result<()> {
    match action {
        LcdCommands::Orientation { orientation } => {
            client.set_orientation(&orientation).await?;
            println!("Orientation set to: {}", orientation);
        }
        LcdCommands::Clear { color } => {
            client.clear_display(&color).await?;
            println!("Display cleared to: {}", color);
        }
        LcdCommands::Face { face } => {
            if let Some(face_name) = face {
                client.set_face(&face_name).await?;
                println!("Face set to: {}", face_name);
            } else {
                let current = client.get_face().await?;
                println!("Current face: {}", current);
            }
        }
        LcdCommands::Refresh { milliseconds } => {
            if let Some(ms) = milliseconds {
                if !(1500..=10000).contains(&ms) {
                    anyhow::bail!("Refresh interval must be between 1500 and 10000 milliseconds");
                }
                client.set_refresh_interval(ms).await?;
                println!("Refresh interval set to: {}ms", ms);
            } else {
                let current = client.get_refresh_interval().await?;
                println!("Current refresh interval: {}ms", current);
            }
        }
        LcdCommands::Info => {
            let connected = client.is_connected().await?;
            let orientation = client.get_orientation().await?;
            let face = client.get_face().await?;
            let refresh = client.get_refresh_interval().await?;
            println!("LCD Status:");
            println!("  Connected: {}", if connected { "yes" } else { "no" });
            println!("  Orientation: {}", orientation);
            println!("  Face: {}", face);
            println!("  Refresh interval: {}ms", refresh);
        }
    }

    Ok(())
}

async fn handle_led(action: LedCommands, client: &DaemonClient) -> Result<()> {
    match action {
        LedCommands::Set {
            theme,
            intensity,
            speed,
        } => {
            // Convert theme string to byte
            let theme_byte = match theme.to_lowercase().as_str() {
                "rainbow" => 1,
                "breathing" => 2,
                "colors" => 3,
                "off" => 4,
                "auto" => 5,
                _ => anyhow::bail!(
                    "Invalid theme: {}. Use: rainbow, breathing, colors, off, auto",
                    theme
                ),
            };

            if !(1..=5).contains(&intensity) {
                anyhow::bail!("Intensity must be between 1 and 5");
            }
            if !(1..=5).contains(&speed) {
                anyhow::bail!("Speed must be between 1 and 5");
            }

            client.set_led(theme_byte, intensity, speed).await?;
            println!(
                "LED set to: {} (intensity: {}, speed: {})",
                theme, intensity, speed
            );
        }
        LedCommands::Off => {
            client.led_off().await?;
            println!("LEDs turned off");
        }
        LedCommands::Status => {
            let (theme, intensity, speed) = client.get_led_settings().await?;
            let theme_name = match theme {
                1 => "rainbow",
                2 => "breathing",
                3 => "colors",
                4 => "off",
                5 => "auto",
                _ => "unknown",
            };
            println!("LED Status:");
            println!("  Theme: {}", theme_name);
            println!("  Intensity: {}", intensity);
            println!("  Speed: {}", speed);
        }
    }

    Ok(())
}

async fn handle_daemon(action: DaemonCommands, client: &DaemonClient) -> Result<()> {
    match action {
        DaemonCommands::Status => {
            let connected = client.is_connected().await?;
            println!("Daemon: running");
            println!("LCD connected: {}", if connected { "yes" } else { "no" });
        }
        DaemonCommands::Quit => {
            client.quit().await?;
            println!("Shutdown request sent to daemon");
        }
    }

    Ok(())
}

async fn handle_theme(action: ThemeCommands, client: &DaemonClient) -> Result<()> {
    match action {
        ThemeCommands::Show => {
            let theme = client.get_theme().await?;
            println!("Current theme: {}", theme);
        }
        ThemeCommands::Set { name } => {
            client.set_theme(&name).await?;
            println!("Theme set to: {}", name);
        }
        ThemeCommands::List => {
            let themes = client.list_themes().await?;
            println!("Available themes:");
            for theme in themes {
                println!("  {}", theme);
            }
        }
    }

    Ok(())
}

async fn handle_network(action: NetworkCommands, client: &DaemonClient) -> Result<()> {
    match action {
        NetworkCommands::Show => {
            let iface = client.get_network_interface().await?;
            println!("Network interface: {}", iface);
        }
        NetworkCommands::Set { interface } => {
            client.set_network_interface(&interface).await?;
            if interface.eq_ignore_ascii_case("auto") {
                println!("Network interface set to auto-detect");
            } else {
                println!("Network interface set to: {}", interface);
            }
        }
        NetworkCommands::List => {
            let interfaces = client.list_network_interfaces().await?;
            println!("Available network interfaces:");
            for iface in interfaces {
                println!("  {}", iface);
            }
        }
    }

    Ok(())
}

async fn handle_complication(action: ComplicationCommands, client: &DaemonClient) -> Result<()> {
    match action {
        ComplicationCommands::List => {
            let face = client.get_face().await?;
            let complications = client.list_complications().await?;
            println!("Complications for '{}' face:", face);
            if complications.is_empty() {
                println!("  (none available)");
            } else {
                for (id, name, description, enabled) in complications {
                    let status = if enabled { "[x]" } else { "[ ]" };
                    println!("  {} {} - {}", status, id, name);
                    println!("      {}", description);
                }
            }
        }
        ComplicationCommands::Enable { id } => {
            client.enable_complication(&id).await?;
            println!("Enabled complication: {}", id);
        }
        ComplicationCommands::Disable { id } => {
            client.disable_complication(&id).await?;
            println!("Disabled complication: {}", id);
        }
    }

    Ok(())
}

async fn handle_screenshot(output: &str, client: &DaemonClient) -> Result<()> {
    let png_data = client.get_screen_png().await?;
    std::fs::write(output, &png_data).context("Failed to write screenshot file")?;
    println!("Screenshot saved to: {}", output);
    Ok(())
}
