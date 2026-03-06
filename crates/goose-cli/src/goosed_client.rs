use anyhow::Result;
use futures::Stream;
use goose::agents::ExtensionConfig;
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::Permission;
use goose::prompt_template::Template;
use goose::recipe::Recipe;
use goose::session::Session;
use reqwest::Client;
use rmcp::model::{GetPromptResult, Prompt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// Re-use the server's MessageEvent type for deserialization.
// We define a deserializable mirror since the server type only derives Serialize.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MessageEvent {
    Message {
        message: Message,
        token_state: TokenState,
    },
    Error {
        error: String,
    },
    Finish {
        reason: String,
        token_state: TokenState,
    },
    ModelChange {
        model: String,
        mode: String,
    },
    Notification {
        request_id: String,
        message: serde_json::Value,
    },
    UpdateConversation {
        conversation: Conversation,
    },
    Ping,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TokenState {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
    pub accumulated_input_tokens: i32,
    pub accumulated_output_tokens: i32,
    pub accumulated_total_tokens: i32,
}

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

#[derive(Deserialize)]
pub struct ExtensionPromptsResponse {
    pub prompts: std::collections::HashMap<String, Vec<Prompt>>,
}

#[derive(Serialize)]
struct GetExtensionPromptRequest {
    session_id: String,
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

#[derive(Serialize)]
struct ChatRequest {
    user_message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_so_far: Option<Vec<Message>>,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmToolActionRequest {
    id: String,
    principal_type: PrincipalType,
    action: Permission,
    session_id: String,
}

#[derive(Serialize)]
struct AddExtensionRequest {
    session_id: String,
    config: ExtensionConfig,
}

#[derive(Serialize)]
struct StartAgentRequest {
    working_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe: Option<Recipe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe_deeplink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension_overrides: Option<Vec<ExtensionConfig>>,
}

#[derive(Serialize)]
struct ResumeAgentRequest {
    session_id: String,
    load_model_and_extensions: bool,
}

#[derive(Serialize)]
struct UpdateProviderRequest {
    provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct ExtendSystemPromptRequest {
    session_id: String,
    prompt: String,
}

#[derive(Serialize)]
struct OverrideSystemPromptRequest {
    session_id: String,
    prompt: String,
}

#[derive(Serialize)]
struct PersistExtensionStateRequest {
    session_id: String,
}

#[derive(Serialize)]
struct AddMessageRequest {
    message: Message,
}

#[derive(Serialize)]
struct CreateRecipeRequest {
    session_id: String,
}

#[derive(Deserialize)]
struct CreateRecipeResponse {
    recipe: Option<Recipe>,
    error: Option<String>,
}

#[derive(Serialize)]
struct StopAgentRequest {
    session_id: String,
}

#[derive(Serialize)]
struct ClearSessionRequest {}

/// SSE event stream from goosed /reply endpoint
pub struct SseEventStream {
    rx: ReceiverStream<Result<MessageEvent>>,
}

impl Stream for SseEventStream {
    type Item = Result<MessageEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

#[derive(Clone)]
pub struct GoosedClient {
    client: Client,
    base_url: String,
    secret_key: String,
}

impl GoosedClient {
    pub fn new(base_url: String, secret_key: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()?;
        Ok(Self {
            client,
            base_url,
            secret_key,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .get(self.url(path))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "GET {} failed ({}): {}",
                path,
                status,
                body
            ));
        }
        Ok(response.json::<T>().await?)
    }

    async fn post_json<T: serde::de::DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .client
            .post(self.url(path))
            .header("X-Secret-Key", &self.secret_key)
            .json(body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "POST {} failed ({}): {}",
                path,
                status,
                body_text
            ));
        }
        Ok(response.json::<T>().await?)
    }

    async fn post_status<B: Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let response = self
            .client
            .post(self.url(path))
            .header("X-Secret-Key", &self.secret_key)
            .json(body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "POST {} failed ({}): {}",
                path,
                status,
                body_text
            ));
        }
        Ok(())
    }

    // --- Prompt template endpoints ---

    pub async fn list_prompts(&self) -> Result<Vec<Template>> {
        let response: PromptsListResponse = self.get_json("/config/prompts").await?;
        Ok(response.prompts)
    }

    pub async fn get_prompt_content(&self, name: &str) -> Result<PromptContentResponse> {
        let path = format!("/config/prompts/{}", urlencoding::encode(name));
        self.get_json(&path).await
    }

    // --- Agent info endpoints ---

    pub async fn get_provider_info(&self, session_id: &str) -> Result<ProviderInfoResponse> {
        let path = format!(
            "/agent/provider_info?session_id={}",
            urlencoding::encode(session_id)
        );
        self.get_json(&path).await
    }

    pub async fn get_plan_prompt(&self, session_id: &str) -> Result<String> {
        let path = format!(
            "/agent/plan_prompt?session_id={}",
            urlencoding::encode(session_id)
        );
        let response: PlanPromptResponse = self.get_json(&path).await?;
        Ok(response.prompt)
    }

    // --- Extension prompts endpoints ---

    pub async fn list_extension_prompts(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashMap<String, Vec<Prompt>>> {
        let path = format!(
            "/agent/extension_prompts?session_id={}",
            urlencoding::encode(session_id)
        );
        let response: ExtensionPromptsResponse = self.get_json(&path).await?;
        Ok(response.prompts)
    }

    pub async fn get_extension_prompt(
        &self,
        session_id: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<GetPromptResult> {
        self.post_json(
            "/agent/get_prompt",
            &GetExtensionPromptRequest {
                session_id: session_id.to_string(),
                name: name.to_string(),
                arguments,
            },
        )
        .await
    }

    // --- Session endpoints ---

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let path = format!("/sessions/{}", urlencoding::encode(session_id));
        self.get_json(&path).await
    }

    pub async fn clear_session(&self, session_id: &str) -> Result<()> {
        let path = format!("/sessions/{}/clear", urlencoding::encode(session_id));
        self.post_status(&path, &ClearSessionRequest {}).await
    }

    pub async fn add_message(&self, session_id: &str, message: &Message) -> Result<()> {
        let path = format!("/sessions/{}/messages", urlencoding::encode(session_id));
        self.post_status(
            &path,
            &AddMessageRequest {
                message: message.clone(),
            },
        )
        .await
    }

    // --- Agent lifecycle endpoints ---

    pub async fn start_agent(
        &self,
        working_dir: String,
        recipe: Option<Recipe>,
        extension_overrides: Option<Vec<ExtensionConfig>>,
    ) -> Result<Session> {
        self.post_json(
            "/agent/start",
            &StartAgentRequest {
                working_dir,
                recipe,
                recipe_id: None,
                recipe_deeplink: None,
                extension_overrides,
            },
        )
        .await
    }

    pub async fn resume_agent(&self, session_id: &str) -> Result<serde_json::Value> {
        self.post_json(
            "/agent/resume",
            &ResumeAgentRequest {
                session_id: session_id.to_string(),
                load_model_and_extensions: true,
            },
        )
        .await
    }

    pub async fn stop_agent(&self, session_id: &str) -> Result<()> {
        self.post_status(
            "/agent/stop",
            &StopAgentRequest {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    pub async fn update_provider(
        &self,
        provider: &str,
        model: Option<String>,
        session_id: &str,
    ) -> Result<()> {
        self.post_status(
            "/agent/update_provider",
            &UpdateProviderRequest {
                provider: provider.to_string(),
                model,
                session_id: session_id.to_string(),
                context_limit: None,
                request_params: None,
            },
        )
        .await
    }

    // --- Extension endpoints ---

    pub async fn add_extension(&self, config: ExtensionConfig, session_id: &str) -> Result<()> {
        self.post_status(
            "/agent/add_extension",
            &AddExtensionRequest {
                session_id: session_id.to_string(),
                config,
            },
        )
        .await
    }

    // --- System prompt endpoints ---

    pub async fn extend_system_prompt(&self, session_id: &str, prompt: &str) -> Result<()> {
        self.post_status(
            "/agent/extend_system_prompt",
            &ExtendSystemPromptRequest {
                session_id: session_id.to_string(),
                prompt: prompt.to_string(),
            },
        )
        .await
    }

    pub async fn override_system_prompt(&self, session_id: &str, prompt: &str) -> Result<()> {
        self.post_status(
            "/agent/override_system_prompt",
            &OverrideSystemPromptRequest {
                session_id: session_id.to_string(),
                prompt: prompt.to_string(),
            },
        )
        .await
    }

    pub async fn persist_extension_state(&self, session_id: &str) -> Result<()> {
        self.post_status(
            "/agent/persist_extension_state",
            &PersistExtensionStateRequest {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    // --- Recipe endpoints ---

    pub async fn create_recipe(&self, session_id: &str) -> Result<Recipe> {
        let response: CreateRecipeResponse = self
            .post_json(
                "/recipes/create",
                &CreateRecipeRequest {
                    session_id: session_id.to_string(),
                },
            )
            .await?;
        match response.recipe {
            Some(recipe) => Ok(recipe),
            None => Err(anyhow::anyhow!(
                "{}",
                response.error.unwrap_or_else(|| "Unknown error".into())
            )),
        }
    }

    // --- Reply (SSE streaming) ---

    pub async fn reply(&self, user_message: Message, session_id: &str) -> Result<SseEventStream> {
        let request = ChatRequest {
            user_message,
            conversation_so_far: None,
            session_id: session_id.to_string(),
            recipe_name: None,
            recipe_version: None,
        };

        let response = self
            .client
            .post(self.url("/reply"))
            .header("X-Secret-Key", &self.secret_key)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("POST /reply failed ({}): {}", status, body));
        }

        let (tx, rx) = mpsc::channel(100);
        let byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            use futures::StreamExt;
            let mut buffer = String::new();
            let mut stream = byte_stream;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Parse SSE events: "data: {json}\n\n"
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_str: String = buffer.chars().take(pos).collect();
                            buffer = buffer.chars().skip(pos + 2).collect();

                            if let Some(data) = event_str.strip_prefix("data: ") {
                                match serde_json::from_str::<MessageEvent>(data) {
                                    Ok(event) => {
                                        if tx.send(Ok(event)).await.is_err() {
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to parse SSE event: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("Stream error: {}", e))).await;
                        return;
                    }
                }
            }
        });

        Ok(SseEventStream {
            rx: ReceiverStream::new(rx),
        })
    }

    // --- Tool confirmation endpoint ---

    pub async fn confirm_tool_action(
        &self,
        id: String,
        principal_type: PrincipalType,
        action: Permission,
        session_id: &str,
    ) -> Result<()> {
        self.post_status(
            "/action-required/tool-confirmation",
            &ConfirmToolActionRequest {
                id,
                principal_type,
                action,
                session_id: session_id.to_string(),
            },
        )
        .await
    }
}
