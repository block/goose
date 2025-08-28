use std::fs::OpenOptions;
use std::io::Write;

use crate::logging;

/// Log a debug event to the goose-debug.log file
///
/// This function logs timestamped events to a debug log file in the appropriate
/// platform-specific logs directory.
///
/// # Arguments
///
/// * `event` - The event string to log
/// ```
pub fn log_debug_event(event: &str) {
    let logs_dir = match logging::get_log_directory("debug", false) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get log directory: {}", e);
            return;
        }
    };

    let log_file = logs_dir.join("goose-debug.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
        let _ = writeln!(file, "{} - {}", timestamp, event);
    }
}

/// Log a debug event with JSON payload to the goose-debug.log file
///
/// This function logs timestamped events with associated JSON data to a debug log file.
///
/// # Arguments
///
/// * `event` - The event string to log
/// * `json_data` - JSON data to log alongside the event
/// ```
pub fn log_debug_event_with_json(event: &str, json_data: &serde_json::Value) {
    // Use the unified logging module to get the debug log directory (no date subdirs)
    let logs_dir = match logging::get_log_directory("debug", false) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get log directory: {}", e);
            return;
        }
    };

    let log_file = logs_dir.join("goose-debug.log");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
        // Format JSON compactly on a single line
        let json_str = serde_json::to_string(json_data).unwrap_or_else(|_| "{}".to_string());
        let _ = writeln!(file, "{} - {} - {}", timestamp, event, json_str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_debug_event() {
        // This test just ensures the function doesn't panic
        log_debug_event("TEST_EVENT");
        log_debug_event(&format!("TEST_EVENT_WITH_DATA: {}", "test_data"));
    }

    #[test]
    fn test_log_debug_event_with_json() {
        use serde_json::json;

        // This test just ensures the function doesn't panic
        log_debug_event_with_json(
            "TEST_JSON_EVENT",
            &json!({
                "test_key": "test_value",
                "test_number": 42
            }),
        );
    }
}
