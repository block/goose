use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

const DARK_THEME: &str = "base16-eighties.dark";
const LIGHT_THEME: &str = "InspiredGitHub";

pub fn highlight_code(code: &str, lang: &str, dark_mode: bool) -> Vec<Line<'static>> {
    let lang = normalize_language(lang);
    let theme_name = if dark_mode { DARK_THEME } else { LIGHT_THEME };
    let theme = THEME_SET
        .themes
        .get(theme_name)
        .or_else(|| THEME_SET.themes.values().next())
        .expect("syntect must have at least one theme");

    let syntax = SYNTAX_SET
        .find_syntax_by_token(lang)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);

    code.lines()
        .map(|line| {
            let line_with_newline = format!("{line}\n");
            highlighter
                .highlight_line(&line_with_newline, &SYNTAX_SET)
                .map(|ranges| {
                    Line::from(
                        ranges
                            .into_iter()
                            .map(|(style, text)| {
                                let text = text.trim_end_matches('\n');
                                Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .unwrap_or_else(|_| Line::from(line.to_string()))
        })
        .collect()
}

fn syntect_to_ratatui_style(style: syntect::highlighting::Style) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);

    let mut modifiers = Modifier::empty();
    if style.font_style.contains(FontStyle::BOLD) {
        modifiers |= Modifier::BOLD;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifiers |= Modifier::ITALIC;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        modifiers |= Modifier::UNDERLINED;
    }

    Style::default().fg(fg).add_modifier(modifiers)
}

fn normalize_language(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rb" => "ruby",
        "sh" | "shell" => "bash",
        "yml" => "yaml",
        "md" => "markdown",
        _ => lang,
    }
}

pub struct CodeBlockIterator<'a> {
    remaining: &'a str,
}

impl<'a> CodeBlockIterator<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { remaining: text }
    }
}

pub enum TextSegment<'a> {
    Text(&'a str),
    CodeBlock { lang: &'a str, code: &'a str },
}

impl<'a> Iterator for CodeBlockIterator<'a> {
    type Item = TextSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        match self.remaining.find("```") {
            Some(0) => {
                let after_fence = &self.remaining[3..];
                match after_fence.find("```") {
                    Some(end) => {
                        let block_content = &after_fence[..end];
                        let (lang, code) = match block_content.find('\n') {
                            Some(newline) => (
                                block_content[..newline].trim(),
                                &block_content[newline + 1..],
                            ),
                            None => ("", block_content),
                        };
                        self.remaining = &after_fence[end + 3..];
                        Some(TextSegment::CodeBlock { lang, code })
                    }
                    None => {
                        let text = self.remaining;
                        self.remaining = "";
                        Some(TextSegment::Text(text))
                    }
                }
            }
            Some(pos) => {
                let text = &self.remaining[..pos];
                self.remaining = &self.remaining[pos..];
                Some(TextSegment::Text(text))
            }
            None => {
                let text = self.remaining;
                self.remaining = "";
                Some(TextSegment::Text(text))
            }
        }
    }
}
