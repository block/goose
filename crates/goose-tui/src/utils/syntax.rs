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
            highlighter
                .highlight_line(line, &SYNTAX_SET)
                .map(|ranges| {
                    Line::from(
                        ranges
                            .into_iter()
                            .map(|(style, text)| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_code_produces_styled_spans() {
        let lines = highlight_code("fn main() {}", "rust", true);
        assert!(!lines.is_empty());
        assert!(lines[0].spans.len() > 1);
    }

    #[test]
    fn test_highlight_code_unknown_language_fallback() {
        let lines = highlight_code("random text", "nonexistent_lang_xyz", true);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_code_empty_input() {
        let lines = highlight_code("", "rust", true);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_normalize_language_aliases() {
        assert_eq!(normalize_language("js"), "javascript");
        assert_eq!(normalize_language("JS"), "javascript");
        assert_eq!(normalize_language("ts"), "typescript");
        assert_eq!(normalize_language("py"), "python");
        assert_eq!(normalize_language("sh"), "bash");
        assert_eq!(normalize_language("shell"), "bash");
        assert_eq!(normalize_language("yml"), "yaml");
        assert_eq!(normalize_language("rust"), "rust");
    }

    #[test]
    fn test_code_block_iterator_no_blocks() {
        let text = "Just plain text";
        let segments: Vec<_> = CodeBlockIterator::new(text).collect();
        assert_eq!(segments.len(), 1);
        assert!(matches!(segments[0], TextSegment::Text("Just plain text")));
    }

    #[test]
    fn test_code_block_iterator_single_block() {
        let text = "```rust\nfn main() {}\n```";
        let segments: Vec<_> = CodeBlockIterator::new(text).collect();
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::CodeBlock { lang, code } => {
                assert_eq!(*lang, "rust");
                assert_eq!(*code, "fn main() {}\n");
            }
            _ => panic!("Expected CodeBlock"),
        }
    }

    #[test]
    fn test_code_block_iterator_mixed_content() {
        let text = "Before\n```python\ndef foo(): pass\n```\nAfter";
        let segments: Vec<_> = CodeBlockIterator::new(text).collect();
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], TextSegment::Text("Before\n")));
        assert!(matches!(
            segments[1],
            TextSegment::CodeBlock { lang: "python", .. }
        ));
        assert!(matches!(segments[2], TextSegment::Text("\nAfter")));
    }

    #[test]
    fn test_code_block_iterator_unclosed_block() {
        let text = "Start\n```rust\nfn main() {}";
        let segments: Vec<_> = CodeBlockIterator::new(text).collect();
        assert_eq!(segments.len(), 2);
        assert!(matches!(segments[0], TextSegment::Text("Start\n")));
        assert!(matches!(
            segments[1],
            TextSegment::Text("```rust\nfn main() {}")
        ));
    }

    #[test]
    fn test_code_block_iterator_consecutive_blocks() {
        let text = "```rust\na\n```\n```python\nb\n```";
        let segments: Vec<_> = CodeBlockIterator::new(text).collect();
        assert_eq!(segments.len(), 3);
        assert!(matches!(
            segments[0],
            TextSegment::CodeBlock { lang: "rust", .. }
        ));
        assert!(matches!(segments[1], TextSegment::Text("\n")));
        assert!(matches!(
            segments[2],
            TextSegment::CodeBlock { lang: "python", .. }
        ));
    }
}
