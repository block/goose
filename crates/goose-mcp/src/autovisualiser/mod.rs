use base64::{engine::general_purpose::STANDARD, Engine as _};
use etcetera::{choose_app_strategy, AppStrategy};
use indoc::formatdoc;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{
        CallToolResult, Content, ErrorCode, ErrorData, Implementation, Role, ServerCapabilities,
        ServerInfo,
    },
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

// Sankey diagram structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SankeyNode {
    /// Name of the node (required)
    pub name: String,
    /// Optional category for grouping/coloring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SankeyLink {
    /// Source node name
    pub source: String,
    /// Target node name
    pub target: String,
    /// Flow value between nodes
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SankeyData {
    /// Array of nodes in the diagram
    pub nodes: Vec<SankeyNode>,
    /// Array of links between nodes
    pub links: Vec<SankeyLink>,
}

/// Parameters for the render_sankey tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderSankeyParams {
    /// Sankey diagram data containing nodes and links
    pub data: SankeyData,
}

// Radar chart structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RadarDataset {
    /// Label for this dataset
    pub label: String,
    /// Data values for each dimension
    pub data: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RadarData {
    /// Array of labels for each dimension/axis
    pub labels: Vec<String>,
    /// Array of datasets to compare
    pub datasets: Vec<RadarDataset>,
}

/// Parameters for the render_radar tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderRadarParams {
    /// Radar chart data containing labels and datasets
    pub data: RadarData,
}

// Donut/Pie chart structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DonutDataItem {
    /// Simple numeric value
    Number(f64),
    /// Labeled value object
    LabeledValue { label: String, value: f64 },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Doughnut,
    Pie,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DonutChart {
    /// Optional chart title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Chart type - 'doughnut' (default) or 'pie'
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub chart_type: Option<ChartType>,
    /// Optional array of labels (if data is just numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Array of values or labeled value objects
    pub data: Vec<DonutDataItem>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DonutData {
    /// Single chart
    Single(DonutChart),
    /// Multiple charts for grid layout
    Multiple(Vec<DonutChart>),
}

/// Parameters for the render_donut tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderDonutParams {
    /// Donut/pie chart data - can be single chart or array of charts
    pub data: DonutData,
}

// Treemap structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TreemapNode {
    /// Name of the node (required)
    pub name: String,
    /// Numeric value for leaf nodes (optional for parent nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Category for coloring (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Array of child nodes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreemapNode>>,
}

/// Parameters for the render_treemap tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderTreemapParams {
    /// Treemap data with hierarchical structure
    pub data: TreemapNode,
}

// Chord diagram structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChordData {
    /// Array of strings representing the entities
    pub labels: Vec<String>,
    /// 2D array of numbers representing flows (matrix[i][j] = flow from i to j)
    pub matrix: Vec<Vec<f64>>,
}

/// Parameters for the render_chord tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderChordParams {
    /// Chord diagram data containing labels and matrix
    pub data: ChordData,
}

// Map structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MapMarker {
    /// Latitude (required)
    pub lat: f64,
    /// Longitude (required)
    pub lng: f64,
    /// Location name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Numeric value for sizing/coloring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Description text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Custom popup HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popup: Option<String>,
    /// Custom marker color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Custom marker label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Use default Leaflet icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_default_icon: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MapCenter {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MapData {
    /// Array of location markers
    pub markers: Vec<MapMarker>,
    /// Optional title for the map (default: "Interactive Map")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional subtitle (default: "Geographic data visualization")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Optional center point {lat, lng} (default: USA center)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<MapCenter>,
    /// Optional initial zoom level (default: 4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoom: Option<u8>,
    /// Optional boolean to enable/disable clustering (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clustering: Option<bool>,
    /// Optional boolean to auto-fit map to markers (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_fit: Option<bool>,
}

/// Parameters for the render_map tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenderMapParams {
    /// Map data containing markers and optional configuration
    pub data: MapData,
}

// Chart structures
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ChartDataType {
    Line,
    Scatter,
    Bar,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ChartDataPoint {
    /// Simple numeric value
    Number(f64),
    /// X,Y coordinate object
    Coordinate { x: f64, y: f64 },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChartDataset {
    /// Label for this dataset
    pub label: String,
    /// Data points for the chart
    pub data: Vec<ChartDataPoint>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChartData {
    /// Chart type: 'line', 'scatter', or 'bar'
    #[serde(rename = "type")]
    pub chart_type: ChartDataType,
    /// Array of datasets to display
    pub datasets: Vec<ChartDataset>,
    /// Optional array of labels for x-axis (mainly for bar charts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Optional chart title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional chart subtitle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Optional x-axis label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis_label: Option<String>,
    /// Optional y-axis label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis_label: Option<String>,
    /// Optional chart configuration options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
}

/// Parameters for the show_chart tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ShowChartParams {
    /// Chart data containing type, datasets, and optional configuration
    pub data: ChartData,
}

#[derive(Debug)]
pub struct AutoVisualiserServer {
    tool_router: ToolRouter<Self>,
    instructions: String,
    cache_dir: PathBuf,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AutoVisualiserServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "goose-autovisualiser".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(self.instructions.clone()),
            ..Default::default()
        }
    }
}

impl Default for AutoVisualiserServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl AutoVisualiserServer {
    pub fn new() -> Self {
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
            tool_router: Self::tool_router(),
            instructions,
            cache_dir,
        }
    }

    /// Validates that the data parameter is a proper JSON value and not a string
    #[cfg(test)]
    fn validate_data_param(data: &Value, allow_array: bool) -> Result<Value, ErrorData> {
        if data.is_string() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "The 'data' parameter must be a JSON object, not a JSON string. Please provide valid JSON without comments.".to_string(),
                None,
            ));
        }

        if allow_array {
            if !data.is_object() && !data.is_array() {
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    "The 'data' parameter must be a JSON object or array.".to_string(),
                    None,
                ));
            }
        } else if !data.is_object() {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "The 'data' parameter must be a JSON object.".to_string(),
                None,
            ));
        }

        Ok(data.clone())
    }

    /// Show a Sankey diagram from flow data
    #[tool(
        name = "render_sankey",
        description = "show a Sankey diagram from flow data. The data must contain:
nodes (Array of objects with 'name' and optional 'category' properties) and 
links (Array of objects with 'source', 'target', and 'value' properties)"
    )]
    pub async fn render_sankey(
        &self,
        params: Parameters<RenderSankeyParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://sankey/diagram".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show a radar chart (spider chart) for multi-dimensional data comparison
    #[tool(
        name = "render_radar",
        description = "show a radar chart (spider chart) for multi-dimensional data comparison. The data must contain:
labels (Array of strings representing the dimensions/axes) and 
datasets (Array of dataset objects with 'label' and 'data' properties)"
    )]
    pub async fn render_radar(
        &self,
        params: Parameters<RenderRadarParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://radar/chart".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show pie or donut charts for categorical data visualization
    #[tool(
        name = "render_donut",
        description = "show pie or donut charts for categorical data visualization. Supports single or multiple charts in a grid layout. Each chart should contain:
data (Array of values or objects with 'label' and 'value'),
type (Optional 'doughnut' or 'pie'),
title (Optional chart title),
labels (Optional array of labels if data is just numbers)"
    )]
    pub async fn render_donut(
        &self,
        params: Parameters<RenderDonutParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://donut/chart".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show a treemap visualization for hierarchical data with proportional area representation as boxes
    #[tool(
        name = "render_treemap",
        description = "show a treemap visualization for hierarchical data with proportional area representation as boxes. The data should be a hierarchical structure with:
name (Name of the node, required),
value (Numeric value for leaf nodes, optional for parent nodes),
children (Array of child nodes, optional),
category (Category for coloring, optional)"
    )]
    pub async fn render_treemap(
        &self,
        params: Parameters<RenderTreemapParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://treemap/visualization".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show a chord diagram visualization for showing relationships and flows between entities
    #[tool(
        name = "render_chord",
        description = "Show a chord diagram visualization for showing relationships and flows between entities. The data must contain:
labels (Array of strings representing the entities) and 
matrix (2D array of numbers representing flows, matrix[i][j] = flow from i to j)"
    )]
    pub async fn render_chord(
        &self,
        params: Parameters<RenderChordParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://chord/diagram".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show an interactive map visualization with location markers using Leaflet
    #[tool(
        name = "render_map",
        description = "show an interactive map visualization with location markers using Leaflet. The data must contain:
markers (Array of objects with 'lat', 'lng', and optional properties),
title (Optional title for the map),
subtitle (Optional subtitle),
center (Optional center point {lat, lng}),
zoom (Optional initial zoom level),
clustering (Optional boolean to enable/disable clustering),
autoFit (Optional boolean to auto-fit map to markers)"
    )]
    pub async fn render_map(
        &self,
        params: Parameters<RenderMapParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Extract title and subtitle from data if provided
        let title = params.data.title.as_deref().unwrap_or("Interactive Map");
        let subtitle = params
            .data
            .subtitle
            .as_deref()
            .unwrap_or("Geographic data visualization");

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://map/visualization".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }

    /// Show interactive line, scatter, or bar charts
    #[tool(
        name = "show_chart",
        description = "show interactive line, scatter, or bar charts.
Required: type ('line', 'scatter', or 'bar'), datasets array.
Optional: labels, title, subtitle, xAxisLabel, yAxisLabel, options"
    )]
    pub async fn show_chart(
        &self,
        params: Parameters<ShowChartParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&params.data).map_err(|e| {
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

        let resource_contents = rmcp::model::ResourceContents::BlobResourceContents {
            uri: "ui://chart/interactive".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
            meta: None,
        };

        Ok(CallToolResult::success(vec![Content::resource(
            resource_contents,
        )
        .with_audience(vec![Role::User])]))
    }
}

impl Clone for AutoVisualiserServer {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            instructions: self.instructions.clone(),
            cache_dir: self.cache_dir.clone(),
        }
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
        let data = json!("{\"labels\": [\"A\", \"B\"], \"matrix\": [[0, 1], [1, 0]]}");

        let result = AutoVisualiserServer::validate_data_param(&data, false);
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
        let data = json!({
            "labels": ["A", "B"],
            "matrix": [[0, 1], [1, 0]]
        });

        let result = AutoVisualiserServer::validate_data_param(&data, false);
        assert!(result.is_ok());

        let validated_data = result.unwrap();
        assert!(validated_data.is_object());
        assert_eq!(validated_data["labels"][0], "A");
    }

    #[test]
    fn test_validate_data_param_rejects_array_when_not_allowed() {
        // Test that an array is rejected when allow_array is false
        let data = json!([
            {"label": "A", "value": 10},
            {"label": "B", "value": 20}
        ]);

        let result = AutoVisualiserServer::validate_data_param(&data, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("must be a JSON object"));
    }

    #[test]
    fn test_validate_data_param_accepts_array_when_allowed() {
        // Test that an array is accepted when allow_array is true
        let data = json!([
            {"label": "A", "value": 10},
            {"label": "B", "value": 20}
        ]);

        let result = AutoVisualiserServer::validate_data_param(&data, true);
        assert!(result.is_ok());

        let validated_data = result.unwrap();
        assert!(validated_data.is_array());
        assert_eq!(validated_data[0]["label"], "A");
    }

    #[test]
    fn test_validate_data_param_rejects_primitive_values() {
        // Test that primitive values (number, boolean) are rejected
        let data_number = json!(42);
        let result = AutoVisualiserServer::validate_data_param(&data_number, false);
        assert!(result.is_err());

        let data_bool = json!(true);
        let result = AutoVisualiserServer::validate_data_param(&data_bool, false);
        assert!(result.is_err());

        let data_null = json!(null);
        let result = AutoVisualiserServer::validate_data_param(&data_null, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_data_param_with_json_containing_comments_as_string() {
        // Test that JSON with comments passed as a string is rejected
        let data = json!(
            r#"{
            "labels": ["A", "B"],
            "matrix": [
                [0, 1],  // This is a comment
                [1, 0]   /* Another comment */
            ]
        }"#
        );

        let result = AutoVisualiserServer::validate_data_param(&data, false);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("not a JSON string"));
        assert!(err.message.contains("without comments"));
    }

    #[tokio::test]
    async fn test_render_sankey() {
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderSankeyParams {
            data: SankeyData {
                nodes: vec![
                    SankeyNode {
                        name: "A".to_string(),
                        category: None,
                    },
                    SankeyNode {
                        name: "B".to_string(),
                        category: None,
                    },
                ],
                links: vec![SankeyLink {
                    source: "A".to_string(),
                    target: "B".to_string(),
                    value: 10.0,
                }],
            },
        });

        let result = server.render_sankey(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);

        // Check it's a resource with HTML content
        if let RawContent::Resource(resource) = &*content[0] {
            if let rmcp::model::ResourceContents::BlobResourceContents { uri, mime_type, .. } =
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
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderRadarParams {
            data: RadarData {
                labels: vec![
                    "Speed".to_string(),
                    "Power".to_string(),
                    "Agility".to_string(),
                ],
                datasets: vec![RadarDataset {
                    label: "Player 1".to_string(),
                    data: vec![80.0, 90.0, 85.0],
                }],
            },
        });

        let result = server.render_radar(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);

        // Check it's a resource with HTML content
        if let RawContent::Resource(resource) = &*content[0] {
            if let rmcp::model::ResourceContents::BlobResourceContents {
                uri,
                mime_type,
                blob,
                ..
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
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderDonutParams {
            data: DonutData::Single(DonutChart {
                title: None,
                chart_type: None,
                labels: Some(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
                data: vec![
                    DonutDataItem::LabeledValue {
                        label: "A".to_string(),
                        value: 30.0,
                    },
                    DonutDataItem::LabeledValue {
                        label: "B".to_string(),
                        value: 40.0,
                    },
                    DonutDataItem::LabeledValue {
                        label: "C".to_string(),
                        value: 30.0,
                    },
                ],
            }),
        });

        let result = server.render_donut(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_treemap() {
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderTreemapParams {
            data: TreemapNode {
                name: "root".to_string(),
                value: None,
                category: None,
                children: Some(vec![
                    TreemapNode {
                        name: "A".to_string(),
                        value: Some(100.0),
                        category: None,
                        children: None,
                    },
                    TreemapNode {
                        name: "B".to_string(),
                        value: Some(200.0),
                        category: None,
                        children: None,
                    },
                ]),
            },
        });

        let result = server.render_treemap(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_chord() {
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderChordParams {
            data: ChordData {
                labels: vec!["A".to_string(), "B".to_string(), "C".to_string()],
                matrix: vec![
                    vec![0.0, 10.0, 5.0],
                    vec![10.0, 0.0, 15.0],
                    vec![5.0, 15.0, 0.0],
                ],
            },
        });

        let result = server.render_chord(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_render_map() {
        let server = AutoVisualiserServer::new();
        let params = Parameters(RenderMapParams {
            data: MapData {
                markers: vec![MapMarker {
                    lat: 0.0,
                    lng: 0.0,
                    name: Some("Origin".to_string()),
                    value: None,
                    description: None,
                    popup: None,
                    color: None,
                    label: None,
                    use_default_icon: None,
                }],
                title: None,
                subtitle: None,
                center: None,
                zoom: None,
                clustering: None,
                auto_fit: None,
            },
        });

        let result = server.render_map(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }

    #[tokio::test]
    async fn test_show_chart() {
        let server = AutoVisualiserServer::new();
        let params = Parameters(ShowChartParams {
            data: ChartData {
                chart_type: ChartDataType::Line,
                datasets: vec![ChartDataset {
                    label: "Test Data".to_string(),
                    data: vec![
                        ChartDataPoint::Coordinate { x: 1.0, y: 2.0 },
                        ChartDataPoint::Coordinate { x: 2.0, y: 4.0 },
                    ],
                }],
                labels: None,
                title: None,
                subtitle: None,
                x_axis_label: None,
                y_axis_label: None,
                options: None,
            },
        });

        let result = server.show_chart(params).await;
        assert!(result.is_ok());
        let call_result = result.unwrap();

        let content = call_result.content;
        assert_eq!(content.len(), 1);

        // Check the audience is set to User
        assert!(content[0].audience().is_some());
        assert_eq!(content[0].audience().unwrap(), &vec![Role::User]);
    }
}
