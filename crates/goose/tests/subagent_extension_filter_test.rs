// Integration tests for the extension filter feature
// These tests verify that tasks can be created with extension filters

use goose::agents::subagent_execution_tool::task_types::{ExtensionFilter, Task};
use serde_json::json;

#[test]
fn test_extension_filter_serialization() {
    // Test that extension filters serialize correctly
    let task = Task {
        id: "test-123".to_string(),
        task_type: "text_instruction".to_string(),
        payload: json!({"text_instruction": "test"}),
        extension_filter: Some(ExtensionFilter::Include {
            extensions: vec!["developer".to_string()],
        }),
    };

    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(json["extension_filter"]["mode"], "include");
    assert_eq!(json["extension_filter"]["extensions"], json!(["developer"]));
}

#[test]
fn test_task_without_extension_filter() {
    // Test backward compatibility - tasks without filters
    let task = Task {
        id: "test-456".to_string(),
        task_type: "text_instruction".to_string(),
        payload: json!({"text_instruction": "test"}),
        extension_filter: None,
    };

    let json = serde_json::to_value(&task).unwrap();
    assert!(!json.as_object().unwrap().contains_key("extension_filter"));
}

#[test]
fn test_extension_filter_modes() {
    // Test Include mode
    let include_filter = ExtensionFilter::Include {
        extensions: vec!["slack".to_string(), "github".to_string()],
    };
    let json = serde_json::to_value(&include_filter).unwrap();
    assert_eq!(json["mode"], "include");
    assert_eq!(json["extensions"], json!(["slack", "github"]));

    // Test Exclude mode
    let exclude_filter = ExtensionFilter::Exclude {
        extensions: vec!["jira".to_string()],
    };
    let json = serde_json::to_value(&exclude_filter).unwrap();
    assert_eq!(json["mode"], "exclude");
    assert_eq!(json["extensions"], json!(["jira"]));

    // Test None mode
    let none_filter = ExtensionFilter::None;
    let json = serde_json::to_value(&none_filter).unwrap();
    assert_eq!(json["mode"], "none");
    assert!(!json.as_object().unwrap().contains_key("extensions"));
}

#[test]
fn test_extension_filter_deserialization() {
    // Test deserializing from JSON
    let json = json!({
        "mode": "include",
        "extensions": ["developer", "slack"]
    });
    let filter: ExtensionFilter = serde_json::from_value(json).unwrap();
    match filter {
        ExtensionFilter::Include { extensions } => {
            assert_eq!(extensions, vec!["developer", "slack"]);
        }
        _ => panic!("Expected Include variant"),
    }

    // Test None mode deserialization
    let json = json!({"mode": "none"});
    let filter: ExtensionFilter = serde_json::from_value(json).unwrap();
    assert!(matches!(filter, ExtensionFilter::None));
}
