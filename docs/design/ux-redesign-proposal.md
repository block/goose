# Goose Desktop UX Redesign Proposal

## Status: Draft
## Date: 2025-02-16

---

## Problem Statement

The current sidebar has 9 menu items (Home, Chat, Recipes, Apps, Scheduler, Agents, Analytics, Extensions, Settings) that feel like a flat list of features rather than a coherent workflow. Users need to:

1. **Work on projects** — open projects, create sessions, use history
2. **Create reusable workflows** — both conversational (prompts/recipes) and visual DAGs
3. **Monitor & evaluate** — track routing accuracy, tool performance, regressions
4. **Configure** — agents, extensions, settings
5. **Get help from anywhere** — a persistent prompt bar that's always available

## Design Principles

1. **Intent-based navigation** — organize by "what am I trying to do" not "what feature exists"
2. **Progressive disclosure** — overview → drill-down, never overwhelm
3. **Prompt-first** — the prompt bar is the primary interaction, pages are secondary
4. **Connected flow** — see a problem → investigate → fix → verify, without context switching

---

## Proposed Navigation: 4 Zones

### Current → New Mapping

| Current (9 items) | New Zone | Sub-area |
|---|---|---|
| Home + Chat (merged) | **Home** | Project workspace + sessions |
| Recipes | **Workflows** | Conversational tab |
| *(new)* DAG Builder | **Workflows** | Visual tab |
| Apps | **Workflows** | Apps tab |
| Scheduler | **Workflows** | Scheduler tab |
| Analytics (eval) | **Observatory** | Evaluation tab |
| Analytics (tools) | **Observatory** | Performance tab |
| Analytics (dashboard) | **Observatory** | Dashboard tab |
| Agents | **Platform** | Agents tab |
| Extensions | **Platform** | Extensions tab |
| Settings | **Platform** | Settings tab |

### Zone Details

#### 1. Home (Project Workspace)
- **Sessions are per-project** — each project directory has its own session history
- Merge Home + Chat into a single workspace view
- Shows: recent sessions, project stats, quick actions
- Multi-session support for multitasking (multiple active conversations)

#### 2. Workflows (Create & Reuse)
Two kinds of reusable workflows coexist:

**Conversational workflows** (existing recipes/sub-recipes):
- Prompt-based, primarily text
- Formats: `.md`, `.mdx` (YAML frontmatter), `.yaml`, `.json`, `.toon`
- Current recipe system already handles this well

**Visual DAG workflows** (new):
- No-code graph editor connecting agents/tools
- Typed nodes: Agent, Tool, Condition, Transform, Human-in-Loop, Trigger
- See [DAG Format section](#dag-workflow-format) below

#### 3. Observatory (Monitor & Evaluate)
Everything about "how well is my system performing":
- **Dashboard**: KPI cards, accuracy trends, regression alerts
- **Evaluation**: Datasets, run history, confusion matrices, topics
- **Performance**: Tool analytics, agent metrics, latency, success rates

#### 4. Platform (Catalogs & Settings)

The "Extensions" concept evolves into **three Catalogs** — shareable, versionable, packageable entities:

| Catalog | What it contains | Package format | Registry |
|---------|-----------------|---------------|----------|
| **Agents** | Agent definitions with modes, prompts, tool access, `when_to_use` | Agent manifest (YAML/JSON via `AgentManifest`) | Shareable packages |
| **Tools** | MCP servers / tool providers (currently "Extensions") | MCP extension config | Shareable packages |
| **Workflows** | Conversational (recipes) + Visual DAG pipelines | `.md`/`.mdx`/`.yaml` for recipes, `.gpf.yaml` for DAGs | Shareable packages |

**Key insight**: "Extensions" is a developer term — users think in terms of **tools** they can use.
"Recipes" are just one kind of **workflow**. Agents, Tools, and Workflows are the three
fundamental building blocks users care about.

Each catalog supports a unified lifecycle:
1. **Browse** — discover available packages (built-in + community)
2. **Install** — add to workspace (with version pinning)
3. **Configure** — customize behavior, permissions, tool access
4. **Version** — track changes, rollback, compare
5. **Share** — export as package, publish to registry

##### Sidebar structure:
- **Agents Catalog**: Browse/install/configure agents with modes
- **Tools Catalog**: Browse/install MCP servers (replaces "Extensions")
- **Settings**: Provider, model, preferences (not a catalog — pure config)

---

## Persistent Context-Adaptive Prompt Bar

The prompt bar appears on **every page** and adapts to the current context:

| Context | Prompt Bar Behavior |
|---------|-------------------|
| Home | Full chat — creates/continues sessions |
| Workflows | "Describe a workflow..." — helps build recipes/DAGs |
| Observatory | "Show me routing accuracy for last week" — queries analytics |
| Platform | "Enable the GitHub extension" — modifies configuration |

The prompt bar uses **Generative UI** to render rich inline responses (charts, tables, action buttons) rather than just text.

### Slash Commands (available everywhere)
- `/new` — new session
- `/recipe <name>` — run a recipe
- `/eval <dataset>` — run evaluation
- `/search <query>` — search across sessions/recipes/settings
- `/help` — contextual help

---

## Generative UI with json-render

### Architecture

Use Vercel's [json-render](https://github.com/vercel-labs/json-render) to define a **component catalog** that the AI can generate against:

```typescript
const catalog = defineCatalog({
  components: {
    MetricCard: {
      props: z.object({
        title: z.string(),
        value: z.string(),
        delta: z.number().optional(),
        trend: z.enum(['up', 'down', 'flat']).optional(),
        sparkline: z.array(z.number()).optional(),
      }),
    },
    Chart: {
      props: z.object({
        type: z.enum(['line', 'bar', 'pie', 'area']),
        data: z.array(z.record(z.unknown())),
        xKey: z.string(),
        yKeys: z.array(z.string()),
        title: z.string().optional(),
      }),
    },
    DataTable: {
      props: z.object({
        columns: z.array(z.object({ key: z.string(), label: z.string() })),
        rows: z.array(z.record(z.unknown())),
        sortable: z.boolean().optional(),
      }),
    },
    RecipeCard: {
      props: z.object({
        name: z.string(),
        description: z.string(),
        tags: z.array(z.string()).optional(),
      }),
    },
    ActionButton: {
      props: z.object({
        label: z.string(),
        action: z.string(),
        variant: z.enum(['primary', 'secondary', 'destructive']).optional(),
      }),
    },
    AlertCard: {
      props: z.object({
        severity: z.enum(['info', 'warning', 'error', 'success']),
        title: z.string(),
        description: z.string(),
      }),
    },
  },
  actions: {
    navigate: { path: z.string() },
    run_recipe: { name: z.string(), params: z.record(z.string()).optional() },
    run_eval: { datasetId: z.string() },
    create_session: { projectDir: z.string().optional() },
    change_model: { provider: z.string(), model: z.string() },
    enable_extension: { name: z.string() },
  },
});
```

### How It Works

1. User types in prompt bar: "How did routing accuracy change this week?"
2. Backend queries analytics data
3. AI generates a json-render spec: `[MetricCard(accuracy), Chart(trend), AlertCard(regression)]`
4. Frontend renders native React components inline in the chat
5. User can interact (click action buttons, sort tables, drill into charts)

### Relationship to MCP Apps

| | Generative UI (json-render) | MCP Apps |
|---|---|---|
| **Scope** | Predefined component catalog | Arbitrary HTML/CSS/JS |
| **Safety** | Constrained to catalog (guardrailed) | Sandboxed iframe |
| **Use case** | Analytics, status, quick actions | Complex interactive apps |
| **Performance** | Native React rendering | iframe overhead |
| **When to use** | AI-generated responses | Extension-provided UIs |

They coexist — Generative UI for structured responses, MCP Apps for complex custom UIs.

---

## DAG Workflow Format

### Research: Existing Standards

| Standard | Type | Format | Agent-specific? | Suitability |
|----------|------|--------|-----------------|-------------|
| Argo Workflows | K8s-native DAG | YAML | No (containers) | Template for YAML structure |
| CWL (Common Workflow Language) | Scientific workflows | YAML/JSON | No | Too scientific-focused |
| LangGraph | Agent orchestration | Python code | Yes | Not declarative |
| CrewAI | Agent crews | Python code | Yes | Not declarative |
| AutoGen | Multi-agent | Python code | Yes | Not declarative |
| n8n | Visual automation | JSON | No (HTTP/services) | Closest visual model |
| Make (Integromat) | Visual automation | Proprietary | No | Good UX reference |
| Copilot Studio | Agent topics | Proprietary | Yes | Good UX reference |
| A2A Protocol | Agent-to-agent | JSON-RPC | Yes | Communication, not orchestration |
| ACP | Agent communication | HTTP/JSON | Yes | Already in goose |

**Key finding**: No established declarative standard exists for AI agent DAG workflows. The closest are:
- **Argo Workflows** for YAML DAG structure (depends/template pattern)
- **n8n** for visual node-based JSON format
- **React Flow** for visual graph editing UX

### Proposed: Goose Pipeline Format

A YAML-based declarative format that extends the existing recipe concept:

```yaml
# .goose/pipelines/code-review.yaml
apiVersion: goose/v1
kind: Pipeline
metadata:
  name: code-review-pipeline
  description: Automated code review with security and quality checks
  tags: [code-quality, security, ci]

# Node type definitions reference goose agents, tools, and conditions
nodes:
  - id: trigger
    type: trigger
    config:
      event: pull_request  # or: manual, schedule, webhook
      
  - id: fetch-diff
    type: tool
    config:
      extension: developer
      tool: shell
      arguments:
        command: "git diff main...HEAD"
    depends: [trigger]

  - id: security-review
    type: agent
    config:
      agent: qa_agent
      mode: security
      prompt: |
        Review the following diff for security vulnerabilities:
        {{fetch-diff.output}}
    depends: [fetch-diff]

  - id: code-quality
    type: agent
    config:
      agent: coding_agent
      mode: code
      prompt: |
        Review the following diff for code quality issues:
        {{fetch-diff.output}}
    depends: [fetch-diff]

  - id: gate
    type: condition
    config:
      expression: |
        security-review.severity != "critical" 
        AND code-quality.score >= 0.7
    depends: [security-review, code-quality]

  - id: approve
    type: tool
    config:
      extension: developer
      tool: shell
      arguments:
        command: "gh pr review --approve"
    depends: [gate]
    condition: gate.passed

  - id: request-changes
    type: human
    config:
      prompt: |
        Security or quality issues found:
        - Security: {{security-review.summary}}
        - Quality: {{code-quality.summary}}
        
        Please review and decide.
    depends: [gate]
    condition: "!gate.passed"

# Edges are implicit from `depends` + `condition`
# Data flows through {{node-id.output}} template references
```

### Node Types

| Type | Description | Config |
|------|-------------|--------|
| `trigger` | Entry point | `event`: manual, schedule, webhook, pull_request |
| `agent` | Run a goose agent in a specific mode | `agent`, `mode`, `prompt`, `max_turns` |
| `tool` | Call a specific MCP tool | `extension`, `tool`, `arguments` |
| `condition` | Boolean gate/branch | `expression` with references to upstream outputs |
| `transform` | Data transformation | `template` (Jinja2/Handlebars) |
| `human` | Human-in-the-loop approval | `prompt`, `timeout`, `default_action` |
| `subpipeline` | Nest another pipeline | `pipeline`, `arguments` |
| `a2a` | Call an external A2A agent | `agent_card_url`, `task` |

### Visual Editor (React Flow)

The visual editor provides a drag-and-drop interface for building pipelines:
- **Node palette** on the left with all node types
- **Canvas** in the center for the graph
- **Properties panel** on the right for node configuration
- **Run/Debug toolbar** at the top with step-by-step execution

The editor reads/writes the same YAML format — visual editing and YAML editing are interchangeable.

### Implementation: React Flow

[React Flow](https://reactflow.dev/) is the standard library for node-based graph editors:
- 20k+ GitHub stars, actively maintained
- Built for React, TypeScript-first
- Custom node types, handles, edges
- Minimap, controls, background grid
- Used by: Stripe, n8n, Langflow, many others

---

## Conversational Workflow Formats

For conversational workflows (current recipes), support multiple formats:

| Format | Extension | Use Case |
|--------|-----------|----------|
| Markdown | `.md` | Simple prompts with instructions |
| MDX | `.mdx` | Markdown + YAML frontmatter for metadata |
| YAML | `.yaml` | Current recipe format (keep as-is) |
| JSON | `.json` | Programmatic generation |
| TOON | `.toon` | Token-efficient for LLM consumption |

### TOON Format

[TOON](https://github.com/toon-format/toon) is an LLM-optimized data format that:
- Uses **~40% fewer tokens** than JSON with equal or better accuracy
- Combines YAML-like indentation with CSV-style tabular arrays
- Deterministic, lossless JSON round-trips
- Multi-language ecosystem (TypeScript, Python, Go, Rust)

Useful for:
- Storing evaluation datasets (tabular test cases)
- Large recipe parameter sets
- Analytics data export/import

---

## Goose's 3-Tier Architecture

```
┌─────────────────────────────────────────────────────┐
│  Tier 1: Interfaces                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ Desktop  │ │   CLI    │ │   Web    │  (future)   │
│  │ Electron │ │  goose   │ │ browser  │             │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘            │
│       │             │            │                   │
├───────┼─────────────┼────────────┼───────────────────┤
│  Tier 2: Server                                      │
│  ┌──────────────────────────────────────────┐        │
│  │           goosed (goose-server)           │        │
│  │  ┌──────┐  ┌──────┐  ┌──────┐           │        │
│  │  │ ACP  │  │ A2A  │  │ REST │           │        │
│  │  └──────┘  └──────┘  └──────┘           │        │
│  │  Sessions | Analytics | Recipes | Eval   │        │
│  └────────────────────┬─────────────────────┘        │
│                       │                              │
├───────────────────────┼──────────────────────────────┤
│  Tier 3: Agents & Tools                              │
│  ┌────────────────────┴───────────────────────┐      │
│  │              IntentRouter                   │      │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────┐  │      │
│  │  │Goose │ │Coding│ │  QA  │ │ Research │  │      │
│  │  │Agent │ │Agent │ │Agent │ │  Agent   │  │      │
│  │  └──┬───┘ └──┬───┘ └──┬───┘ └────┬─────┘  │      │
│  │     │        │        │          │         │      │
│  │  ┌──┴────────┴────────┴──────────┴──┐      │      │
│  │  │     MCP Extensions (Tools)        │      │      │
│  │  │  developer | fetch | context7 ... │      │      │
│  │  └──────────────────────────────────┘      │      │
│  └────────────────────────────────────────────┘      │
└──────────────────────────────────────────────────────┘
```

The UX redesign affects **Tier 1 only** — the navigation, prompt bar, and generative UI are purely frontend concerns. The DAG executor would be a Tier 2 addition.

---

## Implementation Phases

### Phase 1: Navigation Restructure (P1)
- Consolidate sidebar: 9 items → 4 zones
- Each zone has sub-tabs
- Move existing components into new structure
- **No new features** — just reorganization
- *Depends on*: nothing
- *Effort*: ~1 week

### Phase 2: Persistent Prompt Bar (P1)
- Extract `ChatInput` from page-level to layout-level
- Make it context-adaptive (different behavior per zone)
- Add slash command system
- **Foundation for Generative UI**
- *Depends on*: Phase 1
- *Effort*: ~2 weeks

### Phase 3: Generative UI (P2)
- Integrate json-render component catalog
- Define goose-specific components (MetricCard, Chart, DataTable, etc.)
- Wire prompt bar to generate structured UI responses
- *Depends on*: Phase 2
- *Effort*: ~3 weeks

### Phase 4: Visual DAG Builder (P2)
- Install React Flow
- Implement node types (agent, tool, condition, human, etc.)
- Implement pipeline YAML serialization/deserialization
- Add pipeline executor in Rust (Tier 2)
- *Depends on*: Phase 1
- *Effort*: ~4-6 weeks

---

## Design Decisions (Resolved)

### Sessions are project-bound
- Sessions are always bound to a **project** (working directory)
- A **default "General" project** at `$HOME` exists for any-purpose chats (immutable)
- Users can **create projects** by opening a directory (equivalent to `cd` + start working)
- Sessions are **grouped by project** in the sidebar Chat section
- Projects can have **multiple active sessions** for multi-tasking

### "Extensions" → Catalogs
The "Extensions" concept evolves into **three Catalogs**:

| Catalog | Replaces | What |
|---------|----------|------|
| **Agents Catalog** | "Agents" page | Browse/install/configure agents with modes |
| **Tools Catalog** | "Extensions" page | Browse/install MCP servers and tools |
| **Workflows Catalog** | "Recipes" page | Browse/install conversational + DAG workflows |

"Extension" is a developer term — users think in terms of **tools** they can use.
"Recipes" are just one kind of **workflow**. The three catalogs represent
the three fundamental building blocks users care about.

### Active interfaces
Goose has a 3-tier architecture (Interface → Server → Agents/Tools).
Actively maintained interfaces: **Desktop** (Electron) and **CLI**.
Web interface is possible future work (same server API).

---

## Open Questions

1. **TOON adoption**: Worth adding TOON support for recipe format, or just use it for eval datasets?
2. **Pipeline executor**: Build in Rust (Tier 2) or delegate to agents (Tier 3)?
3. **json-render integration**: Render in chat bubbles or in a separate response panel?
4. **Prompt bar routing**: How to decide if input goes to chat vs command vs analytics query?
5. **Catalog registry**: Where do shareable packages live? Local filesystem? Git repo? Dedicated registry?
6. **Catalog versioning**: How to pin versions? Lockfile like `goose.lock`?
