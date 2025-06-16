use std::{iter::FromIterator, ops::Deref};

use crate::message::MessageContent;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Holds the heterogeneous fragments that make up one chat message.
///
/// *   Up to two items are stored inline on the stack.
/// *   Falls back to a heap allocation only when necessary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Contents(SmallVec<[MessageContent; 2]>);

impl Contents {
    /*----------------------------------------------------------
     * 1-line ergonomic helpers
     *---------------------------------------------------------*/

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MessageContent> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, item: impl Into<MessageContent>) {
        self.0.push(item.into());
    }

    pub fn texts(&self) -> impl Iterator<Item = &str> {
        self.0.iter().filter_map(|c| c.as_text())
    }

    pub fn concat_text_str(&self) -> String {
        self.texts().collect::<Vec<_>>().join("\n")
    }

    /// Returns `true` if *any* item satisfies the predicate.
    pub fn any_is<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().any(pred)
    }

    /// Returns `true` if *every* item satisfies the predicate.
    pub fn all_are<P>(&self, pred: P) -> bool
    where
        P: FnMut(&MessageContent) -> bool,
    {
        self.iter().all(pred)
    }
}

impl From<Vec<MessageContent>> for Contents {
    fn from(v: Vec<MessageContent>) -> Self {
        Contents(SmallVec::from_vec(v))
    }
}

impl FromIterator<MessageContent> for Contents {
    fn from_iter<I: IntoIterator<Item = MessageContent>>(iter: I) -> Self {
        Contents(SmallVec::from_iter(iter))
    }
}

/*--------------------------------------------------------------
 * Allow &message.content to behave like a slice of fragments.
 *-------------------------------------------------------------*/
impl Deref for Contents {
    type Target = [MessageContent];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// — Register the contents type with UniFFI, converting to/from Vec<MessageContent> —
// We need to do this because UniFFI’s FFI layer supports only primitive buffers (here Vec<u8>),
uniffi::custom_type!(Contents, Vec<MessageContent>, {
    lower: |contents: &Contents| {
        contents.0.to_vec()
    },
    try_lift: |contents: Vec<MessageContent>| {
        Ok(Contents::from(contents))
    },
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{Content, TextContent, ToolCall, ToolError};
    use serde_json::json;

    // ------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------
    fn make_tool_req_ok(id: &str) -> MessageContent {
        let call = ToolCall::new("echo", json!({"text": "hi"}));
        MessageContent::tool_request(id, Ok(call).into())
    }

    fn make_tool_resp_ok(id: &str) -> MessageContent {
        let body = vec![Content::Text(TextContent {
            text: "done".into(),
        })];
        MessageContent::tool_response(id, Ok(body).into())
    }

    fn make_tool_req_err(id: &str) -> MessageContent {
        let err = ToolError::NotFound(format!(
            "The provided function name '{}' had invalid characters",
            "bad$name"
        ));
        MessageContent::tool_request(id, Err(err).into())
    }

    fn make_tool_resp_err(id: &str) -> MessageContent {
        let err = ToolError::InvalidParameters("Could not interpret tool use parameters".into());
        MessageContent::tool_response(id, Err(err).into())
    }

    // ------------------------------------------------------------
    // Round-trip: success
    // ------------------------------------------------------------
    #[test]
    fn contents_roundtrip_ok() {
        let items: Contents = vec![make_tool_req_ok("req-1"), make_tool_resp_ok("resp-1")].into();

        // ---- serialise
        let json_str = serde_json::to_string(&items).expect("serialise OK");
        println!("JSON: {:?}", json_str);

        assert!(
            json_str.contains(r#""type":"toolReq""#)
                && json_str.contains(r#""type":"toolResp""#)
                && json_str.contains(r#""status":"success""#),
            "JSON should contain both variants and success-status"
        );

        // ---- deserialise
        let parsed: Contents = serde_json::from_str(&json_str).expect("deserialise OK");

        assert_eq!(parsed, items, "full round-trip equality");
    }

    // ------------------------------------------------------------
    // Round-trip: error  (all variants collapse to ExecutionError)
    // ------------------------------------------------------------
    #[test]
    fn contents_roundtrip_err() {
        let original_items: Contents =
            vec![make_tool_req_err("req-e"), make_tool_resp_err("resp-e")].into();

        // ---- serialise
        let json_str = serde_json::to_string(&original_items).expect("serialise OK");
        println!("JSON: {:?}", json_str);

        assert!(json_str.contains(r#""status":"error""#));

        // ---- deserialise
        let parsed: Contents = serde_json::from_str(&json_str).expect("deserialise OK");

        // ─── validate structure ───────────────────────────────────
        assert_eq!(parsed.len(), 2);

        // ToolReq error
        match &parsed[0] {
            MessageContent::ToolReq(req) => match &*req.tool_call {
                Err(ToolError::ExecutionError(msg)) => {
                    assert!(msg.contains("invalid characters"))
                }
                other => panic!("expected ExecutionError, got {:?}", other),
            },
            other => panic!("expected ToolReq, got {:?}", other),
        }

        // ToolResp error
        match &parsed[1] {
            MessageContent::ToolResp(resp) => match &*resp.tool_result {
                Err(ToolError::ExecutionError(msg)) => {
                    assert!(msg.contains("interpret tool use parameters"))
                }
                other => panic!("expected ExecutionError, got {:?}", other),
            },
            other => panic!("expected ToolResp, got {:?}", other),
        }
    }
}
