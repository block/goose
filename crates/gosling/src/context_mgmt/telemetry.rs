use serde_json::json;
use tracing::debug;

use super::packet::ContextPacket;
use super::policy::ContextManagerMode;

/// Renders a packet's metadata into the debug JSON shape documented for the
/// Context Manager MVP, e.g.:
///
/// ```json
/// {
///   "context_manager_mode": "shadow",
///   "estimated_tokens_before": 104000,
///   "estimated_tokens_after": 62000,
///   "context_limit": 128000,
///   "reserved_response_tokens": 8000,
///   "strategy": "recent_plus_summary",
///   "slots": { "system": 3000, "...": 0 },
///   "summarized": ["older_conversation_summary"],
///   "dropped": ["duplicate_tool_output"]
/// }
/// ```
pub fn context_packet_debug_json(
    mode: ContextManagerMode,
    packet: &ContextPacket,
) -> serde_json::Value {
    let slots: serde_json::Map<String, serde_json::Value> = packet
        .metadata
        .slots
        .iter()
        .map(|s| {
            (
                s.slot.as_str().to_string(),
                serde_json::Value::from(s.estimated_tokens),
            )
        })
        .collect();

    let summarized: Vec<&'static str> = packet
        .metadata
        .summarized_blocks
        .iter()
        .map(|r| r.slot.as_str())
        .collect();
    let dropped: Vec<String> = packet
        .metadata
        .dropped_blocks
        .iter()
        .map(|d| d.reason.clone())
        .collect();

    json!({
        "context_manager_mode": mode.as_str(),
        "estimated_tokens_before": packet.metadata.estimated_tokens_before,
        "estimated_tokens_after": packet.metadata.estimated_tokens_after,
        "context_limit": packet.metadata.context_limit,
        "reserved_response_tokens": packet.metadata.reserved_response_tokens,
        "strategy": packet.metadata.strategy.as_str(),
        "slots": slots,
        "summarized": summarized,
        "dropped": dropped,
        "retrieved_memory_empty": packet.metadata.retrieved_memory_empty,
    })
}

/// Logs the packet's metadata as structured debug output. Called in both
/// `shadow` and `on` mode so the packet is always inspectable regardless of
/// whether it actually drives the provider call.
pub fn log_context_packet(mode: ContextManagerMode, packet: &ContextPacket) {
    let payload = context_packet_debug_json(mode, packet);
    debug!(
        target: "gosling::context_mgmt",
        context_packet = %payload,
        "context manager packet built"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_mgmt::packet::{ContextBuildRequest, ContextManager};
    use crate::conversation::message::Message;
    use crate::token_counter::create_token_counter;

    #[tokio::test]
    async fn debug_json_has_expected_shape() {
        let token_counter = create_token_counter().await.unwrap();
        let packet = ContextManager::build_with_counter(
            ContextBuildRequest {
                system_prompt: "system".to_string(),
                project_instructions: None,
                conversation_messages: vec![Message::user().with_text("hi")],
                context_limit: 128_000,
                reserved_response_tokens: 4_000,
                retrieved_memory: Vec::new(),
            },
            &token_counter,
        );

        let json = context_packet_debug_json(ContextManagerMode::Shadow, &packet);
        assert_eq!(json["context_manager_mode"], "shadow");
        assert!(json["slots"]["system"].as_u64().is_some());
        assert!(json["slots"]["retrieved_memory"].as_u64() == Some(0));
    }
}
