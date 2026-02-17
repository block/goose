//! Utility functions for the summon extension — timing, formatting, concurrency limits.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Round a duration to human-friendly format (10s, 20s, 1m, 2m, etc.)
pub fn round_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", (secs / 10) * 10)
    } else {
        format!("{}m", secs / 60)
    }
}

/// Current time as milliseconds since UNIX epoch.
pub fn current_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Get maximum number of concurrent background tasks from env or default.
pub fn max_background_tasks() -> usize {
    std::env::var("GOOSE_MAX_BACKGROUND_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5)
}

/// Check if a string looks like a Goose session ID (e.g., "20240101_abc123").
pub fn is_session_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('_').collect();
    parts.len() == 2 && parts[0].len() == 8 && parts[0].chars().all(|c| c.is_ascii_digit())
}

/// Truncate a string to max_len, adding "..." suffix if truncated.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_duration() {
        assert_eq!(round_duration(Duration::from_secs(5)), "0s");
        assert_eq!(round_duration(Duration::from_secs(15)), "10s");
        assert_eq!(round_duration(Duration::from_secs(65)), "1m");
        assert_eq!(round_duration(Duration::from_secs(125)), "2m");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 2), "hi");
        assert_eq!(truncate("hello", 4), "h...");
    }

    #[test]
    fn test_is_session_id() {
        assert!(is_session_id("20240101_abc123"));
        assert!(!is_session_id("not_a_session"));
        assert!(!is_session_id("abc"));
    }

    #[test]
    fn test_current_epoch_millis() {
        let ms = current_epoch_millis();
        assert!(ms > 1_700_000_000_000); // After ~Nov 2023
    }
}
