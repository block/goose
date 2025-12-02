mod types;

use anyhow::{bail, Context, Result};
use eventsource_stream::Eventsource;
use futures::stream::Stream;
use goose::agents::ExtensionConfig;
use goose::config::ExtensionEntry;
use goose::conversation::message::Message;
use goose::session::Session;
use goose_server::routes::reply::MessageEvent;
use reqwest::{Client as ReqwestClient, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;
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

        let mut headers = reqwest::header::HeaderMap::new();
        let mut secret_value = reqwest::header::HeaderValue::from_str(&secret_key)
            .context("Invalid secret key format")?;
        secret_value.set_sensitive(true);
        headers.insert("X-Secret-Key", secret_value);

        let http = ReqwestClient::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Client {
            base_url: format!("http://{}:{}", self.host, self.port),
            http,
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
    http: ReqwestClient,
}

struct Request(RequestBuilder);

impl Request {
    fn new(inner: RequestBuilder) -> Self {
        Self(inner)
    }

    fn json<T: Serialize>(self, body: &T) -> Self {
        Self(self.0.json(body))
    }

    fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        Self(self.0.query(query))
    }

    async fn send<T: DeserializeOwned>(self) -> Result<T> {
        let response = self.0.send().await.context("Request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!("HTTP {}: {}", status.as_u16(), error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn send_empty(self) -> Result<()> {
        let response = self.0.send().await.context("Request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!("HTTP {}: {}", status.as_u16(), error_text);
        }

        Ok(())
    }
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

    fn get(&self, path: &str) -> Request {
        Request::new(self.http.get(format!("{}{}", self.base_url, path)))
    }

    fn post(&self, path: &str) -> Request {
        Request::new(self.http.post(format!("{}{}", self.base_url, path)))
    }

    fn delete(&self, path: &str) -> Request {
        Request::new(self.http.delete(format!("{}{}", self.base_url, path)))
    }

    pub async fn get_providers(&self) -> Result<Vec<ProviderDetails>> {
        self.get("/config/providers")
            .send()
            .await
            .context("Failed to fetch providers")
    }

    pub async fn get_extensions(&self) -> Result<Vec<ExtensionEntry>> {
        let response: ExtensionResponse = self
            .get("/config/extensions")
            .send()
            .await
            .context("Failed to fetch extensions")?;
        Ok(response.extensions)
    }

    pub async fn get_provider_models(&self, provider: &str) -> Result<Vec<String>> {
        self.get(&format!("/config/providers/{provider}/models"))
            .send()
            .await
            .context("Failed to fetch provider models")
    }

    pub async fn upsert_config(
        &self,
        key: &str,
        value: serde_json::Value,
        is_secret: bool,
    ) -> Result<()> {
        self.post("/config/upsert")
            .json(&UpsertConfigQuery {
                key: key.to_string(),
                value,
                is_secret,
            })
            .send_empty()
            .await
            .context("Failed to upsert config")
    }

    pub async fn read_config(&self) -> Result<serde_json::Value> {
        let response: ConfigResponse = self
            .get("/config")
            .send()
            .await
            .context("Failed to read config")?;
        Ok(serde_json::Value::Object(
            response.config.into_iter().collect(),
        ))
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let response: SessionListResponse = self
            .get("/sessions")
            .send()
            .await
            .context("Failed to list sessions")?;
        Ok(response.sessions)
    }

    pub async fn remove_extension(&self, session_id: &str, name: &str) -> Result<()> {
        self.post("/agent/remove_extension")
            .json(&RemoveExtensionRequest {
                session_id: session_id.to_string(),
                name: name.to_string(),
            })
            .send_empty()
            .await
            .context("Failed to remove extension from session")
    }

    pub async fn add_config_extension(
        &self,
        name: String,
        config: ExtensionConfig,
        enabled: bool,
    ) -> Result<()> {
        self.post("/config/extensions")
            .json(&ExtensionQuery {
                name,
                config,
                enabled,
            })
            .send_empty()
            .await
            .context("Failed to add config extension")
    }

    pub async fn remove_config_extension(&self, name: &str) -> Result<()> {
        self.delete(&format!("/config/extensions/{name}"))
            .send_empty()
            .await
            .context("Failed to remove config extension")
    }

    pub async fn start_agent(&self, working_dir: String) -> Result<Session> {
        self.start_agent_inner(working_dir, None).await
    }

    pub async fn start_agent_with_recipe(
        &self,
        working_dir: String,
        recipe: goose::recipe::Recipe,
    ) -> Result<Session> {
        self.start_agent_inner(working_dir, Some(recipe)).await
    }

    async fn start_agent_inner(
        &self,
        working_dir: String,
        recipe: Option<goose::recipe::Recipe>,
    ) -> Result<Session> {
        self.post("/agent/start")
            .json(&StartAgentRequest {
                working_dir,
                recipe,
                recipe_id: None,
                recipe_deeplink: None,
            })
            .send()
            .await
            .context("Failed to start agent")
    }

    pub async fn resume_agent(&self, session_id: &str) -> Result<Session> {
        self.post("/agent/resume")
            .json(&ResumeAgentRequest {
                session_id: session_id.to_string(),
                load_model_and_extensions: true,
            })
            .send()
            .await
            .context("Failed to resume agent")
    }

    pub async fn update_provider(
        &self,
        session_id: &str,
        provider: String,
        model: Option<String>,
    ) -> Result<()> {
        self.post("/agent/update_provider")
            .json(&UpdateProviderRequest {
                provider,
                model,
                session_id: session_id.to_string(),
            })
            .send_empty()
            .await
            .context("Failed to update provider")
    }

    pub async fn add_extension(&self, session_id: &str, config: ExtensionConfig) -> Result<()> {
        self.post("/agent/add_extension")
            .json(&AddExtensionRequest {
                session_id: session_id.to_string(),
                config,
            })
            .send_empty()
            .await
            .context("Failed to add extension")
    }

    pub async fn get_tools(&self, session_id: &str) -> Result<Vec<ToolInfo>> {
        self.get("/agent/tools")
            .query(&[("session_id", session_id)])
            .send()
            .await
            .context("Failed to fetch tools")
    }

    pub async fn export_session(&self, session_id: &str) -> Result<String> {
        self.get(&format!("/sessions/{session_id}/export"))
            .send()
            .await
            .context("Failed to export session")
    }

    pub async fn import_session(&self, json: &str) -> Result<Session> {
        self.post("/sessions/import")
            .json(&serde_json::json!({ "json": json }))
            .send()
            .await
            .context("Failed to import session")
    }

    pub async fn update_session_name(&self, session_id: &str, name: &str) -> Result<()> {
        self.http
            .put(format!("{}/sessions/{}/name", self.base_url, session_id))
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await
            .context("Request failed")?
            .error_for_status()
            .context("Failed to update session name")?;
        Ok(())
    }

    pub async fn reply(
        &self,
        messages: Vec<Message>,
        session_id: String,
    ) -> Result<impl Stream<Item = Result<MessageEvent>>> {
        let stream = self
            .http
            .post(format!("{}/reply", self.base_url))
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
