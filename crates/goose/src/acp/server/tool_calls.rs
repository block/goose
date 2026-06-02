use super::*;

pub fn get_requested_line(arguments: Option<&rmcp::model::JsonObject>) -> Option<u32> {
    arguments
        .and_then(|args| args.get("line"))
        .and_then(|v| v.as_u64())
        .map(|l| l as u32)
}

pub fn is_developer_file_tool(tool_name: &str) -> bool {
    matches!(tool_name, "read" | "write" | "edit")
}

pub fn extract_locations_from_meta(
    tool_response: &crate::conversation::message::ToolResponse,
) -> Option<Vec<ToolCallLocation>> {
    let result = tool_response.tool_result.as_ref().ok()?;
    let meta = result.meta.as_ref()?;
    let locations_val = meta.get("tool_locations")?;
    let entries: Vec<serde_json::Value> = serde_json::from_value(locations_val.clone()).ok()?;
    let locations = entries
        .into_iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?;
            let line = entry.get("line").and_then(|v| v.as_u64()).map(|l| l as u32);
            Some(ToolCallLocation::new(path).line(line))
        })
        .collect::<Vec<_>>();
    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}

pub fn extract_tool_locations(
    tool_request: &crate::conversation::message::ToolRequest,
    tool_response: &crate::conversation::message::ToolResponse,
) -> Vec<ToolCallLocation> {
    let mut locations = Vec::new();

    if let Ok(tool_call) = &tool_request.tool_call {
        if !is_developer_file_tool(tool_call.name.as_ref()) {
            return locations;
        }

        let tool_name = tool_call.name.as_ref();
        let path_str = tool_call
            .arguments
            .as_ref()
            .and_then(|args| args.get("path"))
            .and_then(|p| p.as_str());

        if let Some(path_str) = path_str {
            if matches!(tool_name, "read") {
                let line = get_requested_line(tool_call.arguments.as_ref());
                locations.push(ToolCallLocation::new(path_str).line(line));
                return locations;
            }

            if matches!(tool_name, "write" | "edit") {
                locations.push(ToolCallLocation::new(path_str).line(1));
                return locations;
            }

            let command = tool_call
                .arguments
                .as_ref()
                .and_then(|args| args.get("command"))
                .and_then(|c| c.as_str());

            if let Ok(result) = &tool_response.tool_result {
                for content in &result.content {
                    if let RawContent::Text(text_content) = &content.raw {
                        let text = &text_content.text;

                        match command {
                            Some("view") => {
                                let line = extract_view_line_range(text)
                                    .map(|range| range.0 as u32)
                                    .or(Some(1));
                                locations.push(ToolCallLocation::new(path_str).line(line));
                            }
                            Some("str_replace") | Some("insert") => {
                                let line = extract_first_line_number(text)
                                    .map(|l| l as u32)
                                    .or(Some(1));
                                locations.push(ToolCallLocation::new(path_str).line(line));
                            }
                            Some("write") => {
                                locations.push(ToolCallLocation::new(path_str).line(1));
                            }
                            _ => {
                                locations.push(ToolCallLocation::new(path_str).line(1));
                            }
                        }
                        break;
                    }
                }
            }

            if locations.is_empty() {
                locations.push(ToolCallLocation::new(path_str).line(1));
            }
        }
    }

    locations
}

pub fn extract_view_line_range(text: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"\(lines (\d+)-(\d+|end)\)").ok()?;
    if let Some(caps) = re.captures(text) {
        let start = caps.get(1)?.as_str().parse::<usize>().ok()?;
        let end = if caps.get(2)?.as_str() == "end" {
            start
        } else {
            caps.get(2)?.as_str().parse::<usize>().ok()?
        };
        return Some((start, end));
    }
    None
}

pub fn extract_first_line_number(text: &str) -> Option<usize> {
    let re = regex::Regex::new(r"```[^\n]*\n(\d+):").ok()?;
    if let Some(caps) = re.captures(text) {
        return caps.get(1)?.as_str().parse::<usize>().ok();
    }
    None
}

pub fn read_resource_link(link: ResourceLink) -> Option<String> {
    let url = Url::parse(&link.uri).ok()?;
    if url.scheme() == "file" {
        let path = url.to_file_path().ok()?;
        let contents = fs::read_to_string(&path).ok()?;

        Some(format!(
            "\n\n# {}\n```\n{}\n```",
            path.to_string_lossy(),
            contents
        ))
    } else {
        None
    }
}

pub fn format_tool_name(tool_name: &str) -> String {
    if let Some((extension, tool)) = tool_name.split_once("__") {
        format!(
            "{}: {}",
            extension.replace('_', " "),
            tool.replace('_', " ")
        )
    } else {
        tool_name.replace('_', " ")
    }
}

/// Build a short fallback title from the tool name and arguments by extracting
/// the most useful value (file path, command, query, url, etc.).
pub fn summarize_tool_call(tool_name: &str, arguments: Option<&serde_json::Value>) -> String {
    let base = format_tool_name(tool_name);

    let detail = arguments.and_then(|args| {
        let obj = args.as_object()?;
        let keys = [
            "path", "file", "command", "query", "url", "uri", "name", "pattern", "source",
        ];
        for key in &keys {
            if let Some(v) = obj.get(*key) {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                if !s.is_empty() {
                    let first_line = s.lines().next().unwrap_or(&s);
                    if first_line.len() > 60 {
                        return Some(format!("{}…", crate::utils::safe_truncate(first_line, 57)));
                    }
                    return Some(first_line.to_string());
                }
            }
        }
        None
    });

    match detail {
        Some(d) => format!("{base} · {d}"),
        None => base,
    }
}

pub fn tool_call_identity_meta(tool_request: &ToolRequest) -> Option<Meta> {
    let tool_call = tool_request.tool_call.as_ref().ok()?;
    let tool_name = tool_call.name.to_string();
    let extension_name = tool_request
        .tool_meta
        .as_ref()
        .and_then(|meta| meta.get("goose_extension"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            tool_name
                .split_once("__")
                .map(|(extension_name, _)| extension_name.to_string())
        });

    let mut tool_call_meta = serde_json::Map::new();
    tool_call_meta.insert("toolName".to_string(), serde_json::Value::String(tool_name));
    if let Some(extension_name) = extension_name {
        tool_call_meta.insert(
            "extensionName".to_string(),
            serde_json::Value::String(extension_name),
        );
    }

    let mut goose_meta = serde_json::Map::new();
    goose_meta.insert(
        "toolCall".to_string(),
        serde_json::Value::Object(tool_call_meta),
    );

    let mut meta = serde_json::Map::new();
    meta.insert("goose".to_string(), serde_json::Value::Object(goose_meta));
    Some(meta)
}

/// Add `goose.toolChainSummary = { summary, count }` to a `Meta` blob,
/// preserving any existing `goose.*` keys (e.g. `goose.toolCall` set by
/// [`tool_call_identity_meta`]).
pub fn with_tool_chain_summary_meta(
    base: Option<Meta>,
    summary: &str,
    count: usize,
) -> Option<Meta> {
    let mut meta = base.unwrap_or_default();
    let goose_entry = meta
        .entry("goose".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    let goose_obj = match goose_entry {
        serde_json::Value::Object(obj) => obj,
        other => {
            *other = serde_json::Value::Object(serde_json::Map::new());
            match other {
                serde_json::Value::Object(obj) => obj,
                _ => unreachable!(),
            }
        }
    };
    let mut chain = serde_json::Map::new();
    chain.insert(
        "summary".to_string(),
        serde_json::Value::String(summary.to_string()),
    );
    chain.insert(
        "count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(count)),
    );
    goose_obj.insert(
        "toolChainSummary".to_string(),
        serde_json::Value::Object(chain),
    );
    Some(meta)
}

pub struct PendingToolCall {
    pub tool_call: ToolCall,
    pub identity_meta: Option<Meta>,
    pub fallback_title: String,
}

/// Extract chains (runs of consecutive `MessageContent::ToolRequest` blocks)
/// from a single message's content. Mirrors the frontend's chain detection in
/// `MessageBubble.groupContentSections`: any non-tool block (text, thinking,
/// image, etc.) breaks the run.
///
/// Returns one inner Vec per detected chain, holding the tool_call_ids in
/// document order. Single-tool runs are included; callers (chain
/// summarization) gate on `chain.len() >= 2`.
///
/// Note: this is the per-message view, kept around for tests and potential
/// replay use. The live runtime path uses a streaming buffer fed by
/// [`register_chain_buffer`] so chains that span multiple `AgentEvent::Message`
/// events (e.g. Bedrock-style streaming, where one LLM message is split across
/// rows — see `f087fa63c`) are still detected.
#[allow(dead_code)]
pub fn extract_tool_chains(
    content: &[crate::conversation::message::MessageContent],
) -> Vec<Vec<String>> {
    use crate::conversation::message::MessageContent;
    let mut chains: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for block in content {
        match block {
            MessageContent::ToolRequest(tr) => current.push(tr.id.clone()),
            MessageContent::ToolResponse(_) => {
                // Server-side, assistant messages don't carry responses;
                // responses arrive in subsequent messages. Treat as
                // chain-neutral so a stray response doesn't split a chain
                // if the data shape ever changes.
            }
            _ => {
                if !current.is_empty() {
                    chains.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        chains.push(current);
    }
    chains
}

/// If `buffer` holds a multi-tool run (≥ 2 tool requests), (re)register a
/// [`ToolChain`] in `chain_membership` anchored on the **first** tool's
/// message_id (the row [`SessionManager::update_tool_request_meta`] will patch
/// when persisting the LLM-generated summary). Does **not** clear the buffer
/// — chains can grow as more tools arrive (sequential tool use), so callers
/// keep accumulating and re-registering with the larger set of ids.
///
/// The buffer contains `(tool_call_id, message_id)` pairs in arrival order,
/// fed by the prompt stream loop. Sequential tool use (Bedrock/Anthropic)
/// interleaves request → response → request → response across separate
/// `AgentEvent::Message` events, so per-event `extract_tool_chains` only
/// sees length-1 chains and would miss the run. Tool responses are
/// chain-neutral (they don't split the run); only non-tool content (text,
/// thinking, image, etc.) does, matching the frontend's
/// `groupContentSections` behavior.
pub fn extend_chain_membership(
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

pub fn pending_tool_call_from_request(tool_request: &ToolRequest) -> PendingToolCall {
    let tool_name = match &tool_request.tool_call {
        Ok(tool_call) => tool_call.name.to_string(),
        Err(_) => "error".to_string(),
    };
    let args_value = tool_request
        .tool_call
        .as_ref()
        .ok()
        .and_then(|tc| tc.arguments.as_ref())
        .map(|a| serde_json::Value::Object(a.clone()));
    let fallback_title = summarize_tool_call(&tool_name, args_value.as_ref());
    let identity_meta = tool_call_identity_meta(tool_request);

    // Prefer the persisted LLM-generated title when available so replay (and
    // any subsequent live initial ToolCall after the title task has already
    // resolved) emits the nice title up front, with no flash of the
    // deterministic fallback.
    let initial_title = tool_request
        .persisted_title()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_title.clone());

    let mut tool_call = ToolCall::new(ToolCallId::new(tool_request.id.clone()), initial_title)
        .status(ToolCallStatus::Pending);
    if let Some(args) = args_value {
        tool_call = tool_call.raw_input(args);
    }

    PendingToolCall {
        tool_call,
        identity_meta,
        fallback_title,
    }
}
