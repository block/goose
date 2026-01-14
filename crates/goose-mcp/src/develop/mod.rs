//! Develop - A developer extension for goose
//!
//! Provides capabilities for:
//! - Process: Shell command execution
//! - Edit: File editing operations
//! - Explore: Codebase exploration and analysis
//! - Image: Screenshot and image processing

pub mod edit;
pub mod explore;
pub mod image;
pub mod process;

use std::sync::Arc;

use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ErrorData, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};

use edit::{EditTools, FileEditParams, FileWriteParams};
use explore::{MapParams, MapTool};
use image::{ImageTool, ReadImageParams};
use process::{
    ProcessAwaitParams, ProcessIdParams, ProcessInputParams, ProcessManager, ProcessOutputParams,
    ProcessTools, ShellParams,
};

/// The Develop MCP server
pub struct DevelopServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    process_tools: Arc<ProcessTools>,
    edit_tools: Arc<EditTools>,
    map_tool: Arc<MapTool>,
    image_tool: Arc<ImageTool>,
}

impl Clone for DevelopServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            instructions: self.instructions.clone(),
            process_tools: Arc::clone(&self.process_tools),
            edit_tools: Arc::clone(&self.edit_tools),
            map_tool: Arc::clone(&self.map_tool),
            image_tool: Arc::clone(&self.image_tool),
        }
    }
}

impl Default for DevelopServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl DevelopServer {
    pub fn new() -> Self {
        let manager = Arc::new(ProcessManager::new());
        let process_tools = Arc::new(ProcessTools::new(Arc::clone(&manager)));
        let edit_tools = Arc::new(EditTools::new());
        let map_tool = Arc::new(MapTool::new());
        let image_tool = Arc::new(ImageTool::new());

        // TODO: The cwd/os info at the end breaks prompt caching since it changes per-session.
        // Consider moving dynamic context to a separate mechanism (e.g., resources or a
        // dedicated context tool) so the static instructions can be cached.
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let os = std::env::consts::OS;

        let mut instructions = formatdoc! {r#"
            The develop extension provides tools for software development tasks.

            Capabilities:
            - **Process**: Execute shell commands with `shell`. Working directory and 
              environment variables persist across calls.
            - **Edit**: Create and modify files with `file_write` and `file_edit`
            - **Explore**: Navigate and analyze codebases with `map`
            - **Image**: Read images from files or capture windows/screen with `read_image`.
              Prefer capturing specific windows over full screen when possible.

            **Process management**: The `timeout_secs` parameter controls how long to wait 
            before promoting a command to a managed process (default 2s). Once promoted, 
            you get a process ID (proc01, etc.) to use with process_status, process_output, 
            process_await, or process_kill. Do NOT use shell backgrounding (`&`).

            **Choosing timeout values:**
            - **Persistent processes** (dev servers, watch modes, `tail -f`): Use default. 
              Let them promote immediately, then poll output to check readiness.
            - **Slow-but-finite commands** (builds, tests, installs): Use higher timeout 
              (30-120s) to get complete output directly without process management overhead.

            Examples: `npm run dev` → default timeout, `cargo build` → `timeout_secs: 60`

            **Strategy**: You are responsible for carefully managing your own context window.
            It is critical to your ability to solve problems to not fill it up too quickly.

            Use `map` and `rg` for efficient exploration. For `rg`, use `rg --heading -n` for
            maximally token efficient output format.

            **Efficiency Tip**: Each tool call has overhead because the full conversation history
            is re-sent. It's often better to do something in one call rather than two or three,
            even if you read a bit more than strictly necessary. Reading extra context to save a
            round-trip is usually worth it. Avoid tool calls that give you information you already
            have in context - re-reading files or sections you've already seen uses tokens and
            adds round trips.

            operating system: {os}
            current directory: {cwd}
        "#,
            os = os,
            cwd = cwd,
        };

        // Add shell warning if using fallback
        if let Some(warning) = manager.shell_warning() {
            instructions.push_str(&format!("\n\n**Note**: {}", warning));
        }

        Self {
            tool_router: Self::tool_router(),
            instructions,
            process_tools,
            edit_tools,
            map_tool,
            image_tool,
        }
    }

    // ========================================================================
    // Process Tools
    // ========================================================================

    #[tool(
        name = "shell",
        description = "Execute a shell command. Returns output directly if it completes within timeout, otherwise returns a process ID (proc01, etc.) for management. Working directory and environment variables persist across calls. Do not use shell backgrounding (`&`)."
    )]
    pub async fn shell(
        &self,
        params: Parameters<ShellParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.shell(params.0))
    }

    #[tool(
        name = "process_list",
        description = "List all tracked processes. Returns CSV: id, command (truncated), status."
    )]
    pub async fn process_list(&self) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_list())
    }

    #[tool(
        name = "process_output",
        description = "Get output from a process buffer. Supports Python slice semantics for start/end (e.g., start=-30 for last 30 lines), and grep with before/after context."
    )]
    pub async fn process_output(
        &self,
        params: Parameters<ProcessOutputParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_output(params.0))
    }

    #[tool(
        name = "process_status",
        description = "Check if a process is RUNNING, EXITED(code), or KILLED."
    )]
    pub async fn process_status(
        &self,
        params: Parameters<ProcessIdParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_status(params.0))
    }

    #[tool(
        name = "process_await",
        description = "Wait for a process to complete. Use for commands you expect to finish but take a while (compiles, tests). Timeout required, max 300 seconds."
    )]
    pub async fn process_await(
        &self,
        params: Parameters<ProcessAwaitParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_await(params.0))
    }

    #[tool(
        name = "process_kill",
        description = "Terminate a process. Sends SIGTERM, then SIGKILL if needed."
    )]
    pub async fn process_kill(
        &self,
        params: Parameters<ProcessIdParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_kill(params.0))
    }

    #[tool(
        name = "process_input",
        description = "Send text to a process's stdin. Experimental."
    )]
    pub async fn process_input(
        &self,
        params: Parameters<ProcessInputParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.process_tools.process_input(params.0))
    }

    // ========================================================================
    // Edit Tools
    // ========================================================================

    #[tool(
        name = "file_write",
        description = "Create a new file or overwrite an existing file. Creates parent directories if needed."
    )]
    pub async fn file_write(
        &self,
        params: Parameters<FileWriteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.edit_tools.file_write(params.0))
    }

    #[tool(
        name = "file_edit",
        description = "Edit a file by finding and replacing text. The old_text must match exactly and uniquely. Use empty new_text to delete. Returns error with context if no match or multiple matches found."
    )]
    pub async fn file_edit(
        &self,
        params: Parameters<FileEditParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.edit_tools.file_edit(params.0))
    }

    // ========================================================================
    // Explore Tools
    // ========================================================================

    #[tool(
        name = "map",
        description = "Build a mental map of a directory. Shows file tree with line counts. Use depth to control traversal depth."
    )]
    pub async fn map(&self, params: Parameters<MapParams>) -> Result<CallToolResult, ErrorData> {
        Ok(self.map_tool.map(params.0))
    }

    // ========================================================================
    // Image Tools
    // ========================================================================

    #[tool(
        name = "read_image",
        description = "Read an image from a file or capture from screen. Source can be: a file path, a window title/substring for fuzzy matching, or omit for full screen. Prefer specific windows over full screen when possible. Images are automatically resized for optimal LLM consumption."
    )]
    pub async fn read_image(
        &self,
        params: Parameters<ReadImageParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.image_tool.read_image(params.0))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DevelopServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-develop".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: None,
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = DevelopServer::new();
        assert!(!server.instructions.is_empty());
    }

    #[test]
    fn test_get_info() {
        let server = DevelopServer::new();
        let info = server.get_info();

        assert_eq!(info.server_info.name, "goose-develop");
        assert!(info.instructions.is_some());
    }
}
