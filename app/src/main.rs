#![cfg_attr(windows, windows_subsystem = "windows")]
//! PAAP Configurator - A graphical application for configuring the PAAP pedal device.
//!
//! This application provides a user-friendly interface to:
//! - Discover and connect to PAAP pedal devices via serial port
//! - View and modify the keyboard key emulation setting
//! - Configure theme preferences (dark, light, or system default)
//! - Manage device connections
//!
//! The application is built using the egui framework for cross-platform GUI support.

mod config;
mod connection;
mod icon;
mod serial;
mod ui;

use eframe::egui;
use log::debug;

/// Main entry point for the PAAP Configurator application.
fn main() -> Result<(), eframe::Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    debug!("Starting application.");

    // Create a simple icon (32x32 keyboard icon).
    let icon_data = icon::create_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 130.0])
            .with_resizable(true)
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        &format!("PAAP Configurator v{}", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|_cc| Ok(Box::<ui::ConfigApp>::default())),
    )
}
