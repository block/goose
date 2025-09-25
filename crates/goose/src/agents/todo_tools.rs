use indoc::indoc;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;

/// Tool name constant for writing task planner content
pub const TODO_WRITE_TOOL_NAME: &str = "todo__write";

/// Creates a tool for writing task planner content.
///
/// This tool writes or overwrites the entire task planner file with new content.
/// It replaces the complete file content with the provided string.
///
/// # Returns
/// A configured `Tool` instance for writing task planner content
pub fn todo_write_tool() -> Tool {
    Tool::new(
        TODO_WRITE_TOOL_NAME.to_string(),
        indoc! {r#"
            Update your task list, notes, or any information you want to track.
            This content will be automatically available in your context for future turns.
            This tool overwrites the entire existing todo list.
        "#}
        .to_string(),
        object!({
            "type": "object",
            "required": ["content"],
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The complete content to write to the TODO file. This will replace all existing content."
                }
            }
        }),
    )
    .annotate(ToolAnnotations {
        title: Some("Write TODO content".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(true), // It overwrites the entire file
        idempotent_hint: Some(true),  // Writing the same content multiple times has the same effect
        open_world_hint: Some(false),
    })
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_todo_write_tool_creation() {
        let tool = todo_write_tool();

        // Verify tool name
        assert_eq!(tool.name, TODO_WRITE_TOOL_NAME);

        // Verify description exists and is not empty
        assert!(tool.description.is_some());
        let description = tool.description.as_ref().unwrap();
        assert!(!description.is_empty());

        // Verify input schema
        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");

        // Verify required parameters
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "content");

        // Verify properties
        assert!(schema["properties"]["content"].is_object());
        assert_eq!(schema["properties"]["content"]["type"], "string");

        // Verify annotations
        let annotations = tool.annotations.as_ref().unwrap();
        assert_eq!(annotations.title, Some("Write TODO content".to_string()));
        assert_eq!(annotations.read_only_hint, Some(false));
        assert_eq!(annotations.destructive_hint, Some(true));
        assert_eq!(annotations.idempotent_hint, Some(true));
        assert_eq!(annotations.open_world_hint, Some(false));
    }

    #[test]
    fn test_tool_name_constants() {
        // Verify the constant follows the naming pattern
        assert!(TODO_WRITE_TOOL_NAME.starts_with("todo__"));
        assert_eq!(TODO_WRITE_TOOL_NAME, "todo__write");
    }
}
