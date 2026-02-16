# Agent Analytics & Evaluation — Backlog

> Inspired by [Copilot Studio Analytics](https://learn.microsoft.com/en-us/microsoft-copilot-studio/analytics-agent-evaluation-create)
> and Goose's multi-agent orchestration needs.

## Vision

A local-first, privacy-preserving analytics system for monitoring, evaluating,
and improving agent orchestration quality. Includes:
- **Routing accuracy** measurement against ground truth test sets
- **Live feedback loops** (user thumbs-up/down + LLM judge)
- **Automatic misrouting detection** from negative feedback
- **Analytics dashboard** in the Electron desktop app

---

## Existing Foundation

| Component | Status | Location |
|---|---|---|
| IntentRouter (keyword) | ✅ Done | `agents/intent_router.rs` |
| OrchestratorAgent (LLM) | ✅ Done | `agents/orchestrator_agent.rs` |
| OTel tracing spans | ✅ Done | `orchestrator.route`, `intent_router.route` |
| Routing eval framework | ✅ Done | `agents/routing_eval.rs` |
| SSE routing decision events | ✅ Done | `routes/reply.rs` |
| `when_to_use` on all modes | ✅ Done | `goose_agent.rs`, `coding_agent.rs` |
| `is_internal` mode filtering | ✅ Done | `acp_discovery.rs`, `agent_card.rs` |

---

## Backlog Items (RPI Process)

### BL-1: Analytics Server Endpoints (Phase 1)

**Research:**
- Review Copilot Studio analytics API patterns
- Review OpenAI evals framework structure
- Study Langfuse/Braintrust/Arize evaluation APIs

**Plan:**
- `POST /analytics/routing/inspect` — route a single message, return full decision
- `POST /analytics/routing/eval` — run eval test set, return metrics + report
- `GET /analytics/routing/history?limit=N` — recent routing decisions (in-memory ring buffer)
- Add `RoutingAnalytics` struct to goose-server state (ring buffer of last N decisions)

**Implement:**
- New route file: `crates/goose-server/src/routes/analytics.rs`
- Wire to `routing_eval.rs` framework
- OpenAPI spec generation via `just generate-openapi`

**Effort:** S-M (1-2 days)

---

### BL-2: Analytics UI Page (Phase 1)

**Research:**
- Review Copilot Studio dashboard layout (see `copilot_studio_analytics.jpg`)
- Review Recharts/Nivo for charting in React
- Study existing Goose UI patterns (AgentsView, BaseChat)

**Plan:**
- New route in Electron app: `/analytics`
- 3 tabs: Overview, Routing Inspector, Evaluation Runner
- Routing Inspector: text input → live routing decision display
- Eval Runner: load test set, run, display per-agent/per-mode accuracy bars
- Overview: mode frequency, confidence distribution, recent decisions table

**Implement:**
- `ui/desktop/src/components/analytics/AnalyticsView.tsx`
- `ui/desktop/src/components/analytics/RoutingInspector.tsx`
- `ui/desktop/src/components/analytics/EvalRunner.tsx`
- `ui/desktop/src/components/analytics/OverviewDashboard.tsx`
- Add to sidebar navigation

**Effort:** M-L (2-4 days)

---

### BL-3: Live User Feedback (Phase 2)

**Research:**
- How does Copilot Studio capture CSAT per-turn?
- Review RLHF feedback collection patterns
- Study thumbs-up/down UX patterns (ChatGPT, Claude, Gemini)

**Plan:**
- Add 👍/👎 buttons to each `GooseMessage` (assistant responses)
- Feedback payload: `{ session_id, message_id, agent, mode, rating: +1/-1, comment? }`
- Store locally in SQLite or JSON append-log
- Negative feedback triggers: "Was the wrong agent assigned?" prompt
  - User can correct: "This should have been handled by [Coding Agent / security mode]"
  - Stores correction as ground truth for future eval

**Implement:**
- `POST /analytics/feedback` — store user rating
- `POST /analytics/feedback/correction` — store routing correction
- UI: thumbs buttons on GooseMessage + correction dialog
- Export corrections as YAML eval test cases for `routing_eval.rs`

**Effort:** M (2-3 days)

---

### BL-4: LLM Judge for Routing Quality (Phase 2)

**Research:**
- Review LLM-as-judge patterns (OpenAI evals, Anthropic model grading)
- Study "was this the right agent?" classification prompts
- Review Copilot Studio's automatic topic evaluation

**Plan:**
- After each conversation turn, optionally run an LLM judge:
  - Input: user message + routing decision + agent response
  - Output: `{ routing_correct: bool, suggested_agent?, suggested_mode?, reasoning }`
- Judge prompt template:
  ```
  Given this user request: "{user_message}"
  The system routed to: {agent}/{mode}
  The agent responded: "{response_summary}"

  Was the routing decision correct? If not, which agent/mode
  should have handled this? Explain briefly.
  ```
- Store judge verdicts alongside routing decisions
- Aggregate judge accuracy as a quality signal

**Implement:**
- `agents/routing_judge.rs` — LLM judge prompt + parsing
- Configuration: `GOOSE_ROUTING_JUDGE=true/false` (disabled by default)
- Background task: judge runs async after response, doesn't block user
- Results feed into analytics dashboard

**Effort:** M (2-3 days)

---

### BL-5: Automatic Misrouting Detection (Phase 3)

**Research:**
- Pattern: user sends follow-up "no, I meant X" → detect intent correction
- Pattern: user explicitly says "switch to [mode]" → implicit misrouting signal
- Pattern: agent fails (error, no useful output) → potential misrouting
- Study conversation repair detection in dialogue systems

**Plan:**
- Heuristic signals for misrouting:
  1. **User correction**: "no", "not what I asked", "I meant..." within 2 turns
  2. **Mode switch**: user explicitly requests different agent/mode
  3. **Agent failure**: tool errors, empty responses, context limit exceeded
  4. **Low confidence routing**: decisions with confidence < 0.3
  5. **LLM judge negative**: judge says routing was wrong (BL-4)
- Aggregate signals into a "misrouting score" per routing decision
- Surface in analytics dashboard as "suspected misroutes"

**Implement:**
- `agents/misrouting_detector.rs` — signal aggregation
- Hook into SSE event stream for real-time detection
- Dashboard widget: "Suspected Misroutes (last 7 days)"
- Auto-generate eval test cases from detected misroutes

**Effort:** M-L (3-4 days)

---

### BL-6: Eval Test Set Management (Phase 3)

**Research:**
- How does Copilot Studio manage test conversations?
- Review pytest-benchmark-style test set management
- Study golden dataset curation patterns

**Plan:**
- UI for managing YAML test sets:
  - View/edit existing test cases
  - Add new cases from routing history
  - Import from user corrections (BL-3)
  - Import from LLM judge corrections (BL-4)
  - Import from misrouting detections (BL-5)
- Version test sets alongside code (in `tests/eval/`)
- Run eval on CI (GitHub Actions) to track routing quality over time

**Implement:**
- `ui/desktop/src/components/analytics/TestSetEditor.tsx`
- `POST /analytics/eval/test-sets` — CRUD for test sets
- CI job: `cargo test -p goose -- routing_eval` with quality gate

**Effort:** M (2-3 days)

---

### BL-7: Session Outcome Tracking (Phase 3)

**Research:**
- How does Copilot Studio define "resolved" vs "escalated" vs "abandoned"?
- Review conversation outcome classification patterns
- Study session quality metrics in customer service platforms

**Plan:**
- Track per-session outcomes:
  - **Resolved**: user's goal achieved (positive feedback or natural end)
  - **Escalated**: user switched modes or asked for different approach
  - **Abandoned**: session ended without completion
  - **Error**: agent hit context limit, tool failure, etc.
- Aggregate per-agent and per-mode
- Surface as "Resolution Rate" in overview dashboard

**Implement:**
- `agents/session_analytics.rs` — outcome classification
- `GET /analytics/sessions/stats` — aggregated metrics
- Overview dashboard: resolution rate chart per mode

**Effort:** M (2-3 days)

---

## Feedback Loop Architecture

```
                         ┌──────────────┐
                         │ User Message │
                         └──────┬───────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  OrchestratorAgent    │
                    │  route() → decision  │──→ OTel Span
                    └───────────┬───────────┘   (recorded)
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Agent processes      │
                    │  (tool calls, reply)  │
                    └───────────┬───────────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
            ┌──────────────┐     ┌──────────────────┐
            │ User Feedback│     │ LLM Judge (async)│
            │ 👍/👎 + fix  │     │ "Was this right?"│
            └──────┬───────┘     └────────┬─────────┘
                   │                       │
                   ▼                       ▼
            ┌──────────────────────────────────────┐
            │         Feedback Store               │
            │  (corrections, judge verdicts,       │
            │   misrouting signals)                │
            └──────────────┬───────────────────────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
    ┌─────────────────┐     ┌───────────────────────┐
    │ Auto-generate   │     │ Analytics Dashboard    │
    │ eval test cases │     │ (accuracy, confusion,  │
    │ (YAML export)   │     │  misroute alerts)      │
    └─────────────────┘     └───────────────────────┘
              │
              ▼
    ┌─────────────────┐
    │ CI Quality Gate │
    │ routing_eval    │
    │ regression test │
    └─────────────────┘
```

---

## Priority Matrix

| ID | Name | Impact | Effort | Priority |
|---|---|---|---|---|
| BL-1 | Analytics Server Endpoints | High | S-M | **P1** |
| BL-2 | Analytics UI Page | High | M-L | **P1** |
| BL-3 | Live User Feedback | High | M | **P2** |
| BL-4 | LLM Judge | Medium | M | **P2** |
| BL-5 | Misrouting Detection | Medium | M-L | **P3** |
| BL-6 | Test Set Management | Medium | M | **P3** |
| BL-7 | Session Outcome Tracking | Medium | M | **P3** |

---

## Key Design Decisions

1. **Local-first**: All analytics data stays on user's machine. No telemetry sent externally.
2. **YAML-native**: Test sets are YAML files, version-controlled alongside code.
3. **Dual strategy**: Keyword router for baseline, LLM orchestrator for quality. Eval measures both.
4. **Feedback → Ground Truth**: User corrections automatically become eval test cases.
5. **LLM Judge optional**: Disabled by default (cost/latency), enabled per-session.
6. **Non-blocking**: Judge + analytics run async, never slow down the user.
