use super::styles::Theme;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

pub struct MarkdownRenderer<'a> {
    text: &'a str,
    width: usize,
    theme: &'a Theme,
}

impl<'a> MarkdownRenderer<'a> {
    pub fn new(text: &'a str, width: usize, theme: &'a Theme) -> Self {
        Self { text, width, theme }
    }

    fn get_syntax_set() -> &'static SyntaxSet {
        SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
    }

    fn get_theme_set() -> &'static ThemeSet {
        THEME_SET.get_or_init(ThemeSet::load_defaults)
    }

    fn syntect_to_ratatui(color: syntect::highlighting::Color) -> Color {
        Color::Rgb(color.r, color.g, color.b)
    }

    pub fn render_lines(&self) -> Vec<Line<'static>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(self.text, options);

        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_width = 0;

        let mut style_stack = Vec::new();
        let mut current_style = Style::default().fg(self.theme.base.foreground);
        let mut code_block_lang: Option<String> = None;

        for event in parser {
            match event {
                Event::Text(ref t) | Event::Code(ref t) => {
                    let is_code_span = matches!(event, Event::Code(_));
                    let content = t.to_string();

                    if code_block_lang.is_some() {
                        // Syntax Highlighting for Code Blocks
                        let ss = Self::get_syntax_set();
                        let ts = Self::get_theme_set();
                        let theme = &ts.themes["base16-ocean.dark"];

                        let lang = code_block_lang.as_ref().unwrap();
                        let syntax = ss
                            .find_syntax_by_token(lang)
                            .unwrap_or_else(|| ss.find_syntax_plain_text());

                        let mut h = HighlightLines::new(syntax, theme);

                        for line in content.split_inclusive('\n') {
                            let ranges: Vec<(syntect::highlighting::Style, &str)> =
                                h.highlight_line(line, ss).unwrap_or_default();

                            for (style, text) in ranges {
                                let fg = Self::syntect_to_ratatui(style.foreground);
                                let span_style = Style::default().fg(fg);

                                if text.ends_with('\n') {
                                    let trimmed =
                                        text.trim_end_matches('\n').trim_end_matches('\r');
                                    if !trimmed.is_empty() {
                                        current_line
                                            .push(Span::styled(trimmed.to_string(), span_style));
                                    }
                                    lines.push(Line::from(current_line.clone()));
                                    current_line.clear();
                                    current_width = 0;
                                } else {
                                    current_line.push(Span::styled(text.to_string(), span_style));
                                    current_width += text.chars().count();
                                }
                            }
                        }
                        continue;
                    }

                    let mut style = current_style;
                    if is_code_span {
                        style = style.fg(Color::Yellow);
                    }

                    let parts: Vec<&str> = content.split(' ').collect();
                    for (i, part) in parts.iter().enumerate() {
                        let is_last = i == parts.len() - 1;
                        let mut word = part.to_string();
                        if !is_last {
                            word.push(' ');
                        }

                        if word.is_empty() {
                            continue;
                        }

                        let sub_parts: Vec<&str> = word.split('\n').collect();
                        for (j, sub) in sub_parts.iter().enumerate() {
                            if j > 0 {
                                lines.push(Line::from(current_line.clone()));
                                current_line.clear();
                                current_width = 0;
                            }

                            if sub.is_empty() {
                                continue;
                            }

                            let sub_width = sub.chars().count();

                            if current_width + sub_width > self.width && current_width > 0 {
                                lines.push(Line::from(current_line.clone()));
                                current_line.clear();
                                current_width = 0;
                            }

                            current_line.push(Span::styled(sub.to_string(), style));
                            current_width += sub_width;
                        }
                    }
                }
                Event::Start(tag) => match tag {
                    Tag::Emphasis => {
                        style_stack.push(current_style);
                        current_style = current_style.add_modifier(Modifier::ITALIC);
                    }
                    Tag::Strong => {
                        style_stack.push(current_style);
                        current_style = current_style.add_modifier(Modifier::BOLD);
                    }
                    Tag::CodeBlock(kind) => {
                        let lang = match kind {
                            CodeBlockKind::Fenced(l) => l.to_string(),
                            CodeBlockKind::Indented => "text".to_string(),
                        };
                        code_block_lang = Some(lang);

                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_width = 0;
                        }
                    }
                    Tag::List(_) | Tag::Item => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_width = 0;
                        }
                        if matches!(tag, Tag::Item) {
                            current_line.push(Span::styled("• ", current_style));
                            current_width += 2;
                        }
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    TagEnd::Emphasis | TagEnd::Strong => {
                        if let Some(s) = style_stack.pop() {
                            current_style = s;
                        }
                    }
                    TagEnd::CodeBlock => {
                        code_block_lang = None;
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_width = 0;
                        }
                    }
                    TagEnd::List(_) | TagEnd::Item => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_width = 0;
                        }
                    }
                    TagEnd::Paragraph => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_width = 0;
                        }
                        lines.push(Line::from(""));
                    }
                    _ => {}
                },
                Event::SoftBreak | Event::HardBreak => {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                    current_width = 0;
                }
                _ => {}
            }
        }

        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }

        lines
    }
}
