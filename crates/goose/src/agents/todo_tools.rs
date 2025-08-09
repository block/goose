use indoc::indoc;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;

/// Tool name constant for reading task planner content
pub const TODO_READ_TOOL_NAME: &str = "todo__read";

/// Tool name constant for writing task planner content
pub const TODO_WRITE_TOOL_NAME: &str = "todo__write";

/// Creates a tool for reading task planner content.
///
/// This tool reads the entire task planner file content as a string.
/// It is marked as read-only and safe to use repeatedly.
///
/// # Returns
/// A configured `Tool` instance for reading task planner content
pub fn todo_read_tool() -> Tool {
    Tool::new(
        TODO_READ_TOOL_NAME.to_string(),
        indoc! {r#"
            Read the entire TODO file content.

            This tool reads the complete TODO file and returns its content as a string.
            Use this to view current tasks, notes, and any other information stored in the TODO file.

            The tool will return an error if the TODO file doesn't exist or cannot be read.
        "#}
        .to_string(),
        object!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
    )
    .annotate(ToolAnnotations {
        title: Some("Read TODO content".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    })
}

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
            Write or overwrite the entire TODO file content.

            This tool replaces the complete TODO file content with the provided string.
            Use this to update tasks, add new items, or reorganize the TODO file.

            WARNING: This operation completely replaces the file content. Make sure to include
            all content you want to keep, not just the changes.

            The tool will create the TODO file if it doesn't exist, or overwrite it if it does.
            Returns an error if the file cannot be written due to permissions or other I/O issues.
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
mod tests {
    use super::*;

    #[test]
    fn test_todo_read_tool_creation() {
        let tool = todo_read_tool();

        // Verify tool name
        assert_eq!(tool.name, TODO_READ_TOOL_NAME);

        // Verify description exists and is not empty
        assert!(tool.description.is_some());
        let description = tool.description.as_ref().unwrap();
        assert!(!description.is_empty());

        // Verify input schema
        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"].as_array().unwrap().len(), 0);

        // Verify annotations
        let annotations = tool.annotations.as_ref().unwrap();
        assert_eq!(annotations.title, Some("Read TODO content".to_string()));
        assert_eq!(annotations.read_only_hint, Some(true));
        assert_eq!(annotations.destructive_hint, Some(false));
        assert_eq!(annotations.idempotent_hint, Some(true));
        assert_eq!(annotations.open_world_hint, Some(false));
    }

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
        // Verify the constants follow the naming pattern
        assert!(TODO_READ_TOOL_NAME.starts_with("todo__"));
        assert!(TODO_WRITE_TOOL_NAME.starts_with("todo__"));
        assert_eq!(TODO_READ_TOOL_NAME, "todo__read");
        assert_eq!(TODO_WRITE_TOOL_NAME, "todo__write");
    }
}
