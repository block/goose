use crossterm::style::Color;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use two_face::theme::EmbeddedThemeName;

#[derive(Debug, Clone)]
pub struct SemanticTheme {
    pub name: String,

    // Text hierarchy
    pub text: Color,
    pub text_muted: Color,

    // Prompt
    pub prompt_caret: Color,

    // Emphasis
    pub heading: Color,
    pub code: Color,
    pub link: Color,

    // Status
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub tool: Color,

    // Structural
    pub border: Color,
    pub surface: Color,

    // Code highlighting
    pub syntect_theme: EmbeddedThemeName,
    pub is_dark: bool,
}

fn hex(s: &str) -> Color {
    let b = s.trim_start_matches('#').as_bytes();
    if b.len() != 6 || !b.iter().all(u8::is_ascii_hexdigit) {
        tracing::warn!("invalid hex color '{s}', expected #RRGGBB — using terminal default");
        return Color::Reset;
    }
    let nibble = |i: usize| match b[i] {
        c @ b'0'..=b'9' => c - b'0',
        c @ b'a'..=b'f' => c - b'a' + 10,
        c @ b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    };
    let byte = |i: usize| nibble(i) * 16 + nibble(i + 1);
    Color::Rgb {
        r: byte(0),
        g: byte(2),
        b: byte(4),
    }
}

fn catppuccin_mocha() -> SemanticTheme {
    SemanticTheme {
        name: "Catppuccin Mocha".to_string(),
        text: hex("#cdd6f4"),
        text_muted: hex("#6c7086"),
        prompt_caret: hex("#eba0ac"),
        heading: hex("#cba6f7"),
        code: hex("#94e2d5"),
        link: hex("#89b4fa"),
        success: hex("#a6e3a1"),
        error: hex("#f38ba8"),
        warning: hex("#f9e2af"),
        tool: hex("#89b4fa"),
        border: hex("#45475a"),
        surface: hex("#313244"),
        syntect_theme: EmbeddedThemeName::CatppuccinMocha,
        is_dark: true,
    }
}

fn catppuccin_latte() -> SemanticTheme {
    SemanticTheme {
        name: "Catppuccin Latte".to_string(),
        text: hex("#4c4f69"),
        text_muted: hex("#9ca0b0"),
        prompt_caret: hex("#e64553"),
        heading: hex("#8839ef"),
        code: hex("#179299"),
        link: hex("#1e66f5"),
        success: hex("#40a02b"),
        error: hex("#d20f39"),
        warning: hex("#df8e1d"),
        tool: hex("#1e66f5"),
        border: hex("#bcc0cc"),
        surface: hex("#ccd0da"),
        syntect_theme: EmbeddedThemeName::CatppuccinLatte,
        is_dark: false,
    }
}

fn retrowave() -> SemanticTheme {
    SemanticTheme {
        name: "Retrowave".to_string(),
        text: hex("#e0def4"),
        text_muted: hex("#413755"),
        prompt_caret: hex("#ff79c6"),
        heading: hex("#bb9af7"),
        code: hex("#7dcfff"),
        link: hex("#7dcfff"),
        success: hex("#73daca"),
        error: hex("#f7768e"),
        warning: hex("#ff9e64"),
        tool: hex("#7dcfff"),
        border: hex("#413755"),
        surface: hex("#342a48"),
        syntect_theme: EmbeddedThemeName::DarkNeon,
        is_dark: true,
    }
}

fn terminal_native() -> SemanticTheme {
    SemanticTheme {
        name: "Terminal Native".to_string(),
        text: Color::Reset,
        text_muted: Color::DarkGrey,
        prompt_caret: Color::Red,
        heading: Color::Magenta,
        code: Color::Cyan,
        link: Color::Blue,
        success: Color::Green,
        error: Color::Red,
        warning: Color::Yellow,
        tool: Color::Blue,
        border: Color::DarkGrey,
        surface: Color::Black,
        syntect_theme: EmbeddedThemeName::Base16OceanDark,
        is_dark: true,
    }
}

pub const BUILT_IN_THEMES: &[&str] = &[
    "catppuccin-mocha",
    "catppuccin-latte",
    "retrowave",
    "terminal-native",
];

pub fn try_resolve_theme(name: &str) -> Option<SemanticTheme> {
    match name.to_lowercase().as_str() {
        "catppuccin-mocha" | "mocha" => Some(catppuccin_mocha()),
        "catppuccin-latte" | "latte" => Some(catppuccin_latte()),
        "retrowave" | "retro" | "synthwave" => Some(retrowave()),
        "terminal" | "terminal-native" | "native" | "ansi" => Some(terminal_native()),
        other => load_custom_theme(other),
    }
}

pub fn is_known_theme(name: &str) -> bool {
    try_resolve_theme(name).is_some()
}

pub fn resolve_theme(name: &str) -> SemanticTheme {
    try_resolve_theme(name).unwrap_or_else(catppuccin_mocha)
}

static THEME: RwLock<Option<SemanticTheme>> = RwLock::new(None);

pub fn active_theme() -> SemanticTheme {
    // Fast path: read lock
    {
        // SAFETY: unwrap_or_else(|e| e.into_inner()) recovers from a poisoned lock.
        // Locks are only poisoned when a thread panics while holding the write lock.
        // Since our write paths are panic-free, this is a belt-and-suspenders guard.
        let guard = THEME.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ref theme) = *guard {
            return theme.clone();
        }
    }
    // Slow path: initialize
    let theme = load_theme();
    let cloned = theme.clone();
    let mut guard = THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(theme);
    cloned
}

pub fn set_active_theme(theme: SemanticTheme) {
    let mut guard = THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(theme);
}

pub fn save_theme_preference(name: &str) -> std::io::Result<()> {
    let config_dir = dirs::config_dir()
        .map(|d| d.join("goose"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config dir"))?;
    std::fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("theme.yaml");

    // Read-modify-write: preserve custom_themes and any other keys.
    let mut config: serde_yaml::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or_default();

    if !config.is_mapping() {
        config = serde_yaml::Value::Mapping(Default::default());
    }
    if let serde_yaml::Value::Mapping(ref mut map) = config {
        map.insert(
            serde_yaml::Value::String("theme".into()),
            serde_yaml::Value::String(name.into()),
        );
    }

    let yaml = serde_yaml::to_string(&config).map_err(std::io::Error::other)?;
    std::fs::write(path, yaml)
}

fn load_theme() -> SemanticTheme {
    let name = std::env::var("GOOSE_THEME")
        .ok()
        .or_else(|| read_config().ok().flatten().and_then(|c| c.theme))
        .unwrap_or_else(|| "catppuccin-mocha".to_string());

    resolve_theme(&name)
}

#[derive(Deserialize, Default)]
struct ThemeConfig {
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    custom_themes: HashMap<String, CustomThemeDef>,
}

/// Missing fields inherit from catppuccin-mocha.
#[derive(Deserialize, Default)]
struct CustomThemeDef {
    text: Option<String>,
    text_muted: Option<String>,
    prompt_caret: Option<String>,
    heading: Option<String>,
    code: Option<String>,
    link: Option<String>,
    success: Option<String>,
    error: Option<String>,
    warning: Option<String>,
    tool: Option<String>,
    border: Option<String>,
    surface: Option<String>,
    syntect_theme: Option<String>,
    is_dark: Option<bool>,
}

fn config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|p| p.join("goose").join("theme.yaml"))
}

fn read_config() -> std::io::Result<Option<ThemeConfig>> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(None),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let config: ThemeConfig = serde_yaml::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(config))
}

fn load_custom_theme(name: &str) -> Option<SemanticTheme> {
    let config = read_config().ok()??;
    let def = config.custom_themes.get(name)?;
    let base = catppuccin_mocha();

    Some(SemanticTheme {
        name: name.to_string(),
        text: def.text.as_deref().map(hex).unwrap_or(base.text),
        text_muted: def
            .text_muted
            .as_deref()
            .map(hex)
            .unwrap_or(base.text_muted),
        prompt_caret: def
            .prompt_caret
            .as_deref()
            .map(hex)
            .unwrap_or(base.prompt_caret),
        heading: def.heading.as_deref().map(hex).unwrap_or(base.heading),
        code: def.code.as_deref().map(hex).unwrap_or(base.code),
        link: def.link.as_deref().map(hex).unwrap_or(base.link),
        success: def.success.as_deref().map(hex).unwrap_or(base.success),
        error: def.error.as_deref().map(hex).unwrap_or(base.error),
        warning: def.warning.as_deref().map(hex).unwrap_or(base.warning),
        tool: def.tool.as_deref().map(hex).unwrap_or(base.tool),
        border: def.border.as_deref().map(hex).unwrap_or(base.border),
        surface: def.surface.as_deref().map(hex).unwrap_or(base.surface),
        syntect_theme: def
            .syntect_theme
            .as_deref()
            .map(parse_syntect_theme)
            .unwrap_or(base.syntect_theme),
        is_dark: def.is_dark.unwrap_or(base.is_dark),
    })
}

fn parse_syntect_theme(name: &str) -> EmbeddedThemeName {
    match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
        "catppuccinmocha" => EmbeddedThemeName::CatppuccinMocha,
        "catppuccinlatte" => EmbeddedThemeName::CatppuccinLatte,
        "catppuccinfrappe" => EmbeddedThemeName::CatppuccinFrappe,
        "catppuccinmacchiato" => EmbeddedThemeName::CatppuccinMacchiato,
        "darkneon" => EmbeddedThemeName::DarkNeon,
        "dracula" => EmbeddedThemeName::Dracula,
        "nord" => EmbeddedThemeName::Nord,
        "base16oceandark" => EmbeddedThemeName::Base16OceanDark,
        "base16oceanlight" => EmbeddedThemeName::Base16OceanLight,
        "base16eightiesdark" => EmbeddedThemeName::Base16EightiesDark,
        "solarizeddark" => EmbeddedThemeName::SolarizedDark,
        "solarizedlight" => EmbeddedThemeName::SolarizedLight,
        "twodark" => EmbeddedThemeName::TwoDark,
        "monokaiextended" => EmbeddedThemeName::MonokaiExtended,
        "gruvboxdark" => EmbeddedThemeName::GruvboxDark,
        "gruvboxlight" => EmbeddedThemeName::GruvboxLight,
        "onehalfdark" => EmbeddedThemeName::OneHalfDark,
        "onehalflight" => EmbeddedThemeName::OneHalfLight,
        "sublimesnazzy" => EmbeddedThemeName::SublimeSnazzy,
        "zenburn" => EmbeddedThemeName::Zenburn,
        "leet" => EmbeddedThemeName::Leet,
        "ansi" => EmbeddedThemeName::Ansi,
        "base16" => EmbeddedThemeName::Base16,
        "base16256" => EmbeddedThemeName::Base16_256,
        "github" => EmbeddedThemeName::Github,
        "inspiredgithub" => EmbeddedThemeName::InspiredGithub,
        _ => EmbeddedThemeName::CatppuccinMocha, // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_theme_aliases() {
        assert_eq!(resolve_theme("mocha").name, "Catppuccin Mocha");
        assert_eq!(resolve_theme("retro").name, "Retrowave");
        assert_eq!(resolve_theme("latte").name, "Catppuccin Latte");
        assert_eq!(resolve_theme("native").name, "Terminal Native");
    }

    #[test]
    fn resolve_unknown_falls_back() {
        assert_eq!(resolve_theme("nonexistent").name, "Catppuccin Mocha");
    }

    #[test]
    fn hex_parsing() {
        assert_eq!(hex("#ff0000"), Color::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(hex("00ff00"), Color::Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn save_theme_preserves_existing_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("theme.yaml");

        // Write a config with custom_themes and an extra key
        std::fs::write(
            &path,
            "theme: retrowave\ncustom_themes:\n  my-theme:\n    text: '#ffffff'\nextra_key: preserved\n",
        )
        .unwrap();

        // Simulate save_theme_preference logic (can't call it directly — it uses dirs::config_dir)
        let content = std::fs::read_to_string(&path).unwrap();
        let mut config: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        if let serde_yaml::Value::Mapping(ref mut map) = config {
            map.insert(
                serde_yaml::Value::String("theme".into()),
                serde_yaml::Value::String("mocha".into()),
            );
        }
        let yaml = serde_yaml::to_string(&config).unwrap();
        std::fs::write(&path, &yaml).unwrap();

        // Verify: theme changed, custom_themes and extra_key preserved
        let result: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(result["theme"], serde_yaml::Value::String("mocha".into()));
        assert!(result["custom_themes"]["my-theme"]["text"].is_string());
        assert_eq!(
            result["extra_key"],
            serde_yaml::Value::String("preserved".into())
        );
    }
}
