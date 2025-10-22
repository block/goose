use anyhow::{anyhow, Result};
use async_trait::async_trait;
use goose::agents::mcp_client::McpClientTrait;
use goose::lsp::{LspClient, LspConfig};
use lsp_types;
use rmcp::model::{Content, ErrorCode, ErrorData, Tool};
use rmcp::object;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub struct LspMcpClient {
    lsp_client: Arc<Mutex<LspClient>>,
    config: LspConfig,
}

impl LspMcpClient {
    pub async fn new(config: LspConfig) -> Result<Self> {
        let lsp_client = LspClient::new(config.clone()).await?;

        Ok(Self {
            lsp_client: Arc::new(Mutex::new(lsp_client)),
            config,
        })
    }

    async fn call_get_diagnostics(&self, args: Value) -> Result<Vec<Content>> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing file_path parameter"))?;

        let path = PathBuf::from(file_path);

        let client = self.lsp_client.lock().await;
        client.text_document_did_open(&path).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let diagnostics = client.get_diagnostics(&path);

        let diag_text = if diagnostics.is_empty() {
            "No diagnostics found.".to_string()
        } else {
            diagnostics
                .iter()
                .map(|d| {
                    let severity = match d.severity {
                        Some(1) => "Error",
                        Some(2) => "Warning",
                        Some(3) => "Info",
                        Some(4) => "Hint",
                        _ => "Unknown",
                    };
                    format!(
                        "[{}:{}] {}: {}{}",
                        d.range.start.line + 1,
                        d.range.start.character + 1,
                        severity,
                        d.message,
                        d.source
                            .as_ref()
                            .map(|s| format!(" ({})", s))
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(vec![Content::text(diag_text)])
    }

    async fn call_hover(&self, args: Value) -> Result<Vec<Content>> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing file_path parameter"))?;
        let line = args
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing line parameter"))? as u32;
        let character = args
            .get("character")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing character parameter"))? as u32;

        let path = PathBuf::from(file_path);

        let client = self.lsp_client.lock().await;
        client.text_document_did_open(&path).await.ok();

        let hover = client.hover(&path, line, character).await?;

        let text = if let Some(hover) = hover {
            match hover.contents {
                lsp_types::HoverContents::Scalar(markup) => match markup {
                    lsp_types::MarkedString::String(s) => s,
                    lsp_types::MarkedString::LanguageString(ls) => ls.value,
                },
                lsp_types::HoverContents::Array(arr) => arr
                    .into_iter()
                    .map(|m| match m {
                        lsp_types::MarkedString::String(s) => s,
                        lsp_types::MarkedString::LanguageString(ls) => ls.value,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                lsp_types::HoverContents::Markup(markup) => markup.value,
            }
        } else {
            "No hover information available.".to_string()
        };

        Ok(vec![Content::text(text)])
    }

    async fn call_goto_definition(&self, args: Value) -> Result<Vec<Content>> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing file_path parameter"))?;
        let line = args
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing line parameter"))? as u32;
        let character = args
            .get("character")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing character parameter"))? as u32;

        let path = PathBuf::from(file_path);

        let client = self.lsp_client.lock().await;
        client.text_document_did_open(&path).await.ok();

        let result = client.goto_definition(&path, line, character).await?;

        let text = if let Some(result) = result {
            match result {
                lsp_types::GotoDefinitionResponse::Scalar(loc) => {
                    format!(
                        "{}:{}:{}",
                        loc.uri,
                        loc.range.start.line + 1,
                        loc.range.start.character + 1
                    )
                }
                lsp_types::GotoDefinitionResponse::Array(locs) => locs
                    .iter()
                    .map(|loc| {
                        format!(
                            "{}:{}:{}",
                            loc.uri,
                            loc.range.start.line + 1,
                            loc.range.start.character + 1
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                lsp_types::GotoDefinitionResponse::Link(links) => links
                    .iter()
                    .map(|link| {
                        format!(
                            "{}:{}:{}",
                            link.target_uri,
                            link.target_range.start.line + 1,
                            link.target_range.start.character + 1
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            }
        } else {
            "No definition found.".to_string()
        };

        Ok(vec![Content::text(text)])
    }

    async fn call_find_references(&self, args: Value) -> Result<Vec<Content>> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing file_path parameter"))?;
        let line = args
            .get("line")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing line parameter"))? as u32;
        let character = args
            .get("character")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing character parameter"))? as u32;

        let path = PathBuf::from(file_path);

        let client = self.lsp_client.lock().await;
        client.text_document_did_open(&path).await.ok();

        let references = client.find_references(&path, line, character).await?;

        let text = if let Some(references) = references {
            if references.is_empty() {
                "No references found.".to_string()
            } else {
                references
                    .iter()
                    .map(|loc| {
                        format!(
                            "{}:{}:{}",
                            loc.uri,
                            loc.range.start.line + 1,
                            loc.range.start.character + 1
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        } else {
            "No references found.".to_string()
        };

        Ok(vec![Content::text(text)])
    }
}

#[async_trait]
impl McpClientTrait for LspMcpClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::ListResourcesResult, rmcp::ServiceError> {
        Ok(rmcp::model::ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::ReadResourceResult, rmcp::ServiceError> {
        Err(rmcp::ServiceError::McpError(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            "LSP extensions do not support resources".to_string(),
            None,
        )))
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ServiceError> {
        let prefix = format!("{}_", self.config.name);

        Ok(rmcp::model::ListToolsResult {
            tools: vec![
                Tool::new(
                    format!("{}get_diagnostics", prefix),
                    "Get LSP diagnostics (errors, warnings) for a file".to_string(),
                    object!({
                        "type": "object",
                        "required": ["file_path"],
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file to check"
                            }
                        }
                    }),
                ),
                Tool::new(
                    format!("{}hover", prefix),
                    "Get hover information at a specific position in a file".to_string(),
                    object!({
                        "type": "object",
                        "required": ["file_path", "line", "character"],
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file"
                            },
                            "line": {
                                "type": "integer",
                                "description": "Line number (0-indexed)"
                            },
                            "character": {
                                "type": "integer",
                                "description": "Character position (0-indexed)"
                            }
                        }
                    }),
                ),
                Tool::new(
                    format!("{}goto_definition", prefix),
                    "Navigate to the definition of a symbol".to_string(),
                    object!({
                        "type": "object",
                        "required": ["file_path", "line", "character"],
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file"
                            },
                            "line": {
                                "type": "integer",
                                "description": "Line number (0-indexed)"
                            },
                            "character": {
                                "type": "integer",
                                "description": "Character position (0-indexed)"
                            }
                        }
                    }),
                ),
                Tool::new(
                    format!("{}find_references", prefix),
                    "Find all references to a symbol".to_string(),
                    object!({
                        "type": "object",
                        "required": ["file_path", "line", "character"],
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "Path to the file"
                            },
                            "line": {
                                "type": "integer",
                                "description": "Line number (0-indexed)"
                            },
                            "character": {
                                "type": "integer",
                                "description": "Character position (0-indexed)"
                            }
                        }
                    }),
                ),
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ServiceError> {
        let tool_name = name.trim_start_matches(&format!("{}_", self.config.name));
        let args = arguments.map(Value::Object).unwrap_or_default();

        debug!("Calling LSP tool: {}", tool_name);

        let content = match tool_name {
            "get_diagnostics" => self.call_get_diagnostics(args).await.map_err(|e| {
                rmcp::ServiceError::McpError(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                ))
            })?,
            "hover" => self.call_hover(args).await.map_err(|e| {
                rmcp::ServiceError::McpError(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                ))
            })?,
            "goto_definition" => self.call_goto_definition(args).await.map_err(|e| {
                rmcp::ServiceError::McpError(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                ))
            })?,
            "find_references" => self.call_find_references(args).await.map_err(|e| {
                rmcp::ServiceError::McpError(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    e.to_string(),
                    None,
                ))
            })?,
            _ => {
                return Err(rmcp::ServiceError::McpError(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Unknown tool: {}", tool_name),
                    None,
                )))
            }
        };

        Ok(rmcp::model::CallToolResult {
            content,
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::ListPromptsResult, rmcp::ServiceError> {
        Ok(rmcp::model::ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancel_token: CancellationToken,
    ) -> Result<rmcp::model::GetPromptResult, rmcp::ServiceError> {
        Err(rmcp::ServiceError::McpError(ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            "LSP extensions do not support prompts".to_string(),
            None,
        )))
    }

    async fn subscribe(&self) -> tokio::sync::mpsc::Receiver<rmcp::model::ServerNotification> {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        rx
    }

    fn get_info(&self) -> Option<&rmcp::model::InitializeResult> {
        None
    }
}
