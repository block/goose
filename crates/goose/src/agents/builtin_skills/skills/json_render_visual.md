---
name: json-render-visual
description: Render visual components inline in chat using json-render code blocks. Use when the user asks to visualize data, show diagrams, create summaries, display tables, or render any structured visual content directly in the conversation.
---

## When to Use What: Visual Components vs Apps

Goose has TWO ways to create visual output. Choose the right one:

### Use `json-render` (this skill) when:
- User wants to **see** data: "show me", "visualize", "display", "render"
- Content is **part of the conversation**: summaries, tables, diagrams, status cards
- Output is **ephemeral** — displayed once, not saved or reused
- Examples: "show me the config flow as a diagram", "display these results in a table", "render a summary card"

### Use `apps__create_app` when:
- User wants to **build** something: "create an app", "build me a tool", "make a calculator"
- Output is a **standalone application** that opens in its own window
- App is **persistent** — saved, iterable, reopenable
- App needs **custom JavaScript logic**, APIs, or complex interactivity
- Examples: "build me a todo app", "create a project dashboard app", "make a code playground"

### Quick decision:
- "Show me X" → `json-render` (inline visual)
- "Build me X" → `apps__create_app` (standalone app)
- "Visualize X" → `json-render`
- "Create an app that X" → `apps__create_app`

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
