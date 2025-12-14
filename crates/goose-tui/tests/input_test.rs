use goose_tui::components::input::{
    is_builtin_command, parse_slash_command, replace_input_placeholder, should_add_to_history,
    MAX_HISTORY_ENTRY_SIZE,
};
use serde_json::json;

#[test]
fn replace_placeholder_in_strings() {
    assert_eq!(
        replace_input_placeholder(&json!("echo {input}"), "hello"),
        json!("echo hello")
    );
    assert_eq!(
        replace_input_placeholder(&json!("{input} and {input}"), "X"),
        json!("X and X")
    );
    assert_eq!(
        replace_input_placeholder(&json!("no placeholder"), "ignored"),
        json!("no placeholder")
    );
    assert_eq!(
        replace_input_placeholder(&json!("prefix {input} suffix"), ""),
        json!("prefix  suffix")
    );
}

#[test]
fn replace_placeholder_in_nested_objects() {
    let args = json!({
        "cmd": "{input}",
        "opts": {"file": "{input}.txt"}
    });

    assert_eq!(
        replace_input_placeholder(&args, "test"),
        json!({
            "cmd": "test",
            "opts": {"file": "test.txt"}
        })
    );
}

#[test]
fn replace_placeholder_in_arrays() {
    assert_eq!(
        replace_input_placeholder(&json!(["{input}", "other", "{input}"]), "value"),
        json!(["value", "other", "value"])
    );
}

#[test]
fn replace_placeholder_preserves_non_strings() {
    let args = json!({
        "count": 42,
        "enabled": true,
        "name": "{input}",
        "ratio": 2.5,
        "nothing": null
    });

    let result = replace_input_placeholder(&args, "test");

    assert_eq!(result["count"], 42);
    assert_eq!(result["enabled"], true);
    assert_eq!(result["name"], "test");
    assert_eq!(result["ratio"], 2.5);
    assert!(result["nothing"].is_null());
}

#[test]
fn replace_placeholder_handles_deep_nesting() {
    let args = json!({
        "level1": {
            "level2": {
                "level3": {"value": "{input}"}
            }
        }
    });

    let result = replace_input_placeholder(&args, "deep");

    assert_eq!(result["level1"]["level2"]["level3"]["value"], "deep");
}

#[test]
fn replace_placeholder_handles_mixed_structures() {
    let args = json!({
        "items": [
            {"name": "{input}"},
            {"name": "static"}
        ]
    });

    let result = replace_input_placeholder(&args, "dynamic");

    assert_eq!(result["items"][0]["name"], "dynamic");
    assert_eq!(result["items"][1]["name"], "static");
}

#[test]
fn should_add_to_history_rejects_empty_and_whitespace() {
    assert!(!should_add_to_history("", None));
    assert!(!should_add_to_history("   ", None));
    assert!(!should_add_to_history("\t\n", None));
}

#[test]
fn should_add_to_history_rejects_duplicates() {
    assert!(!should_add_to_history("hello", Some("hello")));
    assert!(!should_add_to_history("  hello  ", Some("hello")));
}

#[test]
fn should_add_to_history_accepts_new_entries() {
    assert!(should_add_to_history("hello", None));
    assert!(should_add_to_history("hello", Some("world")));
}

#[test]
fn should_add_to_history_rejects_oversized() {
    let huge = "x".repeat(MAX_HISTORY_ENTRY_SIZE + 1);
    assert!(!should_add_to_history(&huge, None));
}

#[test]
fn parse_slash_command_extracts_command_and_args() {
    assert_eq!(parse_slash_command("/exit"), Some(("/exit", "")));
    assert_eq!(parse_slash_command("/theme dark"), Some(("/theme", "dark")));
    assert_eq!(
        parse_slash_command("/mode auto approve"),
        Some(("/mode", "auto approve"))
    );
}

#[test]
fn parse_slash_command_returns_none_for_non_commands() {
    assert_eq!(parse_slash_command("hello"), None);
    assert_eq!(parse_slash_command(""), None);
    assert_eq!(parse_slash_command("  "), None);
}

#[test]
fn is_builtin_command_recognizes_all_builtins() {
    assert!(is_builtin_command("/exit"));
    assert!(is_builtin_command("/quit"));
    assert!(is_builtin_command("/help"));
    assert!(is_builtin_command("/config"));
    assert!(is_builtin_command("/theme"));
    assert!(is_builtin_command("/mode"));
    assert!(is_builtin_command("/clear"));
    assert!(is_builtin_command("/compact"));
}

#[test]
fn is_builtin_command_rejects_unknown() {
    assert!(!is_builtin_command("/foo"));
    assert!(!is_builtin_command("/unknown"));
    assert!(!is_builtin_command("exit"));
}
