#![cfg_attr(windows, windows_subsystem = "windows")]
//! PAAP Configurator - A graphical application for configuring the PAAP pedal device.
//!
//! This application provides a user-friendly interface to:
//! - Discover PAAP pedals and tell them apart by their pedal number
//! - View and modify the keyboard key the pedal emulates
//! - Set the pedal number (1-16)
//! - Configure theme preferences (dark, light, or system default)
//!
//! The user interface is built with Slint (ui/app.slint).

mod config;
mod serial;
mod ui;

fn main() -> Result<(), slint::PlatformError> {
    if std::env::var("RUST_LOG").is_err() {
        // SAFETY: called before any other thread is started.
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    env_logger::init();

    ui::run()
}
