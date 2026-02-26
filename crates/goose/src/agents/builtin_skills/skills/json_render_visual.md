---
name: json-render-visual
description: Render visual UI components (diagrams, tables, dashboards, cards) inline in chat using json-render. Uses Radix UI + Tailwind CSS.
---

# Rendering Visual Components Inline

When asked to **show**, **visualize**, **display**, or **present** data as visual content
in chat, output the json-render spec **directly in your text response** (fenced or unfenced).

## CRITICAL: The spec MUST be in your response text
Tool results are collapsed and hidden from the user. To show visual components,
you MUST include the json-render spec in your TEXT response — not just
as a tool result.

Markdown fences are optional. If you use a fence, it MUST be ```json-render (and closed with ```).

Optionally call `genui__render` first to validate the spec, then paste the validated spec into your response.

## Quick Decision: genui vs apps

- **"Show me X as a table/card/dashboard"** → json-render spec in your response
- **"Build me an app that does X"** → `apps__create_app` tool (standalone window)
- **"Visualize this data"** → json-render spec in your response
- **"Create a tool I can reuse"** → `apps__create_app` tool

## Two Output Formats

### 1. Nested Tree (simple, for small visuals)
Output nested JSON (fenced or unfenced) **in your response text**:

````
```json-render
{
  "root": {
    "type": "Card",
    "props": {
      "title": "Summary",
      "description": "Key metrics",
      "maxWidth": "full",
      "centered": false
    },
    "children": [
      {
        "type": "Stack",
        "props": { "gap": "md" },
        "children": [
          { "type": "Text", "props": { "text": "Revenue: $42,000" } },
          { "type": "Progress", "props": { "value": 75 } }
        ]
      }
    ]
  }
}
```
````

### 2. JSONL Streaming (advanced, for complex/interactive UIs)
Output JSONL patches (fenced or unfenced):

````
```json-render
{"op":"add","path":"/root","value":"dashboard"}
{"op":"add","path":"/elements","value":{}}
{"op":"add","path":"/elements/dashboard","value":{"type":"Card","props":{"title":"Dashboard","maxWidth":"full","centered":false},"children":["metrics","table"]}}
{"op":"add","path":"/elements/metrics","value":{"type":"Grid","props":{"columns":2,"gap":"md"},"children":["metric-1","metric-2"]}}
{"op":"add","path":"/elements/metric-1","value":{"type":"StatCard","props":{"label":"Revenue","value":"$42,000"}}}
{"op":"add","path":"/elements/metric-2","value":{"type":"StatCard","props":{"label":"Conversion","value":"3.1%"}}}
{"op":"add","path":"/elements/table","value":{"type":"Table","props":{"columns":["Month","Revenue"],"rows":[["Jan","$10K"],["Feb","$15K"]],"caption":"Monthly revenue (sample)"}}}
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
