use std::collections::HashMap;

use goose::message::MessageContent;
use goose::telemetry::ToolUsage;

pub fn log_if_telemetry_disabled(tracking_type: &str) {
    if goose::telemetry::global_telemetry().is_none() {
        tracing::debug!(
            "Telemetry is disabled or not initialized for {} tracking",
            tracking_type
        );
    }
}

pub fn extract_tool_usage_from_session(session: &crate::Session) -> Vec<ToolUsage> {
    let messages = session.message_history();
    let mut tool_usage_map: HashMap<String, ToolUsage> = HashMap::new();
    let mut tool_call_times: HashMap<String, i64> = HashMap::new();
    let mut tool_id_to_name: HashMap<String, String> = HashMap::new();

    for message in &messages {
        for content in &message.content {
            match content {
                MessageContent::ToolRequest(tool_request) => {
                    if let Ok(tool_call) = &tool_request.tool_call {
                        let tool_name = &tool_call.name;
                        let tool_id = &tool_request.id;

                        tool_id_to_name.insert(tool_id.clone(), tool_name.clone());
                        tool_call_times.insert(tool_id.clone(), message.created);

                        tool_usage_map
                            .entry(tool_name.clone())
                            .or_insert_with(|| ToolUsage::new(tool_name));
                    }
                }
                MessageContent::ToolResponse(tool_response) => {
                    let tool_id = &tool_response.id;

                    if let Some(tool_name) = tool_id_to_name.get(tool_id) {
                        if let Some(entry) = tool_usage_map.get_mut(tool_name) {
                            let duration = if let Some(start_time) = tool_call_times.get(tool_id) {
                                let duration_ms = (message.created - start_time).max(0) as u64;
                                std::time::Duration::from_millis(duration_ms)
                            } else {
                                std::time::Duration::from_millis(0)
                            };

                            let success = tool_response.tool_result.is_ok();
                            entry.add_call(duration, success);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    tool_usage_map.into_values().collect()
}
