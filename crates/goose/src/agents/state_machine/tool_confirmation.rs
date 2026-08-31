use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::agents::state_machine::messages_since_kickoff;
use crate::agents::state_machine::ops_tool_approval::ApprovalState;
use crate::conversation::message::{
    ActionRequiredData, Message, MessageContent, ToolConfirmationRequest,
};
use crate::conversation::Conversation;
use crate::permission::Permission;
use crate::session::SessionManager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolConfirmationDecision {
    pub request_id: String,
    pub permission: Permission,
}

fn active_turn_messages(conversation: &Conversation) -> &[Message] {
    let messages = conversation.messages();
    messages
        .iter()
        .rposition(|message| {
            message.role == rmcp::model::Role::User
                && message.is_user_visible()
                && !message.is_tool_response()
        })
        .map(|start| &messages[start..])
        .unwrap_or(messages)
}

pub(crate) fn pending_tool_confirmations(
    conversation: &Conversation,
) -> Vec<ToolConfirmationRequest> {
    let mut answered = HashSet::new();
    let mut confirmation_responses = HashSet::new();
    let mut confirmation_requests = Vec::new();

    for message in active_turn_messages(conversation) {
        for content in &message.content {
            match content {
                MessageContent::ToolResponse(response) => {
                    answered.insert(response.id.clone());
                }
                MessageContent::ActionRequired(action) => match &action.data {
                    ActionRequiredData::ToolConfirmation {
                        id,
                        tool_name,
                        arguments,
                        prompt,
                    } => confirmation_requests.push(ToolConfirmationRequest {
                        id: id.clone(),
                        tool_name: tool_name.clone(),
                        arguments: arguments.clone(),
                        prompt: prompt.clone(),
                    }),
                    ActionRequiredData::ToolConfirmationResponse { id, .. } => {
                        confirmation_responses.insert(id.clone());
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    confirmation_requests
        .into_iter()
        .filter(|request| {
            !answered.contains(&request.id) && !confirmation_responses.contains(&request.id)
        })
        .collect()
}

pub(crate) fn has_unapplied_tool_confirmation_response(conversation: &Conversation) -> bool {
    ApprovalState::from_messages(active_turn_messages(conversation))
        .has_unapplied_confirmation_response()
}

pub(crate) async fn persist_tool_confirmation_decisions(
    session_manager: &SessionManager,
    session_id: &str,
    confirmation_decisions: &[ToolConfirmationDecision],
) -> Result<()> {
    if confirmation_decisions.is_empty() {
        return Ok(());
    }

    let session = session_manager.get_session(session_id, true).await?;
    let conversation = session
        .conversation
        .as_ref()
        .ok_or_else(|| anyhow!("Session {session_id} has no conversation"))?;
    let approval_state = ApprovalState::from_messages(messages_since_kickoff(conversation)?);
    let mut decisions_by_request = HashMap::new();
    let mut new_decisions = Vec::new();

    for decision in confirmation_decisions {
        if let Some(previous) =
            decisions_by_request.insert(decision.request_id.as_str(), &decision.permission)
        {
            if previous != &decision.permission {
                return Err(anyhow!(
                    "Conflicting tool confirmation decisions for request {}",
                    decision.request_id
                ));
            }
            continue;
        }

        if !approval_state.has_confirmation_request(&decision.request_id) {
            return Err(anyhow!(
                "Tool request {} is not awaiting confirmation in session {session_id}",
                decision.request_id
            ));
        }

        if let Some(persisted_permission) =
            approval_state.confirmation_response(&decision.request_id)
        {
            if persisted_permission != &decision.permission {
                return Err(anyhow!(
                    "Tool request {} already has a different confirmation decision",
                    decision.request_id
                ));
            }
            continue;
        }

        if approval_state.has_tool_response(&decision.request_id) {
            return Err(anyhow!(
                "Tool request {} already has a response",
                decision.request_id
            ));
        }

        new_decisions.push(decision);
    }

    if new_decisions.is_empty() {
        return Ok(());
    }

    let response_message = new_decisions.iter().fold(
        Message::user().with_visibility(false, false),
        |message, decision| {
            message.with_content(MessageContent::action_required_tool_confirmation_response(
                decision.request_id.clone(),
                decision.permission.clone(),
            ))
        },
    );
    session_manager
        .add_message(session_id, &response_message)
        .await
}
