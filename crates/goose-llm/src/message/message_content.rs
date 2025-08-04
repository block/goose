use serde::{Deserialize, Serialize};
use serde_json::{self, Deserializer, Serializer};

use crate::message::tool_result_serde;
use crate::types::core::{Content, ImageContent, TextContent, ToolCall, ToolResult};

// — Newtype wrappers (local structs) so we satisfy Rust’s orphan rules —
// We need these because we can’t implement UniFFI’s FfiConverter directly on a type alias.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolRequestToolCall(#[serde(with = "tool_result_serde")] pub ToolResult<ToolCall>);

impl ToolRequestToolCall {
    pub fn as_result(&self) -> &ToolResult<ToolCall> {
        &self.0
    }
}
impl std::ops::Deref for ToolRequestToolCall {
    type Target = ToolResult<ToolCall>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Result<ToolCall, crate::types::core::ToolError>> for ToolRequestToolCall {
    fn from(res: Result<ToolCall, crate::types::core::ToolError>) -> Self {
        ToolRequestToolCall(res)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponseToolResult(
    #[serde(with = "tool_result_serde")] pub ToolResult<Vec<Content>>,
);

impl ToolResponseToolResult {
    pub fn as_result(&self) -> &ToolResult<Vec<Content>> {
        &self.0
    }
}
impl std::ops::Deref for ToolResponseToolResult {
    type Target = ToolResult<Vec<Content>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<Result<Vec<Content>, crate::types::core::ToolError>> for ToolResponseToolResult {
    fn from(res: Result<Vec<Content>, crate::types::core::ToolError>) -> Self {
        ToolResponseToolResult(res)
    }
}

// — Register the newtypes with UniFFI, converting via JSON strings —
// UniFFI’s FFI layer supports only primitive buffers (here String), so we JSON-serialize
// through our `tool_result_serde` to preserve the same success/error schema on both sides.
// see https://github.com/mozilla/uniffi-rs/issues/2533

uniffi::custom_type!(ToolRequestToolCall, String, {
    lower: |wrapper: &ToolRequestToolCall| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            // note the borrow on wrapper.0
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolRequestToolCall serialization failed");
        }
        String::from_utf8(buf).expect("ToolRequestToolCall produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolRequestToolCall(result))
    },
});

uniffi::custom_type!(ToolResponseToolResult, String, {
    lower: |wrapper: &ToolResponseToolResult| {
        let mut buf = Vec::new();
        {
            let mut ser = Serializer::new(&mut buf);
            // note the borrow on wrapper.0
            tool_result_serde::serialize(&wrapper.0, &mut ser)
                .expect("ToolResponseToolResult serialization failed");
        }
        String::from_utf8(buf).expect("ToolResponseToolResult produced invalid UTF-8")
    },
    try_lift: |s: String| {
        let mut de = Deserializer::from_str(&s);
        let result = tool_result_serde::deserialize(&mut de)
            .map_err(anyhow::Error::new)?;
        Ok(ToolResponseToolResult(result))
    },
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequest {
    pub id: String,
    pub tool_call: ToolRequestToolCall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ToolResponse {
    pub id: String,
    pub tool_result: ToolResponseToolResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct ThinkingContent {
    pub thinking: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct RedactedThinkingContent {
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
/// Content passed inside a message, which can be both simple content and tool content
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
    ToolReq(ToolRequest),
    ToolResp(ToolResponse),
    Thinking(ThinkingContent),
    RedactedThinking(RedactedThinkingContent),
}

impl MessageContent {
    pub fn text<S: Into<String>>(text: S) -> Self {
        MessageContent::Text(TextContent { text: text.into() })
    }

    pub fn image<S: Into<String>, T: Into<String>>(data: S, mime_type: T) -> Self {
        MessageContent::Image(ImageContent {
            data: data.into(),
            mime_type: mime_type.into(),
        })
    }

    pub fn tool_request<S: Into<String>>(id: S, tool_call: ToolRequestToolCall) -> Self {
        MessageContent::ToolReq(ToolRequest {
            id: id.into(),
            tool_call,
        })
    }

    pub fn tool_response<S: Into<String>>(id: S, tool_result: ToolResponseToolResult) -> Self {
        MessageContent::ToolResp(ToolResponse {
            id: id.into(),
            tool_result,
        })
    }

    pub fn thinking<S1: Into<String>, S2: Into<String>>(thinking: S1, signature: S2) -> Self {
        MessageContent::Thinking(ThinkingContent {
            thinking: thinking.into(),
            signature: signature.into(),
        })
    }

    pub fn redacted_thinking<S: Into<String>>(data: S) -> Self {
        MessageContent::RedactedThinking(RedactedThinkingContent { data: data.into() })
    }

    pub fn as_tool_request(&self) -> Option<&ToolRequest> {
        if let MessageContent::ToolReq(ref tool_request) = self {
            Some(tool_request)
        } else {
            None
        }
    }

    pub fn as_tool_response(&self) -> Option<&ToolResponse> {
        if let MessageContent::ToolResp(ref tool_response) = self {
            Some(tool_response)
        } else {
            None
        }
    }

    pub fn as_tool_response_text(&self) -> Option<String> {
        if let Some(tool_response) = self.as_tool_response() {
            if let Ok(contents) = &tool_response.tool_result.0 {
                let texts: Vec<String> = contents
                    .iter()
                    .filter_map(|content| content.as_text().map(String::from))
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }
        None
    }

    pub fn as_tool_request_id(&self) -> Option<&str> {
        if let Self::ToolReq(r) = self {
            Some(&r.id)
        } else {
            None
        }
    }

    pub fn as_tool_response_id(&self) -> Option<&str> {
        if let Self::ToolResp(r) = self {
            Some(&r.id)
        } else {
            None
        }
    }

    /// Get the text content if this is a TextContent variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(text) => Some(&text.text),
            _ => None,
        }
    }

    /// Get the thinking content if this is a ThinkingContent variant
    pub fn as_thinking(&self) -> Option<&ThinkingContent> {
        match self {
            MessageContent::Thinking(thinking) => Some(thinking),
            _ => None,
        }
    }

    /// Get the redacted thinking content if this is a RedactedThinkingContent variant
    pub fn as_redacted_thinking(&self) -> Option<&RedactedThinkingContent> {
        match self {
            MessageContent::RedactedThinking(redacted) => Some(redacted),
            _ => None,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
    pub fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }
    pub fn is_tool_request(&self) -> bool {
        matches!(self, Self::ToolReq(_))
    }
    pub fn is_tool_response(&self) -> bool {
        matches!(self, Self::ToolResp(_))
    }
}

impl From<Content> for MessageContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => MessageContent::Text(text),
            Content::Image(image) => MessageContent::Image(image),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{ToolCall, ToolError};
    use crate::UniFfiTag;
    use serde_json::json;
    use uniffi::{FfiConverter, RustBuffer};

    // ---------- ToolRequestToolCall ----------------------------------------------------------

    #[test]
    fn tool_request_tool_call_roundtrip_ok() {
        // Build a valid ToolCall
        let call = ToolCall::new("my_function", json!({"a": 1, "b": 2}));

        // Wrap it in the new-type
        let wrapper = ToolRequestToolCall::from(Ok(call.clone()));

        // Serialize → JSON
        let json_str = serde_json::to_string(&wrapper).expect("serialize OK");
        assert!(
            json_str.contains(r#""status":"success""#),
            "must mark success"
        );

        // Deserialize ← JSON
        let parsed: ToolRequestToolCall = serde_json::from_str(&json_str).expect("deserialize OK");

        // Round-trip equality
        assert_eq!(*parsed, Ok(call));
    }

    #[test]
    fn tool_request_tool_call_roundtrip_err() {
        // Typical failure variant that could come from `is_valid_function_name`
        let err = ToolError::NotFound(
            "The provided function name 'bad$name' had invalid characters".into(),
        );

        let wrapper = ToolRequestToolCall::from(Err(err.clone()));

        let json_str = serde_json::to_string(&wrapper).expect("serialize OK");
        assert!(
            json_str.contains(r#""status":"error""#) && json_str.contains("invalid characters"),
            "must mark error and carry message"
        );

        let parsed: ToolRequestToolCall = serde_json::from_str(&json_str).expect("deserialize OK");

        match &*parsed {
            Err(ToolError::ExecutionError(msg)) => {
                assert!(msg.contains("invalid characters"))
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }

    // ---------- ToolResponseToolResult -------------------------------------------------------

    #[test]
    fn tool_response_tool_result_roundtrip_ok() {
        // Minimal content vector (one text item)
        let content_vec = vec![Content::Text(TextContent {
            text: "hello".into(),
        })];

        let wrapper = ToolResponseToolResult::from(Ok(content_vec.clone()));

        let json_str = serde_json::to_string(&wrapper).expect("serialize OK");
        assert!(json_str.contains(r#""status":"success""#));

        let parsed: ToolResponseToolResult =
            serde_json::from_str(&json_str).expect("deserialize OK");

        assert_eq!(*parsed, Ok(content_vec));
    }

    #[test]
    fn tool_response_tool_result_roundtrip_err() {
        let err = ToolError::InvalidParameters("Could not interpret tool use parameters".into());

        let wrapper = ToolResponseToolResult::from(Err(err.clone()));

        let json_str = serde_json::to_string(&wrapper).expect("serialize OK");
        assert!(json_str.contains(r#""status":"error""#));

        let parsed: ToolResponseToolResult =
            serde_json::from_str(&json_str).expect("deserialize OK");

        match &*parsed {
            Err(ToolError::ExecutionError(msg)) => {
                assert!(msg.contains("interpret tool use"))
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }

    // ---------- FFI (lower / lift) round-trips ----------------------------------------------
    // https://mozilla.github.io/uniffi-rs/latest/internals/lifting_and_lowering.html

    #[test]
    fn ffi_roundtrip_tool_request_ok_and_err() {
        // ---------- status: success ----------
        let ok_call = ToolCall::new("echo", json!({"text": "hi"}));
        let ok_wrapper = ToolRequestToolCall::from(Ok(ok_call.clone()));

        // First lower → inspect JSON
        let buf1: RustBuffer =
            <ToolRequestToolCall as FfiConverter<UniFfiTag>>::lower(ok_wrapper.clone());

        let json_ok: String =
            <String as FfiConverter<UniFfiTag>>::try_lift(buf1).expect("lift String OK");
        println!("ToolReq - Lowered JSON (status: success): {:?}", json_ok);
        assert!(json_ok.contains(r#""status":"success""#));

        // Second lower → round-trip wrapper
        let buf2: RustBuffer =
            <ToolRequestToolCall as FfiConverter<UniFfiTag>>::lower(ok_wrapper.clone());

        let lifted_ok = <ToolRequestToolCall as FfiConverter<UniFfiTag>>::try_lift(buf2)
            .expect("lift wrapper OK");
        println!(
            "ToolReq - Lifted wrapper (status: success): {:?}",
            lifted_ok
        );
        assert_eq!(lifted_ok, ok_wrapper);

        // ---------- status: error ----------
        let err_call = ToolError::NotFound("no such function".into());
        let err_wrapper = ToolRequestToolCall::from(Err(err_call.clone()));

        let buf1: RustBuffer =
            <ToolRequestToolCall as FfiConverter<UniFfiTag>>::lower(err_wrapper.clone());
        let json_err: String =
            <String as FfiConverter<UniFfiTag>>::try_lift(buf1).expect("lift String ERR");
        println!("ToolReq - Lowered JSON (status: error): {:?}", json_err);
        assert!(json_err.contains(r#""status":"error""#));

        let buf2: RustBuffer =
            <ToolRequestToolCall as FfiConverter<UniFfiTag>>::lower(err_wrapper.clone());
        let lifted_err = <ToolRequestToolCall as FfiConverter<UniFfiTag>>::try_lift(buf2)
            .expect("lift wrapper ERR");
        println!("ToolReq - Lifted wrapper (status: error): {:?}", lifted_err);

        match &*lifted_err {
            Err(ToolError::ExecutionError(msg)) => {
                assert!(msg.contains("no such function"))
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }

    #[test]
    fn ffi_roundtrip_tool_response_ok_and_err() {
        // ---------- status: success ----------
        let body = vec![Content::Text(TextContent {
            text: "done".into(),
        })];
        let ok_wrapper = ToolResponseToolResult::from(Ok(body.clone()));

        let buf1: RustBuffer =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::lower(ok_wrapper.clone());
        let json_ok: String = <String as FfiConverter<UniFfiTag>>::try_lift(buf1).unwrap();
        println!("ToolResp - Lowered JSON (status: success): {:?}", json_ok);
        assert!(json_ok.contains(r#""status":"success""#));

        let buf2: RustBuffer =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::lower(ok_wrapper.clone());
        let lifted_ok =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::try_lift(buf2).unwrap();
        println!(
            "ToolResp - Lifted wrapper (status: success): {:?}",
            lifted_ok
        );
        assert_eq!(lifted_ok, ok_wrapper);

        // ---------- status: error ----------
        let err_call = ToolError::InvalidParameters("bad params".into());
        let err_wrapper = ToolResponseToolResult::from(Err(err_call.clone()));

        let buf1: RustBuffer =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::lower(err_wrapper.clone());
        let json_err: String = <String as FfiConverter<UniFfiTag>>::try_lift(buf1).unwrap();
        println!("ToolResp - Lowered JSON (status: error): {:?}", json_err);
        assert!(json_err.contains(r#""status":"error""#));

        let buf2: RustBuffer =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::lower(err_wrapper.clone());
        let lifted_err =
            <ToolResponseToolResult as FfiConverter<UniFfiTag>>::try_lift(buf2).unwrap();
        println!(
            "ToolResp - Lifted wrapper (status: error): {:?}",
            lifted_err
        );

        match &*lifted_err {
            Err(ToolError::ExecutionError(msg)) => {
                assert!(msg.contains("bad params"))
            }
            other => panic!("expected ExecutionError, got {:?}", other),
        }
    }
}
