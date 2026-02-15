//! Tool filtering based on active mode's tool groups.
//!
//! When a mode is active, only tools matching the mode's `tool_groups` are
//! available to the LLM. This prevents a "PM" mode from accessing shell commands,
//! or a "read-only" mode from using the text editor.

use crate::registry::manifest::ToolGroupAccess;
use rmcp::model::Tool;

use super::extension_manager::get_tool_owner;

/// Filters a list of tools based on active tool groups.
///
/// If `groups` is empty, all tools pass through (no filtering).
/// Otherwise, a tool is included if it matches ANY of the groups.
pub fn filter_tools(tools: Vec<Tool>, groups: &[ToolGroupAccess]) -> Vec<Tool> {
    if groups.is_empty() {
        return tools;
    }

    tools
        .into_iter()
        .filter(|tool| tool_matches_any_group(tool, groups))
        .collect()
}

fn tool_matches_any_group(tool: &Tool, groups: &[ToolGroupAccess]) -> bool {
    for group in groups {
        match group {
            ToolGroupAccess::Full(name) => {
                if tool_matches_group(tool, name) {
                    return true;
                }
            }
            ToolGroupAccess::Restricted { group, file_regex } => {
                if tool_matches_group(tool, group) {
                    let _ = file_regex;
                    return true;
                }
            }
        }
    }
    false
}

/// Match a tool against a named group.
///
/// - `"mcp"` — wildcard, matches ALL tools
/// - Extension names (`"developer"`, `"memory"`, etc.) — matches by owner
/// - Abstract groups:
///   - `"command"` — shell, command, terminal tools
///   - `"edit"` — text_editor, write tools
///   - `"read"` — read, list, search, view tools
///   - `"fetch"` — fetch, http tools
///   - `"browser"` — computercontroller, screen tools
fn tool_matches_group(tool: &Tool, group_name: &str) -> bool {
    let tool_name: &str = &tool.name;
    let owner = get_tool_owner(tool).unwrap_or_default();

    match group_name {
        "mcp" => true,

        "none" => false,

        "orchestrator" => super::extension::is_orchestrator_extension(&owner),

        "developer" | "memory" | "computercontroller" | "code_execution" => owner == group_name,

        "command" => {
            owner == "developer"
                && (tool_name.contains("shell")
                    || tool_name.contains("command")
                    || tool_name.contains("terminal"))
        }

        "edit" => {
            owner == "developer"
                && (tool_name.contains("editor")
                    || tool_name.contains("write")
                    || tool_name.contains("create"))
        }

        "read" => {
            owner == "developer"
                && (tool_name.contains("read")
                    || tool_name.contains("list")
                    || tool_name.contains("search")
                    || tool_name.contains("view")
                    || tool_name.contains("cat"))
        }

        "fetch" => {
            owner.contains("fetch") || tool_name.contains("fetch") || tool_name.contains("http")
        }

        "browser" => {
            owner == "computercontroller"
                || owner.contains("chrome")
                || tool_name.contains("screen")
                || tool_name.contains("browser")
                || tool_name.contains("screenshot")
        }

        other => owner == other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use serde_json::json;

    fn make_tool(name: &str, owner: &str) -> Tool {
        let schema: std::sync::Arc<serde_json::Map<String, serde_json::Value>> =
            std::sync::Arc::new(serde_json::Map::new());
        let mut tool = Tool::new(name.to_string(), "desc".to_string(), schema);
        let meta_val = json!({ "goose_extension": owner });
        let meta_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(meta_val).unwrap();
        tool.meta = Some(rmcp::model::Meta(meta_map));
        tool
    }

    #[test]
    fn test_empty_groups_passes_all() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
        ];
        let result = filter_tools(tools, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_mcp_wildcard_passes_all() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
            make_tool("github__pr_list", "github"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("mcp".into())]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_extension_name_filter() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("developer__text_editor", "developer"),
            make_tool("memory__search", "memory"),
            make_tool("github__pr_list", "github"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("developer".into())]);
        assert_eq!(result.len(), 2);
        let names: Vec<&str> = result.iter().map(|t| &*t.name).collect();
        assert!(names.iter().all(|n| n.starts_with("developer")));
    }

    #[test]
    fn test_command_group() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("developer__text_editor", "developer"),
            make_tool("memory__search", "memory"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("command".into())]);
        assert_eq!(result.len(), 1);
        assert_eq!(&*result[0].name, "developer__shell");
    }

    #[test]
    fn test_multiple_groups_union() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("developer__text_editor", "developer"),
            make_tool("memory__search", "memory"),
            make_tool("github__pr_list", "github"),
        ];
        let result = filter_tools(
            tools,
            &[
                ToolGroupAccess::Full("command".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_read_group() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("developer__text_editor", "developer"),
            make_tool("developer__read_file", "developer"),
            make_tool("developer__list_directory", "developer"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("read".into())]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_unknown_group_matches_extension_name() {
        let tools = vec![
            make_tool("context7__lookup", "context7"),
            make_tool("developer__shell", "developer"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("context7".into())]);
        assert_eq!(result.len(), 1);
        assert_eq!(&*result[0].name, "context7__lookup");
    }

    #[test]
    fn test_none_group_matches_nothing() {
        let tools = vec![
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
            make_tool("summon__delegate", "summon"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("none".into())]);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_orchestrator_group_matches_orchestrator_extensions() {
        let tools = vec![
            make_tool("summon__delegate", "summon"),
            make_tool("extensionmanager__manage", "extensionmanager"),
            make_tool("chatrecall__search", "chatrecall"),
            make_tool("tom__context", "tom"),
            make_tool("developer__shell", "developer"),
            make_tool("memory__search", "memory"),
        ];
        let result = filter_tools(tools, &[ToolGroupAccess::Full("orchestrator".into())]);
        assert_eq!(result.len(), 4);
    }
}
