use crate::utils::styles::Theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use termimad::MadSkin;

/// Alternative approach: Use termimad's parsed representation directly
pub struct TermimadRenderer2 {
    skin: MadSkin,
}

impl TermimadRenderer2 {
    pub fn new(theme: &Theme, base_style: Option<Style>) -> Self {
        let mut skin = MadSkin::default();

        // Configure skin based on theme
        if let Some(style) = base_style {
            if let Some(fg) = style.fg {
                skin.set_fg(to_crossterm_color(fg));
            }
            if let Some(bg) = style.bg {
                skin.set_bg(to_crossterm_color(bg));
            }
        } else {
            skin.set_fg(to_crossterm_color(theme.base.foreground));
            // theme.base.background is already a Color, not an Option<Color>
            skin.set_bg(to_crossterm_color(theme.base.background));
        }

        // Configure specific styles
        skin.bold.set_fg(to_crossterm_color(theme.base.foreground));
        skin.bold
            .add_attr(termimad::crossterm::style::Attribute::Bold);

        skin.italic
            .set_fg(to_crossterm_color(theme.base.foreground));
        skin.italic
            .add_attr(termimad::crossterm::style::Attribute::Italic);

        // Code blocks with better contrast
        skin.inline_code
            .set_fg(termimad::crossterm::style::Color::Rgb {
                r: 255,
                g: 200,
                b: 100,
            });
        skin.inline_code
            .set_bg(termimad::crossterm::style::Color::Rgb {
                r: 40,
                g: 40,
                b: 40,
            });

        skin.code_block
            .set_fg(termimad::crossterm::style::Color::Rgb {
                r: 200,
                g: 200,
                b: 200,
            });
        skin.code_block
            .set_bg(termimad::crossterm::style::Color::Rgb {
                r: 30,
                g: 30,
                b: 30,
            });

        Self { skin }
    }

    /// Render markdown to Lines using termimad's text formatting
    pub fn render_lines(&self, text: &str, width: usize) -> Vec<Line<'static>> {
        // Use termimad to format the text with proper wrapping
        let fmt_text = self.skin.text(text, Some(width));
        let rendered = format!("{fmt_text}");

        // Split into lines and convert to ratatui Lines
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut last_was_empty = false;

        for line in rendered.lines() {
            let stripped = strip_ansi_codes(line);

            // Handle paragraph spacing
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

/// Convert ratatui Color to crossterm Color
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

fn parse_sgr_code(code: u16, current_style: &mut Style) {
    match code {
        0 => *current_style = Style::default(),
        1 => *current_style = current_style.add_modifier(Modifier::BOLD),
        3 => *current_style = current_style.add_modifier(Modifier::ITALIC),
        4 => *current_style = current_style.add_modifier(Modifier::UNDERLINED),
        30 => *current_style = current_style.fg(Color::Black),
        31 => *current_style = current_style.fg(Color::Red),
        32 => *current_style = current_style.fg(Color::Green),
        33 => *current_style = current_style.fg(Color::Yellow),
        34 => *current_style = current_style.fg(Color::Blue),
        35 => *current_style = current_style.fg(Color::Magenta),
        36 => *current_style = current_style.fg(Color::Cyan),
        37 => *current_style = current_style.fg(Color::Gray),
        39 => *current_style = current_style.fg(Color::Reset),
        40 => *current_style = current_style.bg(Color::Black),
        41 => *current_style = current_style.bg(Color::Red),
        42 => *current_style = current_style.bg(Color::Green),
        43 => *current_style = current_style.bg(Color::Yellow),
        44 => *current_style = current_style.bg(Color::Blue),
        45 => *current_style = current_style.bg(Color::Magenta),
        46 => *current_style = current_style.bg(Color::Cyan),
        47 => *current_style = current_style.bg(Color::Gray),
        49 => *current_style = current_style.bg(Color::Reset),
        90 => *current_style = current_style.fg(Color::DarkGray),
        91 => *current_style = current_style.fg(Color::LightRed),
        92 => *current_style = current_style.fg(Color::LightGreen),
        93 => *current_style = current_style.fg(Color::LightYellow),
        94 => *current_style = current_style.fg(Color::LightBlue),
        95 => *current_style = current_style.fg(Color::LightMagenta),
        96 => *current_style = current_style.fg(Color::LightCyan),
        97 => *current_style = current_style.fg(Color::White),
        100 => *current_style = current_style.bg(Color::DarkGray),
        101 => *current_style = current_style.bg(Color::LightRed),
        102 => *current_style = current_style.bg(Color::LightGreen),
        103 => *current_style = current_style.bg(Color::LightYellow),
        104 => *current_style = current_style.bg(Color::LightBlue),
        105 => *current_style = current_style.bg(Color::LightMagenta),
        106 => *current_style = current_style.bg(Color::LightCyan),
        107 => *current_style = current_style.bg(Color::White),
        _ => {}
    }
}

/// Parse a line with ANSI escape codes into ratatui Spans
fn parse_ansi_line(line: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut current_style = Style::default();
    let mut last_index = 0;

    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Found start of CSI
            if i > last_index {
                let text = &line[last_index..i];
                spans.push(Span::styled(text.to_string(), current_style));
            }

            let start = i + 2;
            let mut end = start;
            while end < bytes.len() {
                if bytes[end] >= 0x40 && bytes[end] <= 0x7E {
                    break;
                }
                end += 1;
            }

            if end < bytes.len() && bytes[end] == b'm' {
                // Parse codes
                let codes_str = &line[start..end];
                if codes_str.is_empty() {
                    current_style = Style::default();
                } else {
                    let codes: Vec<u16> = codes_str
                        .split(';')
                        .filter_map(|s| s.parse().ok())
                        .collect();

                    let mut k = 0;
                    while k < codes.len() {
                        match codes[k] {
                            38 => {
                                // Extended FG
                                if k + 1 < codes.len() {
                                    match codes[k + 1] {
                                        5 => {
                                            // 256 colors
                                            if k + 2 < codes.len() {
                                                current_style = current_style
                                                    .fg(Color::Indexed(codes[k + 2] as u8));
                                                k += 2;
                                            }
                                        }
                                        2 => {
                                            // RGB
                                            if k + 4 < codes.len() {
                                                current_style = current_style.fg(Color::Rgb(
                                                    codes[k + 2] as u8,
                                                    codes[k + 3] as u8,
                                                    codes[k + 4] as u8,
                                                ));
                                                k += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            48 => {
                                // Extended BG
                                if k + 1 < codes.len() {
                                    match codes[k + 1] {
                                        5 => {
                                            // 256 colors
                                            if k + 2 < codes.len() {
                                                current_style = current_style
                                                    .bg(Color::Indexed(codes[k + 2] as u8));
                                                k += 2;
                                            }
                                        }
                                        2 => {
                                            // RGB
                                            if k + 4 < codes.len() {
                                                current_style = current_style.bg(Color::Rgb(
                                                    codes[k + 2] as u8,
                                                    codes[k + 3] as u8,
                                                    codes[k + 4] as u8,
                                                ));
                                                k += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            other => parse_sgr_code(other, &mut current_style),
                        }
                        k += 1;
                    }
                }

                i = end + 1;
                last_index = i;
                continue;
            }
        }
        i += 1;
    }

    if last_index < line.len() {
        spans.push(Span::styled(line[last_index..].to_string(), current_style));
    }

    Line::from(spans)
}

/// Strip ANSI escape codes from a string (for emptiness check)
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else {
            result.push(ch);
        }
    }

    result
}
