use base64::{engine::general_purpose::STANDARD, Engine as _};
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::{formatdoc, indoc};
use serde_json::Value;
use std::{collections::HashMap, future::Future, path::PathBuf, pin::Pin, sync::Arc, sync::Mutex};
use tokio::sync::mpsc;

use mcp_core::{
    handler::{PromptError, ResourceError},
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{
    Content, ErrorCode, ErrorData, JsonRpcMessage, Prompt, Resource, ResourceContents, Role, Tool,
};
use rmcp::object;

/// Validates that the data parameter is a proper JSON value and not a string
fn validate_data_param(params: &Value, allow_array: bool) -> Result<Value, ErrorData> {
    let data_value = params.get("data").ok_or_else(|| {
        ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "Missing 'data' parameter".to_string(),
            None,
        )
    })?;

    if data_value.is_string() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "The 'data' parameter must be a JSON object, not a JSON string. Please provide valid JSON without comments.".to_string(),
            None,
        ));
    }

    if allow_array {
        if !data_value.is_object() && !data_value.is_array() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "The 'data' parameter must be a JSON object or array.".to_string(),
                None,
            ));
        }
    } else if !data_value.is_object() {
        return Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "The 'data' parameter must be a JSON object.".to_string(),
            None,
        ));
    }

    Ok(data_value.clone())
}

/// An extension for automatic data visualization and UI generation
#[derive(Clone)]
pub struct AutoVisualiserRouter {
    tools: Vec<Tool>,
    #[allow(dead_code)]
    cache_dir: PathBuf,
    active_resources: Arc<Mutex<HashMap<String, Resource>>>,
    instructions: String,
}

impl Default for AutoVisualiserRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoVisualiserRouter {
    fn create_sankey_tool() -> Tool {
        Tool::new(
            "render_sankey",
            indoc! {r#"
                show a Sankey diagram from flow data               
                The data must contain:
                - nodes: Array of objects with 'name' and optional 'category' properties
                - links: Array of objects with 'source', 'target', and 'value' properties
                
                Example:
                {
                  "nodes": [
                    {"name": "Source A", "category": "source"},
                    {"name": "Target B", "category": "target"}
                  ],
                  "links": [
                    {"source": "Source A", "target": "Target B", "value": 100}
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["nodes", "links"],
                        "properties": {
                            "nodes": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["name"],
                                    "properties": {
                                        "name": {"type": "string"},
                                        "category": {"type": "string"}
                                    }
                                }
                            },
                            "links": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["source", "target", "value"],
                                    "properties": {
                                        "source": {"type": "string"},
                                        "target": {"type": "string"},
                                        "value": {"type": "number"}
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
    }

    fn create_radar_tool() -> Tool {
        Tool::new(
            "render_radar",
            indoc! {r#"
                show a radar chart (spider chart) for multi-dimensional data comparison             
                
                The data must contain:
                - labels: Array of strings representing the dimensions/axes
                - datasets: Array of dataset objects with 'label' and 'data' properties
                
                Example:
                {
                  "labels": ["Speed", "Strength", "Endurance", "Agility", "Intelligence"],
                  "datasets": [
                    {
                      "label": "Player 1",
                      "data": [85, 70, 90, 75, 80]
                    },
                    {
                      "label": "Player 2", 
                      "data": [75, 85, 80, 90, 70]
                    }
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["labels", "datasets"],
                        "properties": {
                            "labels": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                }
                            },
                            "datasets": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["label", "data"],
                                    "properties": {
                                        "label": {"type": "string"},
                                        "data": {
                                            "type": "array",
                                            "items": {"type": "number"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        )
    }

    fn create_donut_tool() -> Tool {
        Tool::new(
            "render_donut",
            indoc! {r#"
                show pie or donut charts for categorical data visualization
                Supports single or multiple charts in a grid layout.
                
                Each chart should contain:
                - data: Array of values or objects with 'label' and 'value'
                - type: Optional 'doughnut' (default) or 'pie'
                - title: Optional chart title
                - labels: Optional array of labels (if data is just numbers)
                
                Example single chart:
                {
                  "title": "Budget",
                  "type": "doughnut",
                  "data": [
                    {"label": "Marketing", "value": 25000},
                    {"label": "Development", "value": 35000}
                  ]
                }
                
                Example multiple charts:
                [{
                  "title": "Q1 Sales",
                  "labels": ["Product A", "Product B"],
                  "data": [45000, 38000]
                }]
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "type": {"type": "string", "enum": ["doughnut", "pie"]},
                                    "labels": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    },
                                    "data": {
                                        "type": "array",
                                        "items": {
                                            "oneOf": [
                                                {"type": "number"},
                                                {
                                                    "type": "object",
                                                    "required": ["label", "value"],
                                                    "properties": {
                                                        "label": {"type": "string"},
                                                        "value": {"type": "number"}
                                                    }
                                                }
                                            ]
                                        }
                                    }
                                },
                                "required": ["data"]
                            },
                            {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "title": {"type": "string"},
                                        "type": {"type": "string", "enum": ["doughnut", "pie"]},
                                        "labels": {
                                            "type": "array",
                                            "items": {"type": "string"}
                                        },
                                        "data": {
                                            "type": "array",
                                            "items": {
                                                "oneOf": [
                                                    {"type": "number"},
                                                    {
                                                        "type": "object",
                                                        "required": ["label", "value"],
                                                        "properties": {
                                                            "label": {"type": "string"},
                                                            "value": {"type": "number"}
                                                        }
                                                    }
                                                ]
                                            }
                                        }
                                    },
                                    "required": ["data"]
                                }
                            }
                        ]
                    }
                }
            }),
        )
    }

    fn create_treemap_tool() -> Tool {
        Tool::new(
            "render_treemap",
            indoc! {r#"
                show a treemap visualization for hierarchical data with proportional area representation as boxes
                
                The data should be a hierarchical structure with:
                - name: Name of the node (required)
                - value: Numeric value for leaf nodes (optional for parent nodes)
                - children: Array of child nodes (optional)
                - category: Category for coloring (optional)
                
                Example:
                {
                  "name": "Root",
                  "children": [
                    {
                      "name": "Group A",
                      "children": [
                        {"name": "Item 1", "value": 100, "category": "Type1"},
                        {"name": "Item 2", "value": 200, "category": "Type2"}
                      ]
                    },
                    {"name": "Item 3", "value": 150, "category": "Type1"}
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": {"type": "string"},
                            "value": {"type": "number"},
                            "category": {"type": "string"},
                            "children": {
                                "type": "array",
                                "items": {
                                    "$ref": "#/properties/data"
                                }
                            }
                        }
                    }
                }
            }),
        )
    }

    fn create_chord_tool() -> Tool {
        Tool::new(
            "render_chord",
            indoc! {r#"
                Show a chord diagram visualization for showing relationships and flows between entities.
                
                The data must contain:
                - labels: Array of strings representing the entities
                - matrix: 2D array of numbers representing flows (matrix[i][j] = flow from i to j)
                
                Example:
                {
                  "labels": ["North America", "Europe", "Asia", "Africa"],
                  "matrix": [
                    [0, 15, 25, 8],
                    [18, 0, 20, 12],
                    [22, 18, 0, 15],
                    [5, 10, 18, 0]
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["labels", "matrix"],
                        "properties": {
                            "labels": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "matrix": {
                                "type": "array",
                                "items": {
                                    "type": "array",
                                    "items": {"type": "number"}
                                }
                            }
                        }
                    }
                }
            }),
        )
    }

    fn create_map_tool() -> Tool {
        Tool::new(
            "render_map",
            indoc! {r#"
                show an interactive map visualization with location markers using Leaflet.
                
                The data must contain:
                - markers: Array of objects with 'lat', 'lng', and optional properties
                - title: Optional title for the map (default: "Interactive Map")
                - subtitle: Optional subtitle (default: "Geographic data visualization")
                - center: Optional center point {lat, lng} (default: USA center)
                - zoom: Optional initial zoom level (default: 4)
                - clustering: Optional boolean to enable/disable clustering (default: true)
                - autoFit: Optional boolean to auto-fit map to markers (default: true)
                
                Marker properties:
                - lat: Latitude (required)
                - lng: Longitude (required)
                - name: Location name
                - value: Numeric value for sizing/coloring
                - description: Description text
                - popup: Custom popup HTML
                - color: Custom marker color
                - label: Custom marker label
                - useDefaultIcon: Use default Leaflet icon
                
                Example:
                {
                  "title": "Store Locations",
                  "markers": [
                    {"lat": 37.7749, "lng": -122.4194, "name": "SF Store", "value": 150000},
                    {"lat": 40.7128, "lng": -74.0060, "name": "NYC Store", "value": 200000}
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["markers"],
                        "properties": {
                            "markers": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["lat", "lng"],
                                    "properties": {
                                        "lat": {"type": "number"},
                                        "lng": {"type": "number"},
                                        "name": {"type": "string"},
                                        "value": {"type": "number"},
                                        "description": {"type": "string"},
                                        "popup": {"type": "string"},
                                        "color": {"type": "string"},
                                        "label": {"type": "string"},
                                        "useDefaultIcon": {"type": "boolean"}
                                    }
                                }
                            },
                            "title": {"type": "string"},
                            "subtitle": {"type": "string"},
                            "center": {
                                "type": "object",
                                "properties": {
                                    "lat": {"type": "number"},
                                    "lng": {"type": "number"}
                                }
                            },
                            "zoom": {"type": "number"},
                            "clustering": {"type": "boolean"},
                            "clusterRadius": {"type": "number"},
                            "autoFit": {"type": "boolean"}
                        }
                    }
                }
            }),
        )
    }

    fn create_show_chart_tool() -> Tool {
        Tool::new(
            "show_chart",
            indoc! {r#"
                show interactive line, scatter, or bar charts
                
                Required: type ('line', 'scatter', or 'bar'), datasets array
                Optional: labels, title, subtitle, xAxisLabel, yAxisLabel, options
                
                Example:
                {
                  "type": "line",
                  "title": "Monthly Sales",
                  "labels": ["Jan", "Feb", "Mar"],
                  "datasets": [
                    {"label": "Product A", "data": [65, 59, 80]}
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["type", "datasets"],
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["line", "scatter", "bar"]
                            },
                            "title": {"type": "string"},
                            "subtitle": {"type": "string"},
                            "xAxisLabel": {"type": "string"},
                            "yAxisLabel": {"type": "string"},
                            "labels": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "datasets": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["data"],
                                    "properties": {
                                        "label": {"type": "string"},
                                        "data": {
                                            "oneOf": [
                                                {
                                                    "type": "array",
                                                    "items": {"type": "number"}
                                                },
                                                {
                                                    "type": "array",
                                                    "items": {
                                                        "type": "object",
                                                        "required": ["x", "y"],
                                                        "properties": {
                                                            "x": {"type": "number"},
                                                            "y": {"type": "number"}
                                                        }
                                                    }
                                                }
                                            ]
                                        },
                                        "backgroundColor": {"type": "string"},
                                        "borderColor": {"type": "string"},
                                        "borderWidth": {"type": "number"},
                                        "tension": {"type": "number"},
                                        "fill": {"type": "boolean"}
                                    }
                                }
                            },
                            "options": {"type": "object"}
                        }
                    }
                }
            }),
        )
    }

    pub fn new() -> Self {
        let render_sankey_tool = Self::create_sankey_tool();
        let render_radar_tool = Self::create_radar_tool();
        let render_donut_tool = Self::create_donut_tool();
        let render_treemap_tool = Self::create_treemap_tool();
        let render_chord_tool = Self::create_chord_tool();
        let render_map_tool = Self::create_map_tool();
        let show_chart_tool = Self::create_show_chart_tool();

        // choose_app_strategy().cache_dir()
        // - macOS/Linux: ~/.cache/goose/autovisualiser/
        // - Windows:     ~\AppData\Local\Block\goose\cache\autovisualiser\
        let cache_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .unwrap()
            .cache_dir()
            .join("autovisualiser");

        // Create cache directory if it doesn't exist
        let _ = std::fs::create_dir_all(&cache_dir);

        let instructions = formatdoc! {r#"
            This extension provides tools for automatic data visualization
            Use these tools when you are presenting data to the user which could be complemented by a visual expression
            Choose the most appropriate chart type based on the data you have and can provide
            It is important you match the data format as appropriate with the chart type you have chosen
            The user may specify a type of chart or you can pick one of the most appopriate that you can shape the data to

            ## Available Tools:
            - **render_sankey**: Creates interactive Sankey diagrams from flow data
            - **render_radar**: Creates interactive radar charts for multi-dimensional data comparison
            - **render_donut**: Creates interactive donut/pie charts for categorical data (supports multiple charts)
            - **render_treemap**: Creates interactive treemap visualizations for hierarchical data
            - **render_chord**: Creates interactive chord diagrams for relationship/flow visualization
            - **render_map**: Creates interactive map visualizations with location markers
            - **show_chart**: Creates interactive line, scatter, or bar charts for data visualization
        "#};

        Self {
            tools: vec![
                render_sankey_tool,
                render_radar_tool,
                render_donut_tool,
                render_treemap_tool,
                render_chord_tool,
                render_map_tool,
                show_chart_tool,
            ],
            cache_dir,
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            instructions,
        }
    }

    async fn render_sankey(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/sankey_template.html");
        const D3_MIN: &str = include_str!("templates/assets/d3.min.js");
        const D3_SANKEY: &str = include_str!("templates/assets/d3.sankey.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{D3_MIN}}", D3_MIN)
            .replace("{{D3_SANKY}}", D3_SANKEY) // Note: keeping the typo to match template
            .replace("{{SANKEY_DATA}}", &data_json);

        // Save to /tmp/vis.html for debugging
        let debug_path = std::path::Path::new("/tmp/vis.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/vis.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/vis.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://sankey/diagram".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn render_radar(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/radar_template.html");
        const CHART_MIN: &str = include_str!("templates/assets/chart.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{CHART_MIN}}", CHART_MIN)
            .replace("{{RADAR_DATA}}", &data_json);

        // Save to /tmp/radar.html for debugging
        let debug_path = std::path::Path::new("/tmp/radar.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/radar.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/radar.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://radar/chart".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn render_treemap(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/treemap_template.html");
        const D3_MIN: &str = include_str!("templates/assets/d3.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{D3_MIN}}", D3_MIN)
            .replace("{{TREEMAP_DATA}}", &data_json);

        // Save to /tmp/treemap.html for debugging
        let debug_path = std::path::Path::new("/tmp/treemap.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/treemap.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/treemap.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://treemap/visualization".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn render_chord(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/chord_template.html");
        const D3_MIN: &str = include_str!("templates/assets/d3.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{D3_MIN}}", D3_MIN)
            .replace("{{CHORD_DATA}}", &data_json);

        // Save to /tmp/chord.html for debugging
        let debug_path = std::path::Path::new("/tmp/chord.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/chord.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/chord.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://chord/diagram".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn render_donut(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, true)?; // true because donut accepts arrays

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/donut_template.html");
        const CHART_MIN: &str = include_str!("templates/assets/chart.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{CHART_MIN}}", CHART_MIN)
            .replace("{{CHARTS_DATA}}", &data_json);

        // Save to /tmp/donut.html for debugging
        let debug_path = std::path::Path::new("/tmp/donut.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/donut.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/donut.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://donut/chart".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn render_map(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Extract title and subtitle from data if provided
        let title = data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Interactive Map");
        let subtitle = data
            .get("subtitle")
            .and_then(|v| v.as_str())
            .unwrap_or("Geographic data visualization");

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/map_template.html");
        const LEAFLET_JS: &str = include_str!("templates/assets/leaflet.min.js");
        const LEAFLET_CSS: &str = include_str!("templates/assets/leaflet.min.css");
        const MARKERCLUSTER_JS: &str =
            include_str!("templates/assets/leaflet.markercluster.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{LEAFLET_JS}}", LEAFLET_JS)
            .replace("{{LEAFLET_CSS}}", LEAFLET_CSS)
            .replace("{{MARKERCLUSTER_JS}}", MARKERCLUSTER_JS)
            .replace("{{MAP_DATA}}", &data_json)
            .replace("{{TITLE}}", title)
            .replace("{{SUBTITLE}}", subtitle);

        // Save to /tmp/map.html for debugging
        let debug_path = std::path::Path::new("/tmp/map.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/map.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/map.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://map/visualization".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }

    async fn show_chart(&self, params: Value) -> Result<Vec<Content>, ErrorData> {
        let data = validate_data_param(&params, false)?;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data).map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Invalid JSON data: {}", e),
                None,
            )
        })?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("templates/chart_template.html");
        const CHART_MIN: &str = include_str!("templates/assets/chart.min.js");

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{CHART_MIN}}", CHART_MIN)
            .replace("{{CHART_DATA}}", &data_json);

        // Save to /tmp/chart.html for debugging
        let debug_path = std::path::Path::new("/tmp/chart.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/chart.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/chart.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://chart/interactive".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
        };

        Ok(vec![
            Content::resource(resource_contents).with_audience(vec![Role::User])
        ])
    }
}

impl Router for AutoVisualiserRouter {
    fn name(&self) -> String {
        "AutoVisualiserExtension".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ErrorData>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "render_sankey" => this.render_sankey(arguments).await,
                "render_radar" => this.render_radar(arguments).await,
                "render_donut" => this.render_donut(arguments).await,
                "render_treemap" => this.render_treemap(arguments).await,
                "render_chord" => this.render_chord(arguments).await,
                "render_map" => this.render_map(arguments).await,
                "show_chart" => this.show_chart(arguments).await,
                _ => Err(ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Tool {} not found", tool_name),
                    None,
                )),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        let active_resources = self.active_resources.lock().unwrap();
        let resources = active_resources.values().cloned().collect();
        tracing::info!("Listing resources: {:?}", resources);
        resources
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!(
                "Resource not found: {}",
                uri
            )))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::RawContent;
    use serde_json::json;

    #[test]
    fn test_validate_data_param_rejects_string() {
        // Test that a string value for data is rejected
        let params = json!({
            "data": "{\"labels\": [\"A\", \"B\"], \"matrix\": [[0, 1], [1, 0]]}"
        });

        let result = validate_data_param(&params, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err
            .message
            .contains("must be a JSON object, not a JSON string"));
        assert!(err.message.contains("without comments"));
    }

    #[test]
    fn test_validate_data_param_accepts_object() {
        // Test that a proper object is accepted
        let params = json!({
            "data": {
                "labels": ["A", "B"],
                "matrix": [[0, 1], [1, 0]]
            }
        });

        let result = validate_data_param(&params, false);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.is_object());
        assert_eq!(data["labels"][0], "A");
    }

    #[test]
    fn test_validate_data_param_rejects_array_when_not_allowed() {
        // Test that an array is rejected when allow_array is false
        let params = json!({
            "data": [
                {"label": "A", "value": 10},
                {"label": "B", "value": 20}
            ]
        });

        let result = validate_data_param(&params, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("must be a JSON object"));
    }

    #[test]
    fn test_validate_data_param_accepts_array_when_allowed() {
        // Test that an array is accepted when allow_array is true
        let params = json!({
            "data": [
                {"label": "A", "value": 10},
                {"label": "B", "value": 20}
            ]
        });

        let result = validate_data_param(&params, true);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.is_array());
        assert_eq!(data[0]["label"], "A");
    }

    #[test]
    fn test_validate_data_param_missing_data() {
        // Test that missing data parameter is rejected
        let params = json!({
            "other": "value"
        });

        let result = validate_data_param(&params, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("Missing 'data' parameter"));
    }

    #[test]
    fn test_validate_data_param_rejects_primitive_values() {
        // Test that primitive values (number, boolean) are rejected
        let params_number = json!({
            "data": 42
        });

        let result = validate_data_param(&params_number, false);
        assert!(result.is_err());

        let params_bool = json!({
            "data": true
        });

        let result = validate_data_param(&params_bool, false);
        assert!(result.is_err());

        let params_null = json!({
            "data": null
        });

        let result = validate_data_param(&params_null, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_data_param_with_json_containing_comments_as_string() {
        // Test that JSON with comments passed as a string is rejected
        let params = json!({
            "data": r#"{
                "labels": ["A", "B"],
                "matrix": [
                    [0, 1],  // This is a comment
                    [1, 0]   /* Another comment */
                ]
            }"#
        });

        let result = validate_data_param(&params, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("not a JSON string"));
        assert!(err.message.contains("without comments"));
    }

    #[tokio::test]
    async fn test_render_sankey() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "nodes": [{"name": "A"}, {"name": "B"}],
                "links": [{"source": "A", "target": "B", "value": 10}]
            }
        });

        let result = router.render_sankey(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);

        // Check it's a resource with HTML content
        // Content is Annotated<RawContent>, deref to get RawContent
        if let RawContent::Resource(resource) = &**&content[0] {
            if let ResourceContents::BlobResourceContents { uri, mime_type, .. } =
                &resource.resource
            {
                assert_eq!(uri, "ui://sankey/diagram");
                assert_eq!(mime_type.as_ref().unwrap(), "text/html");
            } else {
                panic!("Expected BlobResourceContents");
            }
        } else {
            panic!("Expected Resource content");
        }
    }

    #[tokio::test]
    async fn test_render_radar() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "categories": ["Speed", "Power", "Agility"],
                "series": [
                    {"label": "Player 1", "data": [80, 90, 85]}
                ]
            }
        });

        let result = router.render_radar(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);

        // Check it's a resource with HTML content
        // Content is Annotated<RawContent>, deref to get RawContent
        if let RawContent::Resource(resource) = &**&content[0] {
            if let ResourceContents::BlobResourceContents {
                uri,
                mime_type,
                blob,
            } = &resource.resource
            {
                assert_eq!(uri, "ui://radar/chart");
                assert_eq!(mime_type.as_ref().unwrap(), "text/html");
                assert!(!blob.is_empty(), "HTML content should not be empty");
            } else {
                panic!("Expected BlobResourceContents");
            }
        } else {
            panic!("Expected Resource content");
        }
    }

    #[tokio::test]
    async fn test_render_donut() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "labels": ["A", "B", "C"],
                "values": [30, 40, 30]
            }
        });

        let result = router.render_donut(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_treemap() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "name": "root",
                "children": [
                    {"name": "A", "value": 100},
                    {"name": "B", "value": 200}
                ]
            }
        });

        let result = router.render_treemap(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_chord() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "labels": ["A", "B", "C"],
                "matrix": [[0, 10, 5], [10, 0, 15], [5, 15, 0]]
            }
        });

        let result = router.render_chord(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_map() {
        let router = AutoVisualiserRouter::new();
        let params = json!({
            "data": {
                "features": [
                    {
                        "type": "Feature",
                        "geometry": {"type": "Point", "coordinates": [0, 0]},
                        "properties": {"name": "Origin"}
                    }
                ]
            }
        });

        let result = router.render_map(params).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_show_chart() {
        let router = AutoVisualiserRouter::new();
        // show_chart expects data to be an object, not an array
        let params = json!({
            "data": {
                "datasets": [
                    {
                        "label": "Test Data",
                        "data": [
                            {"x": 1, "y": 2},
                            {"x": 2, "y": 4}
                        ]
                    }
                ]
            }
        });

        let result = router.show_chart(params).await;
        if let Err(e) = &result {
            eprintln!("Error in test_show_chart: {:?}", e);
        }
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }
}
