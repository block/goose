use anyhow::Result;
use futures::StreamExt;
use goose::conversation::message::Message;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::Permission;
use goose::session::Session;
use reqwest::Client;

use super::types::*;
use super::utils::process_sse_buffer;

/// Contains only the connection info needed for HTTP calls, not the child process.
#[derive(Clone)]
pub struct GoosedHandle {
    pub(crate) base_url: String,
    pub(crate) secret_key: String,
    pub(crate) http: Client,
}

impl GoosedHandle {
    pub async fn reply(
        &self,
        session_id: &str,
        message: Message,
        conversation: Option<Vec<Message>>,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<SseEvent>>> {
        self.reply_with_mode(session_id, message, conversation, None)
            .await
    }

    pub async fn reply_with_mode(
        &self,
        session_id: &str,
        message: Message,
        conversation: Option<Vec<Message>>,
        mode: Option<String>,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<SseEvent>>> {
        let chat_request = ChatRequest {
            user_message: message,
            session_id: session_id.to_string(),
            conversation_so_far: conversation,
            recipe_name: None,
            recipe_version: None,
            mode,
            plan: None,
        };

        let response = self
            .http
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&chat_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "HTTP status client error ({}) for url ({}/reply): {}",
                status,
                self.base_url,
                body
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let mut byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            while let Some(chunk) = StreamExt::next(&mut byte_stream).await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        process_sse_buffer(&mut buffer, &tx).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.into())).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    pub async fn send_elicitation_response(
        &self,
        session_id: &str,
        response_message: Message,
    ) -> Result<()> {
        let chat_request = ChatRequest {
            user_message: response_message,
            session_id: session_id.to_string(),
            conversation_so_far: None,
            recipe_name: None,
            recipe_version: None,
            mode: None,
            plan: None,
        };

        self.http
            .post(format!("{}/reply", self.base_url))
            .header("X-Secret-Key", &self.secret_key)
            .json(&chat_request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn confirm_tool_action(
        &self,
        session_id: &str,
        tool_id: &str,
        permission: Permission,
    ) -> Result<()> {
        let request = ToolConfirmationRequest {
            id: tool_id.to_string(),
            principal_type: PrincipalType::Tool,
            action: permission,
            session_id: session_id.to_string(),
        };

        self.http
            .post(format!(
                "{}/action-required/tool-confirmation",
                self.base_url
            ))
            .header("X-Secret-Key", &self.secret_key)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Session> {
        let response: Session = self
            .http
            .get(format!("{}/sessions/{}", self.base_url, session_id))
            .header("X-Secret-Key", &self.secret_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response)
    }
}
