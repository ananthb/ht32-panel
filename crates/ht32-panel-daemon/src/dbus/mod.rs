//! D-Bus interface for HT32 Panel Daemon.
//!
//! Provides the `org.ht32panel.Daemon1` interface on the session bus.

mod interface;

pub use interface::{run_dbus_server, DaemonSignals};
