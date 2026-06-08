use crate::conversation::message::{Message, MessageContent};
use crate::providers::errors::ProviderError;
use rmcp::model::CallToolRequestParams;
use serde_json::Value;
use std::borrow::Cow;
use uuid::Uuid;

use super::super::finalize_usage;
use super::inference_engine::{
    generation_loop, prepare_generation, GenerationContext, StopSuffixTrimmer,
    ThinkingOutputFilter, TokenAction,
};

pub(super) fn generate_with_native_tools(
    ctx: &mut GenerationContext<'_>,
    oai_messages_json: &str,
    full_tools_json: Option<&str>,
    compact_tools: Option<&str>,
) -> Result<(), ProviderError> {
    let prepared = prepare_generation(ctx, oai_messages_json, full_tools_json, compact_tools)?;
    let template_result = prepared.template_result;
    let mut llama_ctx = prepared.llama_ctx;
    let prompt_token_count = prepared.prompt_token_count;
    let effective_ctx = prepared.effective_ctx;

    let message_id = ctx.message_id;
    let tx = ctx.tx;
    let mut generated_text = String::new();
    let mut stop_trimmer = StopSuffixTrimmer::new(&template_result.additional_stops);
    let mut stop_string_emitted = false;

    // Initialize streaming parser — handles thinking tokens, tool calls, etc.
    let mut stream_parser = template_result.streaming_state_oaicompat().map_err(|e| {
        ProviderError::ExecutionError(format!("Failed to init streaming parser: {}", e))
    })?;

    // Feed the generation prompt to the parser so it knows the context.
    // The model may echo this prefix; the parser needs to see it to strip it.
    if !template_result.generation_prompt.is_empty() {
        let _ = stream_parser.update(&template_result.generation_prompt, true);
    }

    // Accumulate tool calls across streaming deltas
    let mut accumulated_tool_calls: Vec<Value> = Vec::new();
    // Accumulate thinking/reasoning across the entire generation so we can
    // attach it to the final tool-call message (mirroring what the OpenAI
    // streaming path does). Streaming chunks are still sent for UI display.
    let mut output_filter = ThinkingOutputFilter::new(
        ctx.settings.enable_thinking,
        &template_result.generation_prompt,
    );

    let output_token_count = generation_loop(
        &ctx.loaded.model,
        &mut llama_ctx,
        ctx.settings,
        prompt_token_count,
        effective_ctx,
        |piece| {
            generated_text.push_str(piece);
            let mut stop_seen = false;

            // Feed the new piece to the streaming parser
            match stream_parser.update(piece, true) {
                Ok(deltas) => {
                    for delta_json in deltas {
                        if let Ok(delta) = serde_json::from_str::<Value>(&delta_json) {
                            // Stream thinking/reasoning content
                            if let Some(reasoning) =
                                delta.get("reasoning_content").and_then(|v| v.as_str())
                            {
                                if let Some(thinking) =
                                    output_filter.push_structured_reasoning(reasoning)
                                {
                                    let mut msg = Message::assistant().with_thinking(thinking, "");
                                    msg.id = Some(message_id.to_string());
                                    if tx.blocking_send(Ok((Some(msg), None))).is_err() {
                                        return Ok(TokenAction::Stop);
                                    }
                                }
                            }
                            // Stream content text to the UI
                            if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                if !content.is_empty() {
                                    let filtered = output_filter.push_text(content);
                                    let (content, seen) = stop_trimmer.push(&filtered.content);
                                    stop_seen |= seen;
                                    if !content.is_empty() {
                                        let mut msg = Message::assistant().with_text(content);
                                        msg.id = Some(message_id.to_string());
                                        if tx.blocking_send(Ok((Some(msg), None))).is_err() {
                                            return Ok(TokenAction::Stop);
                                        }
                                    }
                                }
                            }
                            // Accumulate tool call deltas
                            if let Some(tool_calls) =
                                delta.get("tool_calls").and_then(|v| v.as_array())
                            {
                                for tc in tool_calls {
                                    accumulated_tool_calls.push(tc.clone());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Streaming parser error: {}", e);
                    let filtered = output_filter.push_text(piece);
                    let (content, seen) = stop_trimmer.push(&filtered.content);
                    stop_seen |= seen;
                    if !content.is_empty() {
                        let mut msg = Message::assistant().with_text(content);
                        msg.id = Some(message_id.to_string());
                        if tx.blocking_send(Ok((Some(msg), None))).is_err() {
                            return Ok(TokenAction::Stop);
                        }
                    }
                }
            }

            let should_stop = stop_seen
                || template_result
                    .additional_stops
                    .iter()
                    .any(|stop| generated_text.ends_with(stop));
            if should_stop {
                stop_string_emitted = true;
                Ok(TokenAction::Stop)
            } else {
                Ok(TokenAction::Continue)
            }
        },
    )?;

    // Finalize the streaming parser with is_partial=false
    if let Ok(final_deltas) = stream_parser.update("", false) {
        for delta_json in final_deltas {
            if let Ok(delta) = serde_json::from_str::<Value>(&delta_json) {
                if let Some(reasoning) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                    if let Some(thinking) = output_filter.push_structured_reasoning(reasoning) {
                        let mut msg = Message::assistant().with_thinking(thinking, "");
                        msg.id = Some(message_id.to_string());
                        let _ = tx.blocking_send(Ok((Some(msg), None)));
                    }
                }
                if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        let filtered = output_filter.push_text(content);
                        let (content, stop_seen) = stop_trimmer.push(&filtered.content);
                        stop_string_emitted |= stop_seen;
                        if !content.is_empty() {
                            let mut msg = Message::assistant().with_text(content);
                            msg.id = Some(message_id.to_string());
                            let _ = tx.blocking_send(Ok((Some(msg), None)));
                        }
                    }
                }
                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for tc in tool_calls {
                        accumulated_tool_calls.push(tc.clone());
                    }
                }
            }
        }
    }

    let filtered = output_filter.finish();
    if !filtered.thinking.is_empty() {
        let mut msg = Message::assistant().with_thinking(&filtered.thinking, "");
        msg.id = Some(message_id.to_string());
        let _ = tx.blocking_send(Ok((Some(msg), None)));
    }
    let content = if stop_string_emitted {
        String::new()
    } else {
        let (content, stop_seen) = stop_trimmer.push(&filtered.content);
        let mut content = content;
        if !stop_seen {
            content.push_str(&stop_trimmer.finish());
        }
        content
    };
    if !content.is_empty() {
        let mut msg = Message::assistant().with_text(content);
        msg.id = Some(message_id.to_string());
        let _ = tx.blocking_send(Ok((Some(msg), None)));
    }

    // Build a single message combining thinking + all tool calls, mirroring
    // the structure produced by the OpenAI streaming path. The agent relies
    // on this combined message to:
    //   1. Extract thinking and attach it to per-tool-request messages
    //   2. Enable merge_split_tool_call_messages to reconstruct the standard
    //      OpenAI format (one assistant msg with N tool_calls, then N tool results)
    let mut tool_call_contents = extract_oai_tool_call_contents(&accumulated_tool_calls);
    // Fallback: several local GGUF models (e.g. Qwen2.5-Coder) emit tool calls as
    // JSON in the *content* stream rather than through the native `tool_calls`
    // channel, so the streaming parser accumulates zero structured calls and the
    // call leaks to the user as plain assistant text — the agent never executes
    // it. Recover those by scanning the raw generated text for `{"name",
    // "arguments"}` objects. Guarded on the native channel being empty so
    // well-behaved models are never double-parsed.
    if tool_call_contents.is_empty() {
        tool_call_contents = extract_text_json_tool_calls(&generated_text);
    }
    if !tool_call_contents.is_empty() {
        let mut contents: Vec<MessageContent> = Vec::new();
        if !output_filter.accumulated_thinking().is_empty() {
            contents.push(MessageContent::thinking(
                output_filter.accumulated_thinking(),
                "",
            ));
        }
        contents.extend(tool_call_contents);
        let mut msg = Message::new(
            rmcp::model::Role::Assistant,
            chrono::Utc::now().timestamp(),
            contents,
        );
        msg.id = Some(message_id.to_string());
        let _ = tx.blocking_send(Ok((Some(msg), None)));
    }

    let provider_usage = finalize_usage(
        ctx.log,
        std::mem::take(&mut ctx.model_name),
        "native",
        prompt_token_count,
        output_token_count,
        Some(("generated_text", &generated_text)),
    );
    let _ = ctx.tx.blocking_send(Ok((None, Some(provider_usage))));
    Ok(())
}

/// Merge OpenAI streaming deltas by `index` into `MessageContent` items.
///
/// Returns one `ToolRequest` content per distinct tool call index. The caller
/// is responsible for combining these into a single `Message` (together with
/// any accumulated thinking content).
fn extract_oai_tool_call_contents(deltas: &[Value]) -> Vec<MessageContent> {
    let mut merged: std::collections::BTreeMap<u64, (String, String, String)> =
        std::collections::BTreeMap::new();

    for delta in deltas {
        let index = delta.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
        let entry = merged
            .entry(index)
            .or_insert_with(|| (String::new(), String::new(), String::new()));

        if let Some(id) = delta.get("id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                entry.0 = id.to_string();
            }
        }
        if let Some(func) = delta.get("function") {
            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                if !name.is_empty() {
                    entry.1 = name.to_string();
                }
            }
            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                entry.2.push_str(args);
            }
        }
    }

    merged
        .into_values()
        .filter_map(|(id, name, args_str)| {
            if name.is_empty() {
                return None;
            }

            let id = if id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                id
            };

            let arguments: Option<serde_json::Map<String, Value>> = if args_str.is_empty() {
                None
            } else {
                match serde_json::from_str(&args_str) {
                    Ok(args) => Some(args),
                    Err(_) => return None,
                }
            };

            let tool_call = match arguments {
                Some(args) => CallToolRequestParams::new(Cow::Owned(name)).with_arguments(args),
                None => CallToolRequestParams::new(Cow::Owned(name)),
            };

            Some(MessageContent::tool_request(id, Ok(tool_call)))
        })
        .collect()
}

/// Recover tool calls that a model emitted as JSON in its text output instead of
/// through the native `tool_calls` channel.
///
/// Scans `text` for one or more `{"name": ..., "arguments": {...}}` objects —
/// the format Qwen-Coder and many other local GGUF models produce by instinct,
/// optionally wrapped in ```json fences or `<tool_call>` tags (both ignored,
/// since the scanner matches on the braces themselves). `arguments` may be a
/// JSON object or a JSON-encoded string; `parameters` is accepted as an alias.
/// Each recovered object with a non-empty `name` becomes one `ToolRequest`.
///
/// Only invoked when the native channel produced nothing, so well-behaved models
/// are never affected.
fn extract_text_json_tool_calls(text: &str) -> Vec<MessageContent> {
    let mut out = Vec::new();
    for obj in find_json_objects(text) {
        let name = match obj.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        // Require the advertised tool-call shape: an `arguments` (or `parameters`)
        // field must be present. Without it, ordinary name-bearing JSON — e.g. a
        // generated package.json `{"name":"my-app","version":"1.0.0"}` — would be
        // mis-recovered and executed as a zero-arg tool call.
        let raw_args = match obj.get("arguments").or_else(|| obj.get("parameters")) {
            Some(v) => v,
            None => continue,
        };
        let arguments: Option<serde_json::Map<String, Value>> = match raw_args {
            Value::Object(map) => Some(map.clone()),
            Value::String(s) => serde_json::from_str(s).ok(),
            _ => None,
        };
        let tool_call = match arguments {
            Some(args) => CallToolRequestParams::new(Cow::Owned(name)).with_arguments(args),
            None => CallToolRequestParams::new(Cow::Owned(name)),
        };
        out.push(MessageContent::tool_request(
            Uuid::new_v4().to_string(),
            Ok(tool_call),
        ));
    }
    out
}

/// Extract top-level JSON objects embedded in free text via brace matching that
/// respects string literals and escapes (so braces inside string values don't
/// confuse the depth count). Only objects carrying a `"name"` field are returned,
/// to avoid picking up unrelated JSON. Surrounding prose, code fences, and tags
/// are ignored — the scan keys off the `{` / `}` structure alone.
fn find_json_objects(text: &str) -> Vec<Value> {
    let bytes = text.as_bytes();
    let mut objs = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        // Scan from this `{` to its matching `}`.
        let mut depth = 0usize;
        let mut in_str = false;
        let mut escaped = false;
        let mut end = None;
        let mut j = i;
        while j < bytes.len() {
            let c = bytes[j];
            if in_str {
                if escaped {
                    escaped = false;
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    in_str = false;
                }
            } else {
                match c {
                    b'"' => in_str = true,
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(j);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            j += 1;
        }
        match end {
            Some(e) => {
                // `i` and `e` are ASCII brace byte offsets (char boundaries); use
                // `get` rather than direct indexing to avoid clippy's string-slice
                // panic lint.
                if let Some(v) = text
                    .get(i..=e)
                    .and_then(|s| serde_json::from_str::<Value>(s).ok())
                {
                    if v.get("name").is_some() {
                        objs.push(v);
                    }
                }
                i = e + 1;
            }
            // Unbalanced from here on — nothing more to find.
            None => break,
        }
    }
    objs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn get_content_tool_call_name(content: &MessageContent) -> &str {
        match content {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                &call.name
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    fn get_content_tool_call_args(
        content: &MessageContent,
    ) -> Option<&serde_json::Map<String, Value>> {
        match content {
            MessageContent::ToolRequest(req) => {
                let call = req.tool_call.as_ref().unwrap();
                call.arguments.as_ref()
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    #[test]
    fn test_merge_streaming_deltas() {
        let deltas = vec![
            json!({"index": 0, "id": "call_1", "type": "function", "function": {"name": "developer__shell", "arguments": ""}}),
            json!({"index": 0, "function": {"arguments": "{\"command\":"}}),
            json!({"index": 0, "function": {"arguments": " \"ls\"}"}}),
        ];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "developer__shell");
        let args = get_content_tool_call_args(&contents[0]).unwrap();
        assert_eq!(args.get("command").unwrap(), "ls");
    }

    #[test]
    fn test_multiple_tool_calls_by_index() {
        let deltas = vec![
            json!({"index": 0, "id": "call_1", "function": {"name": "developer__shell", "arguments": "{\"command\": \"ls\"}"}}),
            json!({"index": 1, "id": "call_2", "function": {"name": "developer__shell", "arguments": "{\"command\": \"pwd\"}"}}),
        ];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert_eq!(contents.len(), 2);
        let args0 = get_content_tool_call_args(&contents[0]).unwrap();
        let args1 = get_content_tool_call_args(&contents[1]).unwrap();
        assert_eq!(args0.get("command").unwrap(), "ls");
        assert_eq!(args1.get("command").unwrap(), "pwd");
    }

    #[test]
    fn test_multiple_arguments_streamed() {
        let deltas = vec![
            json!({"index": 0, "id": "call_1", "function": {"name": "developer__shell", "arguments": ""}}),
            json!({"index": 0, "function": {"arguments": "{\"command\""}}),
            json!({"index": 0, "function": {"arguments": ": \"ls -la\","}}),
            json!({"index": 0, "function": {"arguments": " \"timeout\":"}}),
            json!({"index": 0, "function": {"arguments": " 30}"}}),
        ];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert_eq!(contents.len(), 1);
        let args = get_content_tool_call_args(&contents[0]).unwrap();
        assert_eq!(args.get("command").unwrap(), "ls -la");
        assert_eq!(args.get("timeout").unwrap(), 30);
    }

    #[test]
    fn test_empty_name_skipped() {
        let deltas = vec![json!({"index": 0, "function": {"name": "", "arguments": "{}"}})];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert!(contents.is_empty());
    }

    #[test]
    fn test_no_deltas() {
        let contents = extract_oai_tool_call_contents(&[]);
        assert!(contents.is_empty());
    }

    #[test]
    fn test_tool_call_without_arguments() {
        let deltas = vec![json!({"index": 0, "id": "call_1", "function": {"name": "some_tool"}})];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "some_tool");
        assert!(get_content_tool_call_args(&contents[0]).is_none());
    }

    #[test]
    fn test_malformed_arguments_drops_tool_call() {
        let deltas = vec![
            json!({"index": 0, "id": "call_1", "function": {"name": "developer__shell", "arguments": ""}}),
            json!({"index": 0, "function": {"arguments": "{\"command\": \"rm -rf"}}),
        ];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert!(contents.is_empty());
    }

    #[test]
    fn test_generates_id_when_missing() {
        let deltas =
            vec![json!({"index": 0, "function": {"name": "some_tool", "arguments": "{}"}})];
        let contents = extract_oai_tool_call_contents(&deltas);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "some_tool");
        match &contents[0] {
            MessageContent::ToolRequest(req) => {
                assert!(!req.id.is_empty());
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    // ---- text-JSON fallback (extract_text_json_tool_calls / find_json_objects) ----

    #[test]
    fn test_text_json_fenced_write() {
        // The exact shape Qwen2.5-Coder emits for the developer `write` tool.
        let text = "```json\n{\n  \"name\": \"write\",\n  \"arguments\": {\n    \"path\": \"/tmp/spike.txt\",\n    \"content\": \"hello\"\n  }\n}\n```";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "write");
        let args = get_content_tool_call_args(&contents[0]).unwrap();
        assert_eq!(args.get("path").unwrap(), "/tmp/spike.txt");
        assert_eq!(args.get("content").unwrap(), "hello");
    }

    #[test]
    fn test_text_json_bare_object() {
        let text = "{\"name\": \"shell\", \"arguments\": {\"command\": \"ls\"}}";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "shell");
        assert_eq!(
            get_content_tool_call_args(&contents[0])
                .unwrap()
                .get("command")
                .unwrap(),
            "ls"
        );
    }

    #[test]
    fn test_text_json_tool_call_tags() {
        let text = "<tool_call>{\"name\": \"tree\", \"arguments\": {}}</tool_call>";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "tree");
    }

    #[test]
    fn test_text_json_surrounded_by_prose() {
        let text = "Sure, I'll do that:\n```json\n{\"name\": \"write\", \"arguments\": {\"path\": \"a.txt\", \"content\": \"x\"}}\n```\nDone.";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(get_content_tool_call_name(&contents[0]), "write");
    }

    #[test]
    fn test_text_json_arguments_as_string() {
        // Some models double-encode arguments as a JSON string.
        let text = "{\"name\": \"write\", \"arguments\": \"{\\\"path\\\": \\\"b.txt\\\", \\\"content\\\": \\\"y\\\"}\"}";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        let args = get_content_tool_call_args(&contents[0]).unwrap();
        assert_eq!(args.get("path").unwrap(), "b.txt");
    }

    #[test]
    fn test_text_json_parameters_alias() {
        let text = "{\"name\": \"shell\", \"parameters\": {\"command\": \"pwd\"}}";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(
            get_content_tool_call_args(&contents[0])
                .unwrap()
                .get("command")
                .unwrap(),
            "pwd"
        );
    }

    #[test]
    fn test_text_json_multiple_calls() {
        let text = "```json\n{\"name\": \"shell\", \"arguments\": {\"command\": \"ls\"}}\n```\nthen\n```json\n{\"name\": \"shell\", \"arguments\": {\"command\": \"pwd\"}}\n```";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 2);
    }

    #[test]
    fn test_text_json_braces_inside_string_value() {
        // A `}` inside a string value must not end the object early.
        let text = "{\"name\": \"write\", \"arguments\": {\"path\": \"x.txt\", \"content\": \"fn main() { } end\"}}";
        let contents = extract_text_json_tool_calls(text);
        assert_eq!(contents.len(), 1);
        assert_eq!(
            get_content_tool_call_args(&contents[0])
                .unwrap()
                .get("content")
                .unwrap(),
            "fn main() { } end"
        );
    }

    #[test]
    fn test_text_json_no_name_is_skipped() {
        // Plausible JSON without a tool-call shape is ignored.
        let text = "Here is some data: {\"path\": \"x\", \"size\": 10}";
        assert!(extract_text_json_tool_calls(text).is_empty());
    }

    #[test]
    fn test_text_json_plain_prose_returns_empty() {
        let text = "I cannot complete that request without more information.";
        assert!(extract_text_json_tool_calls(text).is_empty());
    }

    #[test]
    fn test_text_json_name_without_arguments_is_not_a_tool_call() {
        // A name-bearing object with no arguments/parameters (e.g. a generated
        // package.json) must NOT be recovered and executed as a tool call.
        let text = "{\"name\": \"my-app\", \"version\": \"1.0.0\", \"private\": true}";
        assert!(extract_text_json_tool_calls(text).is_empty());
    }
}
