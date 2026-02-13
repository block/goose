//! Theme Presets
//!
//! Built-in theme presets that ship with Goose Desktop.
//! These are embedded in the binary and served via API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = ThemePreset)]
pub struct ThemePreset {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub colors: ThemeColors,
    pub version: String,
    #[serde(default)]
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = ThemeColors)]
pub struct ThemeColors {
    pub light: HashMap<String, String>,
    pub dark: HashMap<String, String>,
}

/// Get all built-in theme presets
pub fn get_all_presets() -> Vec<ThemePreset> {
    vec![
        goose_classic(),
        high_contrast(),
        nord(),
        dracula(),
        solarized(),
        monokai(),
        github(),
        gruvbox(),
        tokyo_night(),
        one_dark(),
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
        is_custom: false,
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
        is_custom: false,
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
        is_custom: false,
    }
}

/// High Contrast Theme - Maximum contrast for accessibility
fn high_contrast() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#ffffff".to_string());
    light.insert("color-background-secondary".to_string(), "#f0f0f0".to_string());
    light.insert("color-background-tertiary".to_string(), "#e0e0e0".to_string());
    light.insert("color-background-inverse".to_string(), "#000000".to_string());
    light.insert("color-background-danger".to_string(), "#d32f2f".to_string());
    light.insert("color-background-info".to_string(), "#1976d2".to_string());
    light.insert("color-border-primary".to_string(), "#000000".to_string());
    light.insert("color-border-secondary".to_string(), "#000000".to_string());
    light.insert("color-border-danger".to_string(), "#d32f2f".to_string());
    light.insert("color-border-info".to_string(), "#1976d2".to_string());
    light.insert("color-text-primary".to_string(), "#000000".to_string());
    light.insert("color-text-secondary".to_string(), "#424242".to_string());
    light.insert("color-text-inverse".to_string(), "#ffffff".to_string());
    light.insert("color-text-danger".to_string(), "#d32f2f".to_string());
    light.insert("color-text-success".to_string(), "#2e7d32".to_string());
    light.insert("color-text-warning".to_string(), "#f57c00".to_string());
    light.insert("color-text-info".to_string(), "#1976d2".to_string());
    light.insert("color-ring-primary".to_string(), "#000000".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#000000".to_string());
    dark.insert("color-background-secondary".to_string(), "#1a1a1a".to_string());
    dark.insert("color-background-tertiary".to_string(), "#2a2a2a".to_string());
    dark.insert("color-background-inverse".to_string(), "#ffffff".to_string());
    dark.insert("color-background-danger".to_string(), "#ff5252".to_string());
    dark.insert("color-background-info".to_string(), "#448aff".to_string());
    dark.insert("color-border-primary".to_string(), "#ffffff".to_string());
    dark.insert("color-border-secondary".to_string(), "#ffffff".to_string());
    dark.insert("color-border-danger".to_string(), "#ff5252".to_string());
    dark.insert("color-border-info".to_string(), "#448aff".to_string());
    dark.insert("color-text-primary".to_string(), "#ffffff".to_string());
    dark.insert("color-text-secondary".to_string(), "#e0e0e0".to_string());
    dark.insert("color-text-inverse".to_string(), "#000000".to_string());
    dark.insert("color-text-danger".to_string(), "#ff5252".to_string());
    dark.insert("color-text-success".to_string(), "#69f0ae".to_string());
    dark.insert("color-text-warning".to_string(), "#ffab40".to_string());
    dark.insert("color-text-info".to_string(), "#448aff".to_string());
    dark.insert("color-ring-primary".to_string(), "#ffffff".to_string());

    ThemePreset {
        id: "high-contrast".to_string(),
        name: "High Contrast".to_string(),
        author: "Block".to_string(),
        description: "Maximum contrast theme optimized for accessibility and readability".to_string(),
        tags: vec!["light".to_string(), "dark".to_string(), "high-contrast".to_string(), "accessible".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// Solarized Theme - Precision colors for machines and people
fn solarized() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#fdf6e3".to_string());
    light.insert("color-background-secondary".to_string(), "#eee8d5".to_string());
    light.insert("color-background-tertiary".to_string(), "#e3dcc3".to_string());
    light.insert("color-background-inverse".to_string(), "#002b36".to_string());
    light.insert("color-background-danger".to_string(), "#dc322f".to_string());
    light.insert("color-background-info".to_string(), "#268bd2".to_string());
    light.insert("color-border-primary".to_string(), "#e3dcc3".to_string());
    light.insert("color-border-secondary".to_string(), "#d3cdb3".to_string());
    light.insert("color-border-danger".to_string(), "#dc322f".to_string());
    light.insert("color-border-info".to_string(), "#268bd2".to_string());
    light.insert("color-text-primary".to_string(), "#657b83".to_string());
    light.insert("color-text-secondary".to_string(), "#93a1a1".to_string());
    light.insert("color-text-inverse".to_string(), "#fdf6e3".to_string());
    light.insert("color-text-danger".to_string(), "#dc322f".to_string());
    light.insert("color-text-success".to_string(), "#859900".to_string());
    light.insert("color-text-warning".to_string(), "#b58900".to_string());
    light.insert("color-text-info".to_string(), "#268bd2".to_string());
    light.insert("color-ring-primary".to_string(), "#d3cdb3".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#002b36".to_string());
    dark.insert("color-background-secondary".to_string(), "#073642".to_string());
    dark.insert("color-background-tertiary".to_string(), "#0d4654".to_string());
    dark.insert("color-background-inverse".to_string(), "#fdf6e3".to_string());
    dark.insert("color-background-danger".to_string(), "#dc322f".to_string());
    dark.insert("color-background-info".to_string(), "#268bd2".to_string());
    dark.insert("color-border-primary".to_string(), "#073642".to_string());
    dark.insert("color-border-secondary".to_string(), "#586e75".to_string());
    dark.insert("color-border-danger".to_string(), "#dc322f".to_string());
    dark.insert("color-border-info".to_string(), "#268bd2".to_string());
    dark.insert("color-text-primary".to_string(), "#839496".to_string());
    dark.insert("color-text-secondary".to_string(), "#657b83".to_string());
    dark.insert("color-text-inverse".to_string(), "#002b36".to_string());
    dark.insert("color-text-danger".to_string(), "#dc322f".to_string());
    dark.insert("color-text-success".to_string(), "#859900".to_string());
    dark.insert("color-text-warning".to_string(), "#b58900".to_string());
    dark.insert("color-text-info".to_string(), "#268bd2".to_string());
    dark.insert("color-ring-primary".to_string(), "#586e75".to_string());

    ThemePreset {
        id: "solarized".to_string(),
        name: "Solarized".to_string(),
        author: "Ethan Schoonover".to_string(),
        description: "Precision colors for machines and people - designed for optimal readability".to_string(),
        tags: vec!["light".to_string(), "dark".to_string(), "minimal".to_string(), "retro".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// Monokai Theme - Classic developer theme from Sublime Text
fn monokai() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#fafafa".to_string());
    light.insert("color-background-secondary".to_string(), "#f5f5f5".to_string());
    light.insert("color-background-tertiary".to_string(), "#e8e8e8".to_string());
    light.insert("color-background-inverse".to_string(), "#272822".to_string());
    light.insert("color-background-danger".to_string(), "#f92672".to_string());
    light.insert("color-background-info".to_string(), "#66d9ef".to_string());
    light.insert("color-border-primary".to_string(), "#e8e8e8".to_string());
    light.insert("color-border-secondary".to_string(), "#d8d8d8".to_string());
    light.insert("color-border-danger".to_string(), "#f92672".to_string());
    light.insert("color-border-info".to_string(), "#66d9ef".to_string());
    light.insert("color-text-primary".to_string(), "#272822".to_string());
    light.insert("color-text-secondary".to_string(), "#75715e".to_string());
    light.insert("color-text-inverse".to_string(), "#f8f8f2".to_string());
    light.insert("color-text-danger".to_string(), "#f92672".to_string());
    light.insert("color-text-success".to_string(), "#a6e22e".to_string());
    light.insert("color-text-warning".to_string(), "#e6db74".to_string());
    light.insert("color-text-info".to_string(), "#66d9ef".to_string());
    light.insert("color-ring-primary".to_string(), "#d8d8d8".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#272822".to_string());
    dark.insert("color-background-secondary".to_string(), "#3e3d32".to_string());
    dark.insert("color-background-tertiary".to_string(), "#49483e".to_string());
    dark.insert("color-background-inverse".to_string(), "#f8f8f2".to_string());
    dark.insert("color-background-danger".to_string(), "#f92672".to_string());
    dark.insert("color-background-info".to_string(), "#66d9ef".to_string());
    dark.insert("color-border-primary".to_string(), "#3e3d32".to_string());
    dark.insert("color-border-secondary".to_string(), "#75715e".to_string());
    dark.insert("color-border-danger".to_string(), "#f92672".to_string());
    dark.insert("color-border-info".to_string(), "#66d9ef".to_string());
    dark.insert("color-text-primary".to_string(), "#f8f8f2".to_string());
    dark.insert("color-text-secondary".to_string(), "#75715e".to_string());
    dark.insert("color-text-inverse".to_string(), "#272822".to_string());
    dark.insert("color-text-danger".to_string(), "#f92672".to_string());
    dark.insert("color-text-success".to_string(), "#a6e22e".to_string());
    dark.insert("color-text-warning".to_string(), "#e6db74".to_string());
    dark.insert("color-text-info".to_string(), "#66d9ef".to_string());
    dark.insert("color-ring-primary".to_string(), "#75715e".to_string());

    ThemePreset {
        id: "monokai".to_string(),
        name: "Monokai".to_string(),
        author: "Wimer Hazenberg".to_string(),
        description: "Classic developer theme from Sublime Text with vibrant syntax colors".to_string(),
        tags: vec!["dark".to_string(), "colorful".to_string(), "retro".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// GitHub Theme - Clean and familiar GitHub colors
fn github() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#ffffff".to_string());
    light.insert("color-background-secondary".to_string(), "#f6f8fa".to_string());
    light.insert("color-background-tertiary".to_string(), "#eaeef2".to_string());
    light.insert("color-background-inverse".to_string(), "#24292f".to_string());
    light.insert("color-background-danger".to_string(), "#d1242f".to_string());
    light.insert("color-background-info".to_string(), "#0969da".to_string());
    light.insert("color-border-primary".to_string(), "#d0d7de".to_string());
    light.insert("color-border-secondary".to_string(), "#d0d7de".to_string());
    light.insert("color-border-danger".to_string(), "#d1242f".to_string());
    light.insert("color-border-info".to_string(), "#0969da".to_string());
    light.insert("color-text-primary".to_string(), "#24292f".to_string());
    light.insert("color-text-secondary".to_string(), "#57606a".to_string());
    light.insert("color-text-inverse".to_string(), "#ffffff".to_string());
    light.insert("color-text-danger".to_string(), "#d1242f".to_string());
    light.insert("color-text-success".to_string(), "#1a7f37".to_string());
    light.insert("color-text-warning".to_string(), "#9a6700".to_string());
    light.insert("color-text-info".to_string(), "#0969da".to_string());
    light.insert("color-ring-primary".to_string(), "#d0d7de".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#0d1117".to_string());
    dark.insert("color-background-secondary".to_string(), "#161b22".to_string());
    dark.insert("color-background-tertiary".to_string(), "#21262d".to_string());
    dark.insert("color-background-inverse".to_string(), "#f0f6fc".to_string());
    dark.insert("color-background-danger".to_string(), "#da3633".to_string());
    dark.insert("color-background-info".to_string(), "#58a6ff".to_string());
    dark.insert("color-border-primary".to_string(), "#30363d".to_string());
    dark.insert("color-border-secondary".to_string(), "#484f58".to_string());
    dark.insert("color-border-danger".to_string(), "#da3633".to_string());
    dark.insert("color-border-info".to_string(), "#58a6ff".to_string());
    dark.insert("color-text-primary".to_string(), "#e6edf3".to_string());
    dark.insert("color-text-secondary".to_string(), "#7d8590".to_string());
    dark.insert("color-text-inverse".to_string(), "#0d1117".to_string());
    dark.insert("color-text-danger".to_string(), "#ff7b72".to_string());
    dark.insert("color-text-success".to_string(), "#3fb950".to_string());
    dark.insert("color-text-warning".to_string(), "#d29922".to_string());
    dark.insert("color-text-info".to_string(), "#79c0ff".to_string());
    dark.insert("color-ring-primary".to_string(), "#484f58".to_string());

    ThemePreset {
        id: "github".to_string(),
        name: "GitHub".to_string(),
        author: "GitHub".to_string(),
        description: "Clean, familiar colors from GitHub - professional and easy on the eyes".to_string(),
        tags: vec!["light".to_string(), "dark".to_string(), "minimal".to_string(), "modern".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// Gruvbox Theme - Warm, retro-inspired color palette
fn gruvbox() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#fbf1c7".to_string());
    light.insert("color-background-secondary".to_string(), "#f2e5bc".to_string());
    light.insert("color-background-tertiary".to_string(), "#ebdbb2".to_string());
    light.insert("color-background-inverse".to_string(), "#282828".to_string());
    light.insert("color-background-danger".to_string(), "#cc241d".to_string());
    light.insert("color-background-info".to_string(), "#458588".to_string());
    light.insert("color-border-primary".to_string(), "#ebdbb2".to_string());
    light.insert("color-border-secondary".to_string(), "#d5c4a1".to_string());
    light.insert("color-border-danger".to_string(), "#cc241d".to_string());
    light.insert("color-border-info".to_string(), "#458588".to_string());
    light.insert("color-text-primary".to_string(), "#3c3836".to_string());
    light.insert("color-text-secondary".to_string(), "#7c6f64".to_string());
    light.insert("color-text-inverse".to_string(), "#fbf1c7".to_string());
    light.insert("color-text-danger".to_string(), "#cc241d".to_string());
    light.insert("color-text-success".to_string(), "#98971a".to_string());
    light.insert("color-text-warning".to_string(), "#d79921".to_string());
    light.insert("color-text-info".to_string(), "#458588".to_string());
    light.insert("color-ring-primary".to_string(), "#d5c4a1".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#282828".to_string());
    dark.insert("color-background-secondary".to_string(), "#3c3836".to_string());
    dark.insert("color-background-tertiary".to_string(), "#504945".to_string());
    dark.insert("color-background-inverse".to_string(), "#fbf1c7".to_string());
    dark.insert("color-background-danger".to_string(), "#fb4934".to_string());
    dark.insert("color-background-info".to_string(), "#83a598".to_string());
    dark.insert("color-border-primary".to_string(), "#3c3836".to_string());
    dark.insert("color-border-secondary".to_string(), "#665c54".to_string());
    dark.insert("color-border-danger".to_string(), "#fb4934".to_string());
    dark.insert("color-border-info".to_string(), "#83a598".to_string());
    dark.insert("color-text-primary".to_string(), "#ebdbb2".to_string());
    dark.insert("color-text-secondary".to_string(), "#a89984".to_string());
    dark.insert("color-text-inverse".to_string(), "#282828".to_string());
    dark.insert("color-text-danger".to_string(), "#fb4934".to_string());
    dark.insert("color-text-success".to_string(), "#b8bb26".to_string());
    dark.insert("color-text-warning".to_string(), "#fabd2f".to_string());
    dark.insert("color-text-info".to_string(), "#83a598".to_string());
    dark.insert("color-ring-primary".to_string(), "#665c54".to_string());

    ThemePreset {
        id: "gruvbox".to_string(),
        name: "Gruvbox".to_string(),
        author: "Pavel Pertsev".to_string(),
        description: "Warm, retro groove colors designed for long coding sessions".to_string(),
        tags: vec!["dark".to_string(), "light".to_string(), "warm".to_string(), "retro".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// Tokyo Night Theme - Modern, vibrant night theme
fn tokyo_night() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#d5d6db".to_string());
    light.insert("color-background-secondary".to_string(), "#cbccd1".to_string());
    light.insert("color-background-tertiary".to_string(), "#c4c8da".to_string());
    light.insert("color-background-inverse".to_string(), "#1a1b26".to_string());
    light.insert("color-background-danger".to_string(), "#f52a65".to_string());
    light.insert("color-background-info".to_string(), "#2ac3de".to_string());
    light.insert("color-border-primary".to_string(), "#c4c8da".to_string());
    light.insert("color-border-secondary".to_string(), "#a8aecb".to_string());
    light.insert("color-border-danger".to_string(), "#f52a65".to_string());
    light.insert("color-border-info".to_string(), "#2ac3de".to_string());
    light.insert("color-text-primary".to_string(), "#343b58".to_string());
    light.insert("color-text-secondary".to_string(), "#565a6e".to_string());
    light.insert("color-text-inverse".to_string(), "#d5d6db".to_string());
    light.insert("color-text-danger".to_string(), "#f52a65".to_string());
    light.insert("color-text-success".to_string(), "#33635c".to_string());
    light.insert("color-text-warning".to_string(), "#8c6c3e".to_string());
    light.insert("color-text-info".to_string(), "#2e7de9".to_string());
    light.insert("color-ring-primary".to_string(), "#a8aecb".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#1a1b26".to_string());
    dark.insert("color-background-secondary".to_string(), "#24283b".to_string());
    dark.insert("color-background-tertiary".to_string(), "#414868".to_string());
    dark.insert("color-background-inverse".to_string(), "#c0caf5".to_string());
    dark.insert("color-background-danger".to_string(), "#f7768e".to_string());
    dark.insert("color-background-info".to_string(), "#7dcfff".to_string());
    dark.insert("color-border-primary".to_string(), "#24283b".to_string());
    dark.insert("color-border-secondary".to_string(), "#414868".to_string());
    dark.insert("color-border-danger".to_string(), "#f7768e".to_string());
    dark.insert("color-border-info".to_string(), "#7dcfff".to_string());
    dark.insert("color-text-primary".to_string(), "#c0caf5".to_string());
    dark.insert("color-text-secondary".to_string(), "#565f89".to_string());
    dark.insert("color-text-inverse".to_string(), "#1a1b26".to_string());
    dark.insert("color-text-danger".to_string(), "#f7768e".to_string());
    dark.insert("color-text-success".to_string(), "#9ece6a".to_string());
    dark.insert("color-text-warning".to_string(), "#e0af68".to_string());
    dark.insert("color-text-info".to_string(), "#7aa2f7".to_string());
    dark.insert("color-ring-primary".to_string(), "#414868".to_string());

    ThemePreset {
        id: "tokyo-night".to_string(),
        name: "Tokyo Night".to_string(),
        author: "Folke Lemaitre".to_string(),
        description: "A clean, dark theme inspired by the lights of Tokyo at night".to_string(),
        tags: vec!["dark".to_string(), "modern".to_string(), "colorful".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

/// One Dark Theme - Popular dark theme from Atom editor
fn one_dark() -> ThemePreset {
    let mut light = HashMap::new();
    light.insert("color-background-primary".to_string(), "#fafafa".to_string());
    light.insert("color-background-secondary".to_string(), "#f0f0f0".to_string());
    light.insert("color-background-tertiary".to_string(), "#e5e5e5".to_string());
    light.insert("color-background-inverse".to_string(), "#282c34".to_string());
    light.insert("color-background-danger".to_string(), "#e45649".to_string());
    light.insert("color-background-info".to_string(), "#4078f2".to_string());
    light.insert("color-border-primary".to_string(), "#e5e5e5".to_string());
    light.insert("color-border-secondary".to_string(), "#d0d0d0".to_string());
    light.insert("color-border-danger".to_string(), "#e45649".to_string());
    light.insert("color-border-info".to_string(), "#4078f2".to_string());
    light.insert("color-text-primary".to_string(), "#383a42".to_string());
    light.insert("color-text-secondary".to_string(), "#a0a1a7".to_string());
    light.insert("color-text-inverse".to_string(), "#fafafa".to_string());
    light.insert("color-text-danger".to_string(), "#e45649".to_string());
    light.insert("color-text-success".to_string(), "#50a14f".to_string());
    light.insert("color-text-warning".to_string(), "#c18401".to_string());
    light.insert("color-text-info".to_string(), "#4078f2".to_string());
    light.insert("color-ring-primary".to_string(), "#d0d0d0".to_string());

    let mut dark = HashMap::new();
    dark.insert("color-background-primary".to_string(), "#282c34".to_string());
    dark.insert("color-background-secondary".to_string(), "#21252b".to_string());
    dark.insert("color-background-tertiary".to_string(), "#2c313c".to_string());
    dark.insert("color-background-inverse".to_string(), "#abb2bf".to_string());
    dark.insert("color-background-danger".to_string(), "#e06c75".to_string());
    dark.insert("color-background-info".to_string(), "#61afef".to_string());
    dark.insert("color-border-primary".to_string(), "#21252b".to_string());
    dark.insert("color-border-secondary".to_string(), "#3e4451".to_string());
    dark.insert("color-border-danger".to_string(), "#e06c75".to_string());
    dark.insert("color-border-info".to_string(), "#61afef".to_string());
    dark.insert("color-text-primary".to_string(), "#abb2bf".to_string());
    dark.insert("color-text-secondary".to_string(), "#5c6370".to_string());
    dark.insert("color-text-inverse".to_string(), "#282c34".to_string());
    dark.insert("color-text-danger".to_string(), "#e06c75".to_string());
    dark.insert("color-text-success".to_string(), "#98c379".to_string());
    dark.insert("color-text-warning".to_string(), "#e5c07b".to_string());
    dark.insert("color-text-info".to_string(), "#61afef".to_string());
    dark.insert("color-ring-primary".to_string(), "#3e4451".to_string());

    ThemePreset {
        id: "one-dark".to_string(),
        name: "One Dark".to_string(),
        author: "Atom".to_string(),
        description: "Popular dark theme from Atom editor with balanced colors".to_string(),
        tags: vec!["dark".to_string(), "modern".to_string(), "minimal".to_string()],
        colors: ThemeColors { light, dark },
        version: "1.0.0".to_string(),
        is_custom: false,
    }
}

// ============================================================================
// Custom Theme Management
// ============================================================================

/// Get the path to the saved themes directory
fn get_saved_themes_dir() -> Result<PathBuf, std::io::Error> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found"))?;
    let themes_dir = config_dir.join("goose").join("data").join("saved_themes");
    
    // Create directory if it doesn't exist
    if !themes_dir.exists() {
        fs::create_dir_all(&themes_dir)?;
    }
    
    Ok(themes_dir)
}

/// Load all custom themes from the saved_themes directory
pub fn load_custom_themes() -> Vec<ThemePreset> {
    let themes_dir = match get_saved_themes_dir() {
        Ok(dir) => dir,
        Err(_) => return vec![],
    };
    
    let mut themes = Vec::new();
    
    if let Ok(entries) = fs::read_dir(themes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(mut theme) = serde_json::from_str::<ThemePreset>(&content) {
                        theme.is_custom = true;
                        themes.push(theme);
                    }
                }
            }
        }
    }
    
    themes
}

/// Get all presets including both built-in and custom themes
pub fn get_all_presets_with_custom() -> Vec<ThemePreset> {
    let mut presets = get_all_presets();
    let custom_themes = load_custom_themes();
    presets.extend(custom_themes);
    presets
}

/// Save a custom theme
pub fn save_custom_theme(theme: ThemePreset) -> Result<(), std::io::Error> {
    let themes_dir = get_saved_themes_dir()?;
    let file_path = themes_dir.join(format!("{}.json", theme.id));
    
    let json = serde_json::to_string_pretty(&theme)?;
    fs::write(file_path, json)?;
    
    Ok(())
}

/// Delete a custom theme by ID
pub fn delete_custom_theme(id: &str) -> Result<(), std::io::Error> {
    let themes_dir = get_saved_themes_dir()?;
    let file_path = themes_dir.join(format!("{}.json", id));
    
    if file_path.exists() {
        fs::remove_file(file_path)?;
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Theme not found",
        ))
    }
}
