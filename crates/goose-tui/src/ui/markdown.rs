use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

// Simple Markdown parser for Ratatui
// Supports:
// - Bold (**text**)
// - Italic (*text*)
// - Code blocks (```lang ... ```) - rendered as distinct color
// - Inline code (`text`)
pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str, max_width: usize) -> Text<'static> {
        let mut lines = Vec::new();
        let mut in_code_block = false;

        let wrap_width = max_width.max(10); // Ensure a minimum width

        for line in input.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
                continue;
            }

            if in_code_block {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Yellow), // Code color
                )));
            } else {
                // Wrap text content
                let wrapped = textwrap::wrap(line, wrap_width);

                if wrapped.is_empty() && !line.is_empty() {
                    // Should not happen with non-empty line, but just in case
                    lines.push(Self::parse_inline(line));
                } else if wrapped.is_empty() {
                    // Empty line
                    lines.push(Line::from(""));
                }

                for w in wrapped {
                    lines.push(Self::parse_inline(&w));
                }
            }
        }

        Text::from(lines)
    }

    fn parse_inline(line: &str) -> Line<'static> {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut chars = line.chars().peekable();

        // Styles
        let mut is_bold = false;
        let mut is_italic = false;
        let mut is_code = false;

        while let Some(c) = chars.next() {
            if c == '`' {
                // Flush current text
                if !current_text.is_empty() {
                    spans.push(Span::styled(
                        current_text.clone(),
                        Self::style(is_bold, is_italic, is_code),
                    ));
                    current_text.clear();
                }
                is_code = !is_code;
                continue;
            }

            if !is_code {
                if c == '*' {
                    if chars.peek() == Some(&'*') {
                        // Bold
                        chars.next(); // consume second *
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                Self::style(is_bold, is_italic, is_code),
                            ));
                            current_text.clear();
                        }
                        is_bold = !is_bold;
                        continue;
                    } else {
                        // Italic
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                Self::style(is_bold, is_italic, is_code),
                            ));
                            current_text.clear();
                        }
                        is_italic = !is_italic;
                        continue;
                    }
                }
            }

            current_text.push(c);
        }

        if !current_text.is_empty() {
            spans.push(Span::styled(
                current_text,
                Self::style(is_bold, is_italic, is_code),
            ));
        }

        Line::from(spans)
    }

    fn style(bold: bool, italic: bool, code: bool) -> Style {
        let mut style = Style::default();
        if code {
            style = style.fg(Color::Yellow);
        }
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let input = "Hello world";
        let text = MarkdownParser::parse(input, 80);
        assert_eq!(text.lines.len(), 1);
        assert_eq!(text.lines[0].spans[0].content, "Hello world");
    }

    #[test]
    fn test_parse_bold_italic() {
        let input = "**Bold** and *Italic*";
        let text = MarkdownParser::parse(input, 80);
        let spans = &text.lines[0].spans;

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Bold");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));

        assert_eq!(spans[1].content, " and ");

        assert_eq!(spans[2].content, "Italic");
        assert!(spans[2].style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_parse_inline_code() {
        let input = "Run `cargo build` now";
        let text = MarkdownParser::parse(input, 80);
        let spans = &text.lines[0].spans;

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "Run ");
        assert_eq!(spans[1].content, "cargo build");
        assert_eq!(spans[1].style.fg, Some(Color::Yellow));
        assert_eq!(spans[2].content, " now");
    }

    #[test]
    fn test_parse_code_block() {
        let input = "Start\n```rust\nfn main() {}\n```\nEnd";
        let text = MarkdownParser::parse(input, 80);

        assert_eq!(text.lines.len(), 5);
        assert_eq!(text.lines[0].spans[0].content, "Start");
        assert_eq!(text.lines[1].spans[0].content, "```rust"); // Fence

        assert_eq!(text.lines[2].spans[0].content, "fn main() {}");
        assert_eq!(text.lines[2].spans[0].style.fg, Some(Color::Yellow)); // Code block content

        assert_eq!(text.lines[3].spans[0].content, "```"); // Fence
        assert_eq!(text.lines[4].spans[0].content, "End");
    }
}
