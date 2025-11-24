mod types;

use anyhow::{Context, Result};
use eventsource_stream::Eventsource;
use futures::stream::Stream;
use goose::agents::ExtensionConfig;
use goose::config::ExtensionEntry;
use goose::conversation::message::Message;
use goose::session::Session;
use goose_server::routes::reply::MessageEvent;
use reqwest::Client as ReqwestClient;
use tokio_stream::StreamExt;

use types::{
    AddExtensionRequest, ChatRequest, ConfigResponse, ExtensionQuery, ExtensionResponse,
    RemoveExtensionRequest, ResumeAgentRequest, SessionListResponse, StartAgentRequest,
    UpdateProviderRequest, UpsertConfigQuery,
};
pub use types::{ProviderDetails, ToolInfo};

pub struct ClientBuilder {
    host: String,
    port: u16,
    secret_key: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 0,
            secret_key: None,
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn build(self) -> Result<Client> {
        let secret_key = self
            .secret_key
            .ok_or_else(|| anyhow::anyhow!("Secret key is required"))?;
        Ok(Client {
            base_url: format!("http://{}:{}", self.host, self.port),
            secret_key,
            http_client: ReqwestClient::new(),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Client {
    base_url: String,
    secret_key: String,
    http_client: ReqwestClient,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn new(port: u16, secret_key: String) -> Self {
        Self::builder()
            .port(port)
            .secret_key(secret_key)
            .build()
            .expect("Failed to build client with provided parameters")
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
        // Convert HashMap to Value
        let map: serde_json::Map<String, serde_json::Value> = wrapper.config.into_iter().collect();
        Ok(serde_json::Value::Object(map))
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
            .json(&RemoveExtensionRequest {
                session_id: session_id.to_string(),
                name: name.to_string(),
            })
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
            .json(&ExtensionQuery {
                name,
                config,
                enabled,
            })
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
            .json(&StartAgentRequest {
                working_dir,
                recipe: None,
                recipe_id: None,
                recipe_deeplink: None,
            })
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
            .json(&AddExtensionRequest {
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
    ) -> Result<impl Stream<Item = Result<MessageEvent>>> {
        let stream = self
            .http_client
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&ChatRequest {
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

        Ok(stream.map(|event| match event {
            Ok(event) => serde_json::from_str::<MessageEvent>(&event.data)
                .context("Failed to parse SSE event"),
            Err(e) => Err(anyhow::anyhow!("SSE stream error: {}", e)),
        }))
    }
}
