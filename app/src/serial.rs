//! Serial port communication with the PAAP pedal.
//!
//! The firmware speaks a line-based ASCII protocol at 115200 baud (see DEV.md):
//!
//! | Command      | Response        |
//! |--------------|-----------------|
//! | `HEL`        | `LO`            |
//! | `VER`        | `VER<version>`  |
//! | `KEY`        | `KEY<char>`     |
//! | `KEY<char>`  | `ACK`           |
//! | `PED`        | `PED<number>`   |
//! | `PED<number>`| `ACK` or `ERR`  |
//!
//! Firmware 1.0 doesn't know `PED` and doesn't answer it.

use log::debug;
use serialport::{SerialPort, SerialPortInfo, SerialPortType, UsbPortInfo};
use std::io::{Read, Write};
use std::ops::RangeInclusive;
use std::time::{Duration, Instant};

const BAUD_RATE: u32 = 115_200;
/// Timeout for responses to known commands.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
/// Timeout for `PED`, so connecting to firmware 1.0 (no response) doesn't take long.
const PEDAL_TIMEOUT: Duration = Duration::from_millis(500);

/// Valid pedal numbers.
pub const PEDAL_NUMBERS: RangeInclusive<u8> = 1..=16;

/// Pedal number in the USB serial number of a port, if it is a PAAP pedal.
///
/// The firmware appends `PAAPnn` to the USB serial number (e.g. `HIDPCPAAP03`), so pedals can
/// be told apart without opening their ports.
pub fn pedal_number(port: &SerialPortInfo) -> Option<u8> {
    let SerialPortType::UsbPort(info) = &port.port_type else {
        return None;
    };
    let serial = info.serial_number.as_deref()?;
    let digits = serial.get(serial.find("PAAP")? + 4..)?.get(..2)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits
        .parse()
        .ok()
        .filter(|number| PEDAL_NUMBERS.contains(number))
}

/// Label of a port in the device list, e.g. `PAAP Pedal 3 (COM5) [2341:8037]`.
///
/// USB ports end in `[VID:PID]`: Windows reports many devices with the same generic name.
pub fn port_label(port: &SerialPortInfo) -> String {
    let SerialPortType::UsbPort(info) = &port.port_type else {
        return port.port_name.clone();
    };
    let name = match pedal_number(port) {
        Some(number) => format!("PAAP Pedal {number}"),
        None => match (info.manufacturer.as_deref(), info.product.as_deref()) {
            (Some(manufacturer), Some(product)) if !product.starts_with(manufacturer) => {
                format!("{manufacturer} {product}")
            }
            (_, Some(product)) => product.to_string(),
            (Some(manufacturer), None) => manufacturer.to_string(),
            (None, None) => "USB device".to_string(),
        },
    };
    format!(
        "{name} ({}) [{:04X}:{:04X}]",
        port.port_name, info.vid, info.pid
    )
}

/// Whether a USB device may be a pedal with firmware 1.0, which has no pedal number: an
/// Arduino (or Arduino.org / SparkFun Pro Micro) board, or one of the PAAP IDs from DEV.md.
/// Other devices with the OpenFIRE VID 0xF144, like lightguns, are not pedals.
fn is_pedal_board(info: &UsbPortInfo) -> bool {
    matches!(info.vid, 0x2341 | 0x2A03 | 0x1B4F)
        || (info.vid == 0xF144 && matches!(info.pid, 0x1001 | 0x1002))
}

/// Sort rank of a port in the device list: PAAP pedals by pedal number, then boards a pedal
/// with older firmware may be built on, then other USB ports, then the rest.
fn port_rank(port: &SerialPortInfo) -> (u8, u8) {
    match (&port.port_type, pedal_number(port)) {
        (_, Some(number)) => (0, number),
        (SerialPortType::UsbPort(info), None) if is_pedal_board(info) => (1, 0),
        (SerialPortType::UsbPort(_), None) => (2, 0),
        _ => (3, 0),
    }
}

/// All serial ports, likely pedals first (see [`port_rank`]).
pub fn list_ports() -> Result<Vec<SerialPortInfo>, String> {
    let mut ports =
        serialport::available_ports().map_err(|e| format!("Error scanning ports: {e}."))?;
    ports.sort_by(|a, b| {
        port_rank(a)
            .cmp(&port_rank(b))
            .then_with(|| a.port_name.cmp(&b.port_name))
    });
    debug!("Found {} serial port(s).", ports.len());
    Ok(ports)
}

/// A connected pedal.
pub struct Device {
    port: Box<dyn SerialPort>,
    pub port_name: String,
    pub version: String,
    pub key: char,
    /// `None` for firmware without pedal numbers.
    pub pedal: Option<u8>,
}

impl Device {
    /// Opens the port and reads version, key and pedal number from the firmware.
    pub fn connect(port_name: &str) -> Result<Device, String> {
        debug!("Connecting to {port_name}.");
        let port = serialport::new(port_name, BAUD_RATE)
            .timeout(Duration::from_millis(50))
            // The AVR core drops serial output while both DTR and RTS are low, so make sure
            // DTR is raised on every platform.
            .dtr_on_open(true)
            .open()
            .map_err(|e| format!("Failed to open {port_name}: {e}."))?;
        let _ = port.clear(serialport::ClearBuffer::Input);

        let mut device = Device {
            port,
            port_name: port_name.to_string(),
            version: String::new(),
            key: '\0',
            pedal: None,
        };

        let not_a_pedal = |_| format!("{port_name} is not a PAAP pedal (no response).");
        if device
            .command("HEL", RESPONSE_TIMEOUT)
            .map_err(not_a_pedal)?
            != "LO"
        {
            return Err(format!("{port_name} is not a PAAP pedal."));
        }

        device.version = device
            .command("VER", RESPONSE_TIMEOUT)?
            .strip_prefix("VER")
            .ok_or("Invalid version response.")?
            .to_string();

        let key_response = device.command("KEY", RESPONSE_TIMEOUT)?;
        device.key = key_response
            .strip_prefix("KEY")
            .and_then(|key| key.chars().next())
            .ok_or("Invalid key response.")?;

        device.pedal = device
            .command("PED", PEDAL_TIMEOUT)
            .ok()
            .and_then(|response| response.strip_prefix("PED")?.parse().ok());

        debug!(
            "Connected to {port_name}: version {}, key {:?}, pedal {:?}.",
            device.version, device.key, device.pedal
        );
        Ok(device)
    }

    /// Saves the key the pedal emulates.
    pub fn save_key(&mut self, key: char) -> Result<(), String> {
        self.expect_ack(&format!("KEY{key}"))?;
        self.key = key;
        Ok(())
    }

    /// Saves the pedal number. If it changed, the pedal reconnects to USB afterwards, so its
    /// serial port disappears and comes back with the new number; this `Device` is unusable then.
    pub fn save_pedal(&mut self, number: u8) -> Result<(), String> {
        self.expect_ack(&format!("PED{number}"))?;
        self.pedal = Some(number);
        Ok(())
    }

    fn expect_ack(&mut self, command: &str) -> Result<(), String> {
        match self.command(command, RESPONSE_TIMEOUT)?.as_str() {
            "ACK" => Ok(()),
            response => Err(format!("The pedal rejected {command}: {response}.")),
        }
    }

    /// Sends a command and returns the response line.
    fn command(&mut self, command: &str, timeout: Duration) -> Result<String, String> {
        self.port
            .write_all(format!("{command}\n").as_bytes())
            .and_then(|_| self.port.flush())
            .map_err(|e| format!("Failed to send {command}: {e}."))?;
        self.read_line(timeout)
            .map_err(|e| format!("No response to {command}: {e}"))
    }

    fn read_line(&mut self, timeout: Duration) -> Result<String, String> {
        let start = Instant::now();
        let mut received = Vec::new();
        let mut buffer = [0u8; 64];
        loop {
            if let Some(end) = received.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&received[..end]);
                return Ok(line.trim_end_matches('\r').to_string());
            }
            if start.elapsed() > timeout {
                return Err("timeout.".to_string());
            }
            match self.port.read(&mut buffer) {
                Ok(n) => received.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(format!("{e}.")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usb_port_with_vid(vid: u16, name: &str) -> SerialPortInfo {
        let mut port = usb_port(name, None, None, None);
        if let SerialPortType::UsbPort(info) = &mut port.port_type {
            info.vid = vid;
        }
        port
    }

    #[test]
    fn port_ranks() {
        let pedal = |number: u8| {
            let serial = format!("HIDPCPAAP{number:02}");
            usb_port("COM9", Some(&serial), None, None)
        };
        let mut ports = [
            SerialPortInfo {
                port_name: "/dev/ttyS0".to_string(),
                port_type: SerialPortType::Unknown,
            },
            usb_port_with_vid(0x041E, "sound card"),
            usb_port_with_vid(0xF144, "lightgun"),
            usb_port_with_vid(0x2341, "old pedal"),
            pedal(12),
            pedal(2),
        ];
        ports.sort_by_key(port_rank);
        let names: Vec<_> = ports.iter().map(port_label).collect();
        assert_eq!(
            names,
            [
                "PAAP Pedal 2 (COM9) [2341:8037]",
                "PAAP Pedal 12 (COM9) [2341:8037]",
                "USB device (old pedal) [2341:8037]",
                "USB device (sound card) [041E:8037]",
                "USB device (lightgun) [F144:8037]",
                "/dev/ttyS0",
            ]
        );
    }

    fn usb_port(
        name: &str,
        serial: Option<&str>,
        manufacturer: Option<&str>,
        product: Option<&str>,
    ) -> SerialPortInfo {
        SerialPortInfo {
            port_name: name.to_string(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x2341,
                pid: 0x8037,
                serial_number: serial.map(str::to_string),
                manufacturer: manufacturer.map(str::to_string),
                product: product.map(str::to_string),
            }),
        }
    }

    #[test]
    fn pedal_number_from_serial() {
        let number = |serial| pedal_number(&usb_port("COM1", Some(serial), None, None));
        assert_eq!(number("HIDPCPAAP03"), Some(3));
        assert_eq!(number("PAAP16HIDPC"), Some(16));
        assert_eq!(number("HIDPC"), None);
        assert_eq!(number("HIDPCPAAP00"), None);
        assert_eq!(number("HIDPCPAAP17"), None);
        assert_eq!(number("HIDPCPAAP1"), None);
        assert_eq!(number("HIDPCPAAP+1"), None);
        assert_eq!(pedal_number(&usb_port("COM1", None, None, None)), None);
    }

    #[test]
    fn port_labels() {
        let pedal = usb_port(
            "COM5",
            Some("HIDPCPAAP03"),
            Some("Arduino SA"),
            Some("Arduino Micro"),
        );
        assert_eq!(port_label(&pedal), "PAAP Pedal 3 (COM5) [2341:8037]");

        let micro = usb_port(
            "/dev/ttyACM0",
            Some("HIDPC"),
            Some("Arduino SA"),
            Some("Arduino Micro"),
        );
        assert_eq!(
            port_label(&micro),
            "Arduino SA Arduino Micro (/dev/ttyACM0) [2341:8037]"
        );

        let product_only = usb_port("COM3", None, Some("Arduino"), Some("Arduino Micro"));
        assert_eq!(
            port_label(&product_only),
            "Arduino Micro (COM3) [2341:8037]"
        );

        let unnamed = usb_port("COM4", None, None, None);
        assert_eq!(port_label(&unnamed), "USB device (COM4) [2341:8037]");

        let native = SerialPortInfo {
            port_name: "/dev/ttyS0".to_string(),
            port_type: SerialPortType::Unknown,
        };
        assert_eq!(port_label(&native), "/dev/ttyS0");
    }
}
