use goose_tui::hidden_blocks::strip_hidden_blocks;
use goose_tui::utils::json::has_input_placeholder;
use goose_tui::utils::sanitize::{sanitize_line, strip_ansi_codes};
use goose_tui::utils::styles::{breathing_color, color_to_rgb};
use ratatui::style::Color;
use serde_json::json;

#[test]
fn has_input_placeholder_string() {
    assert!(has_input_placeholder(&json!("{input}")));
    assert!(has_input_placeholder(&json!("prefix {input} suffix")));
    assert!(!has_input_placeholder(&json!("no placeholder")));
}

#[test]
fn has_input_placeholder_nested_object() {
    assert!(has_input_placeholder(&json!({
        "outer": {
            "inner": "{input}"
        }
    })));
    assert!(!has_input_placeholder(&json!({
        "outer": {
            "inner": "no placeholder"
        }
    })));
}

#[test]
fn has_input_placeholder_array() {
    assert!(has_input_placeholder(&json!(["a", "{input}", "b"])));
    assert!(!has_input_placeholder(&json!(["a", "b", "c"])));
}

#[test]
fn has_input_placeholder_mixed() {
    assert!(has_input_placeholder(&json!({
        "array": [{"nested": "{input}"}]
    })));
}

#[test]
fn color_to_rgb_extracts_rgb() {
    assert_eq!(color_to_rgb(Color::Rgb(100, 150, 200)), (100, 150, 200));
}

#[test]
fn color_to_rgb_fallback_for_non_rgb() {
    assert_eq!(color_to_rgb(Color::Red), (128, 128, 128));
    assert_eq!(color_to_rgb(Color::Reset), (128, 128, 128));
}

#[test]
fn breathing_color_inactive_returns_base() {
    let base = Color::Rgb(100, 100, 100);
    let result = breathing_color(base, 0, false);
    assert_eq!(result, Color::Rgb(100, 100, 100));
}

#[test]
fn breathing_color_active_oscillates() {
    let base = Color::Rgb(100, 100, 100);
    let frame0 = breathing_color(base, 0, true);
    let frame10 = breathing_color(base, 10, true);
    assert_ne!(frame0, frame10);
}

// ============================================================================
// hidden_blocks tests
// ============================================================================

#[test]
fn strips_hidden_blocks_with_both_tags() {
    let with_both = "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage\n\n<attached_files_goose_tui>\nfiles\n</attached_files_goose_tui>";
    assert_eq!(strip_hidden_blocks(with_both, true), "Message");
}

#[test]
fn strips_hidden_blocks_preserves_cwd_when_not_first() {
    let with_both = "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage\n\n<attached_files_goose_tui>\nfiles\n</attached_files_goose_tui>";
    assert_eq!(
        strip_hidden_blocks(with_both, false),
        "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage"
    );
}

#[test]
fn strips_hidden_blocks_plain_text_unchanged() {
    assert_eq!(strip_hidden_blocks("plain text", true), "plain text");
}

// ============================================================================
// sanitize tests
// ============================================================================

#[test]
fn sanitize_line_plain_text() {
    let (sanitized, width) = sanitize_line("hello world");
    assert_eq!(sanitized, "hello world");
    assert_eq!(width, 11);
}

#[test]
fn sanitize_line_ansi_codes() {
    let (sanitized, width) = sanitize_line("\x1b[31mred text\x1b[0m");
    assert_eq!(sanitized, "red text");
    assert_eq!(width, 8);
}

#[test]
fn sanitize_line_tab() {
    let (sanitized, width) = sanitize_line("hello\tworld");
    assert_eq!(sanitized, "helloworld");
    assert_eq!(width, 10);
}

#[test]
fn sanitize_line_carriage_return() {
    let (sanitized, width) = sanitize_line("hello\rworld");
    assert_eq!(sanitized, "helloworld");
    assert_eq!(width, 10);
}

#[test]
fn sanitize_line_mixed() {
    let (sanitized, width) = sanitize_line("\x1b[32mgreen\x1b[0m\ttext");
    assert_eq!(sanitized, "greentext");
    assert_eq!(width, 9);
}

#[test]
fn strip_ansi_codes_plain() {
    assert_eq!(strip_ansi_codes("hello"), "hello");
}

#[test]
fn strip_ansi_codes_colored() {
    assert_eq!(strip_ansi_codes("\x1b[31mred\x1b[0m"), "red");
}

#[test]
fn strip_ansi_codes_bold_colored() {
    assert_eq!(
        strip_ansi_codes("\x1b[1;32mbold green\x1b[0m"),
        "bold green"
    );
}

#[test]
fn strip_ansi_codes_mixed() {
    assert_eq!(
        strip_ansi_codes("normal\x1b[33myellow\x1b[0mnormal"),
        "normalyellownormal"
    );
}
