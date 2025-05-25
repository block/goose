use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const ROUTER_VECTOR_SEARCH_TOOL_NAME: &str = "router__vector_search";

pub fn vector_search_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_TOOL_NAME.to_string(),
        indoc! {r#"
            Searches for relevant tools based on the user's messages.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            Examples:
            - {"User": "what is the weather in Tokyo?", "Query": "weather in Tokyo"}
            - {"User": "read this pdf file for me", "Query": "read pdf file"}
            - {"User": "run this command ls -l in the terminal", "Query": "run command in terminal ls -l"}
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Vector search for relevant tools".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn vector_search_tool_prompt() -> String {
    r#"# Tool Selection Instructions
    Imporant: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the vector search tool to actually retrieve the most relevant tools to use according to the user's messages.
    For example, if the user has 3 extensions enabled, but they are asking for a tool to read a pdf file, \
    you would invoke the vector_search tool to find the most relevant read pdf tool.
    By dynamically enabling tools, you (Goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    Be sure to format the query to search rather than pass in the user's messages directly."#.to_string()
}
