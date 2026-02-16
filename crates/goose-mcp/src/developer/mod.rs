pub mod edit;
pub mod shell;
pub mod tree;

use std::sync::Arc;

use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ErrorData, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};

use edit::{EditTools, FileEditParams, FileWriteParams};
use shell::{ShellParams, ShellTool};
use tree::{TreeParams, TreeTool};

pub struct DeveloperServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    shell_tool: Arc<ShellTool>,
    edit_tools: Arc<EditTools>,
    tree_tool: Arc<TreeTool>,
}

impl Clone for DeveloperServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            instructions: self.instructions.clone(),
            shell_tool: Arc::clone(&self.shell_tool),
            edit_tools: Arc::clone(&self.edit_tools),
            tree_tool: Arc::clone(&self.tree_tool),
        }
    }
}

impl Default for DeveloperServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl DeveloperServer {
    pub fn new() -> Self {
        let shell_tool = Arc::new(ShellTool::new());
        let edit_tools = Arc::new(EditTools::new());
        let tree_tool = Arc::new(TreeTool::new());

        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let os = std::env::consts::OS;

        let instructions = formatdoc! {r#"
            The developer extension provides tools for software development tasks.

            Capabilities:
            - **Shell**: Execute shell commands with `shell`
            - **Edit**: Create and modify files with `file_write` and `file_edit`
            - **Explore**: Navigate codebases with `tree`

            `shell` output is limited to 2000 lines. When exceeded, the full output is saved
            to a temp file and the response includes only the last 50 lines plus the file path.
            You can read the temp file with shell commands like `head`, `tail`, or `sed -n '100,200p'`
            up to 2000 lines at a time. For long-running commands, pass `timeout_secs`.

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
        }
    }

    #[tool(
        name = "shell",
        description = "Execute a shell command in the user's default shell in the current dir and return both stdout/stderr. The output is limited to up to 2000 lines, and longer outputs will be saved to a temporary file."
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
        description = "Edit a file by finding and replacing text. The before text must match exactly and uniquely. Use empty after text to delete."
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DeveloperServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-developer".to_string(),
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
        let server = DeveloperServer::new();
        assert!(!server.instructions.is_empty());
    }

    #[test]
    fn test_get_info() {
        let server = DeveloperServer::new();
        let info = server.get_info();

        assert_eq!(info.server_info.name, "goose-developer");
        assert!(info.instructions.is_some());
    }
}
