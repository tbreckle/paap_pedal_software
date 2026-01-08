//! User interface module for the PAAP Configurator application.
//!
//! This module implements the GUI using the egui immediate-mode GUI framework.
//! It provides the main application window, controls for device connection, key configuration,
//! theme selection, and error handling.

use crate::serial;
use crate::config::{AppConfig, ThemeMode};
use crate::connection::ConnectionState;
use eframe::egui;
use log::error;

/// Main application struct for the PAAP Configurator GUI.
///
/// This struct holds all the application state including the connection state, available ports,
/// and user preferences. It implements the `eframe::App` trait to integrate with the egui framework.
pub struct ConfigApp {
    /// List of available serial ports detected on the system.
    pub available_ports: Vec<serialport::SerialPortInfo>,
    /// The current connection state (connected or disconnected).
    pub connection_state: ConnectionState,
    /// The key character currently being edited or to be saved.
    pub key_to_emulate: String,
    /// Optional error message to display to the user.
    pub error_message: Option<String>,
    /// The user's current theme preference.
    pub theme_mode: ThemeMode,
}

impl Default for ConfigApp {
    fn default() -> Self {
        // Load saved theme preference.
        let config: AppConfig = AppConfig::load();

        let mut app = Self {
            available_ports: Vec::new(),
            connection_state: ConnectionState::Disconnected,
            key_to_emulate: String::new(),
            error_message: None,
            theme_mode: config.theme_mode,
        };
        let _ = app.refresh_ports();
        app
    }
}

impl ConfigApp {
    /// Refreshes the list of available serial ports by querying the system.
    ///
    /// Updates the `available_ports` field and stores any errors in `error_message`.
    fn refresh_ports(&mut self) -> Result<(), String> {
        match serial::refresh_ports() {
            Ok(ports) => {
                self.available_ports = ports;
                Ok(())
            }
            Err(e) => {
                self.error_message = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Attempts to connect to a specific serial port.
    ///
    /// Updates the connection state and key value if successful, or stores an error message if it fails.
    fn connect_to_port(&mut self, port_name: &str) {
        match serial::connect_to_port(port_name, &mut self.connection_state) {
            Ok(key) => {
                self.key_to_emulate = key;
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(e);
                self.connection_state = ConnectionState::Disconnected;
            }
        }
    }

    /// Disconnects from the currently connected device.
    ///
    /// Clears the connection state and resets the key input field.
    fn disconnect(&mut self) {
        self.connection_state = ConnectionState::Disconnected;
        self.key_to_emulate.clear();
    }

    /// Saves the currently edited key to the connected device.
    ///
    /// Sends the key to the device if it's a valid single character and device is connected.
    fn save_key_to_device(&mut self) {
        if self.key_to_emulate.len() == 1 {
            if let Some(new_key) = self.key_to_emulate.chars().next() {
                match serial::save_key_to_device(&mut self.connection_state, new_key) {
                    Ok(_) => {
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
            }
        }
    }

    /// Renders the port selector dropdown in the UI.
    ///
    /// Shows available ports when disconnected and allows disconnection when connected.
    fn render_port_selector(&mut self, ui: &mut egui::Ui) {
        let is_connected = self.connection_state.is_connected();

        let selected_text = if is_connected {
            "Disconnect"
        } else {
            "Select serial port"
        };

        egui::ComboBox::from_label("")
            .selected_text(selected_text)
            .width(300.0)
            .show_ui(ui, |ui| {
                if is_connected {
                    if ui.selectable_label(false, "Disconnect").clicked() {
                        self.disconnect();
                    }
                } else {
                    let _ = ui.selectable_label(false, "Select serial port");

                    for port_info in &self.available_ports.clone() {
                        let display_name = serial::get_port_display_name(port_info);
                        if ui.selectable_label(false, &display_name).clicked() {
                            self.connect_to_port(&port_info.port_name);
                        }
                    }
                }
            });

        if ui.button("Refresh Ports").clicked() && !is_connected {
            let _ = self.refresh_ports();
        }
    }

    /// Renders the key input field and save button in the UI.
    ///
    /// Allows the user to input a single ASCII character and save it to the device.
    fn render_key_input(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let is_connected = self.connection_state.is_connected();

        // Prefill with device key if connected and input is empty.
        if is_connected && self.key_to_emulate.is_empty() {
            if let Some(ch) = self.connection_state.current_key() {
                self.key_to_emulate = ch.to_string();
            }
        }

        ui.horizontal(|ui| {
            ui.add_enabled(false, egui::Label::new("Key to emulate:"));

            let text_edit = egui::TextEdit::singleline(&mut self.key_to_emulate)
                .char_limit(1)
                .desired_width(50.0);

            let response = ui.add_enabled(is_connected, text_edit);

            // Handle keyboard input when text edit has focus.
            if response.has_focus() {
                ctx.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Text(text) = event {
                            if let Some(ch) = text.chars().next() {
                                if ch.is_ascii() && !ch.is_control() {
                                    self.key_to_emulate = ch.to_string();
                                }
                            }
                        }
                    }
                });
            }

            if ui
                .add_enabled(
                    is_connected && self.key_to_emulate.len() == 1,
                    egui::Button::new("Save to device"),
                )
                .clicked()
            {
                self.save_key_to_device();
            }
        });
    }

    /// Renders the status bar at the bottom of the window.
    ///
    /// Displays connection status, available port count, and the theme selector.
    fn render_status_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Connection status on the left.
            if let Some(port_name) = self.connection_state.port_name() {
                let version = self
                    .connection_state
                    .version()
                    .unwrap_or("unknown");
                let is_light_theme = ctx.style().visuals.dark_mode == false;
                let label_color = if is_light_theme {
                    egui::Color32::from_rgb(0, 128, 0) // dark green for light theme
                } else {
                    egui::Color32::GREEN // default green for dark theme
                };
                ui.colored_label(
                    label_color,
                    format!("Connected to: {} | Version: {}", port_name, version),
                );
            } else {
                ui.label("Not connected.");
            }

            ui.label(format!("| Available ports: {}", self.available_ports.len()));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_label("Theme")
                    .selected_text(match self.theme_mode {
                        ThemeMode::Dark => "Dark",
                        ThemeMode::Light => "Light",
                        ThemeMode::System => "System",
                    })
                    .show_ui(ui, |ui| {
                        let old_theme = self.theme_mode;
                        ui.selectable_value(&mut self.theme_mode, ThemeMode::Dark, "Dark");
                        ui.selectable_value(&mut self.theme_mode, ThemeMode::Light, "Light");
                        ui.selectable_value(&mut self.theme_mode, ThemeMode::System, "System");

                        // Save to config if theme changed.
                        if old_theme != self.theme_mode {
                            let config = AppConfig {
                                theme_mode: self.theme_mode,
                            };
                            if let Err(e) = config.save() {
                                error!("Failed to save theme preference: {}.", e);
                            }
                        }
                    });
            });
        });
    }

    /// Applies the selected theme to the UI context.
    ///
    /// Sets the visual style based on the current `theme_mode` preference.
    fn apply_theme(&self, ctx: &egui::Context) {
        match self.theme_mode {
            ThemeMode::Dark => {
                ctx.set_visuals(egui::Visuals::dark());
            }
            ThemeMode::Light => {
                ctx.set_visuals(egui::Visuals::light());
            }
            ThemeMode::System => {
                // Use default visuals, egui default is dark.
            }
        }
    }

    /// Renders an error modal dialog if an error message is present.
    ///
    /// Displays the error message and closes when the user clicks OK.
    fn render_error_modal(&mut self, ctx: &egui::Context) {
        if let Some(error) = &self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error);
                    ui.separator();
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Top right dropdown.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.render_port_selector(ui);
                });
            });

            ui.separator();

            // Key input section.
            self.render_key_input(ctx, ui);

            // Add spacing to push content down.
            ui.allocate_space(egui::Vec2::new(1.0, ui.available_height() - 50.0));
        });

        // Bottom panel for status and theme selector.
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            self.render_status_bar(ctx, ui);
        });

        // Apply theme based on selection.
        self.apply_theme(ctx);

        // Error message modal.
        self.render_error_modal(ctx);
    }
}
