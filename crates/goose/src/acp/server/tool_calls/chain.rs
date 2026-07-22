use std::collections::HashMap;
use std::sync::Arc;

/// A run of consecutive ToolRequest blocks within one assistant message,
/// tracked by `GooseAcpSession::chain_membership`. Used to drive a single
/// LLM summary for the whole run once every step has a recorded ToolResponse.
#[derive(Debug, Clone)]
pub(crate) struct ToolChain {
    /// Tool call ids in document order. Always `len() >= 2`.
    pub(crate) ids: Vec<String>,
    /// The message_id of the assistant message containing these tool calls.
    /// Used to persist chain summaries back to the messages table.
    pub(crate) message_id: String,
}

/// If `buffer` holds a multi-tool run (≥ 2 tool requests), (re)register a
/// [`ToolChain`] in `chain_membership` anchored on the **first** tool's
/// message_id (the row `SessionManager::update_tool_request_meta` will patch
/// when persisting the LLM-generated summary). Does **not** clear the buffer
/// — chains can grow as more tools arrive (sequential tool use), so callers
/// keep accumulating and re-registering with the larger set of ids.
///
/// The buffer contains `(tool_call_id, message_id)` pairs in arrival order,
/// fed by the prompt stream loop. Sequential tool use (Bedrock/Anthropic)
/// interleaves request → response → request → response across separate
/// `AgentEvent::Message` events, so a per-event view would only see length-1
/// chains and miss the run. Tool responses are chain-neutral (they don't
/// split the run); only non-tool content (text, thinking, image, etc.) does,
/// matching the frontend's `groupContentSections` behavior.
pub(crate) fn extend_chain_membership(
    buffer: &[(String, String)],
    chain_membership: &mut HashMap<String, Arc<ToolChain>>,
) {
    if buffer.len() >= 2 {
        let ids: Vec<String> = buffer.iter().map(|(id, _)| id.clone()).collect();
        let message_id = buffer[0].1.clone();
        let chain = Arc::new(ToolChain {
            ids: ids.clone(),
            message_id,
        });
        for id in ids {
            chain_membership.insert(id, chain.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    mod extend_chain_membership {
        use super::super::{extend_chain_membership, ToolChain};
        use std::collections::HashMap;
        use std::sync::Arc;

        fn buf_entry(tool_id: &str, msg_id: &str) -> (String, String) {
            (tool_id.to_string(), msg_id.to_string())
        }

        #[test]
        fn skips_singleton_and_leaves_buffer() {
            let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
            let buffer = vec![buf_entry("a", "row_1")];

            extend_chain_membership(&buffer, &mut membership);

            assert_eq!(buffer.len(), 1, "buffer is left intact for caller");
            assert!(
                membership.is_empty(),
                "single-tool runs should not register a chain",
            );
        }

        #[test]
        fn registers_each_id_against_shared_chain() {
            let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
            let buffer = vec![
                buf_entry("a", "row_first"),
                buf_entry("b", "row_second"),
                buf_entry("c", "row_third"),
            ];

            extend_chain_membership(&buffer, &mut membership);

            assert_eq!(membership.len(), 3);
            let chain_a = membership.get("a").expect("a registered");
            let chain_b = membership.get("b").expect("b registered");
            let chain_c = membership.get("c").expect("c registered");
            assert!(
                Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c),
                "every id in the run must point at the same ToolChain Arc",
            );
            assert_eq!(
                chain_a.ids,
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
            );
        }

        #[test]
        fn anchors_on_first_row_for_split_messages() {
            // Sequential tool use (Bedrock/Anthropic) emits each tool request as
            // its own assistant message, with the tool response interleaved in
            // between. The chain should still form, anchored on the *first*
            // tool's row id so `update_tool_request_meta` can find that
            // ToolRequest when persisting the summary.
            let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
            let buffer = vec![
                buf_entry("toolu_bdrk_1", "row_for_tool_1"),
                buf_entry("toolu_bdrk_2", "row_for_tool_2"),
            ];

            extend_chain_membership(&buffer, &mut membership);

            let chain = membership
                .get("toolu_bdrk_1")
                .expect("first tool registered");
            assert_eq!(
                chain.ids,
                vec!["toolu_bdrk_1".to_string(), "toolu_bdrk_2".to_string()],
            );
            let chain_via_second = membership
                .get("toolu_bdrk_2")
                .expect("second tool registered");
            assert!(Arc::ptr_eq(chain, chain_via_second));
        }

        #[test]
        fn grows_chain_as_more_requests_arrive() {
            // The streaming loop re-registers eagerly each time a new request
            // arrives, so a chain that started at length 2 must grow to include
            // a third tool whose response is yet to come. Both the original
            // members and the new member must point at the new (extended) chain.
            let mut membership: HashMap<String, Arc<ToolChain>> = HashMap::new();
            let mut buffer = vec![buf_entry("a", "row_1"), buf_entry("b", "row_2")];
            extend_chain_membership(&buffer, &mut membership);

            buffer.push(buf_entry("c", "row_3"));
            extend_chain_membership(&buffer, &mut membership);

            let chain_a = membership.get("a").expect("a present");
            let chain_b = membership.get("b").expect("b present");
            let chain_c = membership.get("c").expect("c present");
            assert!(Arc::ptr_eq(chain_a, chain_b) && Arc::ptr_eq(chain_b, chain_c));
            assert_eq!(
                chain_a.ids,
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
            );
        }
    }
}
