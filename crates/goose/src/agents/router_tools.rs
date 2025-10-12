use super::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_READ_RESOURCE_TOOL_NAME, PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;

pub const ROUTER_LLM_SEARCH_TOOL_NAME: &str = "router__llm_search";

pub fn llm_search_tool() -> Tool {
    Tool::new(
        ROUTER_LLM_SEARCH_TOOL_NAME.to_string(),
        "Search tools by user intent. Focus on keywords, especially last message. Required: extension_name to filter (e.g., \"developer\" for file ops). Returns tool names/descriptions/schemas for selection.".to_string(),
        object!({
            "type": "object",
            "required": ["query", "extension_name"],
            "properties": {
                "extension_name": {"type": "string", "description": "The name of the extension to filter tools by"},
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("LLM search for relevant tools".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn llm_search_tool_prompt() -> String {
    format!(
        r#"# LLM Tool Selection Instructions
    Important: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the llm search tool to actually retrieve the most relevant tools to use according to the user's messages.
    For example, if the user has 3 extensions enabled, but they are asking for a tool to read a pdf file, \
    you would invoke the llm_search tool to find the most relevant read pdf tool.
    By dynamically enabling tools, you (goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    Be sure to format a query packed with relevant keywords to search for the most relevant tools.
    In addition to the extension names available to you, you also have platform extension tools available to you.
    The platform extension contains the following tools:
    - {}
    - {}
    - {}
    - {}
    "#,
        PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
        PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
        PLATFORM_READ_RESOURCE_TOOL_NAME,
        PLATFORM_LIST_RESOURCES_TOOL_NAME
    )
}
