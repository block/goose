use anyhow::Result;
use async_stream::try_stream;
use goose_acp_provider::{
    schema::{ContentBlock, RequestPermissionRequest, SessionId, ToolCallContent},
    text_content, AcpClient, AcpUpdate, PermissionDecision,
};
use rmcp::model::{CallToolRequestParams, CallToolResult, Content, Role, Tool};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::config::GooseMode;
use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::permission::permission_confirmation::PrincipalType;
use crate::permission::{Permission, PermissionConfirmation};
use crate::providers::base::{MessageStream, PermissionRouting, ProviderUsage, Usage};
use crate::providers::errors::ProviderError;

pub struct AcpProviderCore {
    name: String,
    model: ModelConfig,
    client: Arc<AcpClient>,
    goose_mode: GooseMode,
    pending_confirmations: Arc<Mutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>>,
}

impl std::fmt::Debug for AcpProviderCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcpProviderCore")
            .field("name", &self.name)
            .field("model", &self.model)
            .finish()
    }
}

impl AcpProviderCore {
    pub fn with_client(
        name: String,
        model: ModelConfig,
        client: Arc<AcpClient>,
        goose_mode: GooseMode,
    ) -> Self {
        Self {
            name,
            model,
            client,
            goose_mode,
            pending_confirmations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn permission_routing(&self) -> PermissionRouting {
        PermissionRouting::ActionRequired
    }

    pub fn model(&self) -> ModelConfig {
        self.model.clone()
    }

    pub async fn new_session(&self) -> Result<SessionId> {
        self.client.new_session().await
    }

    pub async fn handle_permission_confirmation(
        &self,
        request_id: &str,
        confirmation: &PermissionConfirmation,
    ) -> bool {
        let mut pending = self.pending_confirmations.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(confirmation.clone());
            return true;
        }
        false
    }

    pub async fn complete_with_model(
        &self,
        session_id: &str,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let stream = self.stream(session_id, system, messages, tools).await?;

        use futures::StreamExt;
        tokio::pin!(stream);

        let mut content: Vec<MessageContent> = Vec::new();
        while let Some(result) = stream.next().await {
            if let Ok((Some(msg), _)) = result {
                content.extend(msg.content);
            }
        }

        if content.is_empty() {
            return Err(ProviderError::RequestFailed(
                "No response received from ACP agent".to_string(),
            ));
        }

        let mut message = Message::assistant();
        message.content = content;

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), Usage::default()),
        ))
    }

    pub async fn stream(
        &self,
        session_id: &str,
        _system: &str,
        messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let prompt_blocks = messages_to_prompt(messages);
        let mut rx = self
            .client
            .prompt(SessionId::new(session_id.to_string()), prompt_blocks)
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to send ACP prompt: {e}")))?;

        let pending_confirmations = self.pending_confirmations.clone();
        let client = self.client.clone();
        let goose_mode = self.goose_mode;

        Ok(Box::pin(try_stream! {
            while let Some(update) = rx.recv().await {
                match update {
                    AcpUpdate::Text(text) => {
                        let message = Message::assistant().with_text(text);
                        yield (Some(message), None);
                    }
                    AcpUpdate::Thought(text) => {
                        let message = Message::assistant()
                            .with_thinking(text, "")
                            .with_visibility(true, false);
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallStart { id, title, raw_input } => {
                        let arguments = raw_input
                            .and_then(|v| v.as_object().cloned())
                            .unwrap_or_default();

                        let tool_call = CallToolRequestParams {
                            meta: None,
                            task: None,
                            name: title.into(),
                            arguments: Some(arguments),
                        };
                        let message = Message::assistant().with_tool_request(id.clone(), Ok(tool_call));
                        yield (Some(message), None);
                    }
                    AcpUpdate::ToolCallComplete { id, status, content } => {
                        let result_text = tool_call_content_to_text(&content);
                        let is_error = client.tool_call_is_error(&id, status).await;

                        let call_result = CallToolResult {
                            content: if result_text.is_empty() {
                                content_blocks_to_rmcp(&content)
                            } else {
                                vec![Content::text(result_text)]
                            },
                            structured_content: None,
                            is_error: Some(is_error),
                            meta: None,
                        };

                        let message = Message::assistant().with_tool_response(id, Ok(call_result));
                        yield (Some(message), None);
                    }
                    AcpUpdate::PermissionRequest { request, response_tx } => {
                        if let Some(decision) = permission_decision_from_mode(goose_mode) {
                            let response = client.permission_response(&request, decision).await;
                            let _ = response_tx.send(response);
                            continue;
                        }

                        let request_id = request.tool_call.tool_call_id.0.to_string();
                        let (tx, rx) = oneshot::channel();

                        pending_confirmations
                            .lock()
                            .await
                            .insert(request_id.clone(), tx);

                        if let Some(action_required) = build_action_required_message(&request) {
                            yield (Some(action_required), None);
                        }

                        let confirmation = rx.await.unwrap_or(PermissionConfirmation {
                            principal_type: PrincipalType::Tool,
                            permission: Permission::Cancel,
                        });

                        pending_confirmations.lock().await.remove(&request_id);

                        let decision = permission_decision_from_confirmation(&confirmation);
                        let response = client.permission_response(&request, decision).await;
                        let _ = response_tx.send(response);
                    }
                    AcpUpdate::Complete(_reason) => {
                        break;
                    }
                    AcpUpdate::Error(e) => {
                        Err(ProviderError::RequestFailed(e))?;
                    }
                }
            }
        }))
    }
}

fn messages_to_prompt(messages: &[Message]) -> Vec<ContentBlock> {
    let mut content_blocks = Vec::new();

    let last_user = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User && m.is_agent_visible());

    if let Some(message) = last_user {
        for content in &message.content {
            if let MessageContent::Text(text) = content {
                content_blocks.push(text_content(text.text.clone()));
            }
        }
    }

    content_blocks
}

fn build_action_required_message(request: &RequestPermissionRequest) -> Option<Message> {
    let tool_title = request
        .tool_call
        .fields
        .title
        .clone()
        .unwrap_or_else(|| "Tool".to_string());

    let arguments = request
        .tool_call
        .fields
        .raw_input
        .as_ref()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let prompt = request
        .tool_call
        .fields
        .content
        .as_ref()
        .and_then(|content| {
            content.iter().find_map(|c| match c {
                ToolCallContent::Content(val) => match &val.content {
                    ContentBlock::Text(text) => Some(text.text.clone()),
                    _ => None,
                },
                _ => None,
            })
        });

    Some(
        Message::assistant()
            .with_action_required(
                request.tool_call.tool_call_id.0.to_string(),
                tool_title,
                arguments,
                prompt,
            )
            .user_only(),
    )
}

fn permission_decision_from_confirmation(
    confirmation: &PermissionConfirmation,
) -> PermissionDecision {
    match confirmation.permission {
        Permission::AlwaysAllow => PermissionDecision::AllowAlways,
        Permission::AllowOnce => PermissionDecision::AllowOnce,
        Permission::DenyOnce => PermissionDecision::RejectOnce,
        Permission::AlwaysDeny => PermissionDecision::RejectAlways,
        Permission::Cancel => PermissionDecision::Cancel,
    }
}

fn permission_decision_from_mode(goose_mode: GooseMode) -> Option<PermissionDecision> {
    match goose_mode {
        GooseMode::Auto => Some(PermissionDecision::AllowOnce),
        GooseMode::Chat => Some(PermissionDecision::RejectOnce),
        GooseMode::Approve | GooseMode::SmartApprove => None,
    }
}

fn tool_call_content_to_text(content: &[ToolCallContent]) -> String {
    content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content(val) => match &val.content {
                ContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn content_blocks_to_rmcp(content: &[ToolCallContent]) -> Vec<Content> {
    content
        .iter()
        .filter_map(|c| match c {
            ToolCallContent::Content(val) => match &val.content {
                ContentBlock::Text(text) => Some(Content::text(text.text.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
