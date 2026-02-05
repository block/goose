//! SSE streaming support for the OpenAI-compatible proxy.
//!
//! Parses custom LLM SSE chunks, converts to OpenAI streaming chunks,
//! and handles buffered strategies for tool calls and structured output.
//!
//! Three streaming strategies:
//! 1. `stream_plain`: Forward content deltas incrementally (no tools/structured output)
//! 2. `stream_buffered_tools`: Buffer all content, parse `<tool_call>` tags, emit tool_calls
//! 3. `stream_buffered_structured`: Buffer all content, clean JSON, emit as single delta

use bytes::Bytes;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::models::CustomSseChunk;
use crate::structured_output::parse_structured_output;
use crate::tool_injection::parse_tool_calls;

// ---------------------------------------------------------------------------
// Custom LLM SSE Parsing
// ---------------------------------------------------------------------------

/// Parse a single `data: {...}` SSE line from the custom LLM.
///
/// Returns the parsed chunk dict, or `None` if empty/keepalive.
pub fn parse_custom_sse_line(raw_line: &str) -> Option<CustomSseChunk> {
    let line = raw_line.trim();
    if line.is_empty() || line == "data:" || line == "data: " {
        return None;
    }
    if !line.starts_with("data:") {
        return None;
    }
    let json_str = line["data:".len()..].trim();
    if json_str.is_empty() {
        return None;
    }
    serde_json::from_str(json_str).ok()
}

/// Async iterator that yields parsed custom LLM SSE chunk dicts from a byte stream.
///
/// Handles byte-level buffering and splits on `data:` lines.
pub async fn collect_sse_chunks_from_bytes(
    chunks: Vec<Bytes>,
) -> Vec<CustomSseChunk> {
    let mut buffer = String::new();
    let mut results = Vec::new();

    for raw_bytes in chunks {
        buffer.push_str(&String::from_utf8_lossy(&raw_bytes));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if let Some(parsed) = parse_custom_sse_line(&line) {
                results.push(parsed);
            }
        }
    }

    // Process any remaining content in the buffer
    let remaining = buffer.trim();
    if !remaining.is_empty() {
        if let Some(parsed) = parse_custom_sse_line(remaining) {
            results.push(parsed);
        }
    }

    results
}

// ---------------------------------------------------------------------------
// OpenAI SSE Chunk Builders
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Build a single OpenAI-format SSE chunk string.
pub fn build_openai_chunk(
    chunk_id: &str,
    model: &str,
    created: u64,
    delta: Value,
    finish_reason: Option<&str>,
    usage: Option<Value>,
) -> String {
    let mut chunk = serde_json::json!({
        "id": chunk_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": finish_reason,
        }],
    });
    if let Some(u) = usage {
        chunk["usage"] = u;
    }
    format!("data: {}\n\n", serde_json::to_string(&chunk).unwrap_or_default())
}

/// Build the initial role chunk.
pub fn build_role_chunk(chunk_id: &str, model: &str, created: u64) -> String {
    build_openai_chunk(
        chunk_id,
        model,
        created,
        serde_json::json!({"role": "assistant"}),
        None,
        None,
    )
}

/// Build a content delta chunk.
pub fn build_content_chunk(chunk_id: &str, model: &str, created: u64, content: &str) -> String {
    build_openai_chunk(
        chunk_id,
        model,
        created,
        serde_json::json!({"content": content}),
        None,
        None,
    )
}

/// Build SSE chunks for tool calls per OpenAI streaming spec.
///
/// Emits two chunks per tool call:
/// 1. Name chunk: index, id, type, function.name, function.arguments=""
/// 2. Arguments chunk: index, function.arguments="<full>"
pub fn build_tool_call_chunks(
    chunk_id: &str,
    model: &str,
    created: u64,
    tool_calls: &[Value],
) -> Vec<String> {
    let mut chunks = Vec::new();
    for (i, tc) in tool_calls.iter().enumerate() {
        let func = tc.get("function").unwrap_or(tc);
        let call_id = tc
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| "call_unknown")
            .to_string();
        let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = func
            .get("arguments")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        // First chunk: introduce the tool call with name
        let name_delta = serde_json::json!({
            "tool_calls": [{
                "index": i,
                "id": call_id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": "",
                }
            }]
        });
        chunks.push(build_openai_chunk(chunk_id, model, created, name_delta, None, None));

        // Second chunk: full arguments
        let args_delta = serde_json::json!({
            "tool_calls": [{
                "index": i,
                "function": {
                    "arguments": arguments,
                }
            }]
        });
        chunks.push(build_openai_chunk(chunk_id, model, created, args_delta, None, None));
    }
    chunks
}

/// Build the final chunk with finish_reason and optional usage.
pub fn build_finish_chunk(
    chunk_id: &str,
    model: &str,
    created: u64,
    finish_reason: &str,
    usage: Option<Value>,
) -> String {
    build_openai_chunk(
        chunk_id,
        model,
        created,
        serde_json::json!({}),
        Some(finish_reason),
        usage,
    )
}

/// Return the SSE done signal.
pub fn build_done_signal() -> String {
    "data: [DONE]\n\n".to_string()
}

// ---------------------------------------------------------------------------
// Buffer Helper
// ---------------------------------------------------------------------------

/// Buffer all SSE chunks and return (full_content, last_chunk_metadata).
///
/// Returns an error string if a FAIL status is encountered during buffering.
pub fn buffer_all_chunks(
    chunks: &[CustomSseChunk],
) -> Result<(String, Option<CustomSseChunk>), String> {
    let mut full_content = String::new();
    let mut last_chunk: Option<CustomSseChunk> = None;

    for chunk in chunks {
        if chunk.status.as_deref() == Some("FAIL") {
            return Err(format!(
                "Custom LLM returned FAIL: {}",
                chunk.response_code.as_deref().unwrap_or("unknown")
            ));
        }
        if let Some(ref c) = chunk.content {
            full_content.push_str(c);
        }
        last_chunk = Some(chunk.clone());
    }

    Ok((full_content, last_chunk))
}

// ---------------------------------------------------------------------------
// Streaming Strategies
// ---------------------------------------------------------------------------

/// Generate SSE output for a plain text response (no tools/structured output).
///
/// Forwards content deltas incrementally.
pub fn generate_plain_sse(chunks: &[CustomSseChunk], chunk_id: &str, model: &str) -> Vec<String> {
    let created = now_secs();
    let mut output = Vec::new();
    output.push(build_role_chunk(chunk_id, model, created));

    let mut usage_data: Option<Value> = None;
    let mut finish = "stop".to_string();

    for chunk in chunks {
        if chunk.status.as_deref() == Some("FAIL") {
            output.push(build_content_chunk(
                chunk_id,
                model,
                created,
                &format!(
                    "\n[Error] Custom LLM returned FAIL: {}",
                    chunk.response_code.as_deref().unwrap_or("unknown")
                ),
            ));
            output.push(build_finish_chunk(chunk_id, model, created, "stop", None));
            output.push(build_done_signal());
            return output;
        }

        if let Some(ref content) = chunk.content {
            if !content.is_empty() {
                output.push(build_content_chunk(chunk_id, model, created, content));
            }
        }

        // Detect final chunk
        let event_status = chunk.event_status.as_deref().unwrap_or("CHUNK");
        let chunk_finish = chunk.finish_reason.as_deref().unwrap_or("");

        if event_status != "CHUNK" || !chunk_finish.is_empty() {
            let prompt_tokens = chunk.prompt_token.unwrap_or(0);
            let completion_tokens = chunk.completion_token.unwrap_or(0);
            if prompt_tokens > 0 || completion_tokens > 0 {
                usage_data = Some(serde_json::json!({
                    "prompt_tokens": prompt_tokens,
                    "completion_tokens": completion_tokens,
                    "total_tokens": prompt_tokens + completion_tokens,
                }));
            }
            if !chunk_finish.is_empty() {
                finish = chunk_finish.to_string();
            }
        }
    }

    output.push(build_finish_chunk(chunk_id, model, created, &finish, usage_data));
    output.push(build_done_signal());
    output
}

/// Generate SSE output with tool call buffering.
///
/// Buffers ALL content, parses tool calls, then emits appropriate chunks.
pub fn generate_buffered_tools_sse(
    chunks: &[CustomSseChunk],
    chunk_id: &str,
    model: &str,
) -> Vec<String> {
    let created = now_secs();
    let mut output = Vec::new();

    let (full_content, last_chunk) = match buffer_all_chunks(chunks) {
        Ok((content, lc)) => (content, lc),
        Err(err_msg) => {
            output.push(build_role_chunk(chunk_id, model, created));
            output.push(build_content_chunk(
                chunk_id,
                model,
                created,
                &format!("[Error] {}", err_msg),
            ));
            output.push(build_finish_chunk(chunk_id, model, created, "stop", None));
            output.push(build_done_signal());
            return output;
        }
    };

    // Build usage
    let usage_data = last_chunk.as_ref().and_then(|lc| {
        let pt = lc.prompt_token.unwrap_or(0);
        let ct = lc.completion_token.unwrap_or(0);
        if pt > 0 || ct > 0 {
            Some(serde_json::json!({
                "prompt_tokens": pt,
                "completion_tokens": ct,
                "total_tokens": pt + ct,
            }))
        } else {
            None
        }
    });

    // Parse tool calls
    let (tool_calls_parsed, remaining_content) = parse_tool_calls(&full_content);

    output.push(build_role_chunk(chunk_id, model, created));

    if let Some(ref calls) = tool_calls_parsed {
        if !remaining_content.trim().is_empty() {
            output.push(build_content_chunk(chunk_id, model, created, &remaining_content));
        }
        output.extend(build_tool_call_chunks(chunk_id, model, created, calls));
        output.push(build_finish_chunk(chunk_id, model, created, "tool_calls", usage_data));
    } else {
        if !full_content.is_empty() {
            output.push(build_content_chunk(chunk_id, model, created, &full_content));
        }
        output.push(build_finish_chunk(chunk_id, model, created, "stop", usage_data));
    }

    output.push(build_done_signal());
    output
}

/// Generate SSE output with structured output buffering.
///
/// Buffers ALL content, cleans JSON, then emits as single delta.
pub fn generate_buffered_structured_sse(
    chunks: &[CustomSseChunk],
    chunk_id: &str,
    model: &str,
) -> Vec<String> {
    let created = now_secs();
    let mut output = Vec::new();

    let (full_content, last_chunk) = match buffer_all_chunks(chunks) {
        Ok((content, lc)) => (content, lc),
        Err(err_msg) => {
            output.push(build_role_chunk(chunk_id, model, created));
            output.push(build_content_chunk(
                chunk_id,
                model,
                created,
                &format!("[Error] {}", err_msg),
            ));
            output.push(build_finish_chunk(chunk_id, model, created, "stop", None));
            output.push(build_done_signal());
            return output;
        }
    };

    let usage_data = last_chunk.as_ref().and_then(|lc| {
        let pt = lc.prompt_token.unwrap_or(0);
        let ct = lc.completion_token.unwrap_or(0);
        if pt > 0 || ct > 0 {
            Some(serde_json::json!({
                "prompt_tokens": pt,
                "completion_tokens": ct,
                "total_tokens": pt + ct,
            }))
        } else {
            None
        }
    });

    let cleaned = if full_content.is_empty() {
        full_content
    } else {
        parse_structured_output(&full_content)
    };

    output.push(build_role_chunk(chunk_id, model, created));
    if !cleaned.is_empty() {
        output.push(build_content_chunk(chunk_id, model, created, &cleaned));
    }
    output.push(build_finish_chunk(chunk_id, model, created, "stop", usage_data));
    output.push(build_done_signal());
    output
}

/// Build a complete SSE error response sequence.
pub fn build_streaming_error_response(error_message: &str) -> Vec<String> {
    let chunk_id = format!("chatcmpl-error-{}", &Uuid::new_v4().as_simple().to_string()[..8]);
    let model = "error";
    let created = now_secs();
    vec![
        build_role_chunk(&chunk_id, model, created),
        build_content_chunk(&chunk_id, model, created, &format!("[Error] {}", error_message)),
        build_finish_chunk(&chunk_id, model, created, "stop", None),
        build_done_signal(),
    ]
}

// ---------------------------------------------------------------------------
// OpenAI SSE Parsing (for OpenAI proxy mode)
// ---------------------------------------------------------------------------

/// Parse OpenAI format SSE lines and collect content.
/// Returns (full_content, usage_data).
pub fn collect_openai_sse_content(chunks: Vec<Bytes>) -> (String, Option<Value>) {
    let mut buffer = String::new();
    let mut full_content = String::new();
    let mut usage_data: Option<Value> = None;

    for raw_bytes in chunks {
        buffer.push_str(&String::from_utf8_lossy(&raw_bytes));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            let line = line.trim();
            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let json_str = line["data:".len()..].trim();
            if json_str.is_empty() || json_str == "[DONE]" {
                continue;
            }

            if let Ok(chunk) = serde_json::from_str::<Value>(json_str) {
                // Extract content delta
                if let Some(delta) = chunk
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        full_content.push_str(content);
                    }
                }
                // Extract usage if present
                if let Some(usage) = chunk.get("usage") {
                    if !usage.is_null() {
                        usage_data = Some(usage.clone());
                    }
                }
            }
        }
    }

    // Process remaining buffer
    let remaining = buffer.trim();
    if !remaining.is_empty() && remaining.starts_with("data:") {
        let json_str = remaining["data:".len()..].trim();
        if !json_str.is_empty() && json_str != "[DONE]" {
            if let Ok(chunk) = serde_json::from_str::<Value>(json_str) {
                if let Some(delta) = chunk
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        full_content.push_str(content);
                    }
                }
                if let Some(usage) = chunk.get("usage") {
                    if !usage.is_null() {
                        usage_data = Some(usage.clone());
                    }
                }
            }
        }
    }

    (full_content, usage_data)
}

/// Generate SSE output for OpenAI streaming with tool call parsing.
///
/// Buffers the OpenAI SSE stream, parses tool calls, and re-emits properly.
pub fn generate_openai_buffered_tools_sse(
    raw_chunks: Vec<Bytes>,
    chunk_id: &str,
    model: &str,
) -> Vec<String> {
    let created = now_secs();
    let mut output = Vec::new();

    let (full_content, usage_data) = collect_openai_sse_content(raw_chunks);

    // Parse tool calls from the content
    let (tool_calls_parsed, remaining_content) = parse_tool_calls(&full_content);

    output.push(build_role_chunk(chunk_id, model, created));

    if let Some(ref calls) = tool_calls_parsed {
        if !calls.is_empty() {
            if !remaining_content.trim().is_empty() {
                output.push(build_content_chunk(chunk_id, model, created, remaining_content.trim()));
            }
            output.extend(build_tool_call_chunks(chunk_id, model, created, calls));
            output.push(build_finish_chunk(chunk_id, model, created, "tool_calls", usage_data));
        } else {
            // No valid tool calls parsed
            if !full_content.is_empty() {
                output.push(build_content_chunk(chunk_id, model, created, &full_content));
            }
            output.push(build_finish_chunk(chunk_id, model, created, "stop", usage_data));
        }
    } else {
        if !full_content.is_empty() {
            output.push(build_content_chunk(chunk_id, model, created, &full_content));
        }
        output.push(build_finish_chunk(chunk_id, model, created, "stop", usage_data));
    }

    output.push(build_done_signal());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_custom_sse_line_valid() {
        let chunk = parse_custom_sse_line(
            r#"data: {"content":"hello","event_status":"CHUNK","status":"SUCCESS"}"#,
        );
        let chunk = chunk.unwrap();
        assert_eq!(chunk.content.as_deref(), Some("hello"));
        assert_eq!(chunk.event_status.as_deref(), Some("CHUNK"));
    }

    #[test]
    fn test_parse_custom_sse_line_empty() {
        assert!(parse_custom_sse_line("").is_none());
        assert!(parse_custom_sse_line("data:").is_none());
        assert!(parse_custom_sse_line("data: ").is_none());
    }

    #[test]
    fn test_parse_custom_sse_line_non_data() {
        assert!(parse_custom_sse_line("event: update").is_none());
        assert!(parse_custom_sse_line("id: 123").is_none());
    }

    #[test]
    fn test_build_openai_chunk() {
        let chunk = build_openai_chunk(
            "test-id",
            "test-model",
            1000,
            serde_json::json!({"content": "hello"}),
            None,
            None,
        );
        assert!(chunk.starts_with("data: "));
        assert!(chunk.ends_with("\n\n"));
        let json_str = chunk.trim_start_matches("data: ").trim();
        let parsed: Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["choices"][0]["delta"]["content"], "hello");
    }

    #[test]
    fn test_build_done_signal() {
        assert_eq!(build_done_signal(), "data: [DONE]\n\n");
    }

    #[test]
    fn test_buffer_all_chunks_success() {
        let chunks = vec![
            CustomSseChunk {
                id: None,
                content: Some("hello ".to_string()),
                event_status: Some("CHUNK".to_string()),
                status: Some("SUCCESS".to_string()),
                response_code: None,
                finish_reason: None,
                prompt_token: None,
                completion_token: None,
            },
            CustomSseChunk {
                id: None,
                content: Some("world".to_string()),
                event_status: Some("DONE".to_string()),
                status: Some("SUCCESS".to_string()),
                response_code: None,
                finish_reason: None,
                prompt_token: Some(10),
                completion_token: Some(5),
            },
        ];
        let (content, last) = buffer_all_chunks(&chunks).unwrap();
        assert_eq!(content, "hello world");
        assert_eq!(last.unwrap().prompt_token, Some(10));
    }

    #[test]
    fn test_buffer_all_chunks_fail() {
        let chunks = vec![CustomSseChunk {
            id: None,
            content: None,
            event_status: None,
            status: Some("FAIL".to_string()),
            response_code: Some("RATE_LIMIT".to_string()),
            finish_reason: None,
            prompt_token: None,
            completion_token: None,
        }];
        let result = buffer_all_chunks(&chunks);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RATE_LIMIT"));
    }

    #[test]
    fn test_generate_plain_sse() {
        let chunks = vec![
            CustomSseChunk {
                id: None,
                content: Some("hello ".to_string()),
                event_status: Some("CHUNK".to_string()),
                status: Some("SUCCESS".to_string()),
                response_code: None,
                finish_reason: None,
                prompt_token: None,
                completion_token: None,
            },
            CustomSseChunk {
                id: None,
                content: Some("world".to_string()),
                event_status: Some("DONE".to_string()),
                status: Some("SUCCESS".to_string()),
                response_code: None,
                finish_reason: Some("stop".to_string()),
                prompt_token: Some(10),
                completion_token: Some(5),
            },
        ];
        let output = generate_plain_sse(&chunks, "test-id", "test-model");
        // Should have: role chunk, 2 content chunks, finish chunk, done signal
        assert!(output.len() >= 4);
        assert!(output.last().unwrap().contains("[DONE]"));
    }

    #[test]
    fn test_generate_buffered_tools_sse_with_tool_calls() {
        let chunks = vec![CustomSseChunk {
            id: None,
            content: Some(
                r#"Let me check. <tool_call>{"name":"get_weather","arguments":{"city":"Seoul"}}</tool_call>"#.to_string(),
            ),
            event_status: Some("DONE".to_string()),
            status: Some("SUCCESS".to_string()),
            response_code: None,
            finish_reason: None,
            prompt_token: Some(50),
            completion_token: Some(20),
        }];
        let output = generate_buffered_tools_sse(&chunks, "test-id", "test-model");
        // Should contain tool_calls in the output
        let combined = output.join("");
        assert!(combined.contains("tool_calls"));
        assert!(combined.contains("get_weather"));
        assert!(combined.contains("[DONE]"));
    }

    #[test]
    fn test_streaming_error_response() {
        let output = build_streaming_error_response("connection failed");
        assert_eq!(output.len(), 4); // role, error content, finish, done
        let combined = output.join("");
        assert!(combined.contains("connection failed"));
        assert!(combined.contains("[DONE]"));
    }
}
