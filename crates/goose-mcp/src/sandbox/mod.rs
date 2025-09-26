use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, ServerCapabilities,
        ServerInfo,
    },
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Parameters for the view_file tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ViewFileParams {
    /// Path to the file (absolute or relative)
    pub path: String,
}

/// Sandbox MCP Server for safe read-only file access
#[derive(Clone)]
pub struct SandboxServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    max_file_size: usize,
}

impl Default for SandboxServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl SandboxServer {
    pub fn new() -> Self {
        let instructions = r#"
This is a read-only sandbox extension for safe file viewing.
It provides limited, secure access to files without write capabilities.

Security Features:
- Read-only access (no write operations)
- File size limit: 10MB (prevents memory issues)
- Text files only (binary files will error)

Available tool:
- sandbox__view_file: Read and display file contents
"#
        .to_string();

        Self {
            tool_router: Self::tool_router(),
            instructions,
            max_file_size: 10 * 1024 * 1024, // 10MB default
        }
    }

    /// View the contents of a file (read-only)
    #[tool(
        name = "view_file",
        description = "Read-only file viewer for safe file inspection"
    )]
    pub async fn view_file(
        &self,
        params: Parameters<ViewFileParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let path = PathBuf::from(&params.0.path);

        // Resolve to absolute path if relative
        let absolute_path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to get current directory: {}", e),
                        None,
                    )
                })?
                .join(path)
        };

        // Basic safety: Check file exists and is a file (not directory)
        if !absolute_path.exists() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("File not found: {}", absolute_path.display()),
                None,
            ));
        }

        if !absolute_path.is_file() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Path is not a file: {}", absolute_path.display()),
                None,
            ));
        }

        // Basic safety: Check file size
        let metadata = fs::metadata(&absolute_path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read file metadata: {}", e),
                None,
            )
        })?;

        if metadata.len() > self.max_file_size as u64 {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "File too large: {} bytes (max: {} bytes)",
                    metadata.len(),
                    self.max_file_size
                ),
                None,
            ));
        }

        // Read file content
        let content = fs::read_to_string(&absolute_path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read file (possibly binary): {}", e),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for SandboxServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-sandbox".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_view_file_success() {
        let server = SandboxServer::new();

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Test content").unwrap();

        let params = ViewFileParams {
            path: temp_file.path().to_str().unwrap().to_string(),
        };

        let result = server.view_file(Parameters(params)).await;
        assert!(result.is_ok());

        let result_content = result.unwrap();
        assert!(!result_content.content.is_empty());
        // The Content type doesn't have a Text variant we can pattern match on
        // We just verify the call succeeded and returned content
    }

    #[tokio::test]
    async fn test_view_nonexistent_file() {
        let server = SandboxServer::new();

        let params = ViewFileParams {
            path: "/nonexistent/file.txt".to_string(),
        };

        let result = server.view_file(Parameters(params)).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.message.contains("File not found"));
        }
    }

    #[tokio::test]
    async fn test_view_directory_error() {
        let server = SandboxServer::new();

        let params = ViewFileParams {
            path: "/tmp".to_string(),
        };

        let result = server.view_file(Parameters(params)).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.message.contains("Path is not a file"));
        }
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let server = SandboxServer::new();

        // Create a file larger than the limit
        let mut temp_file = NamedTempFile::new().unwrap();
        let large_content = "x".repeat(11 * 1024 * 1024); // 11MB
        temp_file.write_all(large_content.as_bytes()).unwrap();

        let params = ViewFileParams {
            path: temp_file.path().to_str().unwrap().to_string(),
        };

        let result = server.view_file(Parameters(params)).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.message.contains("File too large"));
        }
    }

    #[tokio::test]
    async fn test_relative_path() {
        let server = SandboxServer::new();

        // Create a temp file in current directory
        let temp_file = NamedTempFile::new_in(".").unwrap();
        let mut file = temp_file.reopen().unwrap();
        writeln!(file, "Relative path test").unwrap();

        // Get just the filename (relative path)
        let filename = temp_file
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let params = ViewFileParams { path: filename };

        let result = server.view_file(Parameters(params)).await;
        assert!(result.is_ok());

        let result_content = result.unwrap();
        assert!(!result_content.content.is_empty());
        // The Content type doesn't have a Text variant we can pattern match on
        // We just verify the call succeeded and returned content
    }
}
