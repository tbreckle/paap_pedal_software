//! Persistent app settings.

use serde::{Deserialize, Serialize};

const APP_NAME: &str = "paap-configurator";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemeMode {
    /// Index in the theme ComboBox (see `theme` in app.slint).
    pub fn index(self) -> i32 {
        match self {
            ThemeMode::System => 0,
            ThemeMode::Dark => 1,
            ThemeMode::Light => 2,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            1 => ThemeMode::Dark,
            2 => ThemeMode::Light,
            _ => ThemeMode::System,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme_mode: ThemeMode,
}

impl AppConfig {
    pub fn load() -> Self {
        confy::load(APP_NAME, None).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store(APP_NAME, None, self)
    }
}
