use eframe::egui;
use log::{info, error, debug};
use serialport::{self, SerialPort};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::time::Duration;

enum ConnectionState {
    Disconnected,
    Connected {
        port: Box<dyn SerialPort>,
        port_name: String,
        current_key: char,
        version: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    theme_mode: ThemeMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
        }
    }
}

struct SerialPortApp {
    available_ports: Vec<serialport::SerialPortInfo>,
    connection_state: ConnectionState,
    key_to_emulate: String,
    error_message: Option<String>,
    theme_mode: ThemeMode,
}

impl Default for SerialPortApp {
    fn default() -> Self {
        // Load saved theme preference.
        let config: AppConfig = confy::load("paap-configurator", None)
            .unwrap_or_default();
        
        let mut app = Self {
            available_ports: Vec::new(),
            connection_state: ConnectionState::Disconnected,
            key_to_emulate: String::new(),
            error_message: None,
            theme_mode: config.theme_mode,
        };
        app.refresh_ports();
        app
    }
}

impl SerialPortApp {
    fn refresh_ports(&mut self) {
        match serialport::available_ports() {
            Ok(ports) => {
                self.available_ports = ports;
                info!("Found {} serial port(s)", self.available_ports.len());
            }
            Err(e) => {
                self.error_message = Some(format!("Error scanning ports: {}", e));
                error!("Error finding serial ports: {}", e);
            }
        }
    }

    fn get_port_display_name(port: &serialport::SerialPortInfo) -> String {
        let name = if let serialport::SerialPortType::UsbPort(usb_info) = &port.port_type {
            if let Some(product) = &usb_info.product {
                product.clone()
            } else if let Some(manufacturer) = &usb_info.manufacturer {
                manufacturer.clone()
            } else {
                "USB Device".to_string()
            }
        } else {
            "Serial Port".to_string()
        };
        
        format!("{} ({})", name, port.port_name)
    }

    fn wait_for_response(&mut self, prefix: &str, timeout_secs: u64) -> Option<String> {
        if let ConnectionState::Connected { ref mut port, .. } = self.connection_state {
            let mut buffer = [0u8; 64];
            let mut received = Vec::new();
            let start = std::time::Instant::now();
            
            loop {
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    self.error_message = Some(format!("Timeout waiting for {} response.", prefix));
                    error!("Timeout waiting for {} response.", prefix);
                    return None;
                }
                
                match port.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        received.extend_from_slice(&buffer[..n]);
                        
                        // Check if we have a complete response.
                        if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                            let response = &received[..newline_pos];
                            let response_str = String::from_utf8_lossy(response).trim_end_matches('\r').to_string();
                            
                            if response_str.starts_with(prefix) {
                                return Some(response_str);
                            } else {
                                self.error_message = Some(format!("Invalid response: expected prefix '{}', got '{}.'", prefix, response_str));
                                error!("Invalid response: expected '{}', got '{}.'", prefix, response_str);
                                return None;
                            }
                        }
                    }
                    Ok(_) => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Read error: {}.", e));
                        error!("Read error: {}.", e);
                        return None;
                    }
                }
            }
        }
        None
    }

    fn connect_to_port(&mut self, port_name: &str) {
        debug!("Attempting to connect to {}.", port_name);
        
        match serialport::new(port_name, 115200)
            .timeout(Duration::from_secs(2))
            .open()
        {
            Ok(mut port) => {
                // Step 1: Send "HEL\n" to initiate handshake.
                if let Err(e) = port.write_all(b"HEL\n") {
                    self.error_message = Some(format!("Failed to send HEL: {}.", e));
                    error!("Failed to send HEL: {}.", e);
                    return;
                }
                
                if let Err(e) = port.flush() {
                    self.error_message = Some(format!("Failed to flush: {}.", e));
                    error!("Failed to flush: {}.", e);
                    return;
                }
                
                debug!("Sent HEL, waiting for LO response.");
                
                // Step 1a: Wait for "LO\n" response.
                let mut buffer = [0u8; 10];
                let mut received = Vec::new();
                let start = std::time::Instant::now();
                
                loop {
                    if start.elapsed() > Duration::from_secs(2) {
                        self.error_message = Some("Timeout waiting for LO response.".to_string());
                        error!("Timeout waiting for LO response.");
                        return;
                    }
                    
                    match port.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            received.extend_from_slice(&buffer[..n]);
                            
                            if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                                let response: &[u8] = &received[..newline_pos];
                                let response_str = String::from_utf8_lossy(response).trim_end_matches('\r').to_string();
                                
                                if response_str == "LO" {
                                    debug!("Received LO response.");
                                    break;
                                } else {
                                    self.error_message = Some("Invalid LO response from device.".to_string());
                                    error!("Invalid response: expected 'LO', got '{}.'", response_str);
                                    return;
                                }
                            }
                        }
                        Ok(_) => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Read error: {}.", e));
                            error!("Read error: {}.", e);
                            return;
                        }
                    }
                }
                
                // Step 2: Move to connected state temporarily to use wait_for_response helper
                self.connection_state = ConnectionState::Connected {
                    port,
                    port_name: port_name.to_string(),
                    current_key: '\0',
                    version: String::new(),
                };
                
                // Step 2a: Send "VER\n" to get version.
                if let ConnectionState::Connected { ref mut port, .. } = self.connection_state {
                    if let Err(e) = port.write_all(b"VER\n") {
                        self.error_message = Some(format!("Failed to send VER: {}.", e));
                        error!("Failed to send VER: {}.", e);
                        self.connection_state = ConnectionState::Disconnected;
                        return;
                    }
                    if let Err(e) = port.flush() {
                        self.error_message = Some(format!("Failed to flush: {}", e));
                        error!("Failed to flush: {}", e);
                        self.connection_state = ConnectionState::Disconnected;
                        return;
                    }
                }
                
                debug!("Sent VER, waiting for version response.");
                if let Some(version_response) = self.wait_for_response("VER", 2) {
                    debug!("Received version response.");
                    // Parse version from "VER<VERSION>" format
                    let parsed_version = if version_response.len() > 3 {
                        version_response[3..].to_string()
                    } else {
                        "unknown".to_string()
                    };
                    if let ConnectionState::Connected { ref mut version, .. } = &mut self.connection_state {
                        *version = parsed_version;
                    }
                } else {
                    self.connection_state = ConnectionState::Disconnected;
                    return;
                }
                
                // Step 3: Send "KEY\n" to get keycode.
                if let ConnectionState::Connected { ref mut port, .. } = self.connection_state {
                    if let Err(e) = port.write_all(b"KEY\n") {
                        self.error_message = Some(format!("Failed to send KEY: {}.", e));
                        error!("Failed to send KEY: {}.", e);
                        self.connection_state = ConnectionState::Disconnected;
                        return;
                    }
                    if let Err(e) = port.flush() {
                        self.error_message = Some(format!("Failed to flush: {}", e));
                        error!("Failed to flush: {}", e);
                        self.connection_state = ConnectionState::Disconnected;
                        return;
                    }
                }
                
                debug!("Sent KEY, waiting for keycode response.");
                if let Some(key_response) = self.wait_for_response("KEY", 2) {
                    // Parse "KEY<KEYCODE>\n" response
                    if key_response.len() > 3 {
                        let key_char = key_response.chars().nth(3).unwrap_or('\0');
                        if key_char != '\0' {
                            info!("Connected! Device key: {}.", key_char);
                            if let ConnectionState::Connected { ref mut current_key, .. } = self.connection_state {
                                *current_key = key_char;
                            }
                            self.key_to_emulate = key_char.to_string();
                            self.error_message = None;
                            return;
                        }
                    }
                    self.error_message = Some("Invalid keycode in response.".to_string());
                    error!("Invalid keycode in response: {}", key_response);
                    self.connection_state = ConnectionState::Disconnected;
                } else {
                    self.connection_state = ConnectionState::Disconnected;
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to open port: {}.", e));
                error!("Failed to open {}: {}.", port_name, e);
            }
        }
    }

    fn disconnect(&mut self) {
        self.connection_state = ConnectionState::Disconnected;
        self.key_to_emulate.clear();
        info!("Disconnected from serial port.");
    }

    fn save_key_to_device(&mut self) {
        if let ConnectionState::Connected { ref mut port, ref port_name, ref mut current_key, .. } = self.connection_state {
            if self.key_to_emulate.len() == 1 {
                let new_key = self.key_to_emulate.chars().next().unwrap();
                let command = format!("KEY{}\n", new_key);
                
                match port.write_all(command.as_bytes()) {
                    Ok(_) => {
                        if let Err(e) = port.flush() {
                            self.error_message = Some(format!("Failed to flush: {}.", e));
                            error!("Failed to flush: {}.", e);
                            return;
                        }
                        
                        debug!("Sent KEY{}, waiting for response.", new_key);
                        
                        // Wait for "ACK\n" response.
                        let mut buffer = [0u8; 10];
                        let mut received = Vec::new();
                        let start = std::time::Instant::now();
                        
                        loop {
                            if start.elapsed() > Duration::from_secs(2) {
                                self.error_message = Some("Timeout waiting for response from device.".to_string());
                                error!("Timeout waiting for response from device.");
                                return;
                            }
                            
                            match port.read(&mut buffer) {
                                Ok(n) if n > 0 => {
                                    received.extend_from_slice(&buffer[..n]);
                                    
                                    // Check if we have a complete response.
                                    if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                                        let response = &received[..newline_pos];
                                        let response_str = String::from_utf8_lossy(response).trim_end_matches('\r').to_string();
                                        
                                        if response_str == "ACK" {
                                            *current_key = new_key;
                                            self.error_message = None;
                                            info!("Saved key '{}' to device {}.", new_key, port_name);
                                            return;
                                        } else {
                                            self.error_message = Some(format!("Device responded with '{}' instead of ACK.", response_str));
                                            error!("Invalid response from device: {}.", response_str);
                                            return;
                                        }
                                    }
                                }
                                Ok(_) => {
                                    // No data yet, continue.
                                    std::thread::sleep(Duration::from_millis(10));
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                                    // Timeout on read, continue loop.
                                    std::thread::sleep(Duration::from_millis(10));
                                }
                                Err(e) => {
                                    self.error_message = Some(format!("Read error: {}.", e));
                                    error!("Read error: {}.", e);
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to send key: {}.", e));
                        error!("Failed to send key: {}.", e);
                    }
                }
            }
        }
    }
}

impl eframe::App for SerialPortApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Top right dropdown.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (is_connected, _current_port_name) = match &self.connection_state {
                        ConnectionState::Connected { port_name, .. } => (true, Some(port_name.clone())),
                        ConnectionState::Disconnected => (false, None),
                    };

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
                                    let display_name = Self::get_port_display_name(port_info);
                                    if ui.selectable_label(false, &display_name).clicked() {
                                        self.connect_to_port(&port_info.port_name);
                                    }
                                }
                            }
                        });

                    if ui.button("Refresh Ports").clicked() && !is_connected {
                        self.refresh_ports();
                    }
                });
            });

            ui.separator();

            // Key input section.
            let is_connected = matches!(self.connection_state, ConnectionState::Connected { .. });
            
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

                if ui.add_enabled(is_connected && self.key_to_emulate.len() == 1, 
                                 egui::Button::new("Save to device")).clicked() {
                    self.save_key_to_device();
                }
            });
            
            // Add spacing to push content down.
            ui.allocate_space(egui::Vec2::new(1.0, ui.available_height() - 50.0));
        });
        
        // Bottom panel for status and theme selector.
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Connection status on the left.
                match &self.connection_state {
                    ConnectionState::Connected { port_name, version, .. } => {
                        let is_light_theme = ctx.style().visuals.dark_mode == false;
                        let label_color = if is_light_theme {
                            egui::Color32::from_rgb(0, 128, 0) // dark green for light theme
                        } else {
                            egui::Color32::GREEN // default green for dark theme
                        };
                        ui.colored_label(label_color, format!("Connected to: {} | Version: {}", port_name, version));
                    }
                    ConnectionState::Disconnected => {
                        ui.label("Not connected.");
                    }
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
                                if let Err(e) = confy::store("paap-configurator", None, config) {
                                    error!("Failed to save theme preference: {}.", e);
                                }
                            }
                        });
                });
            });
        });
        
        // Apply theme based on selection.
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
        
        // Error message modal.
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

fn create_icon() -> egui::IconData {
    // Create a 32x32 icon with a simple keyboard design.
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];
    
    // Background color (light blue/gray).
    for pixel in rgba.chunks_mut(4) {
        pixel[0] = 70;  // R
        pixel[1] = 130; // G
        pixel[2] = 180; // B
        pixel[3] = 255; // A
    }
    
    // Draw keyboard keys (simplified representation).
    let draw_rect = |rgba: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: [u8; 4]| {
        for py in y..(y + h).min(size) {
            for px in x..(x + w).min(size) {
                let idx = (py * size + px) * 4;
                if idx + 3 < rgba.len() {
                    rgba[idx] = color[0];
                    rgba[idx + 1] = color[1];
                    rgba[idx + 2] = color[2];
                    rgba[idx + 3] = color[3];
                }
            }
        }
    };
    
    let white = [240, 240, 240, 255];
    let dark = [40, 40, 40, 255];
    
    // Draw border.
    for i in 0..size {
        draw_rect(&mut rgba, i, 0, 1, 1, dark);
        draw_rect(&mut rgba, i, size - 1, 1, 1, dark);
        draw_rect(&mut rgba, 0, i, 1, 1, dark);
        draw_rect(&mut rgba, size - 1, i, 1, 1, dark);
    }
    
    // Draw keys (3 rows of keys).
    for row in 0..3 {
        for col in 0..5 {
            let x = 4 + col * 5;
            let y = 8 + row * 7;
            draw_rect(&mut rgba, x, y, 4, 5, white);
        }
    }
    
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

fn main() -> Result<(), eframe::Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    
    debug!("Starting application.");
    
    // Create a simple icon (32x32 keyboard icon).
    let icon_data = create_icon();
    
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
        Box::new(|_cc| Ok(Box::<SerialPortApp>::default())),
    )
}
