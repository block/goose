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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_expected_values() {
        let settings = Settings::default();
        assert!(settings.show_menu_bar_icon);
        assert!(settings.show_dock_icon);
        assert!(!settings.enable_wakelock);
        assert!(settings.spellcheck_enabled);
        assert!(settings.external_goosed.is_none());
        assert!(settings.global_shortcut.is_none());
        assert!(settings.keyboard_shortcuts.is_some());
    }

    #[test]
    fn default_keyboard_shortcuts() {
        let shortcuts = KeyboardShortcuts::default();
        assert_eq!(shortcuts.focus_window, Some("CmdOrCtrl+Alt+G".to_string()));
        assert_eq!(shortcuts.quick_launcher, Some("CmdOrCtrl+Alt+Shift+G".to_string()));
        assert_eq!(shortcuts.new_chat, Some("CmdOrCtrl+T".to_string()));
        assert_eq!(shortcuts.new_chat_window, Some("CmdOrCtrl+N".to_string()));
        assert_eq!(shortcuts.open_directory, Some("CmdOrCtrl+O".to_string()));
        assert_eq!(shortcuts.settings, Some("CmdOrCtrl+,".to_string()));
    }

    #[test]
    fn settings_serialize_deserialize_roundtrip() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).expect("serialize");
        let deserialized: Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(settings.show_menu_bar_icon, deserialized.show_menu_bar_icon);
        assert_eq!(settings.show_dock_icon, deserialized.show_dock_icon);
        assert_eq!(settings.enable_wakelock, deserialized.enable_wakelock);
        assert_eq!(settings.spellcheck_enabled, deserialized.spellcheck_enabled);
    }

    #[test]
    fn settings_deserialize_with_missing_fields_uses_defaults() {
        let json = r#"{}"#;
        let settings: Settings = serde_json::from_str(json).expect("deserialize");
        assert!(settings.show_menu_bar_icon);
        assert!(settings.show_dock_icon);
        assert!(!settings.enable_wakelock);
        assert!(settings.spellcheck_enabled);
    }

    #[test]
    fn settings_camel_case_serialization() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).expect("serialize");
        assert!(json.contains("showMenuBarIcon"));
        assert!(json.contains("showDockIcon"));
        assert!(json.contains("enableWakelock"));
        assert!(json.contains("spellcheckEnabled"));
    }

    #[test]
    fn external_goosed_config_roundtrip() {
        let config = ExternalGoosedConfig {
            enabled: true,
            url: "http://localhost:3000".to_string(),
            secret: "my-secret".to_string(),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: ExternalGoosedConfig = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.enabled);
        assert_eq!(deserialized.url, "http://localhost:3000");
        assert_eq!(deserialized.secret, "my-secret");
    }

    #[test]
    fn settings_save_and_load_from_temp_dir() {
        use std::env;
        let tmp = env::temp_dir().join("goose-test-settings");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let settings_file = tmp.join("settings.json");
        let settings = Settings {
            show_menu_bar_icon: false,
            show_dock_icon: false,
            enable_wakelock: true,
            spellcheck_enabled: false,
            external_goosed: None,
            global_shortcut: Some("CmdOrCtrl+G".to_string()),
            keyboard_shortcuts: None,
        };
        let content = serde_json::to_string_pretty(&settings).expect("serialize");
        fs::write(&settings_file, &content).expect("write");

        let loaded_content = fs::read_to_string(&settings_file).expect("read");
        let loaded: Settings = serde_json::from_str(&loaded_content).expect("deserialize");
        assert!(!loaded.show_menu_bar_icon);
        assert!(!loaded.show_dock_icon);
        assert!(loaded.enable_wakelock);
        assert!(!loaded.spellcheck_enabled);
        assert_eq!(loaded.global_shortcut, Some("CmdOrCtrl+G".to_string()));

        let _ = fs::remove_dir_all(&tmp);
    }
}
