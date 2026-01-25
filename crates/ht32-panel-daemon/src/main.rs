//! HT32 Panel Daemon
//!
//! Background service with HTMX web UI and D-Bus interface for LCD and LED control.

mod config;
mod dbus;
mod rendering;
mod sensors;
mod state;
mod web;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use config::Config;
use dbus::DaemonSignals;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config/default.toml".to_string());

    let config = Config::load(&config_path).context("Failed to load configuration")?;
    info!("Loaded configuration from: {}", config_path);

    // Initialize application state
    let state = Arc::new(AppState::new(config.clone())?);

    // Create channels for D-Bus signals and shutdown
    let (signal_tx, _signal_rx) = broadcast::channel::<DaemonSignals>(16);
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Start D-Bus service
    let dbus_state = state.clone();
    let dbus_signal_tx = signal_tx.clone();
    let _dbus_connection =
        match dbus::run_dbus_server(dbus_state, dbus_signal_tx, shutdown_tx).await {
            Ok(conn) => {
                info!("D-Bus service started");
                Some(conn)
            }
            Err(e) => {
                warn!(
                    "Failed to start D-Bus service: {}. Continuing without D-Bus.",
                    e
                );
                None
            }
        };

    // Start render loop
    let render_state = state.clone();
    tokio::spawn(async move {
        render_loop(render_state).await;
    });

    // Start heartbeat loop
    let heartbeat_state = state.clone();
    let heartbeat_interval = config.heartbeat;
    tokio::spawn(async move {
        heartbeat_loop(heartbeat_state, heartbeat_interval).await;
    });

    // Build router (HTMX web UI)
    let app = web::create_router(state.clone());

    // Start server
    let addr: SocketAddr = config.listen.parse().context("Invalid listen address")?;
    let listener = TcpListener::bind(addr).await?;
    info!("Server listening on http://{}", addr);

    // Run server with shutdown handling
    tokio::select! {
        result = axum::serve(listener, app) => {
            result?;
        }
        _ = shutdown_rx.recv() => {
            info!("Shutdown requested via D-Bus");
        }
    }

    Ok(())
}

async fn render_loop(state: Arc<AppState>) {
    let poll_interval = std::time::Duration::from_millis(state.config().poll);

    loop {
        if let Err(e) = state.render_frame().await {
            warn!("Render error: {}", e);
        }
        tokio::time::sleep(poll_interval).await;
    }
}

async fn heartbeat_loop(state: Arc<AppState>, interval_ms: u64) {
    let interval = std::time::Duration::from_millis(interval_ms);

    loop {
        tokio::time::sleep(interval).await;
        if let Err(e) = state.send_heartbeat() {
            warn!("Heartbeat error: {}", e);
        }
    }
}
