//! Built-in tools that are always available to the agent
//!
//! These tools allow the agent to manage extensions dynamically.

use rig::completion::ToolDefinition;
use serde_json::json;

/// Tool names for built-in extension management
pub const ENABLE_EXTENSION: &str = "platform__enable_extension";
pub const DISABLE_EXTENSION: &str = "platform__disable_extension";

/// Get the built-in tool definitions that are always available.
///
/// These tools allow the agent to enable/disable extensions at runtime.
pub fn builtin_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: ENABLE_EXTENSION.to_string(),
            description: "Enable an extension by name. Once enabled, the extension's tools become available. Use the available extensions list in your system prompt to see what can be enabled.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the extension to enable"
                    }
                },
                "required": ["name"]
            }),
        },
        ToolDefinition {
            name: DISABLE_EXTENSION.to_string(),
            description: "Disable a currently enabled extension. Its tools will no longer be available.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the extension to disable"
                    }
                },
                "required": ["name"]
            }),
        },
    ]
}

/// Check if a tool name is a built-in tool
pub fn is_builtin_tool(name: &str) -> bool {
    matches!(name, ENABLE_EXTENSION | DISABLE_EXTENSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools() {
        let tools = builtin_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t.name == ENABLE_EXTENSION));
        assert!(tools.iter().any(|t| t.name == DISABLE_EXTENSION));
    }

    #[test]
    fn test_is_builtin_tool() {
        assert!(is_builtin_tool(ENABLE_EXTENSION));
        assert!(is_builtin_tool(DISABLE_EXTENSION));
        assert!(!is_builtin_tool("develop__shell"));
        assert!(!is_builtin_tool("random_tool"));
    }
}
