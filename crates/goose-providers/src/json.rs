/// Safely parse a JSON string that may contain doubly-encoded or malformed JSON.
/// This function first attempts to parse the input string as-is. If that fails,
/// it applies control character escaping and truncated JSON repair and tries again.
///
/// This approach preserves valid JSON like `{"key1": "value1",\n"key2": "value"}`
/// (which contains a literal \n but is perfectly valid JSON) while still fixing
/// broken JSON like `{"key1": "value1\n","key2": "value"}` (which contains an
/// unescaped newline character).
pub fn safely_parse_json(s: &str) -> Result<serde_json::Value, serde_json::Error> {
    // First, try parsing the string as-is
    match serde_json::from_str(s) {
        Ok(value) => Ok(value),
        Err(_) => {
            for candidate in [
                repair_truncated_json(s),
                json_escape_control_chars_in_string(s),
            ] {
                if let Ok(value) = serde_json::from_str(&candidate) {
                    return Ok(value);
                }
            }

            let repaired = repair_truncated_json(&json_escape_control_chars_in_string(s));
            serde_json::from_str(&repaired)
        }
    }
}

fn repair_truncated_json(s: &str) -> String {
    let mut repaired = String::with_capacity(s.len() + 8);
    let mut in_string = false;
    let mut escape_next = false;
    let mut closers = Vec::new();

    for c in s.chars() {
        repaired.push(c);

        if in_string {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' => escape_next = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match c {
            '"' => in_string = true,
            '{' => closers.push('}'),
            '[' => closers.push(']'),
            '}' | ']' => {
                if closers.last() == Some(&c) {
                    closers.pop();
                }
            }
            _ => {}
        }
    }

    if in_string {
        if escape_next {
            repaired.push('\\');
        }
        repaired.push('"');
    }

    while let Some(closer) = closers.pop() {
        repaired.push(closer);
    }

    repaired
}

/// Helper to escape control characters in a string that is supposed to be a JSON document.
/// This function iterates through the input string `s` and replaces any literal
/// control characters (U+0000 to U+001F) with their JSON-escaped equivalents
/// (e.g., '\n' becomes "\\n", '\u0001' becomes "\\u0001").
///
/// It does NOT escape quotes (") or backslashes (\) because it assumes `s` is a
/// full JSON document, and these characters might be structural (e.g., object delimiters,
/// existing valid escape sequences). The goal is to fix common LLM errors where
/// control characters are emitted raw into what should be JSON string values,
/// making the overall JSON structure unparsable.
///
/// If the input string `s` has other JSON syntax errors (e.g., an unescaped quote
/// *within* a string value like `{"key": "string with " quote"}`), this function
/// will not fix them. It specifically targets unescaped control characters.
pub fn json_escape_control_chars_in_string(s: &str) -> String {
    let mut r = String::with_capacity(s.len()); // Pre-allocate for efficiency
    for c in s.chars() {
        match c {
            // ASCII Control characters (U+0000 to U+001F)
            '\u{0000}'..='\u{001F}' => {
                match c {
                    '\u{0008}' => r.push_str("\\b"), // Backspace
                    '\u{000C}' => r.push_str("\\f"), // Form feed
                    '\n' => r.push_str("\\n"),       // Line feed
                    '\r' => r.push_str("\\r"),       // Carriage return
                    '\t' => r.push_str("\\t"),       // Tab
                    // Other control characters (e.g., NUL, SOH, VT, etc.)
                    // that don't have a specific short escape sequence.
                    _ => {
                        r.push_str(&format!("\\u{:04x}", c as u32));
                    }
                }
            }
            // Other characters are passed through.
            // This includes quotes (") and backslashes (\). If these are part of the
            // JSON structure (e.g. {"key": "value"}) or part of an already correctly
            // escaped sequence within a string value (e.g. "string with \\\" quote"),
            // they are preserved as is. This function does not attempt to fix
            // malformed quote or backslash usage *within* string values if the LLM
            // generates them incorrectly (e.g. {"key": "unescaped " quote in string"}).
            _ => r.push(c),
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_safely_parse_json() {
        // Test valid JSON that should parse without escaping (contains proper escape sequence)
        let valid_json = r#"{"key1": "value1","key2": "value2"}"#;
        let result = safely_parse_json(valid_json).unwrap();
        assert_eq!(result["key1"], "value1");
        assert_eq!(result["key2"], "value2");

        // Test JSON with actual unescaped newlines that needs escaping
        let invalid_json = "{\"key1\": \"value1\n\",\"key2\": \"value2\"}";
        let result = safely_parse_json(invalid_json).unwrap();
        assert_eq!(result["key1"], "value1\n");
        assert_eq!(result["key2"], "value2");

        // Test already valid JSON - should parse on first try
        let good_json = r#"{"test": "value"}"#;
        let result = safely_parse_json(good_json).unwrap();
        assert_eq!(result["test"], "value");

        // Test truncated JSON with unclosed string, object, and array
        let truncated_json = r#"{"key": "unclosed_string","nested": {"items": [1, 2, 3"#;
        let result = safely_parse_json(truncated_json).unwrap();
        assert_eq!(result["key"], "unclosed_string");
        assert_eq!(result["nested"]["items"], json!([1, 2, 3]));

        // Test dangling backslash at end of a truncated string
        let dangling_escape_json = String::from(r#"{"path":"abc\"#);
        let result = safely_parse_json(&dangling_escape_json).unwrap();
        assert_eq!(result["path"], "abc\\");

        // Test empty object
        let empty_json = "{}";
        let result = safely_parse_json(empty_json).unwrap();
        assert!(result.as_object().unwrap().is_empty());

        // Test JSON with escaped newlines (valid JSON) - should parse on first try
        let escaped_json = r#"{"key": "value with\nnewline"}"#;
        let result = safely_parse_json(escaped_json).unwrap();
        assert_eq!(result["key"], "value with\nnewline");
    }

    #[test]
    fn test_json_escape_control_chars_in_string() {
        // Test basic control character escaping
        assert_eq!(
            json_escape_control_chars_in_string("Hello\nWorld"),
            "Hello\\nWorld"
        );
        assert_eq!(
            json_escape_control_chars_in_string("Hello\tWorld"),
            "Hello\\tWorld"
        );
        assert_eq!(
            json_escape_control_chars_in_string("Hello\rWorld"),
            "Hello\\rWorld"
        );

        // Test multiple control characters
        assert_eq!(
            json_escape_control_chars_in_string("Hello\n\tWorld\r"),
            "Hello\\n\\tWorld\\r"
        );

        // Test that quotes and backslashes are preserved (not escaped)
        assert_eq!(
            json_escape_control_chars_in_string("Hello \"World\""),
            "Hello \"World\""
        );
        assert_eq!(
            json_escape_control_chars_in_string("Hello\\World"),
            "Hello\\World"
        );

        // Test JSON-like string with control characters
        assert_eq!(
            json_escape_control_chars_in_string("{\"message\": \"Hello\nWorld\"}"),
            "{\"message\": \"Hello\\nWorld\"}"
        );

        // Test no changes for normal strings
        assert_eq!(
            json_escape_control_chars_in_string("Hello World"),
            "Hello World"
        );

        // Test other control characters get unicode escapes
        assert_eq!(
            json_escape_control_chars_in_string("Hello\u{0001}World"),
            "Hello\\u0001World"
        );
    }
}
