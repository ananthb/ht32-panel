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
        /// Face name (omit to show current)
        face: Option<String>,
    },
    /// List available faces
    ListFaces,
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
enum ComplicationCommands {
    /// List available complications for the current face with their options
    List,
    /// Enable a complication
    Enable {
        /// Complication ID (e.g., network, disk_io, cpu_temp, ip_address)
        id: String,
    },
    /// Disable a complication
    Disable {
        /// Complication ID (e.g., network, disk_io, cpu_temp, ip_address)
        id: String,
    },
    /// Get a complication option value
    Get {
        /// Complication ID (e.g., ip_address, network)
        complication: String,
        /// Option ID (e.g., ip_type, interface)
        option: String,
    },
    /// Set a complication option value
    Set {
        /// Complication ID (e.g., ip_address, network)
        complication: String,
        /// Option ID (e.g., ip_type, interface)
        option: String,
        /// Value to set
        value: String,
    },
    /// List available network interfaces
    ListInterfaces,
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
        LcdCommands::ListFaces => {
            let faces = client.list_face_ids().await?;
            println!("Available faces:");
            for face in faces {
                println!("  {}", face);
            }
        }
        LcdCommands::Info => {
            let connected = client.is_connected().await?;
            let orientation = client.get_orientation().await?;
            let face = client.get_face().await?;
            println!("LCD Status:");
            println!("  Connected: {}", if connected { "yes" } else { "no" });
            println!("  Orientation: {}", orientation);
            println!("  Face: {}", face);
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

async fn handle_complication(action: ComplicationCommands, client: &DaemonClient) -> Result<()> {
    match action {
        ComplicationCommands::List => {
            let face = client.get_face().await?;
            let complications = client.list_complications_detailed().await?;
            println!("Complications for '{}' face:", face);
            if complications.is_empty() {
                println!("  (none available)");
            } else {
                for comp_json in complications {
                    if let Ok(comp) = serde_json::from_str::<serde_json::Value>(&comp_json) {
                        let id = comp["id"].as_str().unwrap_or("");
                        let name = comp["name"].as_str().unwrap_or("");
                        let description = comp["description"].as_str().unwrap_or("");
                        let enabled = comp["enabled"].as_bool().unwrap_or(false);
                        let status = if enabled { "[x]" } else { "[ ]" };
                        println!("  {} {} - {}", status, id, name);
                        println!("      {}", description);

                        // Show options if any
                        if let Some(options) = comp["options"].as_array() {
                            for opt in options {
                                let opt_id = opt["id"].as_str().unwrap_or("");
                                let opt_name = opt["name"].as_str().unwrap_or("");
                                let current = opt["current_value"].as_str().unwrap_or("");
                                let opt_type = opt["type"].as_str().unwrap_or("choice");

                                if opt_type == "range" {
                                    let min = opt["min"].as_f64().unwrap_or(0.0);
                                    let max = opt["max"].as_f64().unwrap_or(100.0);
                                    let step = opt["step"].as_f64().unwrap_or(1.0);
                                    println!(
                                        "      - {}: {} (current: {}, range: {}-{}, step: {})",
                                        opt_id, opt_name, current, min, max, step
                                    );
                                } else {
                                    println!(
                                        "      - {}: {} (current: {})",
                                        opt_id, opt_name, current
                                    );
                                }
                            }
                        }
                    }
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
        ComplicationCommands::Get {
            complication,
            option,
        } => {
            let value = client
                .get_complication_option(&complication, &option)
                .await?;
            println!("{}.{} = {}", complication, option, value);
        }
        ComplicationCommands::Set {
            complication,
            option,
            value,
        } => {
            client
                .set_complication_option(&complication, &option, &value)
                .await?;
            println!("Set {}.{} = {}", complication, option, value);
        }
        ComplicationCommands::ListInterfaces => {
            let interfaces = client.list_network_interfaces().await?;
            println!("Available network interfaces:");
            println!("  auto (auto-detect)");
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
