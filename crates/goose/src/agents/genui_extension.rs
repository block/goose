use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion, ServerCapabilities, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;

pub const EXTENSION_NAME: &str = "genui";

const KNOWN_COMPONENTS: &[(&str, &str)] = &[
    ("Card", "Container with optional header/footer"),
    ("Stack", "Vertical/horizontal flex layout (gap: sm|md|lg)"),
    ("Grid", "CSS grid layout (columns: 1-6)"),
    ("Separator", "Visual divider"),
    ("Tabs", "Tabbed content panels"),
    ("Accordion", "Collapsible sections"),
    ("Collapsible", "Single collapsible section"),
    ("Carousel", "Horizontal carousel"),
    ("Pagination", "Page navigation"),
    ("Table", "Data table (columns, rows)"),
    ("Heading", "Text heading (level: 1-6)"),
    ("Text", "Body text"),
    ("Image", "Image display (src, alt)"),
    ("Avatar", "User/entity avatar"),
    (
        "Badge",
        "Status badge (variant: default|secondary|destructive|outline)",
    ),
    ("Alert", "Alert (variant: default|destructive)"),
    ("Progress", "Progress bar (value: 0-100)"),
    ("Skeleton", "Loading placeholder"),
    ("Spinner", "Loading spinner"),
    ("Tooltip", "Hover tooltip"),
    ("Popover", "Click popover"),
    ("Dialog", "Modal dialog"),
    ("Drawer", "Side drawer"),
    ("DropdownMenu", "Dropdown menu"),
    ("Input", "Text input"),
    ("Textarea", "Multi-line text input"),
    ("Select", "Dropdown select"),
    ("Checkbox", "Checkbox toggle"),
    ("Switch", "Toggle switch"),
    ("Slider", "Range slider"),
    (
        "Button",
        "Button (variant: default|secondary|outline|ghost|destructive)",
    ),
    ("Toggle", "Toggle button"),
    ("Link", "Hyperlink"),
    ("ButtonGroup", "Group of buttons"),
    ("ToggleGroup", "Group of toggles"),
    ("Radio", "Radio button group"),
];

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
struct RenderParams {
    /// The JSON UI spec to render inline in chat. Must have a "root" object with "type", "props", and optional "children".
    spec: JsonValue,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ComponentsParams {}

pub struct GenUiClient {
    info: InitializeResult,
}

impl GenUiClient {
    pub fn new(_context: PlatformExtensionContext) -> anyhow::Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "genui".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            instructions: Some(
                "Render visual components inline in chat using Radix UI. \
                 Use genui__render to display tables, cards, dashboards, and data visualizations. \
                 Use genui__components to see available components."
                    .to_string(),
            ),
        };

        Ok(Self { info })
    }

    fn validate_spec(spec: &JsonValue) -> Result<(), String> {
        let root = spec.get("root").ok_or("Spec must have a \"root\" object")?;
        Self::validate_element(root)
    }

    fn validate_element(element: &JsonValue) -> Result<(), String> {
        let type_name = element
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("Each element must have a \"type\" string")?;

        if !KNOWN_COMPONENTS.iter().any(|(name, _)| *name == type_name) {
            return Err(format!(
                "Unknown component \"{type_name}\". Use genui__components to see available components."
            ));
        }

        if let Some(children) = element.get("children") {
            if let Some(arr) = children.as_array() {
                for child in arr {
                    Self::validate_element(child)?;
                }
            }
        }

        Ok(())
    }

    fn handle_render(&self, arguments: Option<JsonObject>) -> Result<Vec<Content>, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let spec = args
            .get("spec")
            .ok_or("Missing required parameter: spec")?
            .clone();

        Self::validate_spec(&spec)?;

        let spec_str = serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())?;

        Ok(vec![Content::text(format!(
            "Visual component rendered successfully.\n\n```json-render\n{spec_str}\n```"
        ))])
    }

    fn handle_components(&self) -> Vec<Content> {
        let mut output = String::from("# Available genUI Components\n\n");

        let categories: &[(&str, &[&str])] = &[
            ("Layout", &["Card", "Stack", "Grid", "Separator"]),
            (
                "Navigation",
                &["Tabs", "Accordion", "Collapsible", "Carousel", "Pagination"],
            ),
            (
                "Overlay",
                &["Dialog", "Drawer", "Popover", "Tooltip", "DropdownMenu"],
            ),
            (
                "Content",
                &[
                    "Heading", "Text", "Image", "Avatar", "Badge", "Alert", "Table",
                ],
            ),
            ("Feedback", &["Progress", "Skeleton", "Spinner"]),
            (
                "Input",
                &[
                    "Input", "Textarea", "Select", "Checkbox", "Radio", "Switch", "Slider",
                ],
            ),
            (
                "Actions",
                &["Button", "Toggle", "Link", "ButtonGroup", "ToggleGroup"],
            ),
        ];

        for (category, names) in categories {
            output.push_str(&format!("## {category}\n"));
            for name in *names {
                if let Some((_, desc)) = KNOWN_COMPONENTS.iter().find(|(n, _)| *n == *name) {
                    output.push_str(&format!("- **{name}** — {desc}\n"));
                }
            }
            output.push('\n');
        }

        output.push_str("## Spec Format\n```json\n");
        output.push_str(
            r#"{ "root": { "type": "Card", "props": { "className": "p-4" }, "children": [...] } }"#,
        );
        output.push_str("\n```\n");

        vec![Content::text(output)]
    }

    fn tools() -> Vec<Tool> {
        let render_schema = serde_json::to_value(schema_for!(RenderParams))
            .expect("Failed to serialize RenderParams schema");
        let components_schema = serde_json::to_value(schema_for!(ComponentsParams))
            .expect("Failed to serialize ComponentsParams schema");

        vec![
            Tool::new(
                "render".to_string(),
                "Render visual UI components inline in chat. Pass a JSON spec with a root \
                 element tree. Components are validated and rendered as interactive Radix UI. \
                 Use for tables, cards, dashboards, and data visualizations."
                    .to_string(),
                render_schema.as_object().unwrap().clone(),
            )
            .annotate(ToolAnnotations {
                title: Some("Render Visual Component".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
            Tool::new(
                "components".to_string(),
                "List all available UI components with their props and usage.".to_string(),
                components_schema.as_object().unwrap().clone(),
            )
            .annotate(ToolAnnotations {
                title: Some("List Available Components".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(true),
                open_world_hint: Some(false),
            }),
        ]
    }
}

#[async_trait]
impl McpClientTrait for GenUiClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        _working_dir: Option<&str>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        match name {
            "render" => match self.handle_render(arguments) {
                Ok(content) => Ok(CallToolResult::success(content)),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Error: {e}"
                ))])),
            },
            "components" => Ok(CallToolResult::success(self.handle_components())),
            _ => Ok(CallToolResult::error(vec![Content::text(format!(
                "Unknown tool: {name}"
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
