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
        // Increase HTTP client timeout to 20 minutes (1200 seconds) to handle long-running queries
        let client = Client::builder()
            .timeout(Duration::from_secs(1200))
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

        // Extract statement and check if it's valid
        let statement = params
            .get("statement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionError("SQL statement is required".to_string()))?;

        if statement.trim().is_empty() {
            return Err(ToolError::ExecutionError("SQL statement cannot be empty".to_string()));
        }

        // Create payload with more flexible timeout settings
        let mut payload = json!({
            "warehouse_id": self.warehouse_id,
            "statement": statement,
            // Databricks SQL API has specific timeout formats, remove explicit timeout
            // to use default behavior which is more reliable
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

        
        // Get auth header
        let auth_header = self.ensure_auth_header().await?;
        
        // Submit the query
        let response = self
            .client
            .post(url.clone())
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
                
                // Extract statement ID for asynchronous queries
                let statement_id = match result.get("statement_id").and_then(|id| id.as_str()) {
                    Some(id) => Some(id.to_string()),
                    None => None
                };

                // Maximum number of polling attempts (90 attempts × 2 seconds = 180 seconds max)
                // Databricks SQL warehouses can take time to spin up if they were in standby mode
                let max_polling_attempts = 90; 
                let mut polling_attempts = 0;
                
                // Add an initial delay to allow the warehouse to warm up if needed
                sleep(Duration::from_secs(3)).await;
                
                // Check query status and keep polling if necessary
                let mut is_running = true;
                while is_running && polling_attempts < max_polling_attempts {
                    // Check the status of the query
                    let status = result
                        .get("status")
                        .and_then(|s| s.get("state"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("UNKNOWN");
                    
                    match status {
                        "SUCCEEDED" | "FINISHED" | "CLOSED" => {
                            is_running = false;
                        },
                        "PENDING" | "RUNNING" | "QUEUED" | "WAITING" => {
                            // Query is still running, wait and poll again
                            polling_attempts += 1;
                            
                            // Exponential backoff: Wait longer for later attempts (2-15 seconds)
                            let wait_time = std::cmp::min(2 + (polling_attempts / 10), 15);
                            sleep(Duration::from_secs(wait_time)).await;
                            
                            // Need to poll for results using statement ID
                            if let Some(ref id) = statement_id {
                                let poll_url = base_url
                                    .join(&format!("api/2.0/sql/statements/{}", id))
                                    .map_err(|e| ToolError::ExecutionError(format!("Invalid poll URL: {}", e)))?;
                                
                                let poll_response = self
                                    .client
                                    .get(poll_url)
                                    .header("Authorization", auth_header.clone())
                                    .send()
                                    .await
                                    .map_err(|e| ToolError::ExecutionError(
                                        format!("Failed to poll query status: {}", e)
                                    ))?;
                                
                                if poll_response.status() == StatusCode::OK {
                                    result = poll_response.json::<Value>().await.map_err(|e| {
                                        ToolError::ExecutionError(format!("Failed to parse poll response: {}", e))
                                    })?;
                                } else {
                                    return Err(ToolError::ExecutionError(
                                        format!("Poll request failed with status: {}", poll_response.status())
                                    ));
                                }
                            } else {
                                return Err(ToolError::ExecutionError(
                                    "Query is running but no statement ID was provided for polling".to_string()
                                ));
                            }
                        },
                        "FAILED" | "CANCELED" => {
                            // Query failed or was canceled
                            let error_message = result
                                .get("status")
                                .and_then(|s| s.get("error"))
                                .and_then(|e| e.get("message"))
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown error");
                            
                            return Err(ToolError::ExecutionError(format!(
                                "Query failed with status {}: {}", 
                                status, error_message
                            )));
                        },
                        _ => {
                            // Unknown status
                            return Err(ToolError::ExecutionError(format!(
                                "Query has unknown status: {}", 
                                status
                            )));
                        }
                    }
                }
                
                if polling_attempts >= max_polling_attempts {
                    return Err(ToolError::ExecutionError(
                        "The Databricks SQL query timed out. This could be because:\n\
                        1. The query is complex and needs more time to complete\n\
                        2. The SQL warehouse is starting up from a standby state\n\
                        3. There are connection issues with the Databricks service\n\n\
                        Please try again with a simpler query or after ensuring the SQL warehouse is in a running state.".to_string()
                    ));
                }

                // Now handle chunked results (if query was successful)
                // Check if we need to poll for result chunks
                while let Some(chunk_link) = result
                    .get("result")
                    .and_then(|r| r.get("next_chunk_internal_link"))
                    .and_then(|l| l.as_str())
                {
                    
                    // Wait a bit before fetching next chunk
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

                // Check for empty results
                let has_data = result.get("result")
                    .and_then(|r| r.get("data_array"))
                    .is_some();
                
                if !has_data {
                    // Check if we have a schema but no data (empty result set)
                    let has_schema = result.get("result")
                        .and_then(|r| r.get("schema"))
                        .is_some();
                    
                }

                // Format the results nicely
                let formatted = serde_json::to_string_pretty(&result).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to format results: {}", e))
                })?;

                // Create a more user-friendly format for the results
                let user_friendly = self.format_friendly_output(&result)?;

                Ok(vec![
                    // Raw JSON for the assistant
                    Content::text(formatted).with_audience(vec![Role::Assistant]),
                    // Formatted results for the user
                    Content::text(user_friendly)
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
    
    fn format_friendly_output(&self, result: &Value) -> Result<String, ToolError> {
        // First, check if we have a valid result structure
        let result_obj = match result.get("result") {
            Some(r) => r,
            None => return Ok(format!("No result data available\n{}", serde_json::to_string_pretty(result).unwrap_or_default())),
        };
        
        // Get the schema for column names
        let schema = match result_obj.get("schema") {
            Some(s) => s,
            None => return Ok(format!("No schema available in result\n{}", serde_json::to_string_pretty(result_obj).unwrap_or_default())),
        };
        
        // Extract column names from schema
        let columns = match schema.get("columns") {
            Some(c) if c.is_array() => c.as_array().unwrap(),
            _ => return Ok(format!("Invalid or missing columns in schema\n{}", serde_json::to_string_pretty(schema).unwrap_or_default())),
        };
        
        let column_names: Vec<String> = columns.iter()
            .filter_map(|col| col.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect();
        
        if column_names.is_empty() {
            return Ok(format!("Could not extract column names from schema\n{}", serde_json::to_string_pretty(schema).unwrap_or_default()));
        }
        
        // Get the data rows
        let data_array = match result_obj.get("data_array") {
            Some(d) if d.is_array() => d.as_array().unwrap(),
            _ => return Ok(format!("No data rows available in result\nColumns: {}\n", column_names.join(", "))),
        };
        
        if data_array.is_empty() {
            return Ok(format!("Query returned 0 rows\nColumns: {}", column_names.join(", ")));
        }
        
        // Build the table header
        let mut output = String::new();
        output.push_str(&format!("Query returned {} rows\n\n", data_array.len()));
        
        // Add column headers
        output.push_str(&column_names.join(" | "));
        output.push_str("\n");
        output.push_str(&column_names.iter().map(|_| "---------").collect::<Vec<_>>().join("|"));
        output.push_str("\n");
        
        // Add data rows
        for row in data_array.iter().take(100) { // Limit to 100 rows for display
            if let Some(row_array) = row.as_array() {
                let row_values: Vec<String> = row_array.iter()
                    .map(|v| match v {
                        Value::Null => "NULL".to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s.clone(),
                        _ => serde_json::to_string(v).unwrap_or_default(),
                    })
                    .collect();
                output.push_str(&row_values.join(" | "));
                output.push_str("\n");
            }
        }
        
        // If there are more than 100 rows, indicate that we're truncating
        if data_array.len() > 100 {
            output.push_str(&format!("\n[Output truncated. Showing 100 of {} rows]", data_array.len()));
        }
        
        Ok(output)
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

    fn list_prompts(&self) -> Vec<mcp_core::prompt::Prompt> {
        Vec::new()
    }

    fn get_prompt(
        &self,
        _prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        Box::pin(async move { Err(PromptError::NotFound("No prompts available".to_string())) })
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
