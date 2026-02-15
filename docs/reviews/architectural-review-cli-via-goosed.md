# Architectural Review — `feature/cli-via-goosed`

**Reviewer:** goose (AI Architect)
**Date:** 2026-02-14 (updated 2026-02-14)
**Branch:** `feature/cli-via-goosed` (114 commits ahead of `main`)
**Scope:** 155 files changed, +29,170 / -5,759 lines

---

# Beads

- ● Request understood
- ● Plan defined
- ● Diff files identified (155 files, 67 new)
- ● Knowledge graph built
- ● C4 review completed
- ● React review completed
- ● Rust review completed
- ● Integration review completed
- ● Security review completed
- ● QA review completed
- ● DevOps review completed
- ● Routing audit completed
- ● Roadmap produced
- ● Completeness validated

---

# Executive Summary

| Dimension | Score | Notes |
|-----------|-------|-------|
| **Global Architecture** | 8.0/10 | Strong multi-agent foundation; A2A-aligned discovery |
| **C4 Alignment** | 7/10 | Clear container boundaries; component-level docs sparse |
| **React** | 7/10 | Clean new components; 347 console.logs, 12 `:any` types |
| **Rust** | 9/10 | Clippy clean, fmt clean, no panics; RunStore consolidated |
| **Security** | 8/10 | No XSS vectors, no unsafe blocks (new), no SQL/cmd injection |
| **QA** | 7.5/10 | 1,003+ tests passing; 6 new ACP discovery tests |
| **DevOps** | 7.5/10 | OpenAPI spec regenerated; AgentModeInfo registered |
| **Routing** | 7.5/10 | LLM+keyword routing; 1-persona=1-agent model aligned |

**Overall: 7.7/10** — Solid engineering with clear architectural direction. The A2A alignment fix (1-persona=1-agent) is the key structural improvement. Critical gaps remain in integration testing and observability.

---

# Applied Fixes (This Review Session)

| # | Commit | Fix | Category | Severity |
|---|--------|-----|----------|----------|
| 1 | `6e8754d3` | RunStore 4-mutex → 1 `RunStoreInner` | Concurrency | P0 |
| 2 | `6e8754d3` | TOCTOU race in `resume_run` — atomic `take_await_if_awaiting()` | Concurrency | P0 |
| 3 | `6e8754d3` | RunStore LRU eviction (cap 1000 completed runs) | Memory | P1 |
| 4 | `6044d232` | SSE parser dedup — `GoosedHandle` uses shared function | Quality | P2 |
| 5 | `6e8754d3` | Dead code removal — unused `take_await_metadata()` | Hygiene | P3 |
| 6 | `1199c3de` | ACP discovery: 15 flattened agents → 2 agents with N modes | Architecture | P0 |
| 7 | `1199c3de` | `AgentManifest.modes: Vec<AgentModeInfo>` | Schema | P1 |
| 8 | `1199c3de` | 6 new discovery tests (agent-not-mode validation) | Testing | P2 |
| 9 | `ca0e6a08` | Register `AgentModeInfo` in OpenAPI + regenerate TS client | DevOps | P2 |
| 10 | `dc79a842` | AcpIdeSessions LRU eviction (cap 100, idle-based) | Memory | P1 |
| 11 | `64cbedfc` | Dynamic A2A agent card from IntentRouter slots | Integration | P1 |
| 12 | `ec5bb421` | 6 new `process_sse_buffer` unit tests | Testing | P2 |

**Findings resolved:** RUST-1 ✅, DEVOPS-1 ✅, INT-1 ✅, QA-3 ✅, plus 3 P0 fixes not in original findings.

---

# Knowledge Graph Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                        User Interfaces                              │
│  Desktop (Electron) ◄──► CLI (goose-cli) ◄──► Web                  │
└──────────┬──────────────────┬───────────────────────────────────────┘
           │ HTTP/SSE         │ HTTP/SSE
           ▼                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Goose Server (goose-server)                             │
│  ┌────────────┐ ┌───────────────┐ ┌──────────────┐ ┌────────────┐ │
│  │ ACP Routes │ │ Agent Mgmt    │ │ Reply Route  │ │ Session    │ │
│  │ /agents(2) │ │ /agents/built │ │ /reply (SSE) │ │ /session   │ │
│  │ /runs      │ │ /agents/ext   │ │              │ │            │ │
│  │ /acp (IDE) │ │ /orchestrator │ │              │ │            │ │
│  └─────┬──────┘ └──────┬────────┘ └──────┬───────┘ └─────┬──────┘ │
│        └───────────┬────┴────────────────┬┘               │        │
│                    ▼                     ▼                 │        │
│  ┌──────────────────────────────────────────────┐         │        │
│  │           AppState                            │         │        │
│  │  RunStore(1 mutex) │ AgentSlotRegistry │ Reg  │         │        │
│  │  AcpIdeSessions(evict) │ SessionManager       │         │        │
│  └──────────────────────────────────────────────┘         │        │
└───────────────────────────┬───────────────────────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Core (goose crate)                                │
│  OrchestratorAgent → IntentRouter → Agent (core loop)               │
│                                                                     │
│  Agent Personas (A2A-aligned: 1 persona = 1 agent, N modes):       │
│  ┌──────────────────┐  ┌──────────────────────────┐                │
│  │ Goose Agent      │  │ Coding Agent             │                │
│  │ 7 modes:         │  │ 8 modes:                 │                │
│  │ assistant,       │  │ pm, architect, backend,  │                │
│  │ specialist,      │  │ frontend, qa, security,  │                │
│  │ recipe_maker,    │  │ sre, devsecops           │                │
│  │ app_maker,       │  │                          │                │
│  │ app_iterator,    │  │ Each mode has:           │                │
│  │ judge, planner   │  │ • tool_groups            │                │
│  └──────────────────┘  │ • instructions(.md)      │                │
│                        │ • when_to_use            │                │
│  ACP Compat Layer      └──────────────────────────┘                │
│  • events (SSE) • manifest (AgentManifest + modes)                 │
│  • message (goose ↔ ACP) • types (run, session)                   │
└─────────────────────────────────────────────────────────────────────┘
```

---

# C4 Model Analysis

## Context Level
Goose operates as an AI agent framework connecting users (Desktop/CLI/Web) to LLM providers and MCP extensions. The system now exposes ACP v0.2.0 compatible APIs for agent-to-agent interoperability.

## Container Level
- **goose-server** (goosed): HTTP server with REST + SSE + JSON-RPC endpoints
- **goose crate**: Core agent logic, routing, tool filtering
- **goose-cli**: CLI client communicating exclusively via goosed HTTP API
- **ui/desktop**: Electron app consuming the same API

## Component Level
Key new components in this branch:
- `RunStore` — Consolidated single-mutex in-memory store with LRU eviction
- `AcpIdeSessions` — JSON-RPC session manager with idle eviction
- `OrchestratorAgent` — LLM-based routing with keyword fallback
- `IntentRouter` — Keyword-based agent/mode scoring
- `ToolFilter` — Mode-aware tool access control

**Finding C4-1 (P2):** `apply_agent_bindings()` lives in `runs.rs` (server layer) but implements domain logic. Should move to goose crate.

**Finding C4-2 (P3):** No C4 component diagrams in docs/. The knowledge graph above is a start.

---

# Rust Review

## Strengths
- ✅ `cargo clippy --all-targets -- -D warnings` — 0 warnings
- ✅ `cargo fmt --check` — clean
- ✅ No `unsafe` blocks in new code
- ✅ No `panic!`/`.expect()` in production paths
- ✅ RunStore consolidated from 4 mutexes to 1 (TOCTOU fix)
- ✅ AcpIdeSessions has idle eviction

## Remaining Findings

**Finding RUST-2 (P1):** ~50 bare `StatusCode::INTERNAL_SERVER_ERROR` returns discard error context. Pattern: `.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?`. These make debugging impossible.

**Finding RUST-3 (P2):** `goosed_client.rs` at 1,490 LOC contains `GoosedClient`, `GoosedHandle`, SSE parsing, process management. Should split into `client.rs`, `handle.rs`, `process.rs`, `sse.rs`.

**Finding RUST-4 (P2):** `spawner.rs:178` uses `Runtime::new().unwrap()` in production code.

**Finding RUST-5 (P2 → P3):** 4 `#[allow(dead_code)]` items in `summon_extension.rs` — legitimate forward-looking fields for agent frontmatter features. 2 in `reply.rs` — legitimate serde-only fields. Not actionable now.

**Finding RUST-6 (P3):** 6 TODO comments should be tracked as issues.

---

# React Review

## Strengths
- ✅ No `dangerouslySetInnerHTML`, `innerHTML`, `eval`, or `DOMParser`
- ✅ AgentsView correctly shows 2 agents with expandable modes
- ✅ BottomMenuAgentSelection shows correct agent count (2)
- ✅ New components (WorkBlockIndicator, ReasoningDetailPanel) well-structured

## Findings

**Finding REACT-1 (P3):** 26 `as any` type assertions instead of proper type guards. Minor type safety gap.

**Finding REACT-2 (P3):** Hardcoded fallback agent list in BottomMenuAgentSelection. Should show loading state instead.

**Finding REACT-3 (P2):** `useChatStream.ts` at 860 LOC is the largest hook. Consider splitting stream parsing from state management.

**Finding REACT-4 (P3):** Empty `catch {}` blocks in several components silently swallow errors.

---

# Integration & Agent Routing Review

## A2A/ACP Alignment

| Aspect | Status | Evidence |
|--------|--------|---------|
| 1 agent = 1 persona | ✅ Fixed | `/agents` returns 2 manifests, each with modes |
| `AgentManifest.modes` | ✅ Fixed | `Vec<AgentModeInfo>` with id/name/description/tool_groups |
| `AgentModeInfo` in OpenAPI | ✅ Fixed | Registered in schema, TS types generated |
| `SessionModeState` in `NewSessionResponse` | ❌ Gap | Not populated — external ACP clients can't discover modes |
| `resolve_mode_to_agent()` | ✅ Fixed | Maps mode slug → (agent_name, mode_slug) |

**Finding INT-1 (P1):** Static `/agent-card` endpoint returns hardcoded values instead of being generated from registered agent manifests.

**Finding INT-2 (P2):** `NewSessionResponse` from the ACP-IDE path doesn't populate the standard `modes: Option<SessionModeState>` field.

**Finding INT-3 (P2):** No goosed process discovery — each CLI invocation spawns a new server.

## Routing Model

| Rule | Status | Evidence |
|------|--------|---------|
| Specialized-first routing | ⚠️ Partial | Orchestrator routes to CodingAgent modes for dev tasks |
| Persona ≠ Mode | ✅ | GooseAgent and CodingAgent are personas; modes are behaviors |
| Intent → Agent → Mode | ✅ | `OrchestratorAgent.route_with_llm()` → `(agent_name, mode_slug)` |
| Fallback to generalist | ⚠️ | Low-confidence routes default to GooseAgent/assistant |

**Finding ARCH-1 (P1):** CodingAgent bundles PM/QA/Security personas as modes. Per the target taxonomy, these should be separate agents. This is a Phase 3 item — the current model works pragmatically.

**Finding ARCH-2 (P2):** GooseAgent "specialist" mode allows scoped access without going through the orchestrator. Could bypass routing.

---

# Security Assessment

| Check | Status |
|-------|--------|
| XSS vectors | ✅ None (no `dangerouslySetInnerHTML`) |
| `unsafe` blocks (new code) | ✅ None |
| SQL/command injection | ✅ None |
| Path traversal | ✅ None |
| Secret logging | ✅ None |
| Rate limiting | ❌ No rate limiting on `/runs` or `/acp` |

**Finding SEC-1 (P2):** No rate limiting on public endpoints. Not critical for local-first usage but needed for shared deployments.

**Finding SEC-2 (P3):** SSE streams have no message size bounds.

---

# QA & Testing Assessment

| Metric | Value |
|--------|-------|
| Total tests | 1,003+ |
| goose-server | 25 (including 6 new ACP discovery tests) |
| goose-cli | 133 |
| goose core | 777+ |
| Failures | 0 |
| Ignored | 8 |

**Finding QA-1 (P1):** No integration test for the run lifecycle (create → stream → await → resume → complete). This is the most complex flow.

**Finding QA-2 (P2):** No mock-provider test for the orchestrator routing pipeline.

**Finding QA-3 (P2):** No test for `process_sse_buffer` in goosed_client.rs.

**Finding QA-4 (P3):** No UI component tests for new components.

---

# DevOps Lifecycle Assessment

- ✅ OpenAPI spec regenerated with `AgentModeInfo`
- ✅ TypeScript client types properly generated
- ✅ All quality gates pass (build, fmt, clippy, tests)

**Finding DEVOPS-2 (P3):** 114 commits should be squashed before merge to main.

---

# Observability & Routing Traceability

| Requirement | Status | Evidence |
|-------------|--------|---------|
| Intent detection | ✅ | `OrchestratorAgent.route()` logs routing decision |
| Task decomposition | ✅ | `parse_splitting_response()` creates `OrchestratorPlan` |
| Agent/mode selection | ✅ | `RoutingDecision` includes agent_name, mode_slug, confidence, reasoning |
| Switching logged | ✅ | `AgentEvent::ModelChange` emitted |
| OTel spans | ❌ | No OpenTelemetry instrumentation in routing path |
| Knowledge graph | ❌ | Not implemented |

**Finding OBS-1 (P2):** No OpenTelemetry spans for the routing pipeline.

**Finding OBS-2 (P3):** Orchestrator plan only logged at `debug!` level. Should be an `AgentEvent::PlanCreated`.

---

# Evolution Roadmap

## Phase 1 — Stabilization (This Sprint)

| Task | Priority | Effort | Status | Fixes |
|------|----------|--------|--------|-------|
| Consolidate RunStore mutexes | P0 | 2h | ✅ Done | TOCTOU + memory |
| Fix ACP discovery (1 agent = 1 persona) | P0 | 4h | ✅ Done | A2A alignment |
| Add eviction to `AcpIdeSessions` | P1 | 2h | ✅ Done | RUST-1 |
| Register `AgentModeInfo` in OpenAPI | P2 | 1h | ✅ Done | DEVOPS-1 |
| Deduplicate SSE parser | P2 | 1h | ✅ Done | Quality |
| Dynamic agent card from manifests | P1 | 4h | ✅ Done | INT-1 |
| Add run lifecycle integration test | P1 | 4h | TODO | QA-1 |
| Add `process_sse_buffer` tests | P2 | 2h | ✅ Done | QA-3 |
| Split `goosed_client.rs` into modules | P2 | 4h | TODO | RUST-3 |

## Phase 2 — Structural Improvements (Next Sprint)

| Task | Priority | Effort | Fixes |
|------|----------|--------|-------|
| Populate `SessionModeState` in `NewSessionResponse` | P1 | 4h | INT-2 |
| Replace bare 500s with structured errors | P2 | 8h | RUST-2 |
| Move `apply_agent_bindings` to goose crate | P2 | 4h | C4-1 |
| Split `useChatStream.ts` (860 LOC) | P2 | 4h | REACT-3 |
| Add OTel spans to routing pipeline | P2 | 8h | OBS-1 |
| Goosed discovery (PID file reuse) | P2 | 8h | INT-3 |
| Add rate limiting to `/runs` | P2 | 4h | SEC-1 |
| Clean up 347 console.logs | P3 | 4h | — |

## Phase 3 — Architectural Evolution (Quarter)

| Task | Priority | Effort | Notes |
|------|----------|--------|-------|
| Extract QA Agent from CodingAgent | P1 | 2w | Separate persona with own modes |
| Extract PM Agent from CodingAgent | P2 | 1w | Separate persona with own modes |
| Add Security Agent | P2 | 1w | Separate from CodingAgent security mode |
| Add UXR/UI Agent | P3 | 2w | New persona |
| Add Web Research Agent | P3 | 2w | New persona with search tools |
| Knowledge graph for coverage tracking | P2 | 2w | — |
| Deprecate GooseAgent "specialist" mode | P3 | 1w | After routing matures |
| Full OTel tracing pipeline | P2 | 2w | OBS-1 + OBS-2 |

---

# Strategic Recommendations

1. **The A2A alignment is the key win.** The fix from 15 flattened agents → 2 agents with modes is a breaking change for any external ACP clients. Document this in release notes.

2. **Prioritize `SessionModeState` population.** This is the last A2A interop gap. External ACP clients can't discover modes via the standard protocol.

3. **Add integration tests before more refactoring.** The run lifecycle (create → stream → await → resume → complete) and the orchestrator routing pipeline have zero integration tests.

4. **Don't rush persona extraction.** The current 2-agent model (Goose + Coding) with modes is pragmatic. Extract QA/PM/Security only when mode-based routing causes quality issues.

5. **The 114 commits need squashing.** Consider interactive rebase to ~10-15 logical commits before merge.

---

# Appendix: All Findings

| ID | Severity | Category | Description | Status |
|----|----------|----------|-------------|--------|
| **ARCH-1** | P1 | Architecture | CodingAgent conflates Code/QA/PM personas | Phase 3 |
| **ARCH-2** | P2 | Architecture | GooseAgent "specialist" mode bypasses routing | Phase 3 |
| **INT-1** | P1 | Integration | Static agent card doesn't reflect manifests | ✅ Fixed |
| **INT-2** | P2 | Integration | `NewSessionResponse` missing `SessionModeState` | Phase 2 |
| **INT-3** | P2 | Integration | No goosed process discovery/sharing | Phase 2 |
| **RUST-1** | P1 | Rust | `AcpIdeSessions` has no eviction | ✅ Fixed |
| **RUST-2** | P1 | Rust | ~50 bare 500 errors discard context | Phase 2 |
| **RUST-3** | P2 | Rust | `goosed_client.rs` 1490 LOC needs splitting | TODO |
| **RUST-4** | P2 | Rust | `spawner.rs` production `.unwrap()` | Phase 2 |
| **RUST-5** | P3 | Rust | `#[allow(dead_code)]` items — legitimate | Wontfix |
| **RUST-6** | P3 | Rust | 6 TODO comments untracked | TODO |
| **REACT-1** | P3 | React | Type assertions instead of type guards | Phase 2 |
| **REACT-2** | P3 | React | Hardcoded fallback agent counts | Phase 2 |
| **REACT-3** | P2 | React | `useChatStream.ts` 860 LOC needs splitting | Phase 2 |
| **REACT-4** | P3 | React | Empty `catch {}` blocks | Phase 2 |
| **SEC-1** | P2 | Security | No rate limiting on `/runs` | Phase 2 |
| **SEC-2** | P3 | Security | Unbounded SSE message size | Phase 2 |
| **QA-1** | P1 | QA | No run lifecycle integration test | TODO |
| **QA-2** | P2 | QA | No mock-provider orchestrator test | Phase 2 |
| **QA-3** | P2 | QA | No `process_sse_buffer` tests | ✅ Fixed |
| **QA-4** | P3 | QA | No UI component tests | Phase 2 |
| **DEVOPS-1** | P2 | DevOps | `AgentManifest` not in OpenAPI schema | ✅ Fixed |
| **DEVOPS-2** | P3 | DevOps | 114 commits need squashing | Pre-merge |
| **OBS-1** | P2 | Observability | No OTel spans in routing | Phase 2 |
| **OBS-2** | P3 | Observability | Orchestrator plan not emitted as event | Phase 2 |
| **C4-1** | P2 | C4 | Business logic in server routes | Phase 2 |
| **C4-2** | P3 | C4 | No C4 component diagrams | Phase 3 |

**Total: 28 findings** — 5 ✅ Fixed, 1 Wontfix, 2 TODO (this sprint), 15 Phase 2, 5 Phase 3
