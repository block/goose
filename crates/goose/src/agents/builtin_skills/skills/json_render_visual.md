---
name: json-render-visual
description: Render visual UI components (diagrams, tables, dashboards, cards) inline in chat using json-render. Uses Radix UI + Tailwind CSS.
---

# Rendering Visual Components Inline

When asked to **show**, **visualize**, **display**, or **present** data as visual content
in chat, use the `genui__render` tool or output a `json-render` fenced code block.

## Quick Decision: genui vs apps

- **"Show me X as a table/card/dashboard"** → `genui__render` tool (inline visual)
- **"Build me an app that does X"** → `apps__create_app` tool (standalone window)
- **"Visualize this data"** → `genui__render` tool
- **"Create a tool I can reuse"** → `apps__create_app` tool

## Two Output Formats

### 1. Nested Tree (simple, for small visuals)
Output a `json-render` fenced code block with nested JSON:

````
```json-render
{
  "root": {
    "type": "Card",
    "props": { "title": "Summary", "description": "Key metrics" },
    "children": [
      { "type": "Stack", "props": { "gap": "md" }, "children": [
        { "type": "Text", "props": { "text": "Revenue: $42,000" } },
        { "type": "Progress", "props": { "value": 75 } }
      ]}
    ]
  }
}
```
````

### 2. JSONL Streaming (advanced, for complex/interactive UIs)
Output a `json-render` fenced code block with JSONL patches:

````
```json-render
{"op":"add","path":"/root","value":"dashboard"}
{"op":"add","path":"/elements/dashboard","value":{"type":"Card","props":{"title":"Dashboard"},"children":["metrics","chart"]}}
{"op":"add","path":"/elements/metrics","value":{"type":"Stack","props":{"gap":"md"},"children":["metric-1","metric-2"]}}
{"op":"add","path":"/elements/metric-1","value":{"type":"Badge","props":{"label":"Revenue","variant":"default"}}}
{"op":"add","path":"/elements/metric-2","value":{"type":"Progress","props":{"value":75}}}
{"op":"add","path":"/elements/chart","value":{"type":"Table","props":{"headers":["Month","Revenue"],"rows":[["Jan","$10K"],["Feb","$15K"]]}}}
```
````

## Component Catalog

The full component catalog (33 Radix UI components) with props, events,
state management, and validation is available via the `genui` extension
instructions. Call `genui__components` to see the full list.

### Key Components
- **Layout**: Card, Stack, Grid, Separator, Tabs, Accordion
- **Content**: Heading, Text, Badge, Alert, Table, Image, Avatar
- **Feedback**: Progress, Skeleton, Spinner
- **Input**: Button, Input, Select, Checkbox, Switch, Slider
- **Navigation**: Link, Pagination, Collapsible
- **Overlay**: Dialog, Drawer, Popover, Tooltip, DropdownMenu

## Rules
1. Always wrap content in a Card as the outermost container
2. Use Stack (vertical) or Grid (horizontal) for layout
3. Use real data when available — explore files first, then visualize
4. For state/interactivity, use `$state`, `$bindState`, `on` events, and `visible` conditions
5. For repeated data (lists, grids), use the `repeat` field instead of duplicating elements
