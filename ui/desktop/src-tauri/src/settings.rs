use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalGoosedConfig {
    pub enabled: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardShortcuts {
    pub focus_window: Option<String>,
    pub quick_launcher: Option<String>,
    pub new_chat: Option<String>,
    pub new_chat_window: Option<String>,
    pub open_directory: Option<String>,
    pub settings: Option<String>,
    pub find: Option<String>,
    pub find_next: Option<String>,
    pub find_previous: Option<String>,
    pub always_on_top: Option<String>,
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        Self {
            focus_window: Some("CmdOrCtrl+Alt+G".to_string()),
            quick_launcher: Some("CmdOrCtrl+Alt+Shift+G".to_string()),
            new_chat: Some("CmdOrCtrl+T".to_string()),
            new_chat_window: Some("CmdOrCtrl+N".to_string()),
            open_directory: Some("CmdOrCtrl+O".to_string()),
            settings: Some("CmdOrCtrl+,".to_string()),
            find: Some("CmdOrCtrl+F".to_string()),
            find_next: Some("CmdOrCtrl+G".to_string()),
            find_previous: Some("CmdOrCtrl+Shift+G".to_string()),
            always_on_top: Some("CmdOrCtrl+Shift+T".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_true")]
    pub show_menu_bar_icon: bool,
    #[serde(default = "default_true")]
    pub show_dock_icon: bool,
    #[serde(default)]
    pub enable_wakelock: bool,
    #[serde(default = "default_true")]
    pub spellcheck_enabled: bool,
    #[serde(default)]
    pub external_goosed: Option<ExternalGoosedConfig>,
    #[serde(default)]
    pub global_shortcut: Option<String>,
    #[serde(default)]
    pub keyboard_shortcuts: Option<KeyboardShortcuts>,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_menu_bar_icon: true,
            show_dock_icon: true,
            enable_wakelock: false,
            spellcheck_enabled: true,
            external_goosed: None,
            global_shortcut: None,
            keyboard_shortcuts: Some(KeyboardShortcuts::default()),
        }
    }
}

pub struct SettingsState(pub Mutex<Settings>);

impl Default for SettingsState {
    fn default() -> Self {
        Self(Mutex::new(Settings::load_or_default()))
    }
}

impl Settings {
    fn settings_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
        config_dir.join("Goose").join("settings.json")
    }

    pub fn load_or_default() -> Self {
        let path = Self::settings_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => return settings,
                    Err(e) => log::warn!("Failed to parse settings: {}", e),
                },
                Err(e) => log::warn!("Failed to read settings file: {}", e),
            }
        }
        Settings::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let content =
            serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {}", e))?;
        fs::write(&path, content).map_err(|e| format!("Failed to write settings: {}", e))?;
        Ok(())
    }
}
