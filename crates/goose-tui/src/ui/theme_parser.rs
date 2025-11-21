use serde::Deserialize;
use ratatui::style::Color;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VsCodeTheme {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub theme_type: Option<String>, // "dark" or "light"
    pub colors: HashMap<String, String>,
    // We can ignore tokenColors for now as we aren't doing full syntax highlighting yet
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TuiTheme {
    pub background: Color,
    pub foreground: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub cursor: Color,
    pub border: Color,
    pub border_active: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub info: Color,
    // Thinking/Spinner color
    pub thinking: Color,
}

impl Default for TuiTheme {
    fn default() -> Self {
        // Default Dark Theme (similar to existing Goose dark)
        Self {
            background: Color::Rgb(20, 20, 20),       // Very dark gray
            foreground: Color::Rgb(220, 220, 220),    // Off-white
            selection_bg: Color::Rgb(60, 60, 60),     // Lighter gray
            selection_fg: Color::White,
            cursor: Color::Cyan,
            border: Color::Rgb(80, 80, 80),
            border_active: Color::Cyan,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            info: Color::Blue,
            thinking: Color::Magenta,
        }
    }
}

impl TuiTheme {
    pub fn from_vscode(vscode: VsCodeTheme) -> Self {
        let colors = vscode.colors;
        let get_color = |key: &str, default: Color| -> Color {
            colors.get(key)
                .and_then(|hex| parse_hex_color(hex))
                .unwrap_or(default)
        };

        let bg = get_color("editor.background", Color::Rgb(30, 30, 30));
        let fg = get_color("editor.foreground", Color::White);

        Self {
            background: bg,
            foreground: fg,
            selection_bg: get_color("list.activeSelectionBackground", Color::Rgb(60, 60, 60)),
            selection_fg: get_color("list.activeSelectionForeground", Color::White),
            cursor: get_color("editorCursor.foreground", Color::Cyan),
            border: get_color("panel.border", Color::Rgb(80, 80, 80)),
            border_active: get_color("focusBorder", Color::Cyan),
            error: get_color("editorError.foreground", Color::Red),
            success: Color::Green, // VS Code themes rarely define a generic "success" color
            warning: get_color("editorWarning.foreground", Color::Yellow),
            info: get_color("editorInfo.foreground", Color::Blue),
            thinking: get_color("activityBar.foreground", Color::Magenta), // Creative choice
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    // Handle #RRGGBB and #RRGGBBAA
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    
    Some(Color::Rgb(r, g, b))
}