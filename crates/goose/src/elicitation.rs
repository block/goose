use anyhow::Result;
use rmcp::model::ElicitationAction;
use serde_json::Value;

use crate::action_required_manager::{ActionRequiredManager, ElicitationResponse};
use crate::conversation::message::{Message, MessageContent};
use crate::session::SessionManager;

fn elicitation_response_user_data(response: &ElicitationResponse) -> Value {
    match response {
        ElicitationResponse::Accept(user_data) => user_data.clone(),
        ElicitationResponse::Decline | ElicitationResponse::Cancel => serde_json::json!({}),
    }
}

fn elicitation_response_action(response: &ElicitationResponse) -> ElicitationAction {
    match response {
        ElicitationResponse::Accept(_) => ElicitationAction::Accept,
        ElicitationResponse::Decline => ElicitationAction::Decline,
        ElicitationResponse::Cancel => ElicitationAction::Cancel,
    }
}

fn generated_elicitation_response_message(
    elicitation_id: &str,
    response: &ElicitationResponse,
) -> Message {
    Message::user()
        .with_generated_id()
        .with_content(MessageContent::action_required_elicitation_response(
            elicitation_id.to_string(),
            elicitation_response_user_data(response),
            elicitation_response_action(response),
        ))
        .agent_only()
}

pub(crate) async fn complete_elicitation_with_message(
    session_manager: &SessionManager,
    session_id: &str,
    elicitation_id: &str,
    response: ElicitationResponse,
    response_message: &Message,
) -> Result<()> {
    let claim = ActionRequiredManager::global()
        .claim_response(session_id, elicitation_id)
        .await?;

    session_manager
        .add_message(session_id, response_message)
        .await?;

    claim.submit(response)
}

pub(crate) async fn complete_elicitation_with_generated_message(
    session_manager: &SessionManager,
    session_id: &str,
    elicitation_id: &str,
    response: ElicitationResponse,
) -> Result<()> {
    let response_message = generated_elicitation_response_message(elicitation_id, &response);
    complete_elicitation_with_message(
        session_manager,
        session_id,
        elicitation_id,
        response,
        &response_message,
    )
    .await
}
