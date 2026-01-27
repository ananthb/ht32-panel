//! HT32 Panel Daemon
//!
//! Background service with HTMX web UI and D-Bus interface for LCD and LED control.

mod config;
mod dbus;
mod faces;
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

    // Keep a clone of shutdown_tx to prevent the channel from closing if D-Bus fails
    let _shutdown_tx_keepalive = shutdown_tx.clone();

    // Start D-Bus service
    let dbus_state = state.clone();
    let dbus_signal_tx = signal_tx.clone();
    let dbus_bus_type = config.dbus.bus;
    let _dbus_connection =
        match dbus::run_dbus_server(dbus_state, dbus_signal_tx, shutdown_tx, dbus_bus_type).await {
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

    // Setup Unix signal handlers
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    // Optionally start web server
    if config.web.enable {
        let app = web::create_router(state.clone());
        let addr: SocketAddr = config
            .web
            .listen
            .parse()
            .context("Invalid listen address")?;
        let listener = TcpListener::bind(addr).await?;
        info!("Web server listening on http://{}", addr);

        // Run server with shutdown handling
        tokio::select! {
            result = axum::serve(listener, app) => {
                result?;
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested via D-Bus");
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down");
            }
        }
    } else {
        info!("Web server disabled");
        // Wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown requested via D-Bus");
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, shutting down");
            }
        }
    }

    Ok(())
}

async fn render_loop(state: Arc<AppState>) {
    let mut consecutive_errors: u32 = 0;
    let mut last_error_log = std::time::Instant::now();

    loop {
        if let Err(e) = state.render_frame().await {
            consecutive_errors += 1;
            // Only log errors once per minute or on first error
            let elapsed = last_error_log.elapsed();
            if consecutive_errors == 1 || elapsed >= std::time::Duration::from_secs(60) {
                if consecutive_errors > 1 {
                    warn!(
                        "Render error (repeated {} times in {:?}): {}",
                        consecutive_errors, elapsed, e
                    );
                } else {
                    warn!("Render error: {}", e);
                }
                last_error_log = std::time::Instant::now();
                consecutive_errors = 0;
            }
        } else {
            consecutive_errors = 0;
        }
        let ms = state.refresh_interval_ms();
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
}

async fn heartbeat_loop(state: Arc<AppState>, interval_ms: u64) {
    let interval = std::time::Duration::from_millis(interval_ms);
    let mut consecutive_errors: u32 = 0;
    let mut last_error_log = std::time::Instant::now();

    loop {
        tokio::time::sleep(interval).await;
        if let Err(e) = state.send_heartbeat() {
            consecutive_errors += 1;
            let elapsed = last_error_log.elapsed();
            if consecutive_errors == 1 || elapsed >= std::time::Duration::from_secs(60) {
                if consecutive_errors > 1 {
                    warn!(
                        "Heartbeat error (repeated {} times in {:?}): {}",
                        consecutive_errors, elapsed, e
                    );
                } else {
                    warn!("Heartbeat error: {}", e);
                }
                last_error_log = std::time::Instant::now();
                consecutive_errors = 0;
            }
        } else {
            consecutive_errors = 0;
        }
    }
}
