//! Theme Presets
//!
//! Built-in theme presets that ship with Goose Desktop.
//! These are embedded in the binary and served via API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ThemePreset {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub colors: ThemeColors,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ThemeColors {
    pub light: HashMap<String, String>,
    pub dark: HashMap<String, String>,
}

/// Get all built-in theme presets
pub fn get_all_presets() -> Vec<ThemePreset> {
    vec![
        goose_classic(),
        nord(),
        dracula(),
    ]
}

/// Get a specific preset by ID
pub fn get_preset(id: &str) -> Option<ThemePreset> {
    get_all_presets().into_iter().find(|p| p.id == id)
}

/// Goose Classic Theme - The default theme
fn goose_classic() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#ffffff".to_string());
    light.insert("color-background-secondary".to_string(), "#f4f6f7".to_string());
    light.insert("color-background-tertiary".to_string(), "#e3e6ea".to_string());
    light.insert("color-background-inverse".to_string(), "#000000".to_string());
    light.insert("color-background-danger".to_string(), "#f94b4b".to_string());
    light.insert("color-background-info".to_string(), "#5c98f9".to_string());
    light.insert("color-border-primary".to_string(), "#e3e6ea".to_string());
    light.insert("color-border-secondary".to_string(), "#e3e6ea".to_string());
    light.insert("color-border-danger".to_string(), "#f94b4b".to_string());
    light.insert("color-border-info".to_string(), "#5c98f9".to_string());
    light.insert("color-text-primary".to_string(), "#3f434b".to_string());
    light.insert("color-text-secondary".to_string(), "#878787".to_string());
    light.insert("color-text-inverse".to_string(), "#ffffff".to_string());
    light.insert("color-text-danger".to_string(), "#f94b4b".to_string());
    light.insert("color-text-success".to_string(), "#91cb80".to_string());
    light.insert("color-text-warning".to_string(), "#fbcd44".to_string());
    light.insert("color-text-info".to_string(), "#5c98f9".to_string());
    light.insert("color-ring-primary".to_string(), "#e3e6ea".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#22252a".to_string());
    dark.insert("color-background-secondary".to_string(), "#3f434b".to_string());
    dark.insert("color-background-tertiary".to_string(), "#474e57".to_string());
    dark.insert("color-background-inverse".to_string(), "#cbd1d6".to_string());
    dark.insert("color-background-danger".to_string(), "#ff6b6b".to_string());
    dark.insert("color-background-info".to_string(), "#7cacff".to_string());
    dark.insert("color-border-primary".to_string(), "#3f434b".to_string());
    dark.insert("color-border-secondary".to_string(), "#606c7a".to_string());
    dark.insert("color-border-danger".to_string(), "#ff6b6b".to_string());
    dark.insert("color-border-info".to_string(), "#7cacff".to_string());
    dark.insert("color-text-primary".to_string(), "#ffffff".to_string());
    dark.insert("color-text-secondary".to_string(), "#878787".to_string());
    dark.insert("color-text-inverse".to_string(), "#000000".to_string());
    dark.insert("color-text-danger".to_string(), "#ff6b6b".to_string());
    dark.insert("color-text-success".to_string(), "#a3d795".to_string());
    dark.insert("color-text-warning".to_string(), "#ffd966".to_string());
    dark.insert("color-text-info".to_string(), "#7cacff".to_string());
    dark.insert("color-ring-primary".to_string(), "#606c7a".to_string());

    ThemePreset {
        id: "goose-classic".to_string(),
        name: "Goose Classic".to_string(),
        author: "Block".to_string(),
        description: "The default Goose Desktop theme with clean, professional colors".to_string(),
        tags: vec!["light".to_string(), "dark".to_string(), "default".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
    }
}

/// Nord Theme - Arctic color palette
fn nord() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#eceff4".to_string());
    light.insert("color-background-secondary".to_string(), "#e5e9f0".to_string());
    light.insert("color-background-tertiary".to_string(), "#d8dee9".to_string());
    light.insert("color-background-inverse".to_string(), "#2e3440".to_string());
    light.insert("color-background-danger".to_string(), "#bf616a".to_string());
    light.insert("color-background-info".to_string(), "#5e81ac".to_string());
    light.insert("color-border-primary".to_string(), "#d8dee9".to_string());
    light.insert("color-border-secondary".to_string(), "#d8dee9".to_string());
    light.insert("color-border-danger".to_string(), "#bf616a".to_string());
    light.insert("color-border-info".to_string(), "#5e81ac".to_string());
    light.insert("color-text-primary".to_string(), "#2e3440".to_string());
    light.insert("color-text-secondary".to_string(), "#4c566a".to_string());
    light.insert("color-text-inverse".to_string(), "#eceff4".to_string());
    light.insert("color-text-danger".to_string(), "#bf616a".to_string());
    light.insert("color-text-success".to_string(), "#a3be8c".to_string());
    light.insert("color-text-warning".to_string(), "#ebcb8b".to_string());
    light.insert("color-text-info".to_string(), "#5e81ac".to_string());
    light.insert("color-ring-primary".to_string(), "#d8dee9".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#2e3440".to_string());
    dark.insert("color-background-secondary".to_string(), "#3b4252".to_string());
    dark.insert("color-background-tertiary".to_string(), "#434c5e".to_string());
    dark.insert("color-background-inverse".to_string(), "#eceff4".to_string());
    dark.insert("color-background-danger".to_string(), "#bf616a".to_string());
    dark.insert("color-background-info".to_string(), "#81a1c1".to_string());
    dark.insert("color-border-primary".to_string(), "#3b4252".to_string());
    dark.insert("color-border-secondary".to_string(), "#4c566a".to_string());
    dark.insert("color-border-danger".to_string(), "#bf616a".to_string());
    dark.insert("color-border-info".to_string(), "#81a1c1".to_string());
    dark.insert("color-text-primary".to_string(), "#eceff4".to_string());
    dark.insert("color-text-secondary".to_string(), "#d8dee9".to_string());
    dark.insert("color-text-inverse".to_string(), "#2e3440".to_string());
    dark.insert("color-text-danger".to_string(), "#bf616a".to_string());
    dark.insert("color-text-success".to_string(), "#a3be8c".to_string());
    dark.insert("color-text-warning".to_string(), "#ebcb8b".to_string());
    dark.insert("color-text-info".to_string(), "#88c0d0".to_string());
    dark.insert("color-ring-primary".to_string(), "#4c566a".to_string());

    ThemePreset {
        id: "nord".to_string(),
        name: "Nord".to_string(),
        author: "Arctic Ice Studio".to_string(),
        description: "An arctic, north-bluish color palette with clean and elegant design".to_string(),
        tags: vec!["dark".to_string(), "light".to_string(), "cool".to_string(), "minimal".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
    }
}

/// Dracula Theme - Vibrant dark theme
fn dracula() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#f8f8f2".to_string());
    light.insert("color-background-secondary".to_string(), "#f0f0eb".to_string());
    light.insert("color-background-tertiary".to_string(), "#e6e6e1".to_string());
    light.insert("color-background-inverse".to_string(), "#282a36".to_string());
    light.insert("color-background-danger".to_string(), "#ff5555".to_string());
    light.insert("color-background-info".to_string(), "#8be9fd".to_string());
    light.insert("color-border-primary".to_string(), "#e6e6e1".to_string());
    light.insert("color-border-secondary".to_string(), "#e6e6e1".to_string());
    light.insert("color-border-danger".to_string(), "#ff5555".to_string());
    light.insert("color-border-info".to_string(), "#8be9fd".to_string());
    light.insert("color-text-primary".to_string(), "#282a36".to_string());
    light.insert("color-text-secondary".to_string(), "#6272a4".to_string());
    light.insert("color-text-inverse".to_string(), "#f8f8f2".to_string());
    light.insert("color-text-danger".to_string(), "#ff5555".to_string());
    light.insert("color-text-success".to_string(), "#50fa7b".to_string());
    light.insert("color-text-warning".to_string(), "#f1fa8c".to_string());
    light.insert("color-text-info".to_string(), "#8be9fd".to_string());
    light.insert("color-ring-primary".to_string(), "#e6e6e1".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#282a36".to_string());
    dark.insert("color-background-secondary".to_string(), "#343746".to_string());
    dark.insert("color-background-tertiary".to_string(), "#44475a".to_string());
    dark.insert("color-background-inverse".to_string(), "#f8f8f2".to_string());
    dark.insert("color-background-danger".to_string(), "#ff5555".to_string());
    dark.insert("color-background-info".to_string(), "#8be9fd".to_string());
    dark.insert("color-border-primary".to_string(), "#44475a".to_string());
    dark.insert("color-border-secondary".to_string(), "#6272a4".to_string());
    dark.insert("color-border-danger".to_string(), "#ff5555".to_string());
    dark.insert("color-border-info".to_string(), "#8be9fd".to_string());
    dark.insert("color-text-primary".to_string(), "#f8f8f2".to_string());
    dark.insert("color-text-secondary".to_string(), "#f8f8f2".to_string());
    dark.insert("color-text-inverse".to_string(), "#282a36".to_string());
    dark.insert("color-text-danger".to_string(), "#ff5555".to_string());
    dark.insert("color-text-success".to_string(), "#50fa7b".to_string());
    dark.insert("color-text-warning".to_string(), "#f1fa8c".to_string());
    dark.insert("color-text-info".to_string(), "#8be9fd".to_string());
    dark.insert("color-ring-primary".to_string(), "#6272a4".to_string());

    ThemePreset {
        id: "dracula".to_string(),
        name: "Dracula".to_string(),
        author: "Dracula Theme".to_string(),
        description: "A dark theme with vibrant, high-contrast colors perfect for long coding sessions".to_string(),
        tags: vec!["dark".to_string(), "colorful".to_string(), "high-contrast".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
    }
}
