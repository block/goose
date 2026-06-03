//! Converter for pi-mono `.jsonl` session files.
//!
//! Pi sessions start with a header line `{"type":"session","version":N,"cwd":..}`
//! followed by entries with `type` in `{message, model_change, compaction,
//! branch_summary, thinking_level_change, custom, ...}`. The interesting
//! ones for replay-in-goose are `message`, whose `message` field carries an
//! `AgentMessage` (`role` is one of `user`, `assistant`, `toolResult`,
//! `bashExecution`, ...).
//!
//! Format reference: pi-mono `packages/coding-agent/docs/session.md`.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rmcp::model::{CallToolRequestParams, CallToolResult, Content, ErrorCode, ErrorData};
use serde_json::{json, Map, Value};

use crate::conversation::message::Message;
use crate::conversation::Conversation;

pub fn convert(content: &str) -> Result<String> {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());

    let header: Value = match lines.next() {
        Some(l) => serde_json::from_str(l)
            .map_err(|e| anyhow!("Pi import: header is not valid JSON: {e}"))?,
        None => return Err(anyhow!("Pi import: empty file")),
    };
    if header.get("type").and_then(|v| v.as_str()) != Some("session") {
        return Err(anyhow!("Pi import: missing session header"));
    }

    let cwd = header
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let session_id = header
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("imported")
        .to_string();
    let header_ts = header
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let mut messages: Vec<Message> = Vec::new();
    let mut total_input: i64 = 0;
    let mut total_output: i64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut first_ts: Option<DateTime<Utc>> = header_ts;
    let mut last_ts: Option<DateTime<Utc>> = header_ts;
    let mut first_user_text: Option<String> = None;

    let entries: Vec<Value> = lines
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();

    // Pi entries form a tree, but in practice the file is written in
    // chronological order and the linear view is what users expect on import.
    // We just walk top-to-bottom.
    for (entry_idx, entry) in entries.iter().enumerate() {
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let ts = entry
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        if let Some(t) = ts {
            first_ts.get_or_insert(t);
            last_ts = Some(t);
        }

        if entry_type != "message" {
            continue;
        }
        let Some(inner) = entry.get("message") else {
            continue;
        };
        let role = inner.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let created = ts
            .map(|t| t.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());

        if let Some(usage) = inner.get("usage").and_then(|u| u.as_object()) {
            total_input += usage.get("input").and_then(|v| v.as_i64()).unwrap_or(0);
            total_input += usage.get("cacheRead").and_then(|v| v.as_i64()).unwrap_or(0);
            total_input += usage
                .get("cacheWrite")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            total_output += usage.get("output").and_then(|v| v.as_i64()).unwrap_or(0);
            if let Some(cost) = usage
                .get("cost")
                .and_then(|c| c.get("total"))
                .and_then(|v| v.as_f64())
            {
                total_cost += cost;
            }
        }

        match role {
            "user" => {
                let mut msg = Message::user();
                msg.created = created;
                msg = apply_user_content(msg, inner.get("content"));
                if !msg.content.is_empty() {
                    if first_user_text.is_none() {
                        first_user_text = extract_first_text(&msg);
                    }
                    messages.push(msg);
                }
            }
            "assistant" => {
                let mut msg = Message::assistant();
                msg.created = created;
                msg = apply_assistant_content(msg, inner.get("content"));
                if !msg.content.is_empty() {
                    messages.push(msg);
                }
            }
            "toolResult" => {
                let id = inner
                    .get("toolCallId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let is_error = inner
                    .get("isError")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let result = build_tool_result(inner.get("content"), is_error);
                let mut msg = Message::user();
                msg.created = created;
                msg = msg.with_tool_response(id, result);
                messages.push(msg);
            }
            "bashExecution" => {
                // Synthesize a bash tool round-trip so the export reads naturally.
                let command = inner
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let output = inner
                    .get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let exit_code = inner.get("exitCode").and_then(|v| v.as_i64());

                let mut args = Map::new();
                args.insert("command".into(), json!(command));
                let params = CallToolRequestParams::new("bash".to_string()).with_arguments(args);
                let id = format!("pi_bash_{}", entry_idx);

                let mut req = Message::assistant();
                req.created = created;
                req = req.with_tool_request(id.clone(), Ok(params));
                messages.push(req);

                let result_text = match exit_code {
                    Some(code) if code != 0 => format!("exit {}\n{}", code, output),
                    _ => output,
                };
                let mut resp = Message::user();
                resp.created = created;
                resp = resp.with_tool_response(
                    id,
                    Ok(CallToolResult::success(vec![Content::text(result_text)])),
                );
                messages.push(resp);
            }
            _ => {
                // custom / branchSummary / compactionSummary — emit as text
                // notes from the assistant so the context is preserved.
                if let Some(s) = inner.get("summary").and_then(|v| v.as_str()) {
                    let mut msg = Message::assistant();
                    msg.created = created;
                    msg = msg.with_text(format!("[{}] {}", role, s));
                    messages.push(msg);
                }
            }
        }
    }

    let working_dir = if cwd.is_empty() {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    } else {
        cwd
    };

    let name = first_user_text
        .as_deref()
        .map(super::summarize_first_line)
        .unwrap_or_else(|| format!("Imported pi session {}", session_id));

    let created_at = first_ts.unwrap_or_else(Utc::now);
    let updated_at = last_ts.unwrap_or(created_at);
    let conversation = Conversation::new_unvalidated(messages);

    let session_json = build_session_json(
        &session_id,
        &working_dir,
        &name,
        created_at,
        updated_at,
        Some(total_input as i32),
        Some(total_output as i32),
        if total_cost > 0.0 {
            Some(total_cost)
        } else {
            None
        },
        conversation,
    );

    serde_json::to_string_pretty(&session_json).map_err(Into::into)
}

fn apply_user_content(mut msg: Message, content: Option<&Value>) -> Message {
    match content {
        Some(Value::String(s)) => {
            msg = msg.with_text(s.clone());
        }
        Some(Value::Array(blocks)) => {
            for block in blocks {
                let bt = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match bt {
                    "text" => {
                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                            msg = msg.with_text(t);
                        }
                    }
                    "image" => {
                        if let (Some(data), Some(mime)) = (
                            block.get("data").and_then(|v| v.as_str()),
                            block.get("mimeType").and_then(|v| v.as_str()),
                        ) {
                            msg = msg.with_image(data, mime);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    msg
}

fn apply_assistant_content(mut msg: Message, content: Option<&Value>) -> Message {
    let blocks = match content {
        Some(Value::Array(b)) => b,
        Some(Value::String(s)) => return msg.with_text(s.clone()),
        _ => return msg,
    };
    for block in blocks {
        let bt = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match bt {
            "text" => {
                if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                    if !t.is_empty() {
                        msg = msg.with_text(t);
                    }
                }
            }
            "thinking" => {
                let t = block.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
                if !t.is_empty() {
                    msg = msg.with_thinking(t, "");
                }
            }
            "toolCall" => {
                let id = block
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown_tool");
                let args = block
                    .get("arguments")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                let params = CallToolRequestParams::new(name.to_string()).with_arguments(args);
                msg = msg.with_tool_request(id, Ok(params));
            }
            _ => {}
        }
    }
    msg
}

fn build_tool_result(content: Option<&Value>, is_error: bool) -> Result<CallToolResult, ErrorData> {
    let text = match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| {
                let bt = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match bt {
                    "text" => b.get("text").and_then(|v| v.as_str()).map(str::to_string),
                    _ => Some(serde_json::to_string(b).unwrap_or_default()),
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
        None => String::new(),
    };

    if is_error {
        Err(ErrorData::new(ErrorCode::INTERNAL_ERROR, text, None))
    } else {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

fn extract_first_text(msg: &Message) -> Option<String> {
    use crate::conversation::message::MessageContent;
    for c in &msg.content {
        if let MessageContent::Text(t) = c {
            return Some(t.text.clone());
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn build_session_json(
    session_id: &str,
    working_dir: &str,
    name: &str,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    cost: Option<f64>,
    conversation: Conversation,
) -> Value {
    let total = match (input_tokens, output_tokens) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };
    let mut obj = Map::new();
    obj.insert("id".into(), json!(session_id));
    obj.insert("working_dir".into(), json!(working_dir));
    obj.insert("name".into(), json!(name));
    obj.insert("user_set_name".into(), json!(false));
    obj.insert("session_type".into(), json!("user"));
    obj.insert("created_at".into(), json!(created_at.to_rfc3339()));
    obj.insert("updated_at".into(), json!(updated_at.to_rfc3339()));
    obj.insert("extension_data".into(), json!({}));
    obj.insert("total_tokens".into(), json!(total));
    obj.insert("input_tokens".into(), json!(input_tokens));
    obj.insert("output_tokens".into(), json!(output_tokens));
    obj.insert("accumulated_total_tokens".into(), json!(total));
    obj.insert("accumulated_input_tokens".into(), json!(input_tokens));
    obj.insert("accumulated_output_tokens".into(), json!(output_tokens));
    obj.insert("accumulated_cost".into(), json!(cost));
    obj.insert("schedule_id".into(), json!(null));
    obj.insert("recipe".into(), json!(null));
    obj.insert("user_recipe_values".into(), json!(null));
    obj.insert(
        "conversation".into(),
        serde_json::to_value(&conversation).unwrap(),
    );
    obj.insert("message_count".into(), json!(conversation.messages().len()));
    obj.insert("provider_name".into(), json!(null));
    obj.insert("model_config".into(), json!(null));
    obj.insert("goose_mode".into(), json!("auto"));
    obj.insert("archived_at".into(), json!(null));
    obj.insert("project_id".into(), json!(null));
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_tool_call_and_result() {
        let jsonl = r#"{"type":"session","version":3,"id":"s","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/w"}
{"type":"message","id":"a","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","message":{"role":"user","content":"list files"}}
{"type":"message","id":"b","parentId":"a","timestamp":"2024-12-03T14:00:02.000Z","message":{"role":"assistant","content":[{"type":"toolCall","id":"t1","name":"bash","arguments":{"command":"ls"}}]}}
{"type":"message","id":"c","parentId":"b","timestamp":"2024-12-03T14:00:03.000Z","message":{"role":"toolResult","toolCallId":"t1","toolName":"bash","content":[{"type":"text","text":"a.txt\nb.txt"}],"isError":false}}"#;

        let json = convert(jsonl).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        let msgs = v["conversation"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert!(msgs[1]["content"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["type"] == "toolRequest"));
        assert!(msgs[2]["content"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["type"] == "toolResponse"));
    }

    #[test]
    fn synthesizes_bash_execution() {
        let jsonl = r#"{"type":"session","version":3,"id":"s","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/w"}
{"type":"message","id":"a","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","message":{"role":"user","content":"!ls"}}
{"type":"message","id":"b","parentId":"a","timestamp":"2024-12-03T14:00:02.000Z","message":{"role":"bashExecution","command":"ls","output":"file.txt","exitCode":0,"cancelled":false,"truncated":false}}"#;

        let json = convert(jsonl).unwrap();
        let v: Value = serde_json::from_str(&json).unwrap();
        let msgs = v["conversation"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
    }
}
