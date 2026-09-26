//! Connects the Slint UI (ui/app.slint) with the pedal.
//!
//! The UI only renders properties and reports actions through callbacks. All state lives in
//! [`AppState`]; [`Controller::refresh`] pushes it to the UI after every change. Serial I/O
//! blocks, so it runs on a background thread while the UI shows a busy overlay.

use crate::config::{AppConfig, ThemeMode};
use crate::serial::{self, Device, PEDAL_NUMBERS};
use log::error;
use serialport::SerialPortInfo;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

slint::include_modules!();

const APP_VERSION: &str = env!("PAAP_VERSION");
/// How often the result of a background operation is checked.
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// How often the device list is refreshed while disconnected.
const PORT_SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// How long to wait for a pedal to come back after its pedal number changed.
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Result of a background operation. The operation owns the device while it runs and hands it
/// back here.
enum OpResult {
    Connected {
        device: Device,
        message: Option<String>,
    },
    Saved(Device),
    Failed {
        device: Option<Device>,
        message: String,
    },
}

#[derive(Default)]
struct AppState {
    ports: Vec<SerialPortInfo>,
    /// Port selected in the device list, kept across refreshes.
    selected_port: Option<String>,
    device: Option<Device>,
    /// Edited settings, saved to the pedal on "Save to pedal".
    key: char,
    pedal: u8,
    /// Status line text and whether it is an error.
    message: Option<(String, bool)>,
    busy_message: Option<String>,
    pending_result: Option<mpsc::Receiver<OpResult>>,
}

impl AppState {
    fn set_error(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), true));
    }

    fn set_info(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), false));
    }

    fn connected(&mut self, device: Device) {
        self.key = device.key;
        self.pedal = device.pedal.unwrap_or(*PEDAL_NUMBERS.start());
        self.selected_port = Some(device.port_name.clone());
        self.device = Some(device);
    }

    fn apply_result(&mut self, result: OpResult) {
        self.pending_result = None;
        self.busy_message = None;
        match result {
            OpResult::Connected { device, message } => {
                self.connected(device);
                self.message = message.map(|m| (m, false));
            }
            OpResult::Saved(device) => {
                self.device = Some(device);
                self.set_info("Saved to the pedal.");
            }
            OpResult::Failed { device, message } => {
                self.device = device;
                self.set_error(message);
            }
        }
    }

    /// Another port that already uses the edited pedal number.
    fn pedal_conflict(&self) -> Option<&SerialPortInfo> {
        let device = self.device.as_ref()?;
        self.ports.iter().find(|port| {
            port.port_name != device.port_name && serial::pedal_number(port) == Some(self.pedal)
        })
    }

    fn pedal_changed(&self) -> bool {
        self.device
            .as_ref()
            .and_then(|device| device.pedal)
            .is_some_and(|pedal| pedal != self.pedal)
    }

    fn can_save(&self) -> bool {
        let Some(device) = &self.device else {
            return false;
        };
        self.busy_message.is_none()
            && (device.key != self.key || self.pedal_changed())
            && self.pedal_conflict().is_none()
    }

    fn pedal_hint(&self) -> String {
        let Some(device) = &self.device else {
            return "Tells your pedals apart in the device list.".to_string();
        };
        if device.pedal.is_none() {
            return "Update the pedal's firmware to set a pedal number.".to_string();
        }
        if let Some(port) = self.pedal_conflict() {
            return format!(
                "Pedal {} is already used by {}.",
                self.pedal, port.port_name
            );
        }
        if self.pedal_changed() {
            return "The pedal reconnects after saving.".to_string();
        }
        "Tells your pedals apart in the device list.".to_string()
    }

    fn connection_text(&self) -> String {
        match &self.device {
            Some(device) => {
                let pedal = device
                    .pedal
                    .map(|pedal| format!("Pedal {pedal} · "))
                    .unwrap_or_default();
                format!("{pedal}{} · firmware {}", device.port_name, device.version)
            }
            None => "Not connected".to_string(),
        }
    }
}

/// Text on the keycap for a key.
fn key_text(key: char) -> String {
    match key {
        ' ' => "Space".to_string(),
        key => key.to_string(),
    }
}

/// Caption under the keycap, so similar looking keys (l, I, 1) can be told apart.
fn key_description(key: char) -> String {
    match key {
        ' ' => "space bar".to_string(),
        'a'..='z' => format!("lowercase {}", key.to_ascii_uppercase()),
        'A'..='Z' => format!("uppercase {key} (with Shift)"),
        '0'..='9' => format!("digit {key}"),
        _ => "symbol".to_string(),
    }
}

/// The firmware sends the key as a single printable ASCII character.
fn parse_captured_key(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(key), None) if key == ' ' || key.is_ascii_graphic() => Some(key),
        _ => None,
    }
}

/// Waits for a pedal to reconnect with a new pedal number and connects to it again.
fn reconnect_pedal(number: u8) -> OpResult {
    let deadline = Instant::now() + RECONNECT_TIMEOUT;
    let mut last_error = None;
    // The pedal only disconnects after it has answered.
    thread::sleep(Duration::from_millis(800));
    while Instant::now() < deadline {
        let port = serial::list_ports()
            .unwrap_or_default()
            .into_iter()
            .find(|port| serial::pedal_number(port) == Some(number));
        if let Some(port) = port {
            // The port may exist before it can be opened.
            match Device::connect(&port.port_name) {
                Ok(device) => {
                    return OpResult::Connected {
                        message: Some(format!(
                            "Saved. The pedal is now Pedal {number} on {}.",
                            device.port_name
                        )),
                        device,
                    };
                }
                Err(e) => last_error = Some(e),
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    let mut message = format!(
        "Saved Pedal {number}, but the pedal didn't come back. Replug it and connect again."
    );
    if let Some(e) = last_error {
        message.push(' ');
        message.push_str(&e);
    }
    OpResult::Failed {
        device: None,
        message,
    }
}

/// Update a model in place: rows are only replaced when they changed, so an open ComboBox
/// isn't disturbed by a refresh.
fn sync_model(model: &VecModel<SharedString>, items: Vec<SharedString>) {
    for (i, item) in items.iter().enumerate() {
        if i >= model.row_count() {
            model.push(item.clone());
        } else if model.row_data(i).as_ref() != Some(item) {
            model.set_row_data(i, item.clone());
        }
    }
    while model.row_count() > items.len() {
        model.remove(model.row_count() - 1);
    }
}

struct Controller {
    ui: slint::Weak<AppWindow>,
    state: RefCell<AppState>,
    port_model: Rc<VecModel<SharedString>>,
    poll_timer: Timer,
    port_scan_timer: Timer,
}

impl Controller {
    /// Push the application state to the UI.
    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let st = self.state.borrow();

        // Device list.
        let labels = if st.ports.is_empty() {
            vec!["No device found".into()]
        } else {
            st.ports
                .iter()
                .map(|port| serial::port_label(port).into())
                .collect()
        };
        sync_model(&self.port_model, labels);
        ui.set_has_ports(!st.ports.is_empty());
        let selected = st
            .selected_port
            .as_ref()
            .and_then(|name| st.ports.iter().position(|port| &port.port_name == name))
            .unwrap_or(0);
        ui.set_port_index(selected as i32);

        // Connection and pedal settings.
        let device = st.device.as_ref();
        ui.set_connected(device.is_some());
        ui.set_connection_text(st.connection_text().into());
        ui.set_key_text(
            device
                .map(|_| key_text(st.key))
                .unwrap_or("–".into())
                .into(),
        );
        ui.set_key_description(
            device
                .map(|_| key_description(st.key))
                .unwrap_or_default()
                .into(),
        );
        ui.set_pedal_number(st.pedal.into());
        ui.set_pedal_supported(device.is_some_and(|device| device.pedal.is_some()));
        ui.set_pedal_hint(st.pedal_hint().into());
        ui.set_can_save(st.can_save());

        // Status and background operation.
        let (message, is_error) = st.message.clone().unwrap_or_default();
        ui.set_message(message.into());
        ui.set_message_is_error(is_error);
        ui.set_busy(st.busy_message.is_some());
        ui.set_busy_message(st.busy_message.clone().unwrap_or_default().into());
    }

    /// Run `job` on a background thread while the busy overlay is shown.
    fn start_operation(
        self: &Rc<Self>,
        message: &str,
        job: impl FnOnce() -> OpResult + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        {
            let mut st = self.state.borrow_mut();
            st.busy_message = Some(message.to_string());
            st.message = None;
            st.pending_result = Some(rx);
        }
        thread::spawn(move || {
            let _ = tx.send(job());
        });

        let weak = Rc::downgrade(self);
        self.poll_timer
            .start(TimerMode::Repeated, POLL_INTERVAL, move || {
                if let Some(controller) = weak.upgrade() {
                    controller.poll_pending_result();
                }
            });
        self.refresh();
    }

    fn poll_pending_result(&self) {
        let received = {
            let st = self.state.borrow();
            let Some(rx) = st.pending_result.as_ref() else {
                self.poll_timer.stop();
                return;
            };
            match rx.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
            }
        };

        let Some(received) = received else {
            return;
        };
        self.poll_timer.stop();
        {
            let mut st = self.state.borrow_mut();
            match received {
                Ok(result) => st.apply_result(result),
                Err(()) => {
                    // The thread dropped the sender without sending (e.g. panic).
                    st.pending_result = None;
                    st.busy_message = None;
                    st.set_error("Operation failed unexpectedly.");
                }
            }
        }
        self.scan_ports();
    }

    /// Reads the port list again and updates the UI.
    fn scan_ports(&self) {
        {
            let mut st = self.state.borrow_mut();
            match serial::list_ports() {
                Ok(ports) => st.ports = ports,
                Err(e) => {
                    error!("{e}");
                    st.set_error(e);
                }
            }
        }
        self.refresh();
    }

    /// Refreshes the device list while disconnected, so plugged in pedals show up.
    fn scan_ports_if_idle(&self) {
        let idle = {
            let st = self.state.borrow();
            st.device.is_none() && st.busy_message.is_none()
        };
        if idle {
            self.scan_ports();
        }
    }

    fn port_selected(&self, index: i32) {
        let mut st = self.state.borrow_mut();
        let name = usize::try_from(index)
            .ok()
            .and_then(|index| st.ports.get(index))
            .map(|port| port.port_name.clone());
        st.selected_port = name;
    }

    fn connect(self: &Rc<Self>, index: i32) {
        let port_name = {
            let st = self.state.borrow();
            let port = usize::try_from(index)
                .ok()
                .and_then(|index| st.ports.get(index));
            match port {
                Some(port) => port.port_name.clone(),
                None => return,
            }
        };
        self.state.borrow_mut().selected_port = Some(port_name.clone());

        self.start_operation(
            "Connecting to the pedal…",
            move || match Device::connect(&port_name) {
                Ok(device) => OpResult::Connected {
                    device,
                    message: None,
                },
                Err(message) => OpResult::Failed {
                    device: None,
                    message,
                },
            },
        );
    }

    fn disconnect(&self) {
        {
            let mut st = self.state.borrow_mut();
            st.device = None;
            st.message = None;
        }
        self.scan_ports();
    }

    fn key_captured(&self, text: &str) -> bool {
        let accepted = {
            let mut st = self.state.borrow_mut();
            match parse_captured_key(text) {
                Some(key) => {
                    st.key = key;
                    st.message = None;
                    true
                }
                None => {
                    st.set_error(
                        "Only letters, digits, space and symbols are supported. Try another key.",
                    );
                    false
                }
            }
        };
        self.refresh();
        accepted
    }

    fn pedal_edited(&self, value: i32) {
        if let Ok(pedal) = u8::try_from(value)
            && PEDAL_NUMBERS.contains(&pedal)
        {
            self.state.borrow_mut().pedal = pedal;
        }
        self.refresh();
    }

    fn save(self: &Rc<Self>) {
        let (mut device, key, pedal) = {
            let mut st = self.state.borrow_mut();
            if !st.can_save() {
                return;
            }
            let pedal = st.pedal_changed().then_some(st.pedal);
            let Some(device) = st.device.take() else {
                return;
            };
            (device, st.key, pedal)
        };

        let message = match pedal {
            Some(pedal) => format!("Saving… the pedal reconnects as Pedal {pedal}."),
            None => "Saving…".to_string(),
        };
        self.start_operation(&message, move || {
            if device.key != key
                && let Err(message) = device.save_key(key)
            {
                return OpResult::Failed {
                    device: Some(device),
                    message,
                };
            }
            let Some(pedal) = pedal else {
                return OpResult::Saved(device);
            };
            if let Err(message) = device.save_pedal(pedal) {
                return OpResult::Failed {
                    device: Some(device),
                    message,
                };
            }
            // The pedal reconnects to USB with the new number; the old port is gone.
            drop(device);
            reconnect_pedal(pedal)
        });
    }

    fn theme_selected(&self, index: i32) {
        let config = AppConfig {
            theme_mode: ThemeMode::from_index(index),
        };
        if let Err(e) = config.save() {
            error!("Failed to save theme preference: {e}.");
        }
    }

    /// Wait for a running background operation, so its serial port is closed cleanly.
    fn shutdown(&self) {
        self.poll_timer.stop();
        self.port_scan_timer.stop();
        let mut st = self.state.borrow_mut();
        if let Some(rx) = st.pending_result.take()
            && let Ok(result) = rx.recv_timeout(RECONNECT_TIMEOUT)
        {
            st.apply_result(result);
        }
        st.device = None;
    }
}

/// Create the main window and run the event loop.
pub fn run() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let controller = Rc::new(Controller {
        ui: ui.as_weak(),
        state: RefCell::new(AppState {
            pedal: *PEDAL_NUMBERS.start(),
            ..Default::default()
        }),
        port_model: Rc::new(VecModel::default()),
        poll_timer: Timer::default(),
        port_scan_timer: Timer::default(),
    });

    ui.set_app_version(APP_VERSION.into());
    ui.set_theme(AppConfig::load().theme_mode.index());
    ui.set_port_labels(ModelRc::from(controller.port_model.clone()));

    let c = controller.clone();
    ui.on_refresh_ports(move || c.scan_ports());
    let c = controller.clone();
    ui.on_port_selected(move |index| c.port_selected(index));
    let c = controller.clone();
    ui.on_connect(move |index| c.connect(index));
    let c = controller.clone();
    ui.on_disconnect(move || c.disconnect());
    let c = controller.clone();
    ui.on_key_captured(move |text| c.key_captured(&text));
    let c = controller.clone();
    ui.on_pedal_edited(move |value| c.pedal_edited(value));
    let c = controller.clone();
    ui.on_save(move || c.save());
    let c = controller.clone();
    ui.on_theme_selected(move |index| c.theme_selected(index));

    let weak = Rc::downgrade(&controller);
    controller
        .port_scan_timer
        .start(TimerMode::Repeated, PORT_SCAN_INTERVAL, move || {
            if let Some(controller) = weak.upgrade() {
                controller.scan_ports_if_idle();
            }
        });

    controller.scan_ports();
    let result = ui.run();
    controller.shutdown();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_keys() {
        assert_eq!(parse_captured_key("l"), Some('l'));
        assert_eq!(parse_captured_key("L"), Some('L'));
        assert_eq!(parse_captured_key(" "), Some(' '));
        assert_eq!(parse_captured_key("~"), Some('~'));
        assert_eq!(parse_captured_key(""), None);
        assert_eq!(parse_captured_key("\t"), None);
        assert_eq!(parse_captured_key("ab"), None);
        assert_eq!(parse_captured_key("ä"), None);
        // Slint reports special keys as private use characters.
        assert_eq!(parse_captured_key("\u{F704}"), None);
    }

    #[test]
    fn key_texts() {
        assert_eq!(key_text(' '), "Space");
        assert_eq!(key_text('l'), "l");
        assert_eq!(key_description('l'), "lowercase L");
        assert_eq!(key_description('I'), "uppercase I (with Shift)");
        assert_eq!(key_description('1'), "digit 1");
        assert_eq!(key_description(' '), "space bar");
        assert_eq!(key_description('#'), "symbol");
    }
}
