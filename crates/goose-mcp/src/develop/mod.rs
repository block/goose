pub mod edit;
pub mod explore;
pub mod read;
pub mod shell;

use std::sync::Arc;

use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ErrorData, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};

use edit::{EditTools, FileEditParams, FileWriteParams};
use explore::{TreeParams, TreeTool};
use read::{ReadParams, ReadTool};
use shell::{ShellParams, ShellTool};

pub struct DevelopServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    shell_tool: Arc<ShellTool>,
    edit_tools: Arc<EditTools>,
    tree_tool: Arc<TreeTool>,
    read_tool: Arc<ReadTool>,
}

impl Clone for DevelopServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            instructions: self.instructions.clone(),
            shell_tool: Arc::clone(&self.shell_tool),
            edit_tools: Arc::clone(&self.edit_tools),
            tree_tool: Arc::clone(&self.tree_tool),
            read_tool: Arc::clone(&self.read_tool),
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
        let shell_tool = Arc::new(ShellTool::new());
        let edit_tools = Arc::new(EditTools::new());
        let tree_tool = Arc::new(TreeTool::new());
        let read_tool = Arc::new(ReadTool::new());

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let os = std::env::consts::OS;

        let instructions = formatdoc! {r#"
            The develop extension provides tools for software development tasks.

            Capabilities:
            - **Shell**: Execute shell commands with `shell`
            - **Edit**: Create and modify files with `file_write` and `file_edit`
            - **Explore**: Navigate codebases with `tree`
            - **Read**: Read text files and image files with `read`

            `shell` output is truncated to the last 2000 lines or 50KB, whichever is reached first.
            If truncation happens, full output is written to a temp file and the path is returned.
            For long-running commands, pass `timeout_secs`.

            operating system: {os}
            current directory: {cwd}
        "#,
            os = os,
            cwd = cwd,
        };

        Self {
            tool_router: Self::tool_router(),
            instructions,
            shell_tool,
            edit_tools,
            tree_tool,
            read_tool,
        }
    }

    #[tool(
        name = "shell",
        description = "Execute a shell command. Returns stdout/stderr as text. Output is tail-truncated to 2000 lines or 50KB with a temp-file pointer when truncated."
    )]
    pub async fn shell(
        &self,
        params: Parameters<ShellParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(self.shell_tool.shell(params.0).await)
    }

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

    #[tool(
        name = "tree",
        description = "List a directory tree with line counts. Traversal respects .gitignore rules."
    )]
    pub async fn tree(&self, params: Parameters<TreeParams>) -> Result<CallToolResult, ErrorData> {
        Ok(self.tree_tool.tree(params.0))
    }

    #[tool(
        name = "read",
        description = "Read a text file or image file. For text, supports offset/limit and truncates to 2000 lines or 50KB. For images, returns the image payload."
    )]
    pub async fn read(&self, params: Parameters<ReadParams>) -> Result<CallToolResult, ErrorData> {
        Ok(self.read_tool.read(params.0))
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
                description: None,
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
