use indoc::formatdoc;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, CreateMessageRequestParams, ErrorCode, ErrorData, Implementation,
        SamplingMessage, SamplingMessageContent, ServerCapabilities, ServerInfo,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_handler, tool_router, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KpiInput {
    pub label: String,
    pub value: JsonValue,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChartInput {
    pub title: String,
    pub r#type: String,
    pub x_key: String,
    pub y_keys: Vec<String>,
    pub data: Vec<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TableInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<JsonValue>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LayoutHints {
    #[serde(default)]
    pub columns: Option<u8>,
    #[serde(default)]
    pub dense: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComposeDashboardParams {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub kpis: Vec<KpiInput>,
    #[serde(default)]
    pub charts: Vec<ChartInput>,
    #[serde(default)]
    pub table: Option<TableInput>,
    #[serde(default)]
    pub layout_hints: Option<LayoutHints>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// GenUI MCP server that uses MCP Sampling (sampling/createMessage) to generate json-render specs.
///
/// This server returns **JSONL RFC6902 patch operations only** (no prose, no markdown fences),
/// so any agent can call it and include the result directly in its response.
#[derive(Clone)]
pub struct GenUiServiceServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
}

impl Default for GenUiServiceServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl GenUiServiceServer {
    pub fn new() -> Self {
        let instructions = formatdoc! {r#"
            This extension generates **json-render** UI specs.

            CRITICAL OUTPUT CONTRACT:
            - Return **JSONL RFC6902 JSON Patch operations only** (one JSON object per line).
            - No markdown fences.
            - No prose.
            - Start by adding /root, then /elements/*, then /state/* as needed.

            Tool: compose_dashboard
            "#};

        Self {
            tool_router: Self::tool_router(),
            instructions,
        }
    }

    fn system_prompt() -> String {
        // The purpose of this server is to centralize json-render generation guidance
        // (similar to the Vercel examples) so individual agents don't need to duplicate it.
        formatdoc! {r#"
            You are a json-render UI generator.

            OUTPUT:
            - Output ONLY JSONL RFC6902 JSON Patch operations.
            - One JSON object per line.
            - No markdown.
            - No prose.

            UI RULES (DS-first):
            - Prefer compact dashboards. Avoid viewport-height layouts.
            - Use Card(maxWidth="lg", centered=true) as the top-level container.
            - Use Grid(columns=2) for small KPI groups and side-by-side charts.
            - Use Heading(level=3/4) and Text(text/variant) for labels.
            - Keep charts short (height ~180).
            - Tables should be concise (<= 7-10 rows unless asked otherwise).

            COMPONENTS you may use:
            Card, Stack, Grid, Separator, Heading, Text, Alert, Badge, Chart, Table, Tabs, Progress, Button.

            Charts:
            - Use Chart props: type, title, height, data, xKey, yKeys.

            Tables:
            - Use Table props: columns (string[]), rows (string[][]), optional caption.
            "#}
    }

    fn build_user_message(params: &ComposeDashboardParams) -> Result<String, ErrorData> {
        let mut input = Map::<String, JsonValue>::new();

        if let Some(title) = &params.title {
            input.insert("title".to_string(), JsonValue::String(title.clone()));
        }

        if !params.kpis.is_empty() {
            input.insert(
                "kpis".to_string(),
                JsonValue::Array(
                    params
                        .kpis
                        .iter()
                        .map(|k| {
                            let mut o = Map::new();
                            o.insert("label".to_string(), JsonValue::String(k.label.clone()));
                            o.insert("value".to_string(), k.value.clone());
                            if let Some(desc) = &k.description {
                                o.insert(
                                    "description".to_string(),
                                    JsonValue::String(desc.clone()),
                                );
                            }
                            JsonValue::Object(o)
                        })
                        .collect(),
                ),
            );
        }

        if !params.charts.is_empty() {
            input.insert(
                "charts".to_string(),
                JsonValue::Array(
                    params
                        .charts
                        .iter()
                        .map(|c| {
                            let mut o = Map::new();
                            o.insert("title".to_string(), JsonValue::String(c.title.clone()));
                            o.insert("type".to_string(), JsonValue::String(c.r#type.clone()));
                            o.insert("xKey".to_string(), JsonValue::String(c.x_key.clone()));
                            o.insert(
                                "yKeys".to_string(),
                                JsonValue::Array(
                                    c.y_keys.iter().cloned().map(JsonValue::String).collect(),
                                ),
                            );
                            o.insert("data".to_string(), JsonValue::Array(c.data.clone()));
                            JsonValue::Object(o)
                        })
                        .collect(),
                ),
            );
        }

        if let Some(table) = &params.table {
            let mut o = Map::new();
            if let Some(title) = &table.title {
                o.insert("title".to_string(), JsonValue::String(title.clone()));
            }
            if let Some(caption) = &table.caption {
                o.insert("caption".to_string(), JsonValue::String(caption.clone()));
            }
            o.insert(
                "columns".to_string(),
                JsonValue::Array(
                    table
                        .columns
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            );
            o.insert(
                "rows".to_string(),
                JsonValue::Array(
                    table
                        .rows
                        .iter()
                        .map(|row| JsonValue::Array(row.clone()))
                        .collect(),
                ),
            );
            input.insert("table".to_string(), JsonValue::Object(o));
        }

        if let Some(hints) = &params.layout_hints {
            let mut o = Map::new();
            if let Some(columns) = hints.columns {
                o.insert("columns".to_string(), JsonValue::Number(columns.into()));
            }
            if let Some(dense) = hints.dense {
                o.insert("dense".to_string(), JsonValue::Bool(dense));
            }
            input.insert("layoutHints".to_string(), JsonValue::Object(o));
        }

        if let Some(notes) = &params.notes {
            input.insert("notes".to_string(), JsonValue::String(notes.clone()));
        }

        Ok(formatdoc! {r#"
            Generate a compact json-render dashboard for the following input data.

            INPUT (JSON):
            {payload}
            "#,
            payload = serde_json::to_string_pretty(&JsonValue::Object(input)).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to serialize input payload: {e}"),
                    None,
                )
            })?
        })
    }

    fn validate_jsonl_patches(output: &str) -> Result<(), ErrorData> {
        const MAX_SPEC_CHARS: usize = 250_000;
        const MAX_JSONL_LINES: usize = 5_000;
        const MAX_ELEMENTS: usize = 5_000;

        const ALLOWED_COMPONENT_TYPES: &[&str] = &[
            // shadcn primitives
            "Card",
            "Stack",
            "Grid",
            "Separator",
            "Tabs",
            "Accordion",
            "Collapsible",
            "Dialog",
            "Drawer",
            "Popover",
            "Tooltip",
            "DropdownMenu",
            "Heading",
            "Text",
            "Image",
            "Avatar",
            "Badge",
            "Alert",
            "Table",
            "Carousel",
            "Progress",
            "Skeleton",
            "Spinner",
            "Input",
            "Textarea",
            "Select",
            "Checkbox",
            "Switch",
            "Slider",
            "Button",
            "Toggle",
            "Link",
            "Pagination",
            // Goose DS overlay
            "PageHeader",
            "DataCard",
            "StatCard",
            "ListItem",
            "TreeItem",
            "EmptyState",
            "LoadingState",
            "ErrorState",
            "SearchInput",
            "TabBar",
            "CodeBlock",
            "Chart",
        ];

        if output.chars().count() > MAX_SPEC_CHARS {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Generated spec too large (>{MAX_SPEC_CHARS} chars)"),
                None,
            ));
        }

        let lines: Vec<&str> = output.lines().collect();
        if lines.len() > MAX_JSONL_LINES {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Generated spec too large (>{MAX_JSONL_LINES} lines)"),
                None,
            ));
        }

        let allowed = std::collections::HashSet::<&'static str>::from_iter(
            ALLOWED_COMPONENT_TYPES.iter().copied(),
        );

        let mut saw_root = false;
        let mut element_ids = std::collections::HashSet::<String>::new();

        for (idx, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }

            let value: JsonValue = serde_json::from_str(line).map_err(|e| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Non-JSON line in generated output at line {}: {e}", idx + 1),
                    None,
                )
            })?;

            let obj = value.as_object().ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("JSONL line {} must be an object", idx + 1),
                    None,
                )
            })?;

            let op = obj.get("op").and_then(|v| v.as_str()).unwrap_or("");
            let path = obj.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if op.is_empty() || path.is_empty() {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("JSONL line {} must include 'op' and 'path'", idx + 1),
                    None,
                ));
            }

            if op == "add" && path == "/root" {
                saw_root = true;
            }

            if op == "add" && path.starts_with("/elements/") {
                if let Some(element_id) = path.strip_prefix("/elements/") {
                    element_ids.insert(element_id.to_string());
                    if element_ids.len() > MAX_ELEMENTS {
                        return Err(ErrorData::new(
                            ErrorCode::INVALID_PARAMS,
                            format!("Generated spec too large (>{MAX_ELEMENTS} elements)"),
                            None,
                        ));
                    }
                }

                if let Some(value_obj) = obj.get("value").and_then(|v| v.as_object()) {
                    if let Some(t) = value_obj.get("type").and_then(|v| v.as_str()) {
                        if !allowed.contains(t) {
                            return Err(ErrorData::new(
                                ErrorCode::INVALID_PARAMS,
                                format!("Unknown component type: {t}"),
                                None,
                            ));
                        }
                    }
                }
            }
        }

        if !saw_root {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Generated spec did not include an 'add' operation for /root".to_string(),
                None,
            ));
        }

        Ok(())
    }

    async fn sample_json_render(
        &self,
        params: &ComposeDashboardParams,
        context: &RequestContext<RoleServer>,
    ) -> Result<String, ErrorData> {
        let user_text = Self::build_user_message(params)?;
        let system_prompt = Self::system_prompt();

        let request_params = CreateMessageRequestParams {
            meta: None,
            task: None,
            messages: vec![SamplingMessage::new(
                rmcp::model::Role::User,
                SamplingMessageContent::text(user_text),
            )],
            model_preferences: None,
            system_prompt: Some(system_prompt),
            include_context: Some(rmcp::model::ContextInclusion::None),
            temperature: Some(0.1),
            max_tokens: 2400,
            stop_sequences: None,
            metadata: None,
            tools: None,
            tool_choice: None,
        };

        if !context.peer.supports_sampling_tools() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Client does not support MCP Sampling".to_string(),
                None,
            ));
        }

        let result = context
            .peer
            .create_message(request_params)
            .await
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Sampling request failed: {e}"),
                    None,
                )
            })?;

        let text = result
            .message
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.clone())
            .unwrap_or_default();

        Ok(text)
    }

    #[tool(
        name = "compose_dashboard",
        description = "Generate a DS-first json-render dashboard spec as JSONL RFC6902 patches only (no prose, no fences)."
    )]
    pub async fn compose_dashboard(
        &self,
        params: Parameters<ComposeDashboardParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let text = self.sample_json_render(&params.0, &context).await?;
        Self::validate_jsonl_patches(&text)?;
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for GenUiServiceServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-genui-service".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                title: Some("GenUI Service".to_string()),
                description: Some(
                    "Generate DS-first json-render specs using MCP Sampling (JSONL patches only)"
                        .to_string(),
                ),
                icons: None,
                website_url: None,
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_jsonl_requires_root() {
        let err = GenUiServiceServer::validate_jsonl_patches(
            r#"{"op":"add","path":"/elements/x","value":{}}"#,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("/root"));
    }

    #[test]
    fn validate_jsonl_rejects_prose() {
        let err = GenUiServiceServer::validate_jsonl_patches(
            "hello\n{\"op\":\"add\",\"path\":\"/root\",\"value\":\"main\"}",
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("Non-JSON line"));
    }

    #[test]
    fn validate_jsonl_accepts_minimal_root() {
        GenUiServiceServer::validate_jsonl_patches(r#"{"op":"add","path":"/root","value":"main"}"#)
            .unwrap();
    }
}
