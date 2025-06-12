# MCP UI Resources Support

This document describes the enhanced MCP resource support for UI components that was added to the Rust codebase.

## Overview

The MCP-UI integration allows MCP server tools to respond with interactive UI resources that are automatically rendered by the `@mcp-ui/client` package in the frontend. This enables rich, interactive content to be displayed directly in the chat interface.

## Supported Resource Types

The Rust code now supports the following resource data structures:

### 1. HTML Text Resources
```json
{
  "type": "resource",
  "resource": {
    "uri": "ui://my-component/instance-1",
    "mimeType": "text/html",
    "text": "<p>Hello World</p>"
  }
}
```

### 2. HTML Blob Resources (Base64 encoded)
```json
{
  "type": "resource",
  "resource": {
    "uri": "ui://my-component/instance-2",
    "mimeType": "text/html",
    "blob": "PGRpdj48aDI+Q29tcGxleCBDb250ZW50PC9oMj48c2NyaXB0PmNvbnNvbGUubG9nKFwiTG9hZGVkIVwiKTwvc2NyaXB0PjwvZGl2Pg=="
  }
}
```

### 3. URI List Text Resources
```json
{
  "type": "resource",
  "resource": {
    "uri": "ui://analytics-dashboard/main",
    "mimeType": "text/uri-list",
    "text": "https://my.analytics.com/dashboard/123"
  }
}
```

### 4. URI List Blob Resources (Base64 encoded)
```json
{
  "type": "resource",
  "resource": {
    "uri": "ui://live-chart/session-xyz",
    "mimeType": "text/uri-list",
    "blob": "aHR0cHM6Ly9jaGFydHMuZXhhbXBsZS5jb20vYXBpP3R5cGU9cGllJmRhdGE9MSwyLDM="
  }
}
```

## Rust API

### ResourceContents Enum

The existing `ResourceContents` enum already supported the required structure with both `text` and `blob` variants, and optional `mime_type` fields.

### New Convenience Methods

#### ResourceContents Methods
- `ResourceContents::html_text(uri, content)` - Create HTML resource with text content
- `ResourceContents::html_blob(uri, blob)` - Create HTML resource with base64 blob content
- `ResourceContents::uri_list_text(uri, content)` - Create URI list resource with text content
- `ResourceContents::uri_list_blob(uri, blob)` - Create URI list resource with base64 blob content

#### Helper Methods
- `resource.uri()` - Get the URI of the resource
- `resource.mime_type()` - Get the MIME type of the resource
- `resource.is_ui_resource()` - Check if URI starts with "ui://"
- `resource.is_html()` - Check if MIME type is "text/html"
- `resource.is_uri_list()` - Check if MIME type is "text/uri-list"

#### Content Methods
- `Content::embedded_html_text(uri, content)` - Create embedded HTML text resource
- `Content::embedded_html_blob(uri, blob)` - Create embedded HTML blob resource
- `Content::embedded_uri_list_text(uri, content)` - Create embedded URI list text resource
- `Content::embedded_uri_list_blob(uri, blob)` - Create embedded URI list blob resource

## Usage Example

```rust
use mcp_core::{Content, ResourceContents};

// Create an HTML resource with interactive content
let html_resource = ResourceContents::html_text(
    "ui://dashboard/sales-chart",
    "<div><h2>Sales Chart</h2><canvas id='chart'></canvas><script>/* chart code */</script></div>"
);

// Create a URI list resource pointing to an external dashboard
let uri_resource = ResourceContents::uri_list_text(
    "ui://external/analytics",
    "https://analytics.example.com/dashboard/123"
);

// Use in tool responses
let content = Content::embedded_html_text(
    "ui://my-tool/result-1",
    "<p>Tool execution completed successfully!</p>"
);
```

## Frontend Integration

The UI integration is already in place:

1. **@mcp-ui/client package** - Added to `ui/desktop/package.json`
2. **HtmlResourceRenderer component** - Renders UI resources in `ui/desktop/src/components/HtmlResourceRenderer.tsx`
3. **Type definitions** - Updated in `ui/desktop/src/api/types.gen.ts` and `ui/desktop/src/types/message.ts`
4. **Tool response handling** - Integrated in `ui/desktop/src/components/ToolCallWithResponse.tsx`

Resources with URIs starting with `ui://` are automatically detected and rendered using the `HtmlResource` component from the `@mcp-ui/client` package.

## Testing

Run the example to see the functionality in action:

```bash
cargo run --example mcp_ui_resources
```

This demonstrates creating all four types of UI resources and shows their JSON serialization format.