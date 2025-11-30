use crate::utils::sanitize::strip_ansi_codes;
use crate::utils::styles::Theme;
use ansi_to_tui::IntoText;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use termimad::MadSkin;

pub struct MarkdownRenderer {
    skin: MadSkin,
}

impl MarkdownRenderer {
    pub fn new(theme: &Theme, base_style: Option<Style>) -> Self {
        let mut skin = MadSkin::default();

        if let Some(style) = base_style {
            if let Some(fg) = style.fg {
                skin.set_fg(to_crossterm_color(fg));
            }
            if let Some(bg) = style.bg {
                skin.set_bg(to_crossterm_color(bg));
            }
        } else {
            skin.set_fg(to_crossterm_color(theme.base.foreground));
            skin.set_bg(to_crossterm_color(theme.base.background));
        }

        skin.bold.set_fg(to_crossterm_color(theme.base.foreground));
        skin.bold
            .add_attr(termimad::crossterm::style::Attribute::Bold);

        skin.italic
            .set_fg(to_crossterm_color(theme.base.foreground));
        skin.italic
            .add_attr(termimad::crossterm::style::Attribute::Italic);

        skin.inline_code
            .set_fg(to_crossterm_color(theme.status.warning));
        skin.inline_code
            .set_bg(to_crossterm_color(theme.base.selection));

        skin.code_block
            .set_fg(to_crossterm_color(theme.base.foreground));
        skin.code_block
            .set_bg(to_crossterm_color(theme.base.selection));

        Self { skin }
    }

    pub fn render_lines(&self, text: &str, width: usize) -> Vec<Line<'static>> {
        let fmt_text = self.skin.text(text, Some(width));
        let rendered = format!("{fmt_text}");

        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut last_was_empty = false;

        for line in rendered.lines() {
            let stripped = strip_ansi_codes(line);

            if stripped.trim().is_empty() {
                if !last_was_empty {
                    lines.push(Line::from(""));
                    last_was_empty = true;
                }
            } else {
                lines.push(parse_ansi_line(line));
                last_was_empty = false;
            }
        }

        lines
    }
}

fn to_crossterm_color(color: Color) -> termimad::crossterm::style::Color {
    match color {
        Color::Reset => termimad::crossterm::style::Color::Reset,
        Color::Black => termimad::crossterm::style::Color::Black,
        Color::Red => termimad::crossterm::style::Color::Red,
        Color::Green => termimad::crossterm::style::Color::Green,
        Color::Yellow => termimad::crossterm::style::Color::Yellow,
        Color::Blue => termimad::crossterm::style::Color::Blue,
        Color::Magenta => termimad::crossterm::style::Color::Magenta,
        Color::Cyan => termimad::crossterm::style::Color::Cyan,
        Color::Gray => termimad::crossterm::style::Color::Grey,
        Color::DarkGray => termimad::crossterm::style::Color::DarkGrey,
        Color::LightRed => termimad::crossterm::style::Color::DarkRed,
        Color::LightGreen => termimad::crossterm::style::Color::DarkGreen,
        Color::LightYellow => termimad::crossterm::style::Color::DarkYellow,
        Color::LightBlue => termimad::crossterm::style::Color::DarkBlue,
        Color::LightMagenta => termimad::crossterm::style::Color::DarkMagenta,
        Color::LightCyan => termimad::crossterm::style::Color::DarkCyan,
        Color::White => termimad::crossterm::style::Color::White,
        Color::Rgb(r, g, b) => termimad::crossterm::style::Color::Rgb { r, g, b },
        Color::Indexed(i) => termimad::crossterm::style::Color::AnsiValue(i),
    }
}

fn parse_ansi_line(line: &str) -> Line<'static> {
    match line.as_bytes().into_text() {
        Ok(text) => text
            .lines
            .into_iter()
            .next()
            .unwrap_or_else(|| Line::from("")),
        Err(_) => Line::from(strip_ansi_codes(line)),
    }
}
