use ansi_to_tui::IntoText;
use unicode_width::UnicodeWidthStr;

pub fn sanitize_line(s: &str) -> (String, usize) {
    let plain = if let Ok(text) = s.as_bytes().into_text() {
        text.lines
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        strip_ansi_codes(s)
    };

    let sanitized: String = plain.chars().filter(|c| !c.is_control()).collect();
    let width = UnicodeWidthStr::width(sanitized.as_str());
    (sanitized, width)
}

pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}
