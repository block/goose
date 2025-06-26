use super::platform_tools::{
    PLATFORM_LIST_RESOURCES_TOOL_NAME, PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
    PLATFORM_READ_RESOURCE_TOOL_NAME, PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const ROUTER_VECTOR_SEARCH_TOOL_NAME: &str = "router__vector_search";
pub const ROUTER_VECTOR_SEARCH_WITH_EXTENSION_TOOL_NAME: &str =
    "router__vector_search_with_extension";
pub const ROUTER_LLM_SEARCH_TOOL_NAME: &str = "router__llm_search";
pub const ROUTER_VECTOR_SEARCH_PASSTHROUGH_TOOL_NAME: &str = "router__vector_search_passthrough";
pub const ROUTER_VECTOR_SEARCH_WITH_EXTENSION_PASSTHROUGH_TOOL_NAME: &str =
    "router__vector_search_with_extension_passthrough";
pub const ROUTER_LLM_SEARCH_PASSTHROUGH_TOOL_NAME: &str = "router__llm_search_passthrough";
pub const ROUTER_LLM_SEARCH_PARALLEL_TOOL_NAME: &str = "router__llm_search_parallel";

pub fn vector_search_tool_with_extension() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_WITH_EXTENSION_TOOL_NAME.to_string(),
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
        json!({
            "type": "object",
            "required": ["query", "extension_name"],
            "properties": {
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5},
                "extension_name": {"type": "string", "description": "Name of the extension to filter tools by"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Vector search for relevant tools with extension".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn vector_search_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_TOOL_NAME.to_string(),
        indoc! {r#"
            Searches for relevant tools based on the user's messages.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            This tool searches across all available extensions.
            Example: {"User": "list the files in the current directory", "Query": "list files in current directory", "k": 5}
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
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
    Important: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the vector search tool to actually retrieve the most relevant tools to use according to the user's messages.
    For example, if the user has 3 extensions enabled, but they are asking for a tool to read a pdf file, \
    you would invoke the vector_search tool to find the most relevant read pdf tool.
    By dynamically enabling tools, you (Goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    
    CRITICAL: Format strategic search queries focused on tool types and capabilities, not literal user messages.
    - For coding/development questions → query: "programming development code files shell editor debugging"
    - For data/document questions → query: "data processing documents spreadsheet analysis extraction"
    - For system tasks → query: "automation system control file management"
    - For web tasks → query: "web scraping download content extraction"
    
    You have two vector search tools available:
    - router__vector_search: Searches across all available extensions without filtering
    - router__vector_search_with_extension: Searches within a specific extension (requires extension_name parameter)"#.to_string()
}

pub fn vector_search_tool_with_extension_prompt() -> String {
    format!(
        r#"# Tool Selection Instructions (Extension-Specific)
    Important: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the vector search tool with extension to actually retrieve the most relevant tools to use according to the user's messages from a specific extension.
    You have the list of extension names available to you in your system prompt.
    Use the extension_name parameter to filter tools by the appropriate extension.
    By dynamically enabling tools, you (Goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    
    CRITICAL: Format strategic search queries focused on tool types and capabilities within the target extension:
    - For development-focused extensions → query: "programming development code files shell editor debugging"
    - For data/automation-focused extensions → query: "data processing documents automation analysis"
    - For system/platform extensions → query: "extensions configuration management resources"
    
    Choose the right extension based on the user's task domain, then search within it using capability-focused queries.
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
        json!({
            "type": "object",
            "required": ["query", "extension_name"],
            "properties": {
                "extension_name": {"type": "string", "description": "The name of the extension to filter tools by"},
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
            }
        }),
        Some(ToolAnnotations {
            title: Some("LLM search for relevant tools".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
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

pub fn vector_search_passthrough_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_PASSTHROUGH_TOOL_NAME.to_string(),
        indoc! {r#"
            Search for the most relevant tools to help with the user's request by combining their exact message with strategic additional context.
            This tool enables more accurate tool selection by preserving the user's original intent while adding targeted search context.
            
            The user_query parameter contains the user's exact message (automatically provided).
            The additional_context should contain keywords and concepts that help find the right tools for the user's task:
            
            - For coding/development questions: Include terms like "development", "programming", "code", "files", "shell", "editor", "debugging"
            - For data analysis questions: Include terms like "data processing", "spreadsheet", "document", "analysis", "extraction" 
            - For system tasks: Include terms like "automation", "system control", "file management", "scripting"
            - For web/content tasks: Include terms like "web scraping", "download", "content extraction", "web"
            - For exploration questions: Include terms like "browse", "explore", "investigate", "examine", "read files"
            
            Focus additional_context on the TYPE of tools needed rather than repeating the user's specific question.
            Example: {"user_query": "how does the authentication work?", "additional_context": "code exploration development file browsing", "k": 5}
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["user_query", "additional_context"],
            "properties": {
                "user_query": {"type": "string", "description": "The user's original query/message (passthrough)"},
                "additional_context": {"type": "string", "description": "AI-generated additional context for the search"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Vector search with user message passthrough".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn vector_search_passthrough_tool_prompt() -> String {
    r#"# Tool Selection Instructions (User Message Passthrough)
    Important: the user has opted to dynamically enable tools with passthrough capability.
    The user's exact message will be automatically included in the search - you need to provide strategic additional_context.
    
    Analyze the user's request and provide additional_context keywords based on what TYPE of tools they need:
    - For code/development questions → "development programming code files shell editor debugging exploration"
    - For data/document questions → "data processing documents spreadsheet analysis files extraction"
    - For system/automation questions → "automation system control file management scripting"
    - For web/content questions → "web scraping download content extraction"
    
    DO NOT repeat the user's specific question in additional_context. Focus on tool categories and capabilities.
    Example: User asks "how does authentication work?" → additional_context: "code exploration development files programming"
    
    Invoke the vector search passthrough tool with strategic additional_context to find the right tools."#.to_string()
}

pub fn vector_search_with_extension_passthrough_tool() -> Tool {
    Tool::new(
        ROUTER_VECTOR_SEARCH_WITH_EXTENSION_PASSTHROUGH_TOOL_NAME.to_string(),
        indoc! {r#"
            Search for the most relevant tools within a specific extension to help with the user's request by combining their exact message with strategic additional context.
            This tool enables precise tool selection by targeting the right extension while preserving the user's original intent.
            
            The user_query parameter contains the user's exact message (automatically provided).
            The extension_name parameter specifies which extension to search within (REQUIRED).
            The additional_context should contain keywords that help find the right tools within that extension:
            
            Extension selection guidelines (choose based on available extensions):
            - For programming/development tasks: Use extensions that provide coding, file editing, shell commands, debugging, code exploration
            - For data/document processing tasks: Use extensions that handle document processing, web scraping, automation scripts
            - For system/platform tasks: Use extensions for extension management, resource access, system configuration
            
            Additional context examples by task type:
            - Development tasks: "code files shell terminal programming development debugging exploration"
            - Data/document tasks: "data documents automation scripting processing files analysis"
            - System tasks: "extensions configuration management resources system"
            
            Focus additional_context on specific tool capabilities rather than repeating the user's question.
            Example: {"user_query": "how does the router work?", "additional_context": "code files exploration development", "extension_name": "available_dev_extension", "k": 5}
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["user_query", "additional_context", "extension_name"],
            "properties": {
                "user_query": {"type": "string", "description": "The user's original query/message (passthrough)"},
                "additional_context": {"type": "string", "description": "AI-generated additional context for the search"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5},
                "extension_name": {"type": "string", "description": "Name of the extension to filter tools by"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Vector search with user message passthrough and extension filter".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn vector_search_with_extension_passthrough_tool_prompt() -> String {
    r#"# Tool Selection Instructions (User Message Passthrough with Extension)
    Important: the user has opted to dynamically enable tools with passthrough capability and extension filtering.
    The user's exact message will be automatically included - you must choose the RIGHT extension and provide strategic additional_context.
    
    Extension selection rules (choose based on available extensions and their capabilities):
    - Development-focused extensions: Programming, coding, file editing, shell commands, debugging, code exploration, development workflows
    - Data/automation-focused extensions: Data processing, documents (PDF/Word/Excel), web scraping, automation scripts, business tasks
    - System/platform extensions: Extension management, system configuration, resource access
    
    Additional context guidelines by task type:
    - Development tasks: "code files shell terminal programming development debugging exploration"
    - Data/automation tasks: "data documents automation scripting processing analysis"
    - System tasks: "extensions configuration management resources system"
    
    Choose the extension that matches the user's domain, then provide additional_context focused on tool capabilities within that extension.
    Example: User asks "debug this code issue" → extension_name: "available_dev_extension", additional_context: "debugging development code files"
    
    The system will combine the user's message with your additional_context to search within the specified extension."#.to_string()
}

pub fn llm_search_passthrough_tool() -> Tool {
    Tool::new(
        ROUTER_LLM_SEARCH_PASSTHROUGH_TOOL_NAME.to_string(),
        indoc! {r#"
            Search for the most relevant tools using AI-powered analysis, combining the user's exact message with strategic additional context and extension targeting.
            This tool uses intelligent tool matching to understand user intent and find the best tools for their specific needs.
            
            The user_query parameter contains the user's exact message (automatically provided).
            The extension_name parameter specifies which extension to search within (REQUIRED).
            The additional_context should provide semantic hints about the user's task:
            
            Extension and context guidelines (choose based on available extensions):
            - For development-focused extensions: Use for programming, debugging, code exploration, file editing, shell operations
              Context: "programming development code files shell editor debugging exploration"
            - For data/automation-focused extensions: Use for data processing, document work, automation, web scraping
              Context: "data processing documents automation web scraping files analysis"
            - For system/platform extensions: Use for extension management, system configuration, resource access
              Context: "extensions configuration management system resources"
            
            The LLM will analyze tool descriptions and match them to the user's intent using both the original message and additional context.
            Focus additional_context on task categories and tool types rather than specific technical terms.
            Example: {"user_query": "analyze this spreadsheet data", "additional_context": "data analysis spreadsheet processing", "extension_name": "data_extension", "k": 5}
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["user_query", "additional_context", "extension_name"],
            "properties": {
                "extension_name": {"type": "string", "description": "The name of the extension to filter tools by"},
                "user_query": {"type": "string", "description": "The user's original query/message (passthrough)"},
                "additional_context": {"type": "string", "description": "AI-generated additional context for the search"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
            }
        }),
        Some(ToolAnnotations {
            title: Some("LLM search with user message passthrough".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn llm_search_passthrough_tool_prompt() -> String {
    format!(
        r#"# LLM Tool Selection Instructions (User Message Passthrough)
    Important: the user has opted to dynamically enable tools with passthrough capability using intelligent LLM search.
    The user's exact message will be automatically included - you must choose the RIGHT extension and provide strategic additional_context.
    
    Extension selection based on user's domain (choose from available extensions):
    - Development-focused extensions: Programming, coding, file editing, shell commands, debugging, code exploration, development workflows
    - Data/automation-focused extensions: Data processing, documents (PDF/Word/Excel), web scraping, automation scripts, business tasks
    - System/platform extensions: Extension management, system configuration, resource access
    
    Additional context should focus on tool types and capabilities, not repeat the user's question:
    - For development tasks: "programming development code files shell editor debugging exploration"
    - For data/document tasks: "data processing documents automation scripting analysis"
    - For system tasks: "extensions configuration management resources system"
    
    The LLM will intelligently match tools based on the combined user message and additional context.
    Example: User asks "explore how routing works" → extension_name: "available_dev_extension", additional_context: "code exploration development files programming"
    
    Platform extension tools available: {}, {}, {}, {}
    "#,
        PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
        PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
        PLATFORM_READ_RESOURCE_TOOL_NAME,
        PLATFORM_LIST_RESOURCES_TOOL_NAME
    )
}

pub fn llm_search_parallel_tool() -> Tool {
    Tool::new(
        ROUTER_LLM_SEARCH_PARALLEL_TOOL_NAME.to_string(),
        indoc! {r#"
            Searches for relevant tools based on the user's messages across multiple extensions in parallel.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            Use the extension_names parameter to specify multiple extensions to search across simultaneously.
            For example, if the user is asking for a tool that could be in either "developer" or "data" extensions, you can search both.
            Example: {"User": "list the files in the current directory", "Query": "list files in current directory", "Extension Names": ["developer", "system"], "k": 5}
            Extension names array is not optional, it is required.
            The returned result will be a list of tool names, descriptions, and schemas from all specified extensions.
            If none of the tools are appropriate, then respond with 'no tools relevant'.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["query", "extension_names"],
            "properties": {
                "extension_names": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Array of extension names to search across in parallel"
                },
                "query": {"type": "string", "description": "The query to search for the most relevant tools based on the user's messages"},
                "k": {"type": "integer", "description": "The number of tools to retrieve (defaults to 5)", "default": 5}
            }
        }),
        Some(ToolAnnotations {
            title: Some("LLM parallel search for relevant tools across multiple extensions".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn llm_search_parallel_tool_prompt() -> String {
    format!(
        r#"# LLM Parallel Tool Selection Instructions
    Important: the user has opted to dynamically enable tools with parallel search capability.
    This allows you to search across multiple extensions simultaneously, which is useful when the user's request could be fulfilled by tools from different extensions.
    
    Use the extension_names parameter to specify multiple extensions to search across:
    - For broad searches: Include multiple relevant extensions like ["developer", "data", "system"]
    - For specific domains: Focus on related extensions like ["developer", "automation"] for coding tasks
    - For exploration: Cast a wide net with all available extensions
    
    Extension categories to consider:
    - Development-focused: Programming, coding, file editing, shell commands, debugging, code exploration
    - Data/automation-focused: Data processing, documents, web scraping, automation scripts
    - System/platform: Extension management, system configuration, resource access
    
    Be sure to format a query packed with relevant keywords to search for the most relevant tools.
    The system will fire off parallel LLM requests to each specified extension and collect all relevant tools.
    
    If none of the tools across all extensions are appropriate, the system will respond with 'no tools relevant'.
    
    Platform extension tools available: {}, {}, {}, {}
    "#,
        PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
        PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME,
        PLATFORM_READ_RESOURCE_TOOL_NAME,
        PLATFORM_LIST_RESOURCES_TOOL_NAME
    )
}
