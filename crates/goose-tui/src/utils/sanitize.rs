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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_plain_text() {
        let (sanitized, width) = sanitize_line("hello world");
        assert_eq!(sanitized, "hello world");
        assert_eq!(width, 11);
    }

    #[test]
    fn test_sanitize_ansi_codes() {
        let (sanitized, width) = sanitize_line("\x1b[31mred text\x1b[0m");
        assert_eq!(sanitized, "red text");
        assert_eq!(width, 8);
    }

    #[test]
    fn test_sanitize_tab() {
        let (sanitized, width) = sanitize_line("hello\tworld");
        assert_eq!(sanitized, "helloworld");
        assert_eq!(width, 10);
    }

    #[test]
    fn test_sanitize_carriage_return() {
        let (sanitized, width) = sanitize_line("hello\rworld");
        assert_eq!(sanitized, "helloworld");
        assert_eq!(width, 10);
    }

    #[test]
    fn test_sanitize_mixed() {
        let (sanitized, width) = sanitize_line("\x1b[32mgreen\x1b[0m\ttext");
        assert_eq!(sanitized, "greentext");
        assert_eq!(width, 9);
    }

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("hello"), "hello");
        assert_eq!(strip_ansi_codes("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(
            strip_ansi_codes("\x1b[1;32mbold green\x1b[0m"),
            "bold green"
        );
        assert_eq!(
            strip_ansi_codes("normal\x1b[33myellow\x1b[0mnormal"),
            "normalyellownormal"
        );
    }
}
