//! HT32 Panel Control Tool
//!
//! CLI for controlling the HT32 Panel daemon via D-Bus.

mod dbus_client;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use dbus_client::DaemonClient;

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
enum BusType {
    /// Try session bus first, fall back to system bus
    #[default]
    Auto,
    /// Use session bus (for user services)
    Session,
    /// Use system bus (for system services)
    System,
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
    bus: BusType,

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
    /// Color settings
    Colors {
        #[command(subcommand)]
        action: ColorsCommands,
    },
    /// Background image settings
    Background {
        #[command(subcommand)]
        action: BackgroundCommands,
    },
    /// Network interface settings
    Network {
        #[command(subcommand)]
        action: NetworkCommands,
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
        /// Face name: minimal, detailed (omit to show current)
        face: Option<String>,
    },
    /// Set or show the refresh rate
    Refresh {
        /// Refresh rate in seconds (2-60, omit to show current)
        seconds: Option<u32>,
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
enum ColorsCommands {
    /// Show current colors
    Show,
    /// Set background color
    Background {
        /// Color in hex format (e.g., #000000)
        color: String,
    },
    /// Set foreground/text color
    Foreground {
        /// Color in hex format (e.g., #FFFFFF)
        color: String,
    },
}

#[derive(Subcommand)]
enum BackgroundCommands {
    /// Show current background image
    Show,
    /// Set background image
    Set {
        /// Path to image file
        path: String,
    },
    /// Clear background image
    Clear,
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
    let client = DaemonClient::connect(cli.bus)
        .await
        .context("Failed to connect to daemon. Is ht32paneld running?")?;

    match cli.command {
        Commands::Lcd { action } => handle_lcd(action, &client).await,
        Commands::Led { action } => handle_led(action, &client).await,
        Commands::Colors { action } => handle_colors(action, &client).await,
        Commands::Background { action } => handle_background(action, &client).await,
        Commands::Network { action } => handle_network(action, &client).await,
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
        LcdCommands::Refresh { seconds } => {
            if let Some(secs) = seconds {
                if !(2..=60).contains(&secs) {
                    anyhow::bail!("Refresh rate must be between 2 and 60 seconds");
                }
                client.set_refresh_rate(secs).await?;
                println!("Refresh rate set to: {}s", secs);
            } else {
                let current = client.get_refresh_rate().await?;
                println!("Current refresh rate: {}s", current);
            }
        }
        LcdCommands::Info => {
            let connected = client.is_connected().await?;
            let orientation = client.get_orientation().await?;
            let face = client.get_face().await?;
            let refresh = client.get_refresh_rate().await?;
            println!("LCD Status:");
            println!("  Connected: {}", if connected { "yes" } else { "no" });
            println!("  Orientation: {}", orientation);
            println!("  Face: {}", face);
            println!("  Refresh rate: {}s", refresh);
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

async fn handle_colors(action: ColorsCommands, client: &DaemonClient) -> Result<()> {
    match action {
        ColorsCommands::Show => {
            let bg = client.get_background_color().await?;
            let fg = client.get_foreground_color().await?;
            println!("Colors:");
            println!("  Background: {}", bg);
            println!("  Foreground: {}", fg);
        }
        ColorsCommands::Background { color } => {
            client.set_background_color(&color).await?;
            println!("Background color set to: {}", color);
        }
        ColorsCommands::Foreground { color } => {
            client.set_foreground_color(&color).await?;
            println!("Foreground color set to: {}", color);
        }
    }

    Ok(())
}

async fn handle_background(action: BackgroundCommands, client: &DaemonClient) -> Result<()> {
    match action {
        BackgroundCommands::Show => {
            let path = client.get_background_image().await?;
            if path.is_empty() {
                println!("Background image: none");
            } else {
                println!("Background image: {}", path);
            }
        }
        BackgroundCommands::Set { path } => {
            client.set_background_image(&path).await?;
            println!("Background image set to: {}", path);
        }
        BackgroundCommands::Clear => {
            client.clear_background_image().await?;
            println!("Background image cleared");
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

async fn handle_screenshot(output: &str, client: &DaemonClient) -> Result<()> {
    let png_data = client.get_screen_png().await?;
    std::fs::write(output, &png_data).context("Failed to write screenshot file")?;
    println!("Screenshot saved to: {}", output);
    Ok(())
}
