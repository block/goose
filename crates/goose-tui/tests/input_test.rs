use goose_tui::components::input::replace_input_placeholder;
use serde_json::json;

// ============================================================================
// replace_input_placeholder tests
// ============================================================================

#[test]
fn replace_placeholder_in_string() {
    let args = json!("echo {input}");

    let result = replace_input_placeholder(&args, "hello");

    assert_eq!(result, json!("echo hello"));
}

#[test]
fn replace_placeholder_in_nested_object() {
    let args = json!({
        "cmd": "{input}",
        "opts": {
            "file": "{input}.txt"
        }
    });

    let result = replace_input_placeholder(&args, "test");

    assert_eq!(
        result,
        json!({
            "cmd": "test",
            "opts": {
                "file": "test.txt"
            }
        })
    );
}

#[test]
fn replace_placeholder_in_array() {
    let args = json!(["{input}", "other", "{input}"]);

    let result = replace_input_placeholder(&args, "value");

    assert_eq!(result, json!(["value", "other", "value"]));
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

    assert_eq!(
        result,
        json!({
            "count": 42,
            "enabled": true,
            "name": "test",
            "ratio": 2.5,
            "nothing": null
        })
    );
}

#[test]
fn replace_placeholder_handles_multiple_occurrences() {
    let args = json!("{input} and {input} again");

    let result = replace_input_placeholder(&args, "X");

    assert_eq!(result, json!("X and X again"));
}

#[test]
fn replace_placeholder_handles_no_placeholder() {
    let args = json!("no placeholder here");

    let result = replace_input_placeholder(&args, "ignored");

    assert_eq!(result, json!("no placeholder here"));
}

#[test]
fn replace_placeholder_handles_empty_input() {
    let args = json!("prefix {input} suffix");

    let result = replace_input_placeholder(&args, "");

    assert_eq!(result, json!("prefix  suffix"));
}

#[test]
fn replace_placeholder_deeply_nested() {
    let args = json!({
        "level1": {
            "level2": {
                "level3": {
                    "value": "{input}"
                }
            }
        }
    });

    let result = replace_input_placeholder(&args, "deep");

    assert_eq!(
        result,
        json!({
            "level1": {
                "level2": {
                    "level3": {
                        "value": "deep"
                    }
                }
            }
        })
    );
}

#[test]
fn replace_placeholder_mixed_array_and_object() {
    let args = json!({
        "items": [
            {"name": "{input}"},
            {"name": "static"}
        ]
    });

    let result = replace_input_placeholder(&args, "dynamic");

    assert_eq!(
        result,
        json!({
            "items": [
                {"name": "dynamic"},
                {"name": "static"}
            ]
        })
    );
}

#[test]
fn replace_placeholder_special_characters_in_input() {
    let args = json!("run {input}");

    let result = replace_input_placeholder(&args, "echo 'hello world' && ls -la");

    assert_eq!(result, json!("run echo 'hello world' && ls -la"));
}
