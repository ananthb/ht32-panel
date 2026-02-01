//! HT32 Panel System Tray Applet
//!
//! Provides quick access to LCD and LED controls via the system tray.
//! Works with both GNOME (via AppIndicator extension) and KDE (native SNI).

mod tray;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use ht32_panel_client::DaemonClient;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use tray::{create_tray, TrayCommand, TrayState};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("ht32_panel_applet=info".parse()?),
        )
        .init();

    info!("HT32 Panel Applet starting...");

    // Shared state
    let state = Arc::new(Mutex::new(TrayState::default()));

    // Create tray service and command channel
    let (service, mut command_rx) =
        create_tray(state.clone()).context("Failed to create tray service")?;

    // Spawn the command handler task
    let cmd_state = state.clone();
    tokio::spawn(async move {
        let mut client: Option<DaemonClient> = None;

        loop {
            // Try to connect if not connected
            if client.is_none() {
                match DaemonClient::connect().await {
                    Ok(c) => {
                        info!("Connected to daemon via D-Bus");

                        // Update state from daemon
                        if let Ok(conn) = c.is_connected().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.connected = conn;
                        }
                        if let Ok(web) = c.is_web_enabled().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.web_enabled = web;
                        }
                        if let Ok(orient) = c.get_orientation().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.orientation = orient;
                        }
                        if let Ok((theme, intensity, speed)) = c.get_led_settings().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.led_theme = theme;
                            s.led_intensity = intensity;
                            s.led_speed = speed;
                        }
                        if let Ok(face) = c.get_face().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.face = face;
                        }
                        if let Ok(iface) = c.get_complication_option("network", "interface").await {
                            let mut s = cmd_state.lock().unwrap();
                            s.network_interface = iface;
                        }
                        if let Ok(interfaces) = c.list_network_interfaces().await {
                            let mut s = cmd_state.lock().unwrap();
                            s.network_interfaces = interfaces;
                        }

                        client = Some(c);
                    }
                    Err(e) => {
                        debug!("Failed to connect to daemon: {}. Retrying...", e);
                    }
                }
            }

            // Process commands with timeout for reconnection attempts
            tokio::select! {
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(TrayCommand::SetLedTheme(theme)) => {
                            if let Some(ref c) = client {
                                let (_, intensity, speed) = {
                                    let s = cmd_state.lock().unwrap();
                                    (s.led_theme, s.led_intensity, s.led_speed)
                                };
                                match c.set_led(theme, intensity, speed).await {
                                    Ok(()) => {
                                        let mut s = cmd_state.lock().unwrap();
                                        s.led_theme = theme;
                                        debug!("LED theme set to {}", theme);
                                    }
                                    Err(e) => {
                                        error!("Failed to set LED theme: {}", e);
                                        client = None; // Mark for reconnection
                                    }
                                }
                            }
                        }
                        Some(TrayCommand::SetOrientation(orientation)) => {
                            if let Some(ref c) = client {
                                match c.set_orientation(&orientation).await {
                                    Ok(()) => {
                                        let mut s = cmd_state.lock().unwrap();
                                        s.orientation = orientation.clone();
                                        debug!("Orientation set to {}", orientation);
                                    }
                                    Err(e) => {
                                        error!("Failed to set orientation: {}", e);
                                        client = None; // Mark for reconnection
                                    }
                                }
                            }
                        }
                        Some(TrayCommand::SetFace(face)) => {
                            if let Some(ref c) = client {
                                match c.set_face(&face).await {
                                    Ok(()) => {
                                        let mut s = cmd_state.lock().unwrap();
                                        s.face = face.clone();
                                        debug!("Face set to {}", face);
                                    }
                                    Err(e) => {
                                        error!("Failed to set face: {}", e);
                                        client = None; // Mark for reconnection
                                    }
                                }
                            }
                        }
                        Some(TrayCommand::SetNetworkInterface(interface)) => {
                            if let Some(ref c) = client {
                                match c.set_complication_option("network", "interface", &interface).await {
                                    Ok(()) => {
                                        let mut s = cmd_state.lock().unwrap();
                                        s.network_interface = if interface == "auto" {
                                            String::new()
                                        } else {
                                            interface.clone()
                                        };
                                        debug!("Network interface set to {}", interface);
                                    }
                                    Err(e) => {
                                        error!("Failed to set network interface: {}", e);
                                        client = None; // Mark for reconnection
                                    }
                                }
                            }
                        }
                        Some(TrayCommand::QuitDaemon) => {
                            if let Some(ref c) = client {
                                match c.quit().await {
                                    Ok(()) => info!("Daemon quit request sent"),
                                    Err(e) => error!("Failed to quit daemon: {}", e),
                                }
                            }
                        }
                        None => {
                            // Channel closed, exit
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    // Reconnection timer expired, loop will try to reconnect
                }
            }
        }
    });

    // Run tray in a separate thread since ksni uses glib main loop
    let tray_handle = std::thread::spawn(move || -> Result<()> {
        service.run().context("Tray service failed")?;
        Ok(())
    });

    // Wait for tray thread to finish (it shouldn't unless there's an error)
    match tray_handle.join() {
        Ok(Ok(())) => info!("Tray service stopped"),
        Ok(Err(e)) => error!("Tray service error: {}", e),
        Err(e) => error!("Tray thread panicked: {:?}", e),
    }

    Ok(())
}
