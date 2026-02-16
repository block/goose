# Navigation & UI Architecture

## Current State (as of 2026-02-16)

### Sidebar Structure (4-zone navigation)

```
┌────────────────────────────────────┐
│ 🏠 Home                     →  /  │
│ 💬 Chat                           │
│   📁 Project A (3 sessions)       │  ← grouped by working_dir
│     ├─ Session 1                   │
│     └─ Session 2                   │
│   📁 General (2 sessions)          │  ← $HOME or no working_dir
│     └─ Session 3                   │
│   View All →  /sessions            │
├────────────────────────────────────┤
│ ⚡ WORKFLOWS                       │
│   📄 Workflows      →  /recipes   │
│   🪟 Apps            →  /apps     │
│   ⏰ Scheduler       →  /schedules│
│ 👁 OBSERVATORY                     │
│   📊 Analytics       →  /analytics│
│   🔧 Tools           →  /tools    │
│   🤖 Agent Catalog   →  /agents   │
│ 🧩 CATALOGS          →  /catalogs │  ← clickable header
│   🧩 Tools Catalog   →  /extensions│
│   ⚙️ Settings        →  /settings │
└────────────────────────────────────┘
```

### Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `AppSidebar.tsx` | ~750 | Sidebar with 4 zones, project-grouped sessions, zone headers |
| `AppLayout.tsx` | ~160 | Layout wrapper with sidebar, outlet, prompt bar, reasoning panel |
| `App.tsx` | ~700 | Route definitions, guards, context providers |
| `PromptBarContext.tsx` | ~240 | Zone detection, slash commands, session creation |
| `PromptBar.tsx` | ~180 | Fixed-bottom input with autocomplete, Cmd+K, auto-hide on /pair |

### Persistent Prompt Bar

The prompt bar sits at the bottom of every page (except `/pair` where ChatInput handles it).
It adapts its placeholder, hints, and available slash commands based on the current zone:

| Zone | Placeholder | Special Commands |
|------|-------------|-----------------|
| Home | "Ask anything or type / for commands..." | `/new`, `/recipe` |
| Workflows | "Describe a workflow or search recipes..." | `/recipe` |
| Observatory | "Ask about performance or search analytics..." | `/eval`, `/tools` |
| Platform | "Search catalogs or configure settings..." | `/install`, `/schedule` |

**Global commands** (available everywhere): `/new`, `/recipe`, `/settings`, `/model`, `/project`, `/help`

### Session Grouping

Sessions are grouped by `working_dir` (from the `Session` type):
- Sessions with matching `working_dir` are grouped under a project folder icon
- Sessions with `$HOME` as working_dir or no working_dir → "General" group
- "General" group sorts last
- If all sessions are in one project, flat list is shown (no grouping needed)

## Routing Map

| Path | Component | Zone |
|------|-----------|------|
| `/` | Hub (home) | Home |
| `/pair` | BaseChat (via ChatSessionsContainer) | Chat |
| `/sessions` | SessionsView | Chat |
| `/recipes` | RecipesView | Workflows |
| `/apps` | AppsView | Workflows |
| `/schedules` | SchedulesView | Workflows |
| `/analytics` | AnalyticsView (7 sub-tabs) | Observatory |
| `/tools` | ToolsHealthView | Observatory |
| `/agents` | AgentsView | Observatory |
| `/catalogs` | CatalogsOverview | Catalogs |
| `/extensions` | ExtensionsView | Catalogs |
| `/settings` | SettingsView | Catalogs |

## Design Decisions

1. **Sessions are project-bound** — `working_dir` determines project grouping
2. **Default project = $HOME** — immutable, for general-purpose chats
3. **3 Catalogs** — Tools (MCP extensions), Agents (builtin + A2A), Workflows (recipes + future DAG)
4. **Catalog lifecycle**: browse → install → configure → version → share
5. **Desktop + CLI** are the active interfaces (3-tier: Interface → Server → Agents)
6. **Zone headers can be clickable** — `NavigationZone.route` property enables navigation

## Remaining UX Work

### Phase 4: Generative UI (goose4-u46)
- Install `@vercel-labs/json-render`
- Define component catalog with Zod schemas: MetricCard, Chart, DataTable, ActionButton, AlertCard
- Wire to prompt bar: AI generates JSON specs → render inline
- Complement existing MCP Apps pattern

### Phase 5: Visual DAG Builder (goose4-5jq)
- Install `reactflow`
- Define node types: AgentNode, ToolNode, ConditionNode, HumanNode, InputNode, OutputNode
- Goose Pipeline Format (`.gpf.yaml`) serializer/deserializer
- Rust backend DAG executor
- Integration with Workflows catalog

### Analytics V2
- Live monitoring (goose4-0ih) — WebSocket/SSE
- Version comparison (goose4-v9l) — Side-by-side diffs
- Response quality (goose4-7oa) — Quality metrics
- Sankey diagram (goose4-tm0) — Flow visualization
