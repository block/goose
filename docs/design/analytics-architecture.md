# Analytics & Evaluation Architecture

## Overview

The analytics system provides comprehensive monitoring, evaluation, and performance tracking
for Goose's AI agent orchestration platform.

## Backend Modules

### 1. `eval_storage.rs` (~580 lines)
**Location**: `crates/goose/src/session/eval_storage.rs`

SQLite-backed persistence for evaluation datasets and runs:
- **Tables**: `eval_datasets`, `eval_test_cases`, `eval_runs` (schema v8)
- **Dataset CRUD**: Create/read/update/delete test datasets with tagged cases
- **Run execution**: Evaluates datasets against IntentRouter, stores results as JSON
- **Metrics**: Overall/agent/mode accuracy, confusion matrices, regression detection
- **Overview**: Accuracy trends over time, delta from previous run
- **Topics**: Tag-based accuracy grouping with agent distribution

### 2. `tool_analytics.rs` (~530 lines)
**Location**: `crates/goose/src/session/tool_analytics.rs`

**Zero-instrumentation analytics** — extracts tool usage data from existing messages table
using SQLite JSON functions (`json_extract`, `json_each`):

**JSON structure in content_json**:
```json
[
  {"type":"toolRequest","id":"...","toolCall":{"status":"success","value":{"name":"developer__shell","arguments":{...}}}},
  {"type":"toolResponse","id":"...","toolResult":{"status":"success","value":{"content":[...],"isError":false}}}
]
```

**SQL query pattern**:
```sql
SELECT json_extract(tc.value, '$.toolCall.value.name') as tool_name, COUNT(*) as call_count
FROM messages m, json_each(m.content_json) tc
WHERE json_extract(tc.value, '$.type') = 'toolRequest'
  AND json_extract(tc.value, '$.toolCall.status') = 'success'
GROUP BY tool_name ORDER BY call_count DESC
```

**Provides**: Per-tool stats, extension breakdown, daily activity, session summaries,
agent performance (provider stats, duration distributions)

### 3. `session_manager.rs` extensions
**Location**: `crates/goose/src/session/session_manager.rs`

- `SessionAnalytics`: Daily activity, provider usage, top directories (30-day window)
- Schema v8 migration: Creates eval tables

## API Endpoints (15 total)

### Eval Dataset CRUD
| Method | Path | Handler |
|--------|------|---------|
| GET | `/analytics/eval/datasets` | `list_datasets` |
| POST | `/analytics/eval/datasets` | `create_dataset` |
| GET | `/analytics/eval/datasets/{id}` | `get_dataset` |
| PUT | `/analytics/eval/datasets/{id}` | `update_dataset` |
| DELETE | `/analytics/eval/datasets/{id}` | `delete_dataset` |

### Eval Runs
| Method | Path | Handler |
|--------|------|---------|
| POST | `/analytics/eval/runs` | `run_eval` |
| GET | `/analytics/eval/runs` | `list_runs` |
| GET | `/analytics/eval/runs/{id}` | `get_run` |

### Eval Insights
| Method | Path | Handler |
|--------|------|---------|
| GET | `/analytics/eval/overview` | `get_overview` |
| GET | `/analytics/eval/topics` | `get_topics` |

### Tool Analytics
| Method | Path | Handler |
|--------|------|---------|
| GET | `/analytics/tools` | `get_tool_analytics` |
| GET | `/analytics/tools/agents` | `get_agent_performance` |

### Session Analytics
| Method | Path | Handler |
|--------|------|---------|
| GET | `/sessions/analytics` | `get_session_analytics` |

### Routing (pre-existing)
| Method | Path | Handler |
|--------|------|---------|
| POST | `/analytics/routing/inspect` | `inspect_routing` |
| POST | `/analytics/routing/eval` | `eval_routing` |
| GET | `/analytics/routing/catalog` | `catalog` |

## Frontend Components

### Analytics Page (`/analytics`) — 7 sub-tabs in 3 groups:

**Observe**:
- `AnalyticsDashboard.tsx` (~700 lines) — KPIs, accuracy trends, usage charts
- `ToolAnalyticsTab.tsx` (~330 lines) — Tool usage table, extensions, sessions

**Evaluate**:
- `EvalOverviewTab.tsx` (~250 lines) — KPI cards, accuracy trend, regressions
- `DatasetsTab.tsx` (~300 lines) — CRUD with inline editor + YAML mode
- `RunHistoryTab.tsx` (~280 lines) — Run list, detail, confusion matrix
- `TopicsTab.tsx` (~200 lines) — Tag-based accuracy analysis

**Configure** (pre-existing):
- `RoutingInspector.tsx` — Test single message routing
- `EvalRunner.tsx` — YAML-based eval runner
- `AgentCatalog.tsx` — Agent/mode listing

### Tools Health Page (`/tools`):
- `ToolsHealthView.tsx` (~387 lines) — KPIs, daily chart, tools table, extensions

### Catalogs Overview (`/catalogs`):
- `CatalogsOverview.tsx` (~380 lines) — Unified browse for 3 catalogs

## Data Flow

```
User Session → messages table (content_json with ToolRequest/ToolResponse)
                    ↓
            tool_analytics.rs (SQL + json_extract)
                    ↓
            /analytics/tools endpoint
                    ↓
            ToolAnalyticsTab / ToolsHealthView
```

```
Eval Dataset → eval_test_cases table
                    ↓
            eval_storage.rs → IntentRouter.score_all_modes()
                    ↓
            eval_runs table (metrics JSON)
                    ↓
            /analytics/eval/* endpoints
                    ↓
            EvalOverviewTab / DatasetsTab / RunHistoryTab / TopicsTab
```

## Remaining Work

1. **Live monitoring** (goose4-0ih) — WebSocket/SSE for real-time routing events
2. **Version comparison** (goose4-v9l) — Side-by-side eval run diffs
3. **Response quality** (goose4-7oa) — Quality metrics and time-saved KPIs
4. **Sankey diagram** (goose4-tm0) — Routing flow visualization
5. **Generative UI** (goose4-u46) — json-render for inline analytics
6. **DAG builder** (goose4-5jq) — Visual workflow editor
