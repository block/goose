use goose::agents::subagent_execution_tool::task_types::{Task, TaskType};
use serde_json::json;

#[test]
fn test_task_type_serialization() {
    // Test that TaskType serializes to the expected string format
    assert_eq!(
        serde_json::to_string(&TaskType::InlineRecipe).unwrap(),
        "\"inline_recipe\""
    );
    assert_eq!(
        serde_json::to_string(&TaskType::SubRecipe).unwrap(),
        "\"sub_recipe\""
    );
}

#[test]
fn test_task_type_deserialization() {
    // Test that strings deserialize to the correct TaskType variants
    assert_eq!(
        serde_json::from_str::<TaskType>("\"inline_recipe\"").unwrap(),
        TaskType::InlineRecipe
    );
    assert_eq!(
        serde_json::from_str::<TaskType>("\"sub_recipe\"").unwrap(),
        TaskType::SubRecipe
    );
}

#[test]
fn test_task_serialization_with_enum() {
    let task = Task {
        id: "test-id".to_string(),
        task_type: TaskType::InlineRecipe,
        payload: json!({"recipe": "test"}),
    };

    let serialized = serde_json::to_value(&task).unwrap();
    assert_eq!(serialized["id"], "test-id");
    assert_eq!(serialized["task_type"], "inline_recipe");
    assert_eq!(serialized["payload"]["recipe"], "test");
}

#[test]
fn test_task_deserialization_with_string() {
    // Test backward compatibility - JSON with string task_type should deserialize
    let json_str = r#"{
        "id": "test-id",
        "task_type": "sub_recipe",
        "payload": {"sub_recipe": {"name": "test"}}
    }"#;

    let task: Task = serde_json::from_str(json_str).unwrap();
    assert_eq!(task.id, "test-id");
    assert_eq!(task.task_type, TaskType::SubRecipe);
}

#[test]
fn test_task_type_display() {
    assert_eq!(TaskType::InlineRecipe.to_string(), "inline_recipe");
    assert_eq!(TaskType::SubRecipe.to_string(), "sub_recipe");
}

#[test]
fn test_task_methods_with_sub_recipe() {
    let task = Task {
        id: "test-1".to_string(),
        task_type: TaskType::SubRecipe,
        payload: json!({
            "sub_recipe": {
                "name": "test_recipe",
                "recipe_path": "/path/to/recipe",
                "command_parameters": {"key": "value"},
                "sequential_when_repeated": true
            }
        }),
    };

    assert!(task.get_sub_recipe().is_some());
    assert_eq!(task.get_sub_recipe_name(), Some("test_recipe"));
    assert_eq!(task.get_sub_recipe_path(), Some("/path/to/recipe"));
    assert!(task.get_command_parameters().is_some());
    assert!(task.get_sequential_when_repeated());
}

#[test]
fn test_task_methods_with_inline_recipe() {
    let task = Task {
        id: "test-3".to_string(),
        task_type: TaskType::InlineRecipe,
        payload: json!({
            "recipe": {
                "instructions": "Test instructions"
            },
            "return_last_only": true
        }),
    };

    assert!(task.get_sub_recipe().is_none());
    assert!(task.get_sub_recipe_name().is_none());
    assert!(task.get_sub_recipe_path().is_none());
    assert!(task.get_command_parameters().is_none());
    assert!(!task.get_sequential_when_repeated());
}

#[test]
fn test_invalid_task_type_deserialization() {
    // Test that invalid task_type strings fail to deserialize
    let result = serde_json::from_str::<TaskType>("\"invalid_type\"");
    assert!(result.is_err());
}

#[test]
fn test_task_with_missing_fields() {
    let task = Task {
        id: "test-4".to_string(),
        task_type: TaskType::SubRecipe,
        payload: json!({}), // Missing sub_recipe field
    };

    assert!(task.get_sub_recipe().is_none());
    assert!(task.get_sub_recipe_name().is_none());
    assert!(task.get_sub_recipe_path().is_none());
    assert!(task.get_command_parameters().is_none());
    assert!(!task.get_sequential_when_repeated());
}
