use goose_tui::hidden_blocks::strip_hidden_blocks;
use goose_tui::utils::json::has_input_placeholder;
use goose_tui::utils::sanitize::{sanitize_line, strip_ansi_codes};
use goose_tui::utils::styles::{breathing_color, color_to_rgb};
use ratatui::style::Color;
use serde_json::json;

#[test]
fn has_input_placeholder_finds_in_strings() {
    assert!(has_input_placeholder(&json!("{input}")));
    assert!(has_input_placeholder(&json!("prefix {input} suffix")));
    assert!(!has_input_placeholder(&json!("no placeholder")));
}

#[test]
fn has_input_placeholder_finds_in_nested_structures() {
    assert!(has_input_placeholder(
        &json!({"outer": {"inner": "{input}"}})
    ));
    assert!(has_input_placeholder(&json!(["a", "{input}", "b"])));
    assert!(has_input_placeholder(
        &json!({"array": [{"nested": "{input}"}]})
    ));
    assert!(!has_input_placeholder(&json!({"outer": {"inner": "none"}})));
}

#[test]
fn color_to_rgb_extracts_components() {
    assert_eq!(color_to_rgb(Color::Rgb(100, 150, 200)), (100, 150, 200));
    assert_eq!(color_to_rgb(Color::Red), (128, 128, 128));
}

#[test]
fn breathing_color_animates_when_active() {
    let base = Color::Rgb(100, 100, 100);
    assert_eq!(breathing_color(base, 0, false), Color::Rgb(100, 100, 100));
    assert_ne!(
        breathing_color(base, 0, true),
        breathing_color(base, 10, true)
    );
}

#[test]
fn strip_hidden_blocks_removes_tags() {
    let with_both = "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage\n\n<attached_files_goose_tui>\nfiles\n</attached_files_goose_tui>";
    assert_eq!(strip_hidden_blocks(with_both, true), "Message");
    assert_eq!(
        strip_hidden_blocks(with_both, false),
        "<cwd_analysis_goose_tui>\nanalysis\n</cwd_analysis_goose_tui>\n\nMessage"
    );
    assert_eq!(strip_hidden_blocks("plain text", true), "plain text");
}

#[test]
fn sanitize_line_strips_ansi_and_control_chars() {
    let (text, width) = sanitize_line("hello world");
    assert_eq!(text, "hello world");
    assert_eq!(width, 11);

    let (text, width) = sanitize_line("\x1b[31mred text\x1b[0m");
    assert_eq!(text, "red text");
    assert_eq!(width, 8);

    let (text, _) = sanitize_line("hello\tworld");
    assert_eq!(text, "helloworld");

    let (text, _) = sanitize_line("hello\rworld");
    assert_eq!(text, "helloworld");
}

#[test]
fn strip_ansi_codes_removes_escape_sequences() {
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
