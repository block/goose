---
name: json-render-visual
description: Render visual components inline in chat using json-render code blocks. Use when the user asks to visualize data, show diagrams, create summaries, display tables, or render any structured visual content directly in the conversation.
---

Use this skill when the user asks to:
- Visualize data, configs, or code structure
- Show a diagram, flowchart, or tree
- Display a table, summary card, or dashboard
- Render any structured visual content inline in chat

Do NOT use this skill for:
- Creating standalone apps (use the `apps` extension instead)
- Plain text explanations that don't need visual formatting
- Code output that should remain as code

## How It Works

Output a fenced code block with language `json-render`. The chat UI will render it as live interactive components using Radix UI + Tailwind CSS.

## Available Components

### Layout
- **Card** — Container with optional header/footer. Props: `className`
- **Stack** — Vertical/horizontal flex layout. Props: `direction` (row|column), `gap` (sm|md|lg|xl), `align`, `justify`, `className`
- **Grid** — CSS grid. Props: `columns` (1-6), `gap`, `className`
- **Separator** — Horizontal/vertical divider. Props: `orientation`, `className`

### Navigation
- **Tabs** — Tabbed content panels. Props: `defaultValue`, `tabs` (array of {value, label, content})
- **Accordion** — Collapsible sections. Props: `type` (single|multiple), `items` (array of {value, title, content})

### Content
- **Heading** — h1-h6 headings. Props: `level` (1-6), `text`, `className`
- **Text** — Paragraph text. Props: `text`, `className`
- **Badge** — Small label. Props: `text`, `variant` (default|secondary|destructive|outline)
- **Alert** — Callout box. Props: `title`, `description`, `variant` (default|destructive)
- **Table** — Data table. Props: `headers` (string[]), `rows` (string[][]), `caption`
- **Image** — Image display. Props: `src`, `alt`, `className`
- **Avatar** — User/entity avatar. Props: `src`, `fallback`, `className`

### Feedback
- **Progress** — Progress bar. Props: `value` (0-100), `className`
- **Skeleton** — Loading placeholder. Props: `className`
- **Spinner** — Loading spinner. Props: `size`

### Input (interactive)
- **Button** — Clickable button. Props: `label`, `variant` (default|destructive|outline|secondary|ghost|link), `size` (default|sm|lg|icon)
- **Input** — Text input. Props: `placeholder`, `type`, `defaultValue`
- **Select** — Dropdown. Props: `placeholder`, `options` (array of {value, label})
- **Checkbox** — Toggle checkbox. Props: `label`, `defaultChecked`
- **Switch** — Toggle switch. Props: `label`, `defaultChecked`

## Spec Format

```json
{
  "root": {
    "type": "ComponentName",
    "props": { ... },
    "children": [
      { "type": "AnotherComponent", "props": { ... } }
    ]
  }
}
```

## Examples

### Summary Card
````json-render
{
  "root": {
    "type": "Card",
    "props": { "className": "p-6 max-w-md" },
    "children": [
      { "type": "Heading", "props": { "level": 3, "text": "Project Status" } },
      { "type": "Stack", "props": { "gap": "md" }, "children": [
        { "type": "Text", "props": { "text": "Build: Passing" } },
        { "type": "Progress", "props": { "value": 85 } },
        { "type": "Badge", "props": { "text": "v2.1.0", "variant": "secondary" } }
      ]}
    ]
  }
}
````

### Data Table
````json-render
{
  "root": {
    "type": "Card",
    "props": { "className": "p-4" },
    "children": [
      { "type": "Heading", "props": { "level": 4, "text": "API Endpoints" } },
      { "type": "Table", "props": {
        "headers": ["Method", "Path", "Status"],
        "rows": [
          ["GET", "/api/users", "200"],
          ["POST", "/api/sessions", "201"],
          ["DELETE", "/api/cache", "204"]
        ]
      }}
    ]
  }
}
````

### Tabbed Content
````json-render
{
  "root": {
    "type": "Tabs",
    "props": {
      "defaultValue": "overview",
      "tabs": [
        { "value": "overview", "label": "Overview", "content": "Main project dashboard" },
        { "value": "metrics", "label": "Metrics", "content": "Performance metrics here" },
        { "value": "logs", "label": "Logs", "content": "Recent log entries" }
      ]
    }
  }
}
````

## Rules
1. Always use a `Card` as the outermost container
2. Use `Stack` for vertical layouts, `Grid` for multi-column
3. Keep specs concise — only include what's needed
4. Use `className` for Tailwind CSS customization (padding, max-width, colors)
5. Nest components via `children` arrays
6. This renders INLINE in chat — don't use it for full standalone apps
