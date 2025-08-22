use tokio_util::sync::CancellationToken;
use unicode_normalization::UnicodeNormalization;

/// Check if a character is in the Unicode Tags Block range (U+E0000-U+E007F)
/// These characters are invisible and can be used for steganographic attacks
fn is_in_unicode_tag_range(c: char) -> bool {
    matches!(c, '\u{E0000}'..='\u{E007F}')
}

pub fn contains_unicode_tags(text: &str) -> bool {
    text.chars().any(is_in_unicode_tag_range)
}

/// Sanitize Unicode Tags Block characters from text
pub fn sanitize_unicode_tags(text: &str) -> String {
    let normalized: String = text.nfc().collect();

    normalized
        .chars()
        .filter(|&c| !is_in_unicode_tag_range(c))
        .collect()
}

/// Safely truncate a string at character boundaries, not byte boundaries
///
/// This function ensures that multi-byte UTF-8 characters (like Japanese, emoji, etc.)
/// are not split in the middle, which would cause a panic.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep
///
/// # Returns
/// A truncated string with "..." appended if truncation occurred
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

pub fn is_token_cancelled(cancellation_token: &Option<CancellationToken>) -> bool {
    cancellation_token
        .as_ref()
        .is_some_and(|t| t.is_cancelled())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_unicode_tags() {
        // Test detection of Unicode Tags Block characters
        assert!(contains_unicode_tags("Hello\u{E0041}world"));
        assert!(contains_unicode_tags("\u{E0000}"));
        assert!(contains_unicode_tags("\u{E007F}"));
        assert!(!contains_unicode_tags("Hello world"));
        assert!(!contains_unicode_tags("Hello 世界 🌍"));
        assert!(!contains_unicode_tags(""));
    }

    #[test]
    fn test_sanitize_unicode_tags() {
        // Test that Unicode Tags Block characters are removed
        let malicious = "Hello\u{E0041}\u{E0042}\u{E0043}world"; // Invisible "ABC"
        let cleaned = sanitize_unicode_tags(malicious);
        assert_eq!(cleaned, "Helloworld");
    }

    #[test]
    fn test_sanitize_unicode_tags_preserves_legitimate_unicode() {
        // Test that legitimate Unicode characters are preserved
        let clean_text = "Hello world 世界 🌍";
        let cleaned = sanitize_unicode_tags(clean_text);
        assert_eq!(cleaned, clean_text);
    }

    #[test]
    fn test_sanitize_unicode_tags_empty_string() {
        let empty = "";
        let cleaned = sanitize_unicode_tags(empty);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_sanitize_unicode_tags_only_malicious() {
        // Test string containing only Unicode Tags characters
        let only_malicious = "\u{E0041}\u{E0042}\u{E0043}";
        let cleaned = sanitize_unicode_tags(only_malicious);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_sanitize_unicode_tags_mixed_content() {
        // Test mixed legitimate and malicious Unicode
        let mixed = "Hello\u{E0041} 世界\u{E0042} 🌍\u{E0043}!";
        let cleaned = sanitize_unicode_tags(mixed);
        assert_eq!(cleaned, "Hello 世界 🌍!");
    }

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 20), "hello world");
        assert_eq!(safe_truncate("hello world", 8), "hello...");
        assert_eq!(safe_truncate("hello", 5), "hello");
        assert_eq!(safe_truncate("hello", 3), "...");
    }

    #[test]
    fn test_safe_truncate_japanese() {
        // Japanese characters: "こんにちは世界" (Hello World)
        let japanese = "こんにちは世界";
        assert_eq!(safe_truncate(japanese, 10), japanese);
        assert_eq!(safe_truncate(japanese, 5), "こん...");
        assert_eq!(safe_truncate(japanese, 7), japanese);
    }

    #[test]
    fn test_safe_truncate_mixed() {
        // Mixed ASCII and Japanese
        let mixed = "Hello こんにちは";
        assert_eq!(safe_truncate(mixed, 20), mixed);
        assert_eq!(safe_truncate(mixed, 8), "Hello...");
    }
}
