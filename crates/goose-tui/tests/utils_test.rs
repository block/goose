use goose_tui::utils::json::has_input_placeholder;
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
