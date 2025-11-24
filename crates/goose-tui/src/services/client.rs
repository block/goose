use crate::services::events::Event;
use crate::state::ToolInfo;
use anyhow::{Context, Result};
use eventsource_stream::Eventsource;
use goose::agents::ExtensionConfig;
use goose::config::ExtensionEntry;
use goose::conversation::message::Message;
use goose::providers::base::{ProviderMetadata, ProviderType};
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

#[derive(Serialize)]
struct ResumeAgentRequest {
    session_id: String,
    load_model_and_extensions: bool,
}

#[derive(Deserialize)]
struct SessionListResponse {
    sessions: Vec<Session>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ProviderDetails {
    pub name: String,
    pub metadata: ProviderMetadata,
    pub is_configured: bool,
    pub provider_type: ProviderType,
}

#[derive(Deserialize)]
struct ExtensionResponse {
    extensions: Vec<ExtensionEntry>,
}

#[derive(Serialize)]
struct UpsertConfigQuery {
    key: String,
    value: serde_json::Value,
    is_secret: bool,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct ConfigResponse {
    pub config: serde_json::Value,
}

impl Client {
    pub fn new(port: u16, secret_key: String) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            secret_key,
            http_client: ReqwestClient::new(),
        }
    }

    pub async fn get_providers(&self) -> Result<Vec<ProviderDetails>> {
        let response = self
            .http_client
            .get(format!("{}/config/providers", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to fetch providers")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get providers: {}", error_text);
        }

        let providers: Vec<ProviderDetails> = response
            .json()
            .await
            .context("Failed to parse providers list")?;
        Ok(providers)
    }

    pub async fn get_extensions(&self) -> Result<Vec<ExtensionEntry>> {
        let response = self
            .http_client
            .get(format!("{}/config/extensions", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to fetch extensions")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get extensions: {}", error_text);
        }

        let wrapper: ExtensionResponse = response
            .json()
            .await
            .context("Failed to parse extensions list")?;
        Ok(wrapper.extensions)
    }

    pub async fn get_provider_models(&self, provider: &str) -> Result<Vec<String>> {
        let response = self
            .http_client
            .get(format!(
                "{}/config/providers/{}/models",
                self.base_url, provider
            ))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to fetch provider models")?;

        if !response.status().is_success() {
            // If we get a 400 or similar (e.g. unconfigured), we might just want to return empty list or handle gracefully
            // But for now bail with error text
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get provider models: {}", error_text);
        }

        let models: Vec<String> = response
            .json()
            .await
            .context("Failed to parse provider models list")?;
        Ok(models)
    }

    pub async fn upsert_config(
        &self,
        key: &str,
        value: serde_json::Value,
        is_secret: bool,
    ) -> Result<()> {
        let response = self
            .http_client
            .post(format!("{}/config/upsert", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&UpsertConfigQuery {
                key: key.to_string(),
                value,
                is_secret,
            })
            .send()
            .await
            .context("Failed to upsert config")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upsert config: {}", error_text);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn read_config(&self) -> Result<serde_json::Value> {
        let response = self
            .http_client
            .get(format!("{}/config", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to read config")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to read config: {}", error_text);
        }

        let wrapper: ConfigResponse = response.json().await.context("Failed to parse config")?;
        Ok(wrapper.config)
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let response = self
            .http_client
            .get(format!("{}/sessions", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to send list sessions request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list sessions: {}", error_text);
        }

        let wrapper: SessionListResponse = response
            .json()
            .await
            .context("Failed to parse sessions list")?;
        Ok(wrapper.sessions)
    }

    pub async fn remove_extension(&self, session_id: &str, name: &str) -> Result<()> {
        let response = self
            .http_client
            .post(format!("{}/agent/remove_extension", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&serde_json::json!({
                "session_id": session_id,
                "name": name
            }))
            .send()
            .await
            .context("Failed to remove extension from session")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to remove extension: {}", error_text);
        }
        Ok(())
    }

    pub async fn add_config_extension(
        &self,
        name: String,
        config: ExtensionConfig,
        enabled: bool,
    ) -> Result<()> {
        let response = self
            .http_client
            .post(format!("{}/config/extensions", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&serde_json::json!({
                "name": name,
                "config": config,
                "enabled": enabled
            }))
            .send()
            .await
            .context("Failed to add config extension")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to add config extension: {}", error_text);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn remove_config_extension(&self, name: &str) -> Result<()> {
        let response = self
            .http_client
            .delete(format!("{}/config/extensions/{}", self.base_url, name))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await
            .context("Failed to remove config extension")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to remove config extension: {}", error_text);
        }
        Ok(())
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

    pub async fn resume_agent(&self, session_id: &str) -> Result<Session> {
        let response = self
            .http_client
            .post(format!("{}/agent/resume", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ResumeAgentRequest {
                session_id: session_id.to_string(),
                load_model_and_extensions: true,
            })
            .send()
            .await
            .context("Failed to send resume request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to resume agent: {}", error_text);
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
        tx: mpsc::UnboundedSender<Event>,
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
            .error_for_status()?
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
                            let _ = tx.send(Event::Server(std::sync::Arc::new(msg)));
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse SSE event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("SSE stream error: {}", e);
                    let _ = tx.send(Event::Error(e.to_string()));
                }
            }
        }

        Ok(())
    }
}
