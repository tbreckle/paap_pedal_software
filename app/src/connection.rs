use serialport::SerialPort;

pub enum ConnectionState {
    Disconnected,
    Connected {
        port: Box<dyn SerialPort>,
        port_name: String,
        current_key: char,
        version: String,
    },
}

impl ConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected { .. })
    }

    pub fn port_name(&self) -> Option<&str> {
        match self {
            ConnectionState::Connected { port_name, .. } => Some(port_name),
            ConnectionState::Disconnected => None,
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            ConnectionState::Connected { version, .. } => Some(version),
            ConnectionState::Disconnected => None,
        }
    }

    pub fn current_key(&self) -> Option<char> {
        match self {
            ConnectionState::Connected { current_key, .. } => Some(*current_key),
            ConnectionState::Disconnected => None,
        }
    }
}
