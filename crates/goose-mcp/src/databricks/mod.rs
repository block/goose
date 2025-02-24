use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin, time::Duration};
use tokio::time::sleep;
use url::Url;

mod oauth;

use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    role::Role,
    tool::Tool,
};
use mcp_server::router::{CapabilitiesBuilder, Router};

const DEFAULT_CLIENT_ID: &str = "databricks-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";
const DEFAULT_SCOPES: &[&str] = &["all-apis"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabricksAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl DatabricksAuth {
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn token(token: String) -> Self {
        Self::Token(token)
    }
}

/// DatabricksRouter provides MCP tools for interacting with Databricks SQL endpoints
pub struct DatabricksRouter {
    tools: Vec<Tool>,
    client: Client,
    host: String,
    warehouse_id: String,
    auth: DatabricksAuth,
}

impl Default for DatabricksRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabricksRouter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("Failed to create HTTP client");

        // Load configuration from environment
        let host =
            std::env::var("DATABRICKS_HOST").expect("DATABRICKS_HOST environment variable not set");
        let warehouse_id = std::env::var("DATABRICKS_SQL_WAREHOUSE_ID")
            .expect("DATABRICKS_SQL_WAREHOUSE_ID environment variable not set");

        // Try token first, fall back to OAuth if not present
        let auth = if let Ok(token) = std::env::var("DATABRICKS_TOKEN") {
            DatabricksAuth::token(token)
        } else {
            DatabricksAuth::oauth(host.clone())
        };

        // Define the SQL query tool
        let query_tool = Tool::new(
            "sql_query".to_string(),
            "Execute a SQL query against a Databricks SQL warehouse".to_string(),
            json!({
                "type": "object",
                "required": ["statement"],
                "properties": {
                    "statement": {
                        "type": "string",
                        "description": "SQL query to execute"
                    },
                    "catalog": {
                        "type": "string",
                        "description": "Optional: Databricks catalog to use"
                    },
                    "schema": {
                        "type": "string",
                        "description": "Optional: Database schema to use"
                    },
                    "parameters": {
                        "type": "array",
                        "description": "Optional: Query parameters",
                        "items": {
                            "type": "object",
                            "required": ["name", "value", "type"],
                            "properties": {
                                "name": {"type": "string"},
                                "value": {"type": "string"},
                                "type": {"type": "string"}
                            }
                        }
                    }
                }
            }),
        );

        Self {
            tools: vec![query_tool],
            client,
            host,
            warehouse_id,
            auth,
        }
    }

    async fn ensure_auth_header(&self) -> Result<String, ToolError> {
        match &self.auth {
            DatabricksAuth::Token(token) => Ok(format!("Bearer {}", token)),
            DatabricksAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => oauth::get_oauth_token(host, client_id, redirect_url, scopes)
                .await
                .map(|token| format!("Bearer {}", token))
                .map_err(|e| {
                    ToolError::ExecutionError(format!("OAuth authentication failed: {}", e))
                }),
        }
    }

    async fn execute_query(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ToolError::ExecutionError(format!("Invalid base URL: {}", e)))?;
        let url = base_url
            .join("api/2.0/sql/statements/")
            .map_err(|e| ToolError::ExecutionError(format!("Failed to construct URL: {}", e)))?;

        let mut payload = json!({
            "warehouse_id": self.warehouse_id,
            "statement": params.get("statement").and_then(|v| v.as_str()).unwrap(),
        });

        // Add optional parameters if provided
        if let Some(catalog) = params.get("catalog").and_then(|v| v.as_str()) {
            payload
                .as_object_mut()
                .unwrap()
                .insert("catalog".to_string(), json!(catalog));
        }
        if let Some(schema) = params.get("schema").and_then(|v| v.as_str()) {
            payload
                .as_object_mut()
                .unwrap()
                .insert("schema".to_string(), json!(schema));
        }
        if let Some(parameters) = params.get("parameters") {
            payload
                .as_object_mut()
                .unwrap()
                .insert("parameters".to_string(), parameters.clone());
        }

        let auth_header = self.ensure_auth_header().await?;
        let response = self
            .client
            .post(url)
            .header("Authorization", auth_header.clone())
            .json(&payload)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Request failed: {}", e)))?;

        match response.status() {
            StatusCode::OK => {
                let mut result = response.json::<Value>().await.map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to parse response: {}", e))
                })?;

                // Check if we need to poll for results
                while let Some(chunk_link) = result
                    .get("result")
                    .and_then(|r| r.get("next_chunk_internal_link"))
                    .and_then(|l| l.as_str())
                {
                    // Wait a bit before polling
                    sleep(Duration::from_millis(500)).await;

                    let chunk_url = base_url.join(chunk_link).map_err(|e| {
                        ToolError::ExecutionError(format!("Invalid chunk URL: {}", e))
                    })?;

                    let chunk_response = self
                        .client
                        .get(chunk_url)
                        .header("Authorization", auth_header.clone())
                        .send()
                        .await
                        .map_err(|e| {
                            ToolError::ExecutionError(format!("Failed to get chunk: {}", e))
                        })?;

                    result = chunk_response.json::<Value>().await.map_err(|e| {
                        ToolError::ExecutionError(format!("Failed to parse chunk: {}", e))
                    })?;
                }

                // Format the results nicely
                let formatted = serde_json::to_string_pretty(&result).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to format results: {}", e))
                })?;

                Ok(vec![
                    Content::text(formatted.clone()).with_audience(vec![Role::Assistant]),
                    Content::text(formatted)
                        .with_audience(vec![Role::User])
                        .with_priority(0.0),
                ])
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ToolError::ExecutionError(
                "Authentication failed. Please check your Databricks token.".to_string(),
            )),
            _ => {
                let error = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(ToolError::ExecutionError(format!(
                    "Query failed: {}",
                    error
                )))
            }
        }
    }
}

#[async_trait]
impl Router for DatabricksRouter {
    fn name(&self) -> String {
        "databricks".to_string()
    }

    fn instructions(&self) -> String {
        String::from("The Databricks extension provides tools for interacting with Databricks SQL warehouses.\n\
        You can execute SQL queries and retrieve results using the sql_query tool.")
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(true)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "sql_query" => this.execute_query(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async move { Ok("".to_string()) })
    }

    fn list_prompts(&self) -> Option<Vec<mcp_core::prompt::Prompt>> {
        None
    }

    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Option<Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>>> {
        None
    }
}

impl Clone for DatabricksRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            client: self.client.clone(),
            host: self.host.clone(),
            warehouse_id: self.warehouse_id.clone(),
            auth: self.auth.clone(),
        }
    }
}
