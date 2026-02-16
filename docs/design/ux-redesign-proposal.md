# Goose UX Redesign Proposal

## Problem Statement

The current sidebar has **9 menu items** that grew organically:
```
Home → Hub with session insights + chat input
Chat → Active conversation (pair view)
Recipes → Conversational workflow templates
Apps → MCP app gallery
Scheduler → Cron-like task scheduling
Agents → Agent registry & modes
Analytics → Eval/tool/routing analytics (NEW)
Extensions → MCP extension management
Settings → Provider, model, preferences
```

**Issues:**
1. **Too many items** — 9 entries creates cognitive overload
2. **Home ≠ Chat** — starting a conversation requires navigating between views
3. **No persistent prompt** — the chat input disappears when browsing other pages
4. **Flat hierarchy** — Recipes, Scheduler, and Agents are related but scattered
5. **No workflow builder** — only conversational (recipe) workflows, no visual DAG editor
6. **Analytics buried** — insights should surface proactively, not require navigation

---

## Design Principles

1. **Conversation-first**: The prompt bar is always visible, everywhere
2. **Progressive disclosure**: Show what matters, hide complexity
3. **Connected flow**: See problem → investigate → fix → verify
4. **Adaptive context**: The prompt bar adapts to what you're doing
5. **Generative UI**: Rich responses inline in conversation (charts, forms, tables)

---

## Proposed Information Architecture

### Navigation: 4 Primary Zones

```
┌─────────────────────────────────────────────────────────┐
│  ┌──────┐                                               │
│  │ Logo │  ← Goose brand, collapse toggle               │
│  ├──────┤                                               │
│  │  ⌂   │  Home / Workspace                             │
│  │  ◉   │  Workflows (Recipes + DAG Builder)            │
│  │  ◈   │  Observatory (Analytics + Monitoring)          │
│  │  ⚙   │  Platform (Extensions + Agents + Settings)    │
│  ├──────┤                                               │
│  │ Hist │  Recent Sessions (collapsible)                │
│  └──────┘                                               │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │                Main Content                      │    │
│  │                                                  │    │
│  │  (adapts to selected zone)                       │    │
│  │                                                  │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  🔍 Ask anything... (Persistent Prompt Bar)      │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Zone 1: Home / Workspace (⌂)
**Purpose**: Start working immediately. Merges current Home + Chat.

- **Default view**: Chat conversation (replaces the separate Hub/Pair split)
- When no active session: shows quick-start cards (recent projects, pinned recipes)
- Session history integrated as a sidebar panel
- Project context (working directory) visible at top

### Zone 2: Workflows (◉)
**Purpose**: Create, manage, and run reusable workflows.

Sub-tabs:
- **Recipes** (existing) — Conversational prompt workflows
- **Pipelines** (NEW) — Visual DAG workflow builder
  - Drag-and-drop nodes: Agent, Tool, Condition, Transform, Human-in-loop
  - Visual graph rendering (React Flow)
  - Connect agents/tools together with data flow edges
  - Run/debug pipelines with step-by-step execution view
- **Schedules** (existing, moved here) — Cron triggers for both recipes and pipelines
- **Templates** — Community/shared workflow gallery

### Zone 3: Observatory (◈)
**Purpose**: Monitor, evaluate, and understand your agents.

Sub-tabs:
- **Dashboard** — KPIs, health indicators, recent alerts
  - Routing accuracy, tool success rate, sessions today
  - Regression alerts (prominent, actionable)
  - Quick sparklines for trends
- **Evaluate** — Test datasets, run evals, compare versions
  - Dataset CRUD, run history, topics
- **Tools** — Tool usage analytics, error patterns
  - Per-tool call counts, error rates
  - Extension health
- **Inspect** — Live routing inspector, agent catalog
  - Routing decisions debugger
  - Agent registry viewer

### Zone 4: Platform (⚙)
**Purpose**: Configure the system.

Sub-tabs:
- **Extensions** (existing) — MCP extension management
- **Agents** (existing) — Agent registry & modes
- **Apps** (existing) — MCP app gallery
- **Settings** (existing) — Provider, model, preferences

---

## The Persistent Prompt Bar

The most important UX change: **the prompt bar is always visible at the bottom of every view**.

### Context-Adaptive Behavior

| Current Zone | Prompt Behavior |
|-------------|-----------------|
| Home | Standard chat — creates/continues sessions |
| Workflows | "Create a recipe that...", "Run pipeline X with..." |
| Observatory | "Show me routing accuracy for last week", "What tools failed today?" |
| Platform | "Enable the GitHub extension", "Change model to Claude" |

### How It Works

1. **Always visible** — fixed at bottom of every page
2. **Smart routing** — detects intent from context:
   - `/search` prefix → search across everything
   - `/settings` prefix → navigate to settings
   - Natural language → either chat or command depending on context
3. **Generative UI responses** — when on Observatory, analytics queries return inline charts/tables
4. **Slash commands** — quick access to any action:
   - `/new` — new session
   - `/recipe create` — new recipe
   - `/eval run` — run evaluation
   - `/settings model` — change model
   - `/help` — contextual help

### Generative UI Integration

When the prompt bar returns structured data (e.g., analytics queries), it renders rich UI inline:

```
User: "Show me tool error rates this week"
AI: [Renders inline chart showing error rates]
    [Action button: "View full analytics →"]
    [Action button: "Create alert for >5% error rate"]
```

This uses the MCP Apps renderer pattern — the response contains structured HTML/components
that render in the conversation thread, not as a separate page navigation.

---

## Visual DAG Workflow Builder

### Concept

A no-code visual editor where users connect agents, tools, conditions, and transforms
into executable pipelines.

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Trigger   │────▶│ Coding   │────▶│ QA Agent │
│ (webhook) │     │ Agent    │     │ (review) │
└──────────┘     └──────────┘     └─────┬────┘
                                        │
                                   ┌────▼────┐
                                   │ Condition│
                                   │ pass?    │
                                   └────┬────┘
                                   yes │  │ no
                               ┌──────▼┐ ┌▼──────┐
                               │ Deploy │ │ Fix   │
                               │ Tool   │ │ Agent │
                               └────────┘ └───────┘
```

### Node Types
- **Agent Node**: Select agent + mode, configure instructions
- **Tool Node**: Select specific tool from extensions
- **Condition Node**: If/else branching on output
- **Transform Node**: Map/filter/format data between steps
- **Human Node**: Pause for human review/approval
- **Trigger Node**: Webhook, schedule, event, manual

### Implementation
- Use React Flow (https://reactflow.dev) for the graph editor
- Store as JSON/YAML in the same recipe format (extended)
- Execute via a new DAG executor in the Rust backend
- Each node execution is tracked for analytics

---

## Mapping: Old → New

| Old Menu Item | New Location | Notes |
|--------------|-------------|-------|
| Home | **Home** (⌂) | Merged with Chat |
| Chat | **Home** (⌂) | Same view, always available |
| Recipes | **Workflows** (◉) > Recipes | Grouped with related items |
| Apps | **Platform** (⚙) > Apps | Configuration concern |
| Scheduler | **Workflows** (◉) > Schedules | Triggers for workflows |
| Agents | **Platform** (⚙) > Agents | Configuration concern |
| Analytics | **Observatory** (◈) | Promoted to top-level zone |
| Extensions | **Platform** (⚙) > Extensions | Configuration concern |
| Settings | **Platform** (⚙) > Settings | Configuration concern |

**Result: 9 items → 4 zones** + persistent prompt bar

---

## Implementation Phases

### Phase 1: Navigation Restructure (1-2 weeks)
- Consolidate sidebar to 4 zones
- Move existing pages into zone sub-tabs
- Keep existing components, just reorganize routing
- Add persistent prompt bar (extract ChatInput to a global position)

### Phase 2: Adaptive Prompt Bar (2-3 weeks)
- Context detection based on current zone
- Slash command system
- Generative UI responses for analytics queries
- Search across sessions, recipes, settings

### Phase 3: Visual Workflow Builder (4-6 weeks)
- React Flow integration
- Node palette (agents, tools, conditions)
- Pipeline execution engine (Rust backend)
- Step-by-step execution debugging

### Phase 4: Live Observatory (2-3 weeks)
- Real-time event streaming
- Live routing confidence scores
- Tool execution monitoring
- Automated alert rules

---

## Generative UI with json-render

### Concept

The persistent prompt bar doesn't just navigate — it **generates UI inline**.
Using [json-render](https://github.com/vercel-labs/json-render), we define a catalog
of safe, schema-validated components that the AI can compose in response to natural
language queries.

### Component Catalog

```typescript
const gooseCatalog = defineCatalog(schema, {
  components: {
    // Analytics widgets
    MetricCard: {
      props: z.object({
        label: z.string(),
        value: z.string(),
        delta: z.number().nullable(),
        trend: z.enum(['up', 'down', 'flat']).nullable(),
      }),
      description: "Display a KPI metric with optional trend indicator",
    },
    Chart: {
      props: z.object({
        type: z.enum(['line', 'bar', 'pie', 'area']),
        data: z.array(z.record(z.unknown())),
        xKey: z.string(),
        yKeys: z.array(z.string()),
        title: z.string().nullable(),
      }),
      description: "Render a chart from data",
    },
    DataTable: {
      props: z.object({
        columns: z.array(z.object({ key: z.string(), label: z.string() })),
        rows: z.array(z.record(z.unknown())),
        sortable: z.boolean().nullable(),
      }),
      description: "Render a sortable data table",
    },
    // Workflow widgets
    RecipeCard: {
      props: z.object({
        name: z.string(),
        description: z.string(),
        runCount: z.number().nullable(),
      }),
      slots: ["actions"],
      description: "Display a recipe with action buttons",
    },
    // Action widgets
    ActionButton: {
      props: z.object({
        label: z.string(),
        action: z.string(),
        variant: z.enum(['primary', 'secondary', 'danger']).nullable(),
      }),
      description: "Button that triggers a platform action",
    },
    AlertCard: {
      props: z.object({
        severity: z.enum(['info', 'warning', 'error', 'success']),
        title: z.string(),
        message: z.string(),
      }),
      description: "Display an alert or notification",
    },
  },
  actions: {
    navigate: {
      params: z.object({ path: z.string() }),
      description: "Navigate to a page in the app",
    },
    run_recipe: {
      params: z.object({ recipeId: z.string() }),
      description: "Execute a recipe",
    },
    run_eval: {
      params: z.object({ datasetId: z.string() }),
      description: "Run an evaluation on a dataset",
    },
    create_session: {
      params: z.object({ workingDir: z.string().nullable() }),
      description: "Start a new chat session",
    },
    change_model: {
      params: z.object({ provider: z.string(), model: z.string() }),
      description: "Switch the active model",
    },
    enable_extension: {
      params: z.object({ name: z.string() }),
      description: "Enable an MCP extension",
    },
  },
});
```

### Flow

1. User types in persistent prompt bar: "Show me tool error rates this week"
2. AI detects this is an analytics query (not a chat message)
3. AI generates a json-render spec with MetricCard + Chart components
4. Components render inline in a response panel (not a full page navigation)
5. User can interact (click chart points, drill down) or ask follow-up

### MCP Apps Integration

The existing MCP Apps renderer (`McpAppRenderer.tsx`) already handles sandboxed
HTML rendering. json-render adds a **structured, validated** layer on top:
- MCP Apps: arbitrary HTML in iframe (powerful but unvalidated)
- json-render: schema-validated component trees (safe, predictable, themed)

Both can coexist — json-render for standard analytics/workflow widgets,
MCP Apps for custom rich experiences.

---

## Open Questions

1. **Session management**: Should sessions be per-project or global?
2. **Multi-session**: Should users be able to have multiple active conversations?
3. **DAG persistence**: Store pipelines as extended recipes or new entity type?
4. **Generative UI scope**: How much analytics should render inline vs dedicated page?
5. **Mobile/responsive**: Is the desktop-only or should it work in browser too?
6. **json-render integration**: Should it be a new MCP extension or built into the agent?
7. **Prompt bar routing**: How to distinguish "chat with AI" from "search/command"?
