---
markmap:
  colorFreezeLevel: 3
  maxWidth: 300
---

# Goose Multi-Agent Roadmap

## Phase 1 — Stabilization ✅/🔧
### ✅ Done (This Review)
#### ✅ RunStore mutex consolidation
- 4 mutexes → 1 `RunStoreInner`
- TOCTOU race fix (atomic `take_await_if_awaiting`)
- LRU eviction (cap 1000)
#### ✅ ACP Discovery A2A alignment
- 15 flattened agents → 2 personas with modes
- `AgentManifest.modes: Vec<AgentModeInfo>`
- 6 new tests
#### ✅ AcpIdeSessions eviction
- `last_activity` tracking
- LRU cap at 100 sessions
#### ✅ OpenAPI + TS codegen
- `AgentModeInfo` registered
- TypeScript types regenerated
#### ✅ SSE parser dedup
- `GoosedHandle` uses shared `process_sse_buffer`
#### ✅ Dynamic A2A agent card
- Generated from IntentRouter slots
- Skills from all agent personas
#### ✅ process_sse_buffer tests
- 6 tests: single, multi, partial, malformed, empty, non-data

### 🔧 Remaining (This Sprint)
#### 🔧 Run lifecycle integration test (QA-1)
- create → stream → await → resume → complete
- **P1 · 4h**
#### 🔧 Split goosed_client.rs (RUST-3)
- client.rs, handle.rs, process.rs, sse.rs
- **P2 · 4h**

## Phase 2 — Structural Improvements
### A2A Interop
#### SessionModeState in NewSessionResponse (INT-2)
- External ACP clients can't discover modes
- **P1 · 4h**
#### Goosed process discovery (INT-3)
- PID file: `~/.config/goose/goosed.pid`
- CLI reuses existing server
- **P2 · 8h**

### Error Handling
#### Replace bare 500s (RUST-2)
- ~50 `.map_err(|_| 500)?` calls
- Structured `ApiError` with codes
- **P2 · 8h**
#### Rate limiting on /runs (SEC-1)
- tower-governor or axum-limit
- **P2 · 4h**

### Code Quality
#### Move `apply_agent_bindings` to goose crate (C4-1)
- Domain logic in server routes
- **P2 · 4h**
#### Split `useChatStream.ts` (REACT-3)
- 860 LOC → stream parsing + state mgmt
- **P2 · 4h**
#### Clean 347 console.logs
- **P3 · 4h**

### Observability
#### OTel spans in routing (OBS-1)
- Intent → Agent → Mode → Completion
- **P2 · 8h**
#### Emit `AgentEvent::PlanCreated` (OBS-2)
- Orchestrator plan visible in UI
- **P3 · 2h**

## Phase 3 — Architectural Evolution
### Agent Persona Extraction
#### Extract QA Agent from CodingAgent
- Own test strategies, coverage analysis
- Modes: analyze, test-design, coverage-audit, review
- **P1 · 2w**
#### Extract PM Agent from CodingAgent
- Own roadmap, prioritization
- Modes: roadmap, prioritize, impact-analysis
- **P2 · 1w**
#### Extract Security Agent
- Own threat modeling, SAST
- Modes: analyze, audit, review
- **P2 · 1w**
#### Add UXR/UI Agent
- Double diamond, usability audit
- Modes: research, synthesize, design-review
- **P3 · 2w**
#### Add Web Research Agent
- DuckDuckGo + citations
- Modes: explore, compare, validate, summarize
- **P3 · 2w**

### Infrastructure
#### Knowledge graph for coverage
- Track what's been reviewed/tested
- **P2 · 2w**
#### Deprecate GooseAgent "specialist" mode
- After routing matures
- **P3 · 1w**
#### Full OTel tracing pipeline
- Every routing decision traced
- **P2 · 2w**
#### Squash commits before merge
- Interactive rebase to ~10-15 logical commits
- **P3 · Pre-merge**
