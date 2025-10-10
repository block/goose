use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    ServerCapabilities, ServerNotification,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// Re-use the existing load_hints module from core goose
use crate::agents::context_hints::load_hints::{load_hint_files, GOOSE_HINTS_FILENAME};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};

pub static EXTENSION_NAME: &str = "context";

pub struct ContextClient {
    info: InitializeResult,
    #[allow(dead_code)]
    context: PlatformExtensionContext,
}

impl ContextClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        // Get current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Get configured hint filenames from environment or use defaults
        let hints_filenames: Vec<String> = std::env::var("CONTEXT_FILE_NAMES")
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| vec!["AGENTS.md".to_string(), GOOSE_HINTS_FILENAME.to_string()]);

        // Build ignore patterns
        let ignore_patterns = Self::build_ignore_patterns(&cwd);

        // Load hints using the existing function
        let hints = load_hint_files(&cwd, &hints_filenames, &ignore_patterns);

        // Only set instructions if we have hints
        let instructions = if hints.is_empty() { None } else { Some(hints) };

        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Context Files".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions,
        };

        Ok(Self { info, context })
    }

    fn build_ignore_patterns(cwd: &Path) -> Gitignore {
        let mut builder = GitignoreBuilder::new(cwd);

        // Check for local .gooseignore
        let local_ignore_path = cwd.join(".gooseignore");
        let mut has_ignore_file = false;

        if local_ignore_path.is_file() {
            let _ = builder.add(local_ignore_path);
            has_ignore_file = true;
        } else {
            // Fallback to .gitignore
            let gitignore_path = cwd.join(".gitignore");
            if gitignore_path.is_file() {
                let _ = builder.add(gitignore_path);
                has_ignore_file = true;
            }
        }

        // Add default patterns if no ignore files found
        if !has_ignore_file {
            let _ = builder.add_line(None, "**/.env");
            let _ = builder.add_line(None, "**/.env.*");
            let _ = builder.add_line(None, "**/secrets.*");
        }

        builder.build().expect("Failed to build ignore patterns")
    }
}

#[async_trait]
impl McpClientTrait for ContextClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        // No tools for this extension
        Ok(ListToolsResult {
            tools: vec![],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        _name: &str,
        _arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        // Context extension has no tools, so this should never be called
        Err(Error::TransportClosed)
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: serde_json::Value,
        _cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
