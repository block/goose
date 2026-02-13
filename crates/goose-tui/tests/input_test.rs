use goose_tui::components::input::replace_input_placeholder;
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
