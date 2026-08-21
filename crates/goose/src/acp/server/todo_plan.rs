use super::*;
use crate::session::extension_data::{TodoItemPriority, TodoItemStatus, TodoState};

pub(super) fn is_todo_write_request(tool_request: &ToolRequest) -> bool {
    tool_request.tool_name_parts().is_some_and(|parts| {
        parts.tool_name == "todo_write"
            && (parts.extension_name.is_none() || parts.extension_name == Some("todo"))
    })
}

pub(super) fn tool_response_succeeded(tool_response: &ToolResponse) -> bool {
    tool_response
        .tool_result
        .as_ref()
        .is_ok_and(|result| result.is_error != Some(true))
}

pub(super) async fn send_current_todo_plan(
    cx: &ConnectionTo<Client>,
    session_manager: &SessionManager,
    session_id: &SessionId,
) -> std::result::Result<(), agent_client_protocol::Error> {
    let session = session_manager
        .get_session(session_id.0.as_ref(), false)
        .await
        .map_err(|error| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to read todo state: {error}"))
        })?;
    let entries = TodoState::from_extension_data(&session.extension_data)
        .map(|state| state.items.into_iter().map(plan_entry).collect())
        .unwrap_or_default();

    cx.send_notification(SessionNotification::new(
        session_id.clone(),
        SessionUpdate::Plan(Plan::new(entries)),
    ))
}

fn plan_entry(item: crate::session::extension_data::TodoItem) -> PlanEntry {
    let priority = match item.priority {
        TodoItemPriority::High => PlanEntryPriority::High,
        TodoItemPriority::Medium => PlanEntryPriority::Medium,
        TodoItemPriority::Low => PlanEntryPriority::Low,
    };
    let status = match item.status {
        TodoItemStatus::Pending => PlanEntryStatus::Pending,
        TodoItemStatus::InProgress => PlanEntryStatus::InProgress,
        TodoItemStatus::Completed => PlanEntryStatus::Completed,
    };
    let mut goose = serde_json::Map::new();
    goose.insert(
        "todo".to_string(),
        serde_json::json!({ "depth": item.depth }),
    );
    let mut meta = serde_json::Map::new();
    meta.insert("goose".to_string(), serde_json::Value::Object(goose));

    PlanEntry::new(item.content, priority, status).meta(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{CallToolRequestParams, CallToolResult};

    fn tool_request(name: &str) -> ToolRequest {
        ToolRequest {
            id: "tool-1".to_string(),
            tool_call: Ok(CallToolRequestParams::new(name.to_string())),
            metadata: None,
            tool_meta: None,
        }
    }

    #[test]
    fn recognizes_prefixed_and_unprefixed_todo_write() {
        assert!(is_todo_write_request(&tool_request("todo__todo_write")));
        assert!(is_todo_write_request(&tool_request("todo_write")));
        assert!(!is_todo_write_request(&tool_request(
            "calendar__todo_write"
        )));
    }

    #[test]
    fn rejects_error_tool_results() {
        let success = ToolResponse {
            id: "tool-1".to_string(),
            tool_result: Ok(CallToolResult::success(Vec::new())),
            metadata: None,
        };
        let failure = ToolResponse {
            id: "tool-1".to_string(),
            tool_result: Ok(CallToolResult::error(Vec::new())),
            metadata: None,
        };

        assert!(tool_response_succeeded(&success));
        assert!(!tool_response_succeeded(&failure));
    }
}
