use crate::acp::tools::AcpAwareToolMeta;
use crate::agents::extension_manager::TRUSTED_TOOL_UPDATE_META_KEY;
use crate::conversation::message::{ToolRequest, ToolResponse};
use crate::mcp_utils::ToolResult;
use agent_client_protocol::schema::v1::{
    BlobResourceContents, Content, ContentBlock, EmbeddedResource, EmbeddedResourceResource,
    ImageContent, Meta, TextContent, TextResourceContents, ToolCall, ToolCallContent, ToolCallId,
    ToolCallLocation, ToolCallStatus, ToolCallUpdateFields,
};
use rmcp::model::{CallToolResult, RawContent, ResourceContents};

pub(crate) struct PendingToolCall {
    pub(crate) tool_call: ToolCall,
    pub(crate) identity_meta: Option<Meta>,
    pub(crate) fallback_title: String,
}

pub(crate) fn format_tool_name(tool_name: &str) -> String {
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
fn summarize_tool_call(tool_name: &str, arguments: Option<&serde_json::Value>) -> String {
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

pub(crate) fn tool_call_identity_meta(tool_request: &ToolRequest) -> Option<Meta> {
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

pub(crate) fn pending_tool_call_from_request(tool_request: &ToolRequest) -> PendingToolCall {
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

fn get_requested_line(arguments: Option<&rmcp::model::JsonObject>) -> Option<u32> {
    arguments
        .and_then(|args| args.get("line"))
        .and_then(|v| v.as_u64())
        .map(|l| l as u32)
}

fn is_developer_file_tool(tool_name: &str) -> bool {
    matches!(tool_name, "read" | "write" | "edit")
}

fn extract_locations_from_meta(tool_response: &ToolResponse) -> Option<Vec<ToolCallLocation>> {
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

fn extract_tool_locations(
    tool_request: &ToolRequest,
    tool_response: &ToolResponse,
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

fn extract_view_line_range(text: &str) -> Option<(usize, usize)> {
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

fn extract_first_line_number(text: &str) -> Option<usize> {
    let re = regex::Regex::new(r"```[^\n]*\n(\d+):").ok()?;
    if let Some(caps) = re.captures(text) {
        return caps.get(1)?.as_str().parse::<usize>().ok();
    }
    None
}

pub(crate) fn extract_tool_call_update_meta(tool_response: &ToolResponse) -> Option<Meta> {
    let tool_result = tool_response.tool_result.as_ref().ok()?;
    let goose_meta = tool_result
        .meta
        .as_ref()?
        .0
        .get(TRUSTED_TOOL_UPDATE_META_KEY)?
        .clone();
    let mut meta_map = serde_json::Map::new();
    meta_map.insert("goose".to_string(), goose_meta);
    Some(meta_map)
}

fn build_tool_call_content(tool_result: &ToolResult<CallToolResult>) -> Vec<ToolCallContent> {
    match tool_result {
        Ok(result) => result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                RawContent::Text(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Text(TextContent::new(val.text.clone())),
                ))),
                RawContent::Image(val) => Some(ToolCallContent::Content(Content::new(
                    ContentBlock::Image(ImageContent::new(val.data.clone(), val.mime_type.clone())),
                ))),
                RawContent::Resource(val) => {
                    let resource = match &val.resource {
                        ResourceContents::TextResourceContents {
                            mime_type,
                            text,
                            uri,
                            ..
                        } => EmbeddedResourceResource::TextResourceContents(
                            TextResourceContents::new(text.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                        ResourceContents::BlobResourceContents {
                            mime_type,
                            blob,
                            uri,
                            ..
                        } => EmbeddedResourceResource::BlobResourceContents(
                            BlobResourceContents::new(blob.clone(), uri.clone())
                                .mime_type(mime_type.clone()),
                        ),
                    };
                    Some(ToolCallContent::Content(Content::new(
                        ContentBlock::Resource(EmbeddedResource::new(resource)),
                    )))
                }
                RawContent::Audio(_) | RawContent::ResourceLink(_) => None,
            })
            .collect(),
        Err(error) => vec![ToolCallContent::Content(Content::new(ContentBlock::Text(
            TextContent::new(error.message.to_string()),
        )))],
    }
}

fn extract_tool_raw_output(tool_result: &ToolResult<CallToolResult>) -> Option<serde_json::Value> {
    tool_result
        .as_ref()
        .ok()
        .and_then(|result| result.structured_content.clone())
}

pub(crate) fn tool_call_update_fields_from_response(
    tool_response: &ToolResponse,
    tool_request: Option<&ToolRequest>,
) -> ToolCallUpdateFields {
    let is_failed = match &tool_response.tool_result {
        Ok(result) => result.is_error == Some(true),
        Err(_) => true,
    };
    let status = if is_failed {
        ToolCallStatus::Failed
    } else {
        ToolCallStatus::Completed
    };

    let mut fields = ToolCallUpdateFields::new().status(status);
    if let Some(raw_output) = extract_tool_raw_output(&tool_response.tool_result) {
        fields = fields.raw_output(raw_output);
    }
    let is_acp_aware = tool_response
        .tool_result
        .as_ref()
        .is_ok_and(|result| result.is_acp_aware());

    if is_failed || !is_acp_aware {
        fields = fields.content(build_tool_call_content(&tool_response.tool_result));
    }

    if !is_acp_aware {
        let locations = extract_locations_from_meta(tool_response).unwrap_or_else(|| {
            tool_request
                .map(|request| extract_tool_locations(request, tool_response))
                .unwrap_or_default()
        });
        if !locations.is_empty() {
            fields = fields.locations(locations);
        }
    }

    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{CallToolRequestParams, Content as RmcpContent};
    use std::path::PathBuf;
    use test_case::test_case;

    mod format_tool_name {
        use super::*;

        #[test]
        fn with_extension() {
            assert_eq!(format_tool_name("developer__edit"), "developer: edit");
            assert_eq!(
                format_tool_name("platform__manage_extensions"),
                "platform: manage extensions"
            );
            assert_eq!(format_tool_name("todo__write"), "todo: write");
        }

        #[test]
        fn without_extension() {
            assert_eq!(format_tool_name("simple_tool"), "simple tool");
            assert_eq!(format_tool_name("another_name"), "another name");
            assert_eq!(format_tool_name("single"), "single");
        }
    }

    mod summarize_tool_call {
        use super::*;

        #[test]
        fn no_args() {
            assert_eq!(
                summarize_tool_call("developer__shell", None),
                "developer: shell"
            );
        }

        #[test]
        fn with_path() {
            let args = serde_json::json!({"path": "/src/main.rs", "content": "fn main() {}"});
            assert_eq!(
                summarize_tool_call("developer__edit", Some(&args)),
                "developer: edit · /src/main.rs"
            );
        }

        #[test]
        fn with_command() {
            let args = serde_json::json!({"command": "cargo build"});
            assert_eq!(
                summarize_tool_call("developer__shell", Some(&args)),
                "developer: shell · cargo build"
            );
        }

        #[test]
        fn long_value_is_truncated() {
            let long_path = "a".repeat(80);
            let args = serde_json::json!({"path": long_path});
            let result = summarize_tool_call("developer__read_file", Some(&args));
            assert!(result.ends_with('…'));
            assert!(result.len() < 90);
        }
    }

    #[test]
    fn test_tool_call_identity_meta_uses_goose_extension_metadata() {
        let request = ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("context7__query-docs")),
            metadata: None,
            tool_meta: Some(serde_json::json!({"goose_extension": "context7"})),
        };

        let meta = tool_call_identity_meta(&request).expect("expected metadata");

        assert_eq!(
            meta.get("goose"),
            Some(&serde_json::json!({
                "toolCall": {
                    "toolName": "context7__query-docs",
                    "extensionName": "context7",
                },
            })),
        );
    }

    fn json_object(pairs: Vec<(&str, serde_json::Value)>) -> rmcp::model::JsonObject {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test_case(None => None ; "none arguments")]
    #[test_case(Some(json_object(vec![])) => None ; "missing line key")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!(5))])) => Some(5) ; "line present")]
    #[test_case(Some(json_object(vec![("line", serde_json::json!("not_a_number"))])) => None ; "line not a number")]
    fn test_get_requested_line(arguments: Option<rmcp::model::JsonObject>) -> Option<u32> {
        get_requested_line(arguments.as_ref())
    }

    #[test_case("read", true ; "read is developer file tool")]
    #[test_case("write", true ; "write is developer file tool")]
    #[test_case("edit", true ; "edit is developer file tool")]
    #[test_case("shell", false ; "shell is not developer file tool")]
    #[test_case("analyze", false ; "analyze is not developer file tool")]
    fn test_is_developer_file_tool(tool_name: &str, expected: bool) {
        assert_eq!(is_developer_file_tool(tool_name), expected);
    }

    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "line": 5}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(5))]
        ; "read returns requested line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("read").with_arguments(serde_json::json!({"path": "/tmp/f.txt"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), None)]
        ; "read without line"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("write").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "content": "hi"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "write returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("edit").with_arguments(serde_json::json!({"path": "/tmp/f.txt", "before": "a", "after": "b"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => vec![(PathBuf::from("/tmp/f.txt"), Some(1))]
        ; "edit returns line 1"
    )]
    #[test_case(
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(CallToolRequestParams::new("shell").with_arguments(serde_json::json!({"command": "ls"}).as_object().unwrap().clone())),
            metadata: None, tool_meta: None,
        },
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(CallToolResult::success(vec![RmcpContent::text("")])),
            metadata: None,
        }
        => Vec::<(PathBuf, Option<u32>)>::new()
        ; "non file tool returns empty"
    )]
    fn test_extract_tool_locations(
        request: ToolRequest,
        response: ToolResponse,
    ) -> Vec<(PathBuf, Option<u32>)> {
        extract_tool_locations(&request, &response)
            .into_iter()
            .map(|loc| (loc.path, loc.line))
            .collect()
    }

    fn response_with_meta(meta: Option<serde_json::Value>) -> ToolResponse {
        let mut result = CallToolResult::success(vec![RmcpContent::text("")]);
        result.meta = meta.map(|v| serde_json::from_value(v).unwrap());
        ToolResponse {
            id: "req_1".to_string(),
            tool_result: Ok(result),
            metadata: None,
        }
    }

    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt", "line": 5}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), Some(5))])
        ; "meta with path and line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({"tool_locations": [{"path": "/tmp/f.txt"}]})))
        => Some(vec![(PathBuf::from("/tmp/f.txt"), None)])
        ; "meta with path no line"
    )]
    #[test_case(
        response_with_meta(Some(serde_json::json!({})))
        => None
        ; "meta without tool_locations key"
    )]
    #[test_case(
        response_with_meta(None)
        => None
        ; "no meta"
    )]
    fn test_extract_locations_from_meta(
        response: ToolResponse,
    ) -> Option<Vec<(PathBuf, Option<u32>)>> {
        extract_locations_from_meta(&response)
            .map(|locs| locs.into_iter().map(|loc| (loc.path, loc.line)).collect())
    }

    mod extract_tool_call_update_meta {
        use super::*;

        #[test]
        fn ignores_untrusted_goose_meta() {
            let response = response_with_meta(Some(serde_json::json!({
                "goose": {
                    "mcpApp": {
                        "resourceUri": "ui://spoofed/app",
                    },
                },
            })));

            assert_eq!(extract_tool_call_update_meta(&response), None);
        }

        #[test]
        fn uses_trusted_meta_only() {
            let response = response_with_meta(Some(serde_json::json!({
                "goose": {
                    "mcpApp": {
                        "resourceUri": "ui://spoofed/app",
                    },
                },
                TRUSTED_TOOL_UPDATE_META_KEY: {
                    "mcpApp": {
                        "resourceUri": "ui://trusted/app",
                        "extensionName": "weather",
                        "toolName": "weather__render",
                    },
                },
            })));

            let extracted =
                extract_tool_call_update_meta(&response).expect("expected trusted meta");
            assert_eq!(
                extracted.get("goose"),
                Some(&serde_json::json!({
                    "mcpApp": {
                        "resourceUri": "ui://trusted/app",
                        "extensionName": "weather",
                        "toolName": "weather__render",
                    },
                })),
            );
        }
    }

    #[test]
    fn test_extract_tool_raw_output_preserves_structured_content() {
        let mut result = CallToolResult::success(vec![RmcpContent::text("fallback")]);
        result.structured_content = Some(serde_json::json!({
            "restaurants": [
                {
                    "name": "Coffee Shop",
                    "unitToken": "unit-1",
                },
            ],
        }));

        assert_eq!(
            extract_tool_raw_output(&Ok(result)),
            Some(serde_json::json!({
                "restaurants": [
                    {
                        "name": "Coffee Shop",
                        "unitToken": "unit-1",
                    },
                ],
            })),
        );
    }

    fn response_from_tool_result(tool_result: ToolResult<CallToolResult>) -> ToolResponse {
        ToolResponse {
            id: "req_1".to_string(),
            tool_result,
            metadata: None,
        }
    }

    fn write_request(path: &str) -> ToolRequest {
        ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(
                CallToolRequestParams::new("write").with_arguments(json_object(vec![
                    ("path", serde_json::json!(path)),
                    ("content", serde_json::json!("updated")),
                ])),
            ),
            metadata: None,
            tool_meta: None,
        }
    }

    fn first_tool_call_text(fields: &ToolCallUpdateFields) -> Option<&str> {
        fields.content.as_ref()?.iter().find_map(|content| {
            let ToolCallContent::Content(content) = content else {
                return None;
            };
            let ContentBlock::Text(text) = &content.content else {
                return None;
            };
            Some(text.text.as_str())
        })
    }

    mod tool_call_update_fields_from_response {
        use super::*;

        #[test]
        fn includes_ordinary_success_details() {
            let raw_output = serde_json::json!({ "changed": true });
            let mut result = CallToolResult::success(vec![RmcpContent::text("write completed")]);
            result.structured_content = Some(raw_output.clone());
            let response = response_from_tool_result(Ok(result));
            let request = write_request("/tmp/request.txt");

            let fields = tool_call_update_fields_from_response(&response, Some(&request));

            assert_eq!(fields.status, Some(ToolCallStatus::Completed));
            assert_eq!(fields.raw_output, Some(raw_output));
            assert_eq!(first_tool_call_text(&fields), Some("write completed"));
            let locations = fields.locations.as_deref().expect("expected location");
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].path, PathBuf::from("/tmp/request.txt"));
            assert_eq!(locations[0].line, Some(1));
        }

        #[test]
        fn includes_ordinary_error_content() {
            let response =
                response_from_tool_result(Ok(CallToolResult::error(vec![RmcpContent::text(
                    "write failed",
                )])));

            let fields = tool_call_update_fields_from_response(&response, None);

            assert_eq!(fields.status, Some(ToolCallStatus::Failed));
            assert_eq!(first_tool_call_text(&fields), Some("write failed"));
            assert!(fields.locations.is_none());
        }

        #[test]
        fn suppresses_acp_aware_success_details() {
            let raw_output = serde_json::json!({ "changed": true });
            let mut result = CallToolResult::success(vec![RmcpContent::text("write completed")]);
            result.structured_content = Some(raw_output.clone());
            let response = response_from_tool_result(Ok(result.with_acp_aware_meta()));
            let request = write_request("/tmp/request.txt");

            let fields = tool_call_update_fields_from_response(&response, Some(&request));

            assert_eq!(fields.status, Some(ToolCallStatus::Completed));
            assert_eq!(fields.raw_output, Some(raw_output));
            assert!(fields.content.is_none());
            assert!(fields.locations.is_none());
        }

        #[test]
        fn prefers_explicit_location() {
            let response = response_with_meta(Some(serde_json::json!({
                "tool_locations": [{ "path": "/tmp/response.txt", "line": 7 }]
            })));
            let request = write_request("/tmp/request.txt");

            let fields = tool_call_update_fields_from_response(&response, Some(&request));

            let locations = fields.locations.as_deref().expect("expected location");
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].path, PathBuf::from("/tmp/response.txt"));
            assert_eq!(locations[0].line, Some(7));
        }

        #[test]
        fn includes_acp_aware_error_content() {
            let result = CallToolResult::error(vec![RmcpContent::text("write failed")])
                .with_acp_aware_meta();
            let response = response_from_tool_result(Ok(result));
            let request = write_request("/tmp/request.txt");

            let fields = tool_call_update_fields_from_response(&response, Some(&request));

            assert_eq!(fields.status, Some(ToolCallStatus::Failed));
            assert_eq!(first_tool_call_text(&fields), Some("write failed"));
            assert!(fields.locations.is_none());
        }

        #[test]
        fn includes_transport_error_content() {
            let response = response_from_tool_result(Err(rmcp::model::ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                "transport failed",
                None,
            )));

            let fields = tool_call_update_fields_from_response(&response, None);

            assert_eq!(fields.status, Some(ToolCallStatus::Failed));
            assert_eq!(first_tool_call_text(&fields), Some("transport failed"));
        }
    }
}
