use goose::agents::{AgentEvent, ExtensionConfig};
use goose::conversation::message::{Message, TokenState};
use goose::conversation::Conversation;
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::Permission;
use goose::recipe::Recipe;
use rmcp::model::PromptArgument;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SseEvent {
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
    RoutingDecision {
        agent_name: String,
        mode_slug: String,
        confidence: f32,
        reasoning: String,
    },
    Notification {
        request_id: String,
        message: serde_json::Value,
    },
    UpdateConversation {
        conversation: Vec<Message>,
    },
    ToolAvailabilityChange {
        previous_count: usize,
        current_count: usize,
    },
    PlanProposal {
        is_compound: bool,
        tasks: Vec<PlanProposalTask>,
        #[serde(default)]
        clarifying_questions: Option<Vec<String>>,
    },
    Ping,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanProposalTask {
    pub agent_name: String,
    pub mode_slug: String,
    pub mode_name: String,
    pub confidence: f32,
    pub reasoning: String,
    pub description: String,
    pub tool_groups: Vec<String>,
}

impl SseEvent {
    /// Convert an SseEvent into an AgentEvent where possible.
    /// Returns None for events that don't map (Finish, Error, Ping, etc.)
    pub fn into_agent_event(self) -> Option<AgentEvent> {
        match self {
            SseEvent::Message { message, .. } => Some(AgentEvent::Message(message)),
            SseEvent::ModelChange { model, mode } => Some(AgentEvent::ModelChange { model, mode }),
            SseEvent::RoutingDecision {
                agent_name,
                mode_slug,
                confidence,
                reasoning,
            } => Some(AgentEvent::RoutingDecision {
                agent_name,
                mode_slug,
                confidence,
                reasoning,
            }),
            SseEvent::UpdateConversation { conversation } => Conversation::new(conversation)
                .ok()
                .map(AgentEvent::HistoryReplaced),
            SseEvent::ToolAvailabilityChange {
                previous_count,
                current_count,
            } => Some(AgentEvent::ToolAvailabilityChange {
                previous_count,
                current_count,
            }),
            SseEvent::Notification { .. }
            | SseEvent::Error { .. }
            | SseEvent::Finish { .. }
            | SseEvent::PlanProposal { .. }
            | SseEvent::Ping => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatRequest {
    pub(crate) user_message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_so_far: Option<Vec<Message>>,
    pub(crate) session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recipe_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recipe_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) plan: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartAgentRequest {
    pub(crate) working_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recipe: Option<Recipe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recipe_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recipe_deeplink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) extension_overrides: Option<Vec<ExtensionConfig>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumeAgentRequest {
    pub(crate) session_id: String,
    pub(crate) load_model_and_extensions: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopAgentRequest {
    pub(crate) session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolConfirmationRequest {
    pub(crate) id: String,
    pub(crate) principal_type: PrincipalType,
    pub(crate) action: Permission,
    pub(crate) session_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AddExtensionRequest {
    pub(crate) session_id: String,
    pub(crate) config: ExtensionConfig,
}

#[derive(Debug, Serialize)]
pub(crate) struct RemoveExtensionRequest {
    pub(crate) name: String,
    pub(crate) session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateProviderRequest {
    pub(crate) provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    pub(crate) session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) context_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestartAgentRequest {
    pub(crate) session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateWorkingDirRequest {
    pub(crate) session_id: String,
    pub(crate) working_dir: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForkSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timestamp: Option<i64>,
    pub(crate) truncate: bool,
    pub(crate) copy: bool,
}

#[derive(Debug, Deserialize)]
pub struct ToolInfoResponse {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub permission: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExtensionLoadResultResponse {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptResponse {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Option<Vec<PromptArgumentResponse>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptArgumentResponse {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
}

impl PromptArgumentResponse {
    pub fn into_prompt_argument(self) -> PromptArgument {
        PromptArgument {
            name: self.name,
            title: self.title,
            description: self.description,
            required: self.required,
        }
    }
}

impl PromptResponse {
    pub fn into_prompt_arguments(&self) -> Option<Vec<PromptArgument>> {
        self.arguments.as_ref().map(|args| {
            args.iter()
                .map(|a| a.clone().into_prompt_argument())
                .collect()
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetPromptResultResponse {
    #[serde(default)]
    pub description: Option<String>,
    pub messages: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestartAgentResponseBody {
    pub(crate) extension_results: Vec<ExtensionLoadResultResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForkSessionResponseBody {
    pub(crate) session_id: String,
}
