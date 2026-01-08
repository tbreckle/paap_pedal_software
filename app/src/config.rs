use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme_mode: ThemeMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        confy::load("paap-configurator", None).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), confy::ConfyError> {
        confy::store("paap-configurator", None, self)
    }
}
