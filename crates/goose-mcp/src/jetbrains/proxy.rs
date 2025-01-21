use anyhow::{anyhow, Result};
use mcp_core::{Content, Tool};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

const PORT_RANGE_START: u16 = 63342;
const PORT_RANGE_END: u16 = 63352;
const ENDPOINT_CHECK_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
struct IDEResponseOk {
    status: String,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IDEResponseErr {
    status: Option<String>,
    error: String,
}

#[derive(Debug, Serialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    pub is_error: bool,
}

#[derive(Debug)]
pub struct JetBrainsProxy {
    cached_endpoint: Arc<RwLock<Option<String>>>,
    previous_response: Arc<RwLock<Option<String>>>,
    client: Client,
}

impl JetBrainsProxy {
    pub fn new() -> Self {
        Self {
            cached_endpoint: Arc::new(RwLock::new(None)),
            previous_response: Arc::new(RwLock::new(None)),
            client: Client::new(),
        }
    }

    async fn test_list_tools(&self, endpoint: &str) -> Result<bool> {
        debug!("Sending test request to {}/mcp/list_tools", endpoint);

        let response = match self
            .client
            .get(format!("{}/mcp/list_tools", endpoint))
            .send()
            .await
        {
            Ok(resp) => {
                debug!("Got response with status: {}", resp.status());
                resp
            }
            Err(e) => {
                debug!("Error testing endpoint {}: {}", endpoint, e);
                return Ok(false);
            }
        };

        if !response.status().is_success() {
            debug!("Test request failed with status {}", response.status());
            return Ok(false);
        }

        let current_response = response.text().await?;
        debug!("Received response: {}", current_response);

        // Try to parse as JSON array to validate format
        if serde_json::from_str::<Vec<Value>>(&current_response).is_err() {
            debug!("Response is not a valid JSON array of tools");
            return Ok(false);
        }

        let mut prev_response = self.previous_response.write().await;
        if let Some(prev) = prev_response.as_ref() {
            if prev != &current_response {
                debug!("Response changed since last check");
                self.send_tools_changed().await;
            }
        }
        *prev_response = Some(current_response);

        Ok(true)
    }

    async fn find_working_ide_endpoint(&self) -> Result<String> {
        debug!("Attempting to find working IDE endpoint...");

        // Check IDE_PORT environment variable first
        if let Ok(port) = env::var("IDE_PORT") {
            debug!("Found IDE_PORT environment variable: {}", port);
            let test_endpoint = format!("http://127.0.0.1:{}/api", port);
            if self.test_list_tools(&test_endpoint).await? {
                debug!("IDE_PORT {} is working", port);
                return Ok(test_endpoint);
            }
            debug!("IDE_PORT {} is not responding correctly", port);
            return Err(anyhow!(
                "Specified IDE_PORT={} is not responding correctly",
                port
            ));
        }

        debug!(
            "No IDE_PORT environment variable, scanning port range {}-{}",
            PORT_RANGE_START, PORT_RANGE_END
        );

        // Scan port range
        for port in PORT_RANGE_START..=PORT_RANGE_END {
            let candidate_endpoint = format!("http://127.0.0.1:{}/api", port);
            debug!("Testing port {}...", port);

            if self.test_list_tools(&candidate_endpoint).await? {
                debug!("Found working IDE endpoint at {}", candidate_endpoint);
                return Ok(candidate_endpoint);
            }
        }

        debug!("No working IDE endpoint found in port range");
        Err(anyhow!(
            "No working IDE endpoint found in range {}-{}",
            PORT_RANGE_START,
            PORT_RANGE_END
        ))
    }

    async fn update_ide_endpoint(&self) {
        debug!("Updating IDE endpoint...");
        match self.find_working_ide_endpoint().await {
            Ok(endpoint) => {
                let mut cached = self.cached_endpoint.write().await;
                *cached = Some(endpoint.clone());
                debug!("Updated cached endpoint to: {}", endpoint);
            }
            Err(e) => {
                debug!("Failed to update IDE endpoint: {}", e);
                error!("Failed to update IDE endpoint: {}", e);
            }
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        debug!("Listing tools...");
        let endpoint = {
            let cached = self.cached_endpoint.read().await;
            match cached.as_ref() {
                Some(ep) => {
                    debug!("Using cached endpoint: {}", ep);
                    ep.clone()
                }
                None => {
                    debug!("No cached endpoint available");
                    return Ok(vec![]);
                }
            }
        };

        debug!("Sending list_tools request to {}/mcp/list_tools", endpoint);
        let response = match self
            .client
            .get(format!("{}/mcp/list_tools", endpoint))
            .send()
            .await
        {
            Ok(resp) => {
                debug!("Got response with status: {}", resp.status());
                resp
            }
            Err(e) => {
                debug!("Failed to send request: {}", e);
                return Err(anyhow!("Failed to send request: {}", e));
            }
        };

        if !response.status().is_success() {
            debug!("Request failed with status: {}", response.status());
            return Err(anyhow!(
                "Failed to fetch tools with status {}",
                response.status()
            ));
        }

        let response_text = response.text().await?;
        debug!("Got response text: {}", response_text);

        let tools_response: Value = serde_json::from_str(&response_text).map_err(|e| {
            debug!("Failed to parse response as JSON: {}", e);
            anyhow!("Failed to parse response as JSON: {}", e)
        })?;

        debug!("Parsed JSON response: {:?}", tools_response);

        let tools: Vec<Tool> = tools_response
            .as_array()
            .ok_or_else(|| {
                debug!("Response is not a JSON array");
                anyhow!("Invalid tools response format: not an array")
            })?
            .iter()
            .filter_map(|t| {
                if let (Some(name), Some(description)) =
                    (t["name"].as_str(), t["description"].as_str())
                {
                    // Get just the first sentence of the description
                    let first_sentence = description
                        .split('.')
                        .next()
                        .unwrap_or(description)
                        .trim()
                        .to_string()
                        + ".";

                    // Handle input_schema as either a string or an object
                    let input_schema = match &t["inputSchema"] {
                        Value::String(s) => Value::String(s.clone()),
                        Value::Object(o) => Value::Object(o.clone()),
                        _ => {
                            debug!(
                                "Invalid inputSchema format for tool {}: {:?}",
                                name, t["inputSchema"]
                            );
                            return None;
                        }
                    };

                    Some(Tool {
                        name: name.to_string(),
                        description: first_sentence,
                        input_schema,
                    })
                } else {
                    debug!("Skipping invalid tool entry: {:?}", t);
                    None
                }
            })
            .collect();

        debug!("Collected {} tools", tools.len());
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult> {
        let endpoint = self
            .cached_endpoint
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("No working IDE endpoint available"))?;

        debug!(
            "ENDPOINT: {} | Tool name: {} | args: {}",
            endpoint, name, args
        );

        let response = self
            .client
            .post(format!("{}/mcp/{}", endpoint, name))
            .json(&args)
            .send()
            .await?;

        if !response.status().is_success() {
            debug!("Response failed with status: {}", response.status());
            return Err(anyhow!("Response failed: {}", response.status()));
        }

        let ide_response: Value = response.json().await?;
        let (is_error, text) = match ide_response {
            Value::Object(map) => {
                let status = map.get("status").and_then(|v| v.as_str());
                let error = map.get("error").and_then(|v| v.as_str());

                match (status, error) {
                    (Some(s), None) => (false, s.to_string()),
                    (None, Some(e)) => (true, e.to_string()),
                    _ => {
                        debug!("Invalid response format from IDE");
                        return Err(anyhow!("Invalid response format from IDE"));
                    }
                }
            }
            _ => {
                debug!("Unexpected response type from IDE");
                return Err(anyhow!("Unexpected response type from IDE"));
            }
        };

        Ok(CallToolResult {
            content: vec![Content::text(text)],
            is_error,
        })
    }

    async fn send_tools_changed(&self) {
        debug!("Sending tools changed notification");
        // TODO: Implement notification mechanism when needed
    }

    pub async fn start(&self) -> Result<()> {
        debug!("Initializing JetBrains Proxy...");
        info!("Initializing JetBrains Proxy...");

        // Initial endpoint check
        debug!("Performing initial endpoint check...");
        self.update_ide_endpoint().await;

        // Schedule periodic endpoint checks
        let proxy = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(ENDPOINT_CHECK_INTERVAL).await;
                debug!("Performing periodic endpoint check...");
                proxy.update_ide_endpoint().await;
            }
        });

        debug!("JetBrains Proxy running");
        info!("JetBrains Proxy running");
        Ok(())
    }
}

impl Clone for JetBrainsProxy {
    fn clone(&self) -> Self {
        Self {
            cached_endpoint: Arc::clone(&self.cached_endpoint),
            previous_response: Arc::clone(&self.previous_response),
            client: Client::new(),
        }
    }
}
