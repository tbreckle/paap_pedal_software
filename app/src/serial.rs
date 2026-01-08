//! Serial port communication module for the PAAP Configurator.
//!
//! This module handles all communication with the connected hardware device over a serial port.
//! It provides functions for port discovery, connection establishment, and device communication
//! including handshaking, version retrieval, and key configuration.

use crate::connection::ConnectionState;
use log::{debug, error};
use serialport;
use std::io::{Read, Write};
use std::time::Duration;

/// Refreshes the list of available serial ports.
///
/// Queries the system for all available serial ports and returns their information.
/// This function should be called periodically to detect when devices are connected or disconnected.
///
/// # Returns
///
/// Returns `Ok(Vec<serialport::SerialPortInfo>)` containing all detected ports,
/// or `Err(String)` if the port detection failed.
///
/// # Example
///
/// ```ignore
/// match refresh_ports() {
///     Ok(ports) => println!("Found {} ports", ports.len()),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn refresh_ports() -> Result<Vec<serialport::SerialPortInfo>, String> {
    match serialport::available_ports() {
        Ok(ports) => {
            debug!("Found {} serial port(s)", ports.len());
            Ok(ports)
        }
        Err(e) => {
            let error_msg = format!("Error scanning ports: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// Returns a display name for a serial port.
///
/// Generates a human-readable display name by extracting product and manufacturer information
/// from USB ports, or using a generic label for other port types.
///
/// # Arguments
///
/// * `port` - The serial port information to format.
///
/// # Returns
///
/// A formatted string containing the port display name and port name in parentheses.
pub fn get_port_display_name(port: &serialport::SerialPortInfo) -> String {
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

/// Waits for a response from the connected serial port with a given prefix.
///
/// This helper function reads from the serial port until it receives a complete line
/// (terminated by `\n`) that starts with the specified prefix. It handles timeouts
/// and validates that the response format matches expectations.
///
/// # Arguments
///
/// * `connection_state` - The current connection state, must be in `Connected` state.
/// * `prefix` - The expected prefix of the response (e.g., "LO", "VER", "KEY", "ACK").
/// * `timeout_secs` - Maximum time in seconds to wait for the response.
///
/// # Returns
///
/// Returns `Some(String)` containing the complete response if received successfully,
/// or `None` if a timeout occurs, the device is not connected, or an invalid response is received.
pub fn wait_for_response(
    connection_state: &mut ConnectionState,
    prefix: &str,
    timeout_secs: u64,
) -> Option<String> {
    if let ConnectionState::Connected { ref mut port, .. } = connection_state {
        let mut buffer = [0u8; 64];
        let mut received = Vec::new();
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > Duration::from_secs(timeout_secs) {
                let error_msg = format!("Timeout waiting for {} response.", prefix);
                error!("{}", error_msg);
                return None;
            }

            match port.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    received.extend_from_slice(&buffer[..n]);

                    // Check if we have a complete response.
                    if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                        let response = &received[..newline_pos];
                        let response_str = String::from_utf8_lossy(response)
                            .trim_end_matches('\r')
                            .to_string();

                        if response_str.starts_with(prefix) {
                            return Some(response_str);
                        } else {
                            let error_msg = format!(
                                "Invalid response: expected prefix '{}', got '{}'.",
                                prefix, response_str
                            );
                            error!("{}", error_msg);
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
                    let error_msg = format!("Read error: {}.", e);
                    error!("{}", error_msg);
                    return None;
                }
            }
        }
    }
    None
}

/// Attempts to connect to a serial port and performs the handshake protocol.
///
/// Establishes a connection to the specified serial port and performs a three-step handshake:
/// 1. Sends "HEL" and waits for "LO" response
/// 2. Sends "VER" and retrieves the device firmware version
/// 3. Sends "KEY" and retrieves the currently configured key
///
/// The device is configured with a baud rate of 115200 and a 1-second timeout.
///
/// # Arguments
///
/// * `port_name` - The name of the serial port to connect to (e.g., "COM3", "/dev/ttyUSB0").
/// * `connection_state` - A mutable reference to the connection state to be updated.
///
/// # Returns
///
/// Returns `Ok(String)` containing the current key character if connection succeeds,
/// or `Err(String)` with a descriptive error message if any step of the handshake fails.
pub fn connect_to_port(
    port_name: &str,
    connection_state: &mut ConnectionState,
) -> Result<String, String> {
    debug!("Attempting to connect to {}.", port_name);

    let mut port = serialport::new(port_name, 115200)
        .timeout(Duration::from_secs(1))
        .open()
        .map_err(|e| format!("Failed to open port: {}.", e))?;

    // Step 1: Send "HEL\n" to initiate handshake.
    port.write_all(b"HEL\n")
        .map_err(|e| format!("Failed to send HEL: {}.", e))?;
    port.flush()
        .map_err(|e| format!("Failed to flush: {}.", e))?;

    debug!("Sent HEL, waiting for LO response.");

    // Step 1a: Wait for "LO\n" response.
    let mut buffer = [0u8; 10];
    let mut received = Vec::new();
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > Duration::from_secs(1) {
            return Err("Timeout waiting for LO response.".to_string());
        }

        match port.read(&mut buffer) {
            Ok(n) if n > 0 => {
                received.extend_from_slice(&buffer[..n]);

                if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                    let response: &[u8] = &received[..newline_pos];
                    let response_str = String::from_utf8_lossy(response)
                        .trim_end_matches('\r')
                        .to_string();

                    if response_str == "LO" {
                        debug!("Received LO response.");
                        break;
                    } else {
                        return Err(format!(
                            "Invalid response: expected 'LO', got '{}'.",
                            response_str
                        ));
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
                return Err(format!("Read error: {}.", e));
            }
        }
    }

    // Step 2: Move to connected state to use wait_for_response helper
    *connection_state = ConnectionState::Connected {
        port,
        port_name: port_name.to_string(),
        current_key: '\0',
        version: String::new(),
    };

    // Step 2a: Send "VER\n" to get version.
    if let ConnectionState::Connected { ref mut port, .. } = connection_state {
        port.write_all(b"VER\n")
            .map_err(|e| format!("Failed to send VER: {}.", e))?;
        port.flush()
            .map_err(|e| format!("Failed to flush: {}.", e))?;
    }

    debug!("Sent VER, waiting for version response.");
    let version_response = wait_for_response(connection_state, "VER", 2)
        .ok_or_else(|| "Failed to get version response.".to_string())?;

    let parsed_version = if version_response.len() > 3 {
        version_response[3..].to_string()
    } else {
        "unknown".to_string()
    };

    if let ConnectionState::Connected {
        ref mut version, ..
    } = connection_state
    {
        *version = parsed_version;
    }

    // Step 3: Send "KEY\n" to get keycode.
    if let ConnectionState::Connected { ref mut port, .. } = connection_state {
        port.write_all(b"KEY\n")
            .map_err(|e| format!("Failed to send KEY: {}.", e))?;
        port.flush()
            .map_err(|e| format!("Failed to flush: {}.", e))?;
    }

    debug!("Sent KEY, waiting for keycode response.");
    let key_response = wait_for_response(connection_state, "KEY", 2)
        .ok_or_else(|| "Failed to get keycode response.".to_string())?;

    // Parse "KEY<KEYCODE>\n" response
    if key_response.len() > 3 {
        let key_char = key_response.chars().nth(3).unwrap_or('\0');
        if key_char != '\0' {
            if let ConnectionState::Connected {
                ref mut current_key,
                ..
            } = connection_state
            {
                *current_key = key_char;
                debug!("Connected! Device key: {}.", key_char);
            }
            return Ok(key_char.to_string());
        }
    }

    *connection_state = ConnectionState::Disconnected;
    Err("Invalid keycode in response.".to_string())
}

/// Saves a key to the connected device.
///
/// Sends a key configuration command to the connected device and waits for acknowledgment.
/// If successful, updates the current key stored in the connection state.
///
/// # Arguments
///
/// * `connection_state` - A mutable reference to the connection state. Must be in `Connected` state.
/// * `new_key` - The single ASCII character key to save to the device.
///
/// # Returns
///
/// Returns `Ok(())` if the key was successfully saved and acknowledged,
/// or `Err(String)` with a descriptive error message if the operation fails.
pub fn save_key_to_device(
    connection_state: &mut ConnectionState,
    new_key: char,
) -> Result<(), String> {
    if let ConnectionState::Connected {
        ref mut port,
        ref mut current_key,
        ..
    } = connection_state
    {
        let command = format!("KEY{}\n", new_key);

        port.write_all(command.as_bytes())
            .map_err(|e| format!("Failed to send key: {}.", e))?;
        port.flush()
            .map_err(|e| format!("Failed to flush: {}.", e))?;

        debug!("Sent KEY{}, waiting for response.", new_key);

        // Wait for "ACK\n" response.
        let mut buffer = [0u8; 10];
        let mut received = Vec::new();
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > Duration::from_secs(1) {
                return Err("Timeout waiting for response from device.".to_string());
            }

            match port.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    received.extend_from_slice(&buffer[..n]);

                    // Check if we have a complete response.
                    if let Some(newline_pos) = received.iter().position(|&b| b == b'\n') {
                        let response = &received[..newline_pos];
                        let response_str = String::from_utf8_lossy(response)
                            .trim_end_matches('\r')
                            .to_string();

                        if response_str == "ACK" {
                            *current_key = new_key;
                            debug!("Saved key '{}' to device.", new_key);
                            return Ok(());
                        } else {
                            return Err(format!(
                                "Device responded with '{}' instead of ACK.",
                                response_str
                            ));
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
                    return Err(format!("Read error: {}.", e));
                }
            }
        }
    }

    Err("Not connected to a device.".to_string())
}
