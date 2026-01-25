//! HT32 Panel Control Tool
//!
//! CLI for controlling the HT32 Panel daemon via D-Bus.

mod dbus_client;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use dbus_client::DaemonClient;

#[derive(Parser)]
#[command(name = "ht32panelctl")]
#[command(about = "Control tool for HT32 Panel daemon")]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

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
    /// Send a heartbeat to keep the device alive
    Heartbeat,
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
    let client = DaemonClient::connect()
        .await
        .context("Failed to connect to daemon. Is ht32paneld running?")?;

    match cli.command {
        Commands::Lcd { action } => handle_lcd(action, &client).await,
        Commands::Led { action } => handle_led(action, &client).await,
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
        LcdCommands::Heartbeat => {
            client.heartbeat().await?;
            println!("Heartbeat sent");
        }
        LcdCommands::Info => {
            let connected = client.is_connected().await?;
            let orientation = client.get_orientation().await?;
            println!("LCD Status:");
            println!("  Connected: {}", if connected { "yes" } else { "no" });
            println!("  Orientation: {}", orientation);
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
