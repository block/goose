use super::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_READ_RESOURCE_TOOL_NAME, PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use indoc::indoc;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;

pub const ROUTER_VECTOR_SEARCH_TOOL_NAME: &str = "router__vector_search";
pub const ROUTER_LLM_SEARCH_TOOL_NAME: &str = "router__llm_search";

pub fn vector_search_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_TOOL_NAME.to_string(),
        indoc! {r#"
            Searches for relevant tools based on the user's messages.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            You have the list of extension names available to you in your system prompt.
            Use the extension_name parameter to filter tools by the appropriate extension.
            For example, if the user is asking to list the files in the current directory, you filter for the "developer" extension.
            Example: {"User": "list the files in the current directory", "Query": "list files in current directory", "Extension Name": "developer", "k": 5}
            Extension name is not optional, it is required.
        "#}
        .to_string(),
        object!({
            "type": "object",
            "required": ["query", "extension_name"],
            "properties": {
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5},
                "extension_name": {"type": "string", "description": "Name of the extension to filter tools by"}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("Vector search for relevant tools".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn vector_search_tool_prompt() -> String {
    format!(
        r#"# Tool Selection Instructions
    Important: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the vector search tool to actually retrieve the most relevant tools to use according to the user's messages.
    For example, if the user has 3 extensions enabled, but they are asking for a tool to read a pdf file, \
    you would invoke the vector_search tool to find the most relevant read pdf tool.
    By dynamically enabling tools, you (Goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    Be sure to format the query to search rather than pass in the user's messages directly.
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

pub fn llm_search_tool() -> Tool {
    Tool::new(
        ROUTER_LLM_SEARCH_TOOL_NAME.to_string(),
        indoc! {r#"
            Searches for relevant tools based on the user's messages.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            Use the extension_name parameter to filter tools by the appropriate extension.
            For example, if the user is asking to list the files in the current directory, you filter for the "developer" extension.
            Example: {"User": "list the files in the current directory", "Query": "list files in current directory", "Extension Name": "developer", "k": 5}
            Extension name is not optional, it is required.
            The returned result will be a list of tool names, descriptions, and schemas from which you, the agent can select the most relevant tool to invoke.
        "#}
        .to_string(),
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
    By dynamically enabling tools, you (Goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
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
