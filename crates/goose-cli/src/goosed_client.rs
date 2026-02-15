use anyhow::Result;
use goose::prompt_template::Template;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::session::Session;
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PromptsListResponse {
    pub prompts: Vec<Template>,
}

#[derive(Deserialize)]
pub struct PromptContentResponse {
    pub name: String,
    pub content: String,
    pub default_content: String,
    pub is_customized: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfoResponse {
    pub provider_name: String,
    pub model_name: String,
    pub context_limit: usize,
    pub total_tokens: Option<i32>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

#[derive(Deserialize)]
pub struct PlanPromptResponse {
    pub prompt: String,
}

pub struct GoosedClient {
    api_client: ApiClient,
}

impl GoosedClient {
    pub fn from_env_if_configured() -> Result<Option<Self>> {
        if Self::is_configured() {
            Self::from_env().map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = Self::base_url_from_env();
        let secret_key =
            std::env::var("GOOSE_SERVER__SECRET_KEY").unwrap_or_else(|_| "test".to_string());
        Self::new(base_url, secret_key)
    }

    pub fn new(base_url: String, secret_key: String) -> Result<Self> {
        let api_client = ApiClient::new(
            base_url,
            AuthMethod::ApiKey {
                header_name: "X-Secret-Key".to_string(),
                key: secret_key,
            },
        )?;
        Ok(Self { api_client })
    }

    pub fn is_configured() -> bool {
        std::env::var("GOOSE_SERVER_URL").is_ok()
            || std::env::var("GOOSE_HOST").is_ok()
            || std::env::var("GOOSE_PORT").is_ok()
            || std::env::var("GOOSE_SERVER__SECRET_KEY").is_ok()
    }

    fn base_url_from_env() -> String {
        if let Ok(url) = std::env::var("GOOSE_SERVER_URL") {
            return url;
        }

        let host = std::env::var("GOOSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("GOOSE_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);

        if host.starts_with("http://") || host.starts_with("https://") {
            if host.contains(':') {
                return host;
            }
            return format!("{}:{}", host, port);
        }

        format!("http://{}:{}", host, port)
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        session_id: Option<&str>,
        path: &str,
    ) -> Result<T> {
        let response = self.api_client.response_get(session_id, path).await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Request failed ({}): {}", status, body));
        }
        Ok(response.json::<T>().await?)
    }

    async fn post_empty(&self, session_id: Option<&str>, path: &str) -> Result<()> {
        let response = self
            .api_client
            .response_post(session_id, path, &serde_json::json!({}))
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Request failed ({}): {}", status, body));
        }
        Ok(())
    }

    pub async fn list_prompts(&self) -> Result<Vec<Template>> {
        let response: PromptsListResponse = self.get_json(None, "/config/prompts").await?;
        Ok(response.prompts)
    }

    pub async fn get_prompt_content(&self, name: &str) -> Result<PromptContentResponse> {
        let path = format!("/config/prompts/{}", urlencoding::encode(name));
        self.get_json(None, &path).await
    }

    pub async fn get_provider_info(&self, session_id: &str) -> Result<ProviderInfoResponse> {
        let path = format!(
            "/agent/provider_info?session_id={}",
            urlencoding::encode(session_id)
        );
        self.get_json(Some(session_id), &path).await
    }

    pub async fn get_plan_prompt(&self, session_id: &str) -> Result<String> {
        let path = format!(
            "/agent/plan_prompt?session_id={}",
            urlencoding::encode(session_id)
        );
        let response: PlanPromptResponse = self.get_json(Some(session_id), &path).await?;
        Ok(response.prompt)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let path = format!("/sessions/{}", urlencoding::encode(session_id));
        self.get_json(Some(session_id), &path).await
    }

    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let path = format!("/sessions/{}/clear", urlencoding::encode(session_id));
        self.post_empty(Some(session_id), &path).await
    }
}
