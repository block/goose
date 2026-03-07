use std::sync::OnceLock;

use crossterm::style::{Color, ContentStyle, StyledContent, Stylize};
use two_face::theme::EmbeddedThemeName;

pub fn no_color() -> bool {
    static NO_COLOR: OnceLock<bool> = OnceLock::new();
    *NO_COLOR.get_or_init(|| {
        std::env::var("NO_COLOR").is_ok() || std::env::var("TERM").as_deref() == Ok("dumb")
    })
}

fn styled(s: &str, c: Color) -> StyledContent<&str> {
    if no_color() {
        ContentStyle::new().apply(s)
    } else {
        ContentStyle::new().with(c).apply(s)
    }
}

// `active_theme()` acquires a read lock on each call. The lock is uncontended
// in normal operation (only write on init or /theme command), so the overhead
// is negligible even when called hundreds of times per render_to_lines pass.
macro_rules! style_fn {
    ($name:ident, $color:expr) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color)
        }
    };
    ($name:ident, $color:expr, bold) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color).bold()
        }
    };
    ($name:ident, $color:expr, italic) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color).italic()
        }
    };
    ($name:ident, $color:expr, underlined) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color).underlined()
        }
    };
    ($name:ident, $color:expr, bold, underlined) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color).bold().underlined()
        }
    };
    ($name:ident, $color:expr, bold, italic) => {
        pub fn $name(s: &str) -> StyledContent<&str> {
            styled(s, $color).bold().italic()
        }
    };
}

// Returns `String` (not `StyledContent`): reedline's prompt API requires owned ANSI strings.
macro_rules! prompt_fn {
    ($name:ident, $color:expr) => {
        pub fn $name() -> String {
            ansi_fg($color)
        }
    };
}

pub fn banner() -> String {
    let version = env!("CARGO_PKG_VERSION");
    if no_color() {
        return format!("goose {version}");
    }
    let t = super::theme::active_theme();
    format!(
        "{} {}",
        styled("goose", t.warning).bold(),
        styled(version, t.warning)
    )
}

style_fn!(tool_name, super::theme::active_theme().tool);
style_fn!(success, super::theme::active_theme().success);
style_fn!(error, super::theme::active_theme().error);
style_fn!(pending, super::theme::active_theme().warning);
style_fn!(dim, super::theme::active_theme().text_muted);
style_fn!(hotkey, super::theme::active_theme().warning, bold);

style_fn!(
    heading_h1,
    super::theme::active_theme().heading,
    bold,
    underlined
);
style_fn!(heading_h2, super::theme::active_theme().heading, bold);
style_fn!(heading_h3, super::theme::active_theme().heading, italic);
style_fn!(link_text, super::theme::active_theme().link, underlined);
style_fn!(link_url, super::theme::active_theme().text_muted);
style_fn!(inline_code, super::theme::active_theme().code);
style_fn!(blockquote_bar, super::theme::active_theme().border);
style_fn!(blockquote_text, super::theme::active_theme().text_muted);
style_fn!(table_border, super::theme::active_theme().border);
style_fn!(table_header, super::theme::active_theme().text, bold);
style_fn!(list_marker, super::theme::active_theme().text_muted);
style_fn!(rule, super::theme::active_theme().border);
style_fn!(strong, super::theme::active_theme().text, bold);
style_fn!(emphasis, super::theme::active_theme().text, italic);
style_fn!(
    strong_emphasis,
    super::theme::active_theme().text,
    bold,
    italic
);

// Strikethrough uses raw ANSI escapes rather than StyledContent because
// crossterm does not expose strikethrough as a composable attribute on
// borrowed &str — it requires an owned String. Raw SGR codes are the
// lightest-weight alternative and are universally supported.
pub fn strikethrough_on() -> &'static str {
    if no_color() {
        ""
    } else {
        "\x1b[9m"
    }
}

pub fn strikethrough_off() -> &'static str {
    if no_color() {
        ""
    } else {
        "\x1b[29m"
    }
}

pub fn reset() -> &'static str {
    if no_color() {
        ""
    } else {
        "\x1b[0m"
    }
}

pub fn ansi_fg(c: Color) -> String {
    if no_color() {
        return String::new();
    }
    match c {
        Color::Rgb { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m"),
        Color::Reset => String::new(),
        Color::Black => "\x1b[30m".to_string(),
        Color::DarkRed => "\x1b[31m".to_string(),
        Color::DarkGreen => "\x1b[32m".to_string(),
        Color::DarkYellow => "\x1b[33m".to_string(),
        Color::DarkBlue => "\x1b[34m".to_string(),
        Color::DarkMagenta => "\x1b[35m".to_string(),
        Color::DarkCyan => "\x1b[36m".to_string(),
        Color::Grey => "\x1b[37m".to_string(),
        Color::DarkGrey => "\x1b[90m".to_string(),
        Color::Red => "\x1b[91m".to_string(),
        Color::Green => "\x1b[92m".to_string(),
        Color::Yellow => "\x1b[93m".to_string(),
        Color::Blue => "\x1b[94m".to_string(),
        Color::Magenta => "\x1b[95m".to_string(),
        Color::Cyan => "\x1b[96m".to_string(),
        Color::White => "\x1b[97m".to_string(),
        Color::AnsiValue(n) => format!("\x1b[38;5;{n}m"),
    }
}

prompt_fn!(prompt_cwd, super::theme::active_theme().link);
prompt_fn!(prompt_branch, super::theme::active_theme().heading);
prompt_fn!(prompt_model, super::theme::active_theme().link);
prompt_fn!(prompt_context_ok, super::theme::active_theme().success);
prompt_fn!(prompt_context_warn, super::theme::active_theme().warning);
prompt_fn!(prompt_context_crit, super::theme::active_theme().error);
prompt_fn!(prompt_indicator, super::theme::active_theme().prompt_caret);
prompt_fn!(prompt_separator, super::theme::active_theme().border);
prompt_fn!(prompt_session_id, super::theme::active_theme().text_muted);

pub fn hinter_color() -> nu_ansi_term::Color {
    if no_color() {
        return nu_ansi_term::Color::Default;
    }
    crossterm_to_nu(super::theme::active_theme().text_muted)
}

fn crossterm_to_nu(c: Color) -> nu_ansi_term::Color {
    match c {
        Color::Rgb { r, g, b } => nu_ansi_term::Color::Rgb(r, g, b),
        Color::Black => nu_ansi_term::Color::Black,
        Color::DarkRed => nu_ansi_term::Color::Red,
        Color::DarkGreen => nu_ansi_term::Color::Green,
        Color::DarkYellow => nu_ansi_term::Color::Yellow,
        Color::DarkBlue => nu_ansi_term::Color::Blue,
        Color::DarkMagenta => nu_ansi_term::Color::Purple,
        Color::DarkCyan => nu_ansi_term::Color::Cyan,
        Color::Grey => nu_ansi_term::Color::White,
        Color::DarkGrey => nu_ansi_term::Color::DarkGray,
        Color::Red => nu_ansi_term::Color::LightRed,
        Color::Green => nu_ansi_term::Color::LightGreen,
        Color::Yellow => nu_ansi_term::Color::LightYellow,
        Color::Blue => nu_ansi_term::Color::LightBlue,
        Color::Magenta => nu_ansi_term::Color::LightPurple,
        Color::Cyan => nu_ansi_term::Color::LightCyan,
        Color::White => nu_ansi_term::Color::LightGray,
        _ => nu_ansi_term::Color::Default,
    }
}

pub fn syntect_theme_name() -> EmbeddedThemeName {
    super::theme::active_theme().syntect_theme
}
