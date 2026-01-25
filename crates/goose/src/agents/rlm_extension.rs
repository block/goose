//! RLM (Recursive Language Model) Platform Extension
//!
//! This extension provides tools for handling large contexts through the RLM technique.
//! It exposes tools for reading context slices, querying sub-agents, storing variables,
//! and finalizing answers.

use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::rlm::context_store::ContextStore;
use crate::rlm::prompts::RLM_SYSTEM_PROMPT;
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "rlm";

// Tool parameter schemas

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReadContextSliceParams {
    /// Starting character position (0-indexed)
    start: usize,
    /// Ending character position (exclusive)
    end: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct RlmQueryParams {
    /// The prompt/question to send to the sub-agent
    prompt: String,
    /// Starting character position of context to include
    start: usize,
    /// Ending character position of context to include (exclusive)
    end: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct StoreVariableParams {
    /// Name of the variable to store
    name: String,
    /// Value to store
    value: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GetVariableParams {
    /// Name of the variable to retrieve
    name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FinalizeParams {
    /// The final answer to return
    answer: String,
}

/// RLM Extension Client
pub struct RlmClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    context_store: Arc<Mutex<Option<ContextStore>>>,
    variables: Arc<Mutex<HashMap<String, String>>>,
    final_answer: Arc<Mutex<Option<String>>>,
    session_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl RlmClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tasks: None,
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("RLM (Recursive Language Model)".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(RLM_SYSTEM_PROMPT.to_string()),
        };

        Ok(Self {
            info,
            context,
            context_store: Arc::new(Mutex::new(None)),
            variables: Arc::new(Mutex::new(HashMap::new())),
            final_answer: Arc::new(Mutex::new(None)),
            session_dir: Arc::new(Mutex::new(None)),
        })
    }

    /// Initialize the context store for a session
    pub async fn initialize_context(&self, session_dir: PathBuf, content: &str) -> Result<()> {
        let store = ContextStore::new(session_dir.clone());
        store.store_context(content).await?;

        *self.context_store.lock().await = Some(store);
        *self.session_dir.lock().await = Some(session_dir);

        Ok(())
    }

    /// Check if RLM has been finalized
    pub async fn is_finalized(&self) -> bool {
        self.final_answer.lock().await.is_some()
    }

    /// Get the final answer if available
    pub async fn get_final_answer(&self) -> Option<String> {
        self.final_answer.lock().await.clone()
    }

    async fn ensure_context_store(&self, session_id: &str) -> Result<(), String> {
        let store = self.context_store.lock().await;
        if store.is_none() {
            // Try to initialize from session working directory
            std::mem::drop(store);

            if let Ok(session) = self
                .context
                .session_manager
                .get_session(session_id, false)
                .await
            {
                let context_store = ContextStore::new(session.working_dir.clone());
                if context_store.exists().await {
                    *self.context_store.lock().await = Some(context_store);
                    *self.session_dir.lock().await = Some(session.working_dir);
                    return Ok(());
                }
            }
            return Err("RLM context not initialized. Store context first.".to_string());
        }
        Ok(())
    }

    async fn handle_get_context_metadata(
        &self,
        session_id: &str,
    ) -> Result<Vec<Content>, String> {
        self.ensure_context_store(session_id).await?;

        let store = self.context_store.lock().await;
        let store = store.as_ref().ok_or("Context store not available")?;

        let metadata = store.get_metadata().await.map_err(|e| e.to_string())?;

        let response = serde_json::json!({
            "length": metadata.length,
            "chunk_count": metadata.chunk_count,
            "chunk_boundaries": metadata.chunk_boundaries,
            "path": metadata.path.to_string_lossy(),
        });

        Ok(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap_or_else(|_| response.to_string()),
        )])
    }

    async fn handle_read_context_slice(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        self.ensure_context_store(session_id).await?;

        let args = arguments.ok_or("Missing arguments")?;
        let start = args
            .get("start")
            .and_then(|v| v.as_u64())
            .ok_or("Missing required parameter: start")? as usize;
        let end = args
            .get("end")
            .and_then(|v| v.as_u64())
            .ok_or("Missing required parameter: end")? as usize;

        // Limit slice size to prevent overwhelming context
        const MAX_SLICE_SIZE: usize = 600_000; // 600K chars max
        if end - start > MAX_SLICE_SIZE {
            return Err(format!(
                "Requested slice too large: {} chars (max: {}). Use smaller slices.",
                end - start,
                MAX_SLICE_SIZE
            ));
        }

        let store = self.context_store.lock().await;
        let store = store.as_ref().ok_or("Context store not available")?;

        let slice = store.read_slice(start, end).await.map_err(|e| e.to_string())?;

        Ok(vec![Content::text(format!(
            "[Context slice from {} to {} ({} chars)]:\n{}",
            start,
            end,
            slice.len(),
            slice
        ))])
    }

    async fn handle_rlm_query(
        &self,
        session_id: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        self.ensure_context_store(session_id).await?;

        let args = arguments.ok_or("Missing arguments")?;
        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: prompt")?
            .to_string();
        let start = args
            .get("start")
            .and_then(|v| v.as_u64())
            .ok_or("Missing required parameter: start")? as usize;
        let end = args
            .get("end")
            .and_then(|v| v.as_u64())
            .ok_or("Missing required parameter: end")? as usize;

        // Read the context slice
        let context_slice = {
            let guard = self.context_store.lock().await;
            let store = guard.as_ref().ok_or("Context store not available")?;
            store.read_slice(start, end).await.map_err(|e| e.to_string())?
        };

        // For now, we return a message indicating this would spawn a sub-agent
        // In a full implementation, this would use the subagent_tool to spawn an actual sub-agent
        // with the context slice and prompt

        // TODO: Integrate with actual sub-agent system
        // This is a placeholder that returns the context slice for the parent agent to process
        // In a production implementation, you would:
        // 1. Create a sub-agent with the context slice injected
        // 2. Run the sub-agent with the given prompt
        // 3. Return the sub-agent's response

        Ok(vec![Content::text(format!(
            "[RLM Sub-Query]\nPrompt: {}\nContext range: {} to {} ({} chars)\n\n---\n\nNote: Sub-agent queries are currently processed by the parent agent. Process this context slice and answer the prompt:\n\n{}",
            prompt,
            start,
            end,
            context_slice.len(),
            if context_slice.len() > 10000 {
                format!("{}...\n[truncated, {} more chars]", &context_slice[..10000], context_slice.len() - 10000)
            } else {
                context_slice
            }
        ))])
    }

    async fn handle_store_variable(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: name")?
            .to_string();
        let value = args
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: value")?
            .to_string();

        let mut variables = self.variables.lock().await;
        variables.insert(name.clone(), value.clone());

        Ok(vec![Content::text(format!(
            "Stored variable '{}' ({} chars)",
            name,
            value.len()
        ))])
    }

    async fn handle_get_variable(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: name")?;

        let variables = self.variables.lock().await;
        match variables.get(name) {
            Some(value) => Ok(vec![Content::text(value.clone())]),
            None => Err(format!("Variable '{}' not found", name)),
        }
    }

    async fn handle_list_variables(&self) -> Result<Vec<Content>, String> {
        let variables = self.variables.lock().await;
        let names: Vec<&String> = variables.keys().collect();

        if names.is_empty() {
            Ok(vec![Content::text("No variables stored yet.")])
        } else {
            let list = names
                .iter()
                .map(|n| format!("- {}", n))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(vec![Content::text(format!(
                "Stored variables ({}):\n{}",
                names.len(),
                list
            ))])
        }
    }

    async fn handle_finalize(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let answer = args
            .get("answer")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: answer")?
            .to_string();

        *self.final_answer.lock().await = Some(answer.clone());

        Ok(vec![Content::text(format!(
            "[RLM FINALIZED]\n\nFinal Answer:\n{}",
            answer
        ))])
    }

    fn get_tools() -> Vec<Tool> {
        vec![
            // Get context metadata
            Tool::new(
                "rlm_get_context_metadata".to_string(),
                indoc! {r#"
                    Get metadata about the stored RLM context.

                    Returns:
                    - length: Total number of characters
                    - chunk_count: Number of recommended chunks
                    - chunk_boundaries: Array of [start, end] pairs for each chunk

                    Call this first to understand the context size before reading.
                "#}
                .to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })
                .as_object()
                .unwrap()
                .clone(),
            )
            .annotate(ToolAnnotations {
                title: Some("Get Context Metadata".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
            // Read context slice
            {
                let schema = schema_for!(ReadContextSliceParams);
                let schema_value = serde_json::to_value(schema)
                    .expect("Failed to serialize ReadContextSliceParams schema");
                Tool::new(
                    "rlm_read_context_slice".to_string(),
                    indoc! {r#"
                        Read a slice of the stored context by character position.

                        Parameters:
                        - start: Starting character position (0-indexed)
                        - end: Ending character position (exclusive)

                        Maximum slice size is 600,000 characters. For larger ranges, use multiple calls
                        or use rlm_query to delegate to a sub-agent.
                    "#}
                    .to_string(),
                    schema_value.as_object().unwrap().clone(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Read Context Slice".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                })
            },
            // RLM Query (sub-agent)
            {
                let schema = schema_for!(RlmQueryParams);
                let schema_value =
                    serde_json::to_value(schema).expect("Failed to serialize RlmQueryParams schema");
                Tool::new(
                    "rlm_query".to_string(),
                    indoc! {r#"
                        Query a sub-agent with a portion of the context.

                        Parameters:
                        - prompt: The question or task for the sub-agent
                        - start: Starting character position of context to include
                        - end: Ending character position of context to include

                        The sub-agent will receive the specified context slice and your prompt,
                        then return a response. Use this to delegate processing of large context
                        chunks. Aim for ~500,000 characters per query for optimal performance.
                    "#}
                    .to_string(),
                    schema_value.as_object().unwrap().clone(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Query Sub-Agent".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(false),
                    open_world_hint: Some(true),
                })
            },
            // Store variable
            {
                let schema = schema_for!(StoreVariableParams);
                let schema_value = serde_json::to_value(schema)
                    .expect("Failed to serialize StoreVariableParams schema");
                Tool::new(
                    "rlm_store_variable".to_string(),
                    indoc! {r#"
                        Store a value in a named variable for later retrieval.

                        Parameters:
                        - name: Name of the variable
                        - value: Value to store (string)

                        Use this to save intermediate results when processing large contexts
                        across multiple iterations or sub-queries.
                    "#}
                    .to_string(),
                    schema_value.as_object().unwrap().clone(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Store Variable".to_string()),
                    read_only_hint: Some(false),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                })
            },
            // Get variable
            {
                let schema = schema_for!(GetVariableParams);
                let schema_value = serde_json::to_value(schema)
                    .expect("Failed to serialize GetVariableParams schema");
                Tool::new(
                    "rlm_get_variable".to_string(),
                    indoc! {r#"
                        Retrieve a previously stored variable by name.

                        Parameters:
                        - name: Name of the variable to retrieve

                        Returns the stored value or an error if the variable doesn't exist.
                    "#}
                    .to_string(),
                    schema_value.as_object().unwrap().clone(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Get Variable".to_string()),
                    read_only_hint: Some(true),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(true),
                    open_world_hint: Some(false),
                })
            },
            // List variables
            Tool::new(
                "rlm_list_variables".to_string(),
                "List all stored variable names.".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })
                .as_object()
                .unwrap()
                .clone(),
            )
            .annotate(ToolAnnotations {
                title: Some("List Variables".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
            // Finalize
            {
                let schema = schema_for!(FinalizeParams);
                let schema_value =
                    serde_json::to_value(schema).expect("Failed to serialize FinalizeParams schema");
                Tool::new(
                    "rlm_finalize".to_string(),
                    indoc! {r#"
                        Finalize the RLM session with your final answer.

                        Parameters:
                        - answer: The final answer to return

                        Call this when you have completed processing and have your final answer.
                        This marks the RLM session as complete.
                    "#}
                    .to_string(),
                    schema_value.as_object().unwrap().clone(),
                )
                .annotate(ToolAnnotations {
                    title: Some("Finalize Answer".to_string()),
                    read_only_hint: Some(false),
                    destructive_hint: Some(false),
                    idempotent_hint: Some(false),
                    open_world_hint: Some(false),
                })
            },
        ]
    }
}

#[async_trait]
impl McpClientTrait for RlmClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "rlm_get_context_metadata" => self.handle_get_context_metadata(session_id).await,
            "rlm_read_context_slice" => {
                self.handle_read_context_slice(session_id, arguments).await
            }
            "rlm_query" => self.handle_rlm_query(session_id, arguments).await,
            "rlm_store_variable" => self.handle_store_variable(arguments).await,
            "rlm_get_variable" => self.handle_get_variable(arguments).await,
            "rlm_list_variables" => self.handle_list_variables().await,
            "rlm_finalize" => self.handle_finalize(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(&self, _session_id: &str) -> Option<String> {
        // Check if we're in RLM mode by seeing if context exists
        let store = self.context_store.lock().await;
        if store.is_some() {
            let store = store.as_ref().unwrap();
            if let Ok(metadata) = store.get_metadata().await {
                return Some(format!(
                    "RLM Mode Active: {} chars in {} chunks. Use rlm_* tools to process.\n",
                    metadata.length, metadata.chunk_count
                ));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_context() -> PlatformExtensionContext {
        PlatformExtensionContext {
            extension_manager: None,
            session_manager: Arc::new(SessionManager::new(
                std::env::temp_dir().join("goose_test"),
            )),
        }
    }

    #[tokio::test]
    async fn test_rlm_client_creation() {
        let ctx = create_test_context();
        let client = RlmClient::new(ctx).unwrap();
        assert!(client.get_info().is_some());
        assert_eq!(client.get_info().unwrap().server_info.name, "rlm");
    }

    #[tokio::test]
    async fn test_rlm_tools_list() {
        let tools = RlmClient::get_tools();
        let tool_names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();

        assert!(tool_names.contains(&"rlm_get_context_metadata"));
        assert!(tool_names.contains(&"rlm_read_context_slice"));
        assert!(tool_names.contains(&"rlm_query"));
        assert!(tool_names.contains(&"rlm_store_variable"));
        assert!(tool_names.contains(&"rlm_get_variable"));
        assert!(tool_names.contains(&"rlm_list_variables"));
        assert!(tool_names.contains(&"rlm_finalize"));
    }

    #[tokio::test]
    async fn test_variable_storage() {
        let ctx = create_test_context();
        let client = RlmClient::new(ctx).unwrap();

        // Store a variable
        let store_args = serde_json::json!({
            "name": "test_var",
            "value": "test_value"
        });
        let result = client
            .handle_store_variable(store_args.as_object().cloned())
            .await;
        assert!(result.is_ok());

        // Retrieve the variable
        let get_args = serde_json::json!({
            "name": "test_var"
        });
        let result = client
            .handle_get_variable(get_args.as_object().cloned())
            .await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content[0].as_text().unwrap().text, "test_value");
    }

    #[tokio::test]
    async fn test_finalize() {
        let ctx = create_test_context();
        let client = RlmClient::new(ctx).unwrap();

        assert!(!client.is_finalized().await);

        let args = serde_json::json!({
            "answer": "The answer is 42"
        });
        let result = client.handle_finalize(args.as_object().cloned()).await;
        assert!(result.is_ok());

        assert!(client.is_finalized().await);
        assert_eq!(
            client.get_final_answer().await,
            Some("The answer is 42".to_string())
        );
    }

    #[tokio::test]
    async fn test_context_initialization() {
        let ctx = create_test_context();
        let client = RlmClient::new(ctx).unwrap();

        let temp_dir = TempDir::new().unwrap();
        let content = "Hello, World! This is test content for RLM.";

        client
            .initialize_context(temp_dir.path().to_path_buf(), content)
            .await
            .unwrap();

        // Now we should be able to get metadata
        // Note: We can't easily test this without a real session, but the initialize works
        assert!(client.context_store.lock().await.is_some());
    }
}
