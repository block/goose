use anyhow::{Context, Result};
use eventsource_stream::Eventsource;
use goose::agents::ExtensionConfig;
use goose::conversation::message::Message;
use goose::session::Session;
use goose_server::routes::reply::MessageEvent;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct Client {
    base_url: String,
    secret_key: String,
    http_client: ReqwestClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
}

#[derive(Serialize)]
struct ChatRequestPayload {
    messages: Vec<Message>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
}

#[derive(Serialize)]
struct UpdateProviderRequest {
    provider: String,
    model: Option<String>,
    session_id: String,
}

#[derive(Serialize)]
struct AddExtensionRequestPayload {
    session_id: String,
    config: ExtensionConfig,
}

impl Client {
    pub fn new(port: u16, secret_key: String) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{}", port),
            secret_key,
            http_client: ReqwestClient::new(),
        }
    }

    pub async fn start_agent(&self, working_dir: String) -> Result<Session> {
        let response = self
            .http_client
            .post(format!("{}/agent/start", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&serde_json::json!({
                "working_dir": working_dir
            }))
            .send()
            .await
            .context("Failed to send start request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start agent: {}", error_text);
        }

        response
            .json::<Session>()
            .await
            .context("Failed to parse session response")
    }

    pub async fn update_provider(
        &self,
        session_id: &str,
        provider: String,
        model: Option<String>,
    ) -> Result<()> {
        let response = self
            .http_client
            .post(format!("{}/agent/update_provider", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&UpdateProviderRequest {
                provider,
                model,
                session_id: session_id.to_string(),
            })
            .send()
            .await
            .context("Failed to send update provider request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update provider: {}", error_text);
        }

        Ok(())
    }

    pub async fn add_extension(&self, session_id: &str, config: ExtensionConfig) -> Result<()> {
        let response = self
            .http_client
            .post(format!("{}/agent/add_extension", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&AddExtensionRequestPayload {
                session_id: session_id.to_string(),
                config,
            })
            .send()
            .await
            .context("Failed to send add extension request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to add extension: {}", error_text);
        }

        Ok(())
    }

    pub async fn get_tools(&self, session_id: &str) -> Result<Vec<ToolInfo>> {
        let response = self
            .http_client
            .get(format!("{}/agent/tools", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .query(&[("session_id", session_id)])
            .send()
            .await
            .context("Failed to fetch tools")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get tools: {}", error_text);
        }

        response
            .json::<Vec<ToolInfo>>()
            .await
            .context("Failed to parse tools list")
    }

    pub async fn reply(
        &self,
        messages: Vec<Message>,
        session_id: String,
        tx: mpsc::UnboundedSender<super::event::Event>,
    ) -> Result<()> {
        let mut stream = self
            .http_client
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ChatRequestPayload {
                messages,
                session_id,
                recipe_name: None,
                recipe_version: None,
            })
            .send()
            .await?
            .bytes_stream()
            .eventsource();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        break;
                    }
                    match serde_json::from_str::<MessageEvent>(&event.data) {
                        Ok(msg) => {
                            let _ = tx.send(super::event::Event::Server(msg));
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse SSE event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("SSE stream error: {}", e);
                    let _ = tx.send(super::event::Event::Error(e.to_string()));
                }
            }
        }

        Ok(())
    }
}
