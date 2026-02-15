# RFC: Extension-Agent Separation — ACP-Aligned Architecture

**Status:** Proposal  
**Author:** jmercier (with goose)  
**Date:** 2026-02-13  
**Depends on:** multi-layer-orchestrator.md

---

## 1. Problem Statement

Goose currently has a **monolithic coupling** between extensions (MCP services) and agents.
Every extension is loaded into a single `Agent` struct's `ExtensionManager`, and all agents
share the same extension pool. This violates ACP's architecture where:

> "Each agent declares its dependencies in its manifest, and the server wires services to agents."

### Current Architecture (Anti-Patterns)

```
┌──────────────────────────────────────────┐
│                  Agent                    │
│  ┌────────────────────────────────────┐  │
│  │       ExtensionManager             │  │
│  │  ┌─────────┐ ┌─────────┐          │  │
│  │  │developer│ │ memory  │ ← ALL    │  │
│  │  ├─────────┤ ├─────────┤   loaded │  │
│  │  │ todo    │ │ apps    │   for    │  │
│  │  ├─────────┤ ├─────────┤   ALL    │  │
│  │  │ summon  │ │  tom    │   modes  │  │
│  │  ├─────────┤ ├─────────┤          │  │
│  │  │chatrecall│ │code_exec│          │  │
│  │  └─────────┘ └─────────┘          │  │
│  └────────────────────────────────────┘  │
│  GooseAgent(assistant) uses ALL tools    │
│  GooseAgent(judge) uses ALL tools ←BUG  │
│  CodingAgent(backend) tool_filter only   │
└──────────────────────────────────────────┘
```

**Problems:**
1. **Every mode sees every tool** — GooseAgent modes have `tool_groups: vec![]` (empty = all pass)
2. **Judge mode has tool access** — A read-only mode shouldn't have shell access
3. **Platform extensions blur the agent/service boundary** — `summon`, `extensionmanager`, `tom` are orchestration concerns, not agent tools
4. **Extension loading is per-Agent, not per-mode** — All extensions load at startup regardless of active mode
5. **No manifest-driven wiring** — Extensions don't declare which agents need them

---

## 2. Analysis: Extensions as Agent Modes vs. MCP Services

### Current Extension Inventory

| Extension | Type | Location | Purpose | Should Be |
|-----------|------|----------|---------|-----------|
| **developer** | Builtin (goose-mcp) | DeveloperServer | Shell, editor, file ops | **MCP Service** ✅ |
| **memory** | Builtin (goose-mcp) | MemoryServer | Session memory store | **MCP Service** ✅ |
| **computercontroller** | Builtin (goose-mcp) | ComputerControllerServer | Screen/keyboard automation | **MCP Service** ✅ |
| **autovisualiser** | Builtin (goose-mcp) | AutoVisualiserRouter | Auto-screenshot on changes | **MCP Service** ✅ |
| **tutorial** | Builtin (goose-mcp) | TutorialServer | Onboarding guide | **MCP Service** ✅ |
| **todo** | Platform | TodoClient | Track task lists | **GooseAgent tool** → should be a mode or bound service |
| **apps** | Platform | AppsManagerClient | Create/iterate HTML apps | **GooseAgent mode dependency** — only app_maker/app_iterator need it |
| **chatrecall** | Platform | ChatRecallClient | Search past conversations | **Orchestrator service** — cross-session awareness |
| **extensionmanager** | Platform | ExtensionManagerClient | Add/remove extensions at runtime | **Orchestrator tool** — meta-management concern |
| **summon** | Platform | SummonClient | Delegate to specialists, load knowledge | **Orchestrator tool** — delegation is orchestration |
| **code_execution** | Platform | CodeExecutionClient | Batch multiple tool calls into single code execution, saving tokens | **Orchestrator/GooseAgent optimization** — meta-tool that wraps other MCP calls via CodeMode, not a standalone service |
| **tom** | Platform | TomClient | Inject top-of-mind context | **Orchestrator concern** — context injection is pre-routing |

### Verdict

| Category | Extensions | Rationale |
|----------|-----------|-----------|
| **True MCP Services** (agent-independent) | developer, memory, computercontroller, autovisualiser, tutorial | Generic tools any agent can use |
| **Orchestrator-owned** (should move up) | summon, extensionmanager, chatrecall, tom | Cross-agent concerns: delegation, meta-management, context injection |
| **Meta-optimization** (orchestrator or GooseAgent) | code_execution | Wraps other MCP tool calls into batched code execution via CodeMode — saves tokens by consolidating multiple tool invocations into one. Should be owned by whichever agent is executing (OrchestratorAgent or GooseAgent), not loaded globally |
| **Agent-mode-specific** (should bind via manifest) | apps (→ app_maker mode), todo (→ GooseAgent) | Only specific modes need them |

---

## 3. Target Architecture (ACP-Aligned)

```
┌────────────────────────────────────────────────────────────────┐
│  Layer 0: Extension Registry (MCP Service Pool)                │
│  ┌──────────┐ ┌──────────┐ ┌───────────────┐ ┌──────────┐    │
│  │developer │ │ memory   │ │computercontrol│ │autovisual│    │
│  └──────────┘ └──────────┘ └───────────────┘ └──────────┘    │
│  ┌──────────┐ ┌──────────┐                                    │
│  │ tutorial │ │ user MCP │ ← user-installed                   │
│  └──────────┘ └──────────┘                                    │
│  These are SERVICES — agents request them, server wires them  │
└──────────────────────────────┬─────────────────────────────────┘
                               │ ServiceBroker resolves
┌──────────────────────────────▼─────────────────────────────────┐
│  Layer 1: OrchestratorAgent (internal, mandatory)              │
│  Own tools: summon, extensionmanager, chatrecall, tom          │
│  Meta-optimization: code_execution (batches MCP calls,         │
│    saving tokens — owned by executing agent, not global)       │
│  Responsibilities:                                             │
│    - LLM routing / compound splitting                          │
│    - Context injection (TOM) before delegation                 │
│    - Chat recall for cross-session context                     │
│    - Extension discovery and management                        │
│    - Compaction coordination                                   │
│  Manifest declares: summon, extensionmanager, chatrecall, tom  │
└─────────────────┬──────────────────────┬───────────────────────┘
                  │ delegates             │ delegates
┌─────────────────▼───────────┐ ┌────────▼──────────────────────┐
│  GooseAgent (builtin)       │ │  CodingAgent (builtin)        │
│  Modes:                     │ │  Modes: pm, architect,        │
│    assistant → developer,   │ │    backend, frontend, qa,     │
│      memory, todo           │ │    security, sre, devsecops   │
│    app_maker → apps         │ │  Each mode declares           │
│    app_iterator → apps      │ │    tool_groups (already done!) │
│    recipe_maker → (none)    │ │  Manifest declares per-mode   │
│    planner → (none)         │ │    extension deps             │
│    judge → (none, read-only)│ │                               │
│    specialist → developer,  │ │                               │
│      memory                 │ │                               │
│  Manifest declares deps     │ │  Manifest declares deps       │
│  per-mode via tool_groups   │ │  per-mode via tool_groups     │
└─────────────────────────────┘ └───────────────────────────────┘
```

### Key Principle: "Agents ASK for services, Server PROVIDES them"

In ACP, the manifest's `metadata.dependencies` field lists what an agent needs.
The server (via ServiceBroker) resolves those to concrete MCP connections.

```json
{
  "name": "coding-agent",
  "metadata": {
    "dependencies": ["developer", "memory"],
    "capabilities": ["code_generation", "testing"]
  },
  "modes": [
    {
      "slug": "backend",
      "tool_groups": ["developer", "edit", "command", "mcp"],
      "dependencies": ["developer", "memory"]
    }
  ]
}
```

---

## 4. What Changes

### 4.1 Extensions that should become Orchestrator-owned

| Extension | Why | Migration |
|-----------|-----|-----------|
| **summon** | Delegation is orchestration. An agent shouldn't delegate to other agents — that's the orchestrator's job. | Move `load`/`delegate` tools from SummonClient into OrchestratorAgent's tool set |
| **extensionmanager** | Adding/removing extensions is a meta-concern. Individual agents shouldn't modify the extension pool. | Move to OrchestratorAgent. Agent can REQUEST extensions via manifest, not install them. |
| **chatrecall** | Cross-session memory is orchestrator-level context. Individual agents operate within a session. | Orchestrator queries chatrecall before delegation to inject relevant history. |
| **tom** | Top-of-mind context injection happens BEFORE routing. It's a pre-processing step. | Orchestrator reads TOM env vars and prepends to the prompt before delegating. |

### 4.2 Extensions that need manifest-based binding

| Extension | Current Binding | Target Binding |
|-----------|----------------|----------------|
| **apps** | Loaded for ALL modes | Only bound to `app_maker` and `app_iterator` modes |
| **todo** | Loaded for ALL modes | Only bound to GooseAgent `assistant` mode |
| **developer** | Loaded for ALL modes | Bound per manifest: CodingAgent all modes, GooseAgent specialist/assistant |
| **memory** | Loaded for ALL modes | Bound per manifest: most modes |

### 4.3 GooseAgent modes that should have tool_groups

Currently GooseAgent modes have `tool_groups: vec![]` (all tools pass). This should be:

| Mode | Should Have | Rationale |
|------|------------|-----------|
| **assistant** | `["developer", "memory", "todo", "mcp"]` | General access but explicit |
| **specialist** | `["developer", "memory", "mcp"]` | Like assistant but focused |
| **app_maker** | `["apps"]` | Only needs apps extension |
| **app_iterator** | `["apps"]` | Only needs apps extension |
| **recipe_maker** | `[]` | LLM-only, no tools |
| **planner** | `[]` | LLM-only, no tools |
| **judge** | `[]` | LLM-only, MUST NOT have tool access |

---

## 5. Separate Crates?

### Analysis

| Option | Pros | Cons |
|--------|------|------|
| **Keep agents in goose crate** | Simple, fast compilation, shared types | Coupling, harder to test in isolation |
| **Separate crate per agent** | Clean boundaries, independent testing, versioning | More crates to manage, cross-crate type sharing overhead |
| **goose-agents crate** (all agents together) | Clean boundary but agents share types naturally | Still need shared types crate |

### Recommendation: **Stay in `goose` crate for now, use module boundaries**

Reasons:
1. **GooseAgent and CodingAgent share `Agent` struct, `ExtensionManager`, `Provider`** — separating requires a lot of trait gymnastics
2. **The real boundary is the manifest** — ACP doesn't require separate binaries for agents. It requires agents to declare their capabilities/dependencies
3. **When to split**: If/when an agent needs a different runtime (different language, container isolation), create a separate crate that exposes it as an ACP server
4. **OrchestratorAgent is already in `goose` crate** — and it's the most "special" agent. If anything splits first, it should be the external agents

### Module-level separation (do now):

```
crates/goose/src/
├── agents/
│   ├── mod.rs                    ← public API surface
│   ├── agent.rs                  ← Agent struct (runtime engine)
│   ├── orchestrator_agent.rs     ← OrchestratorAgent (routing, splitting)
│   ├── goose_agent.rs            ← GooseAgent modes + manifests
│   ├── coding_agent.rs           ← CodingAgent modes + manifests
│   ├── tool_filter.rs            ← Tool group enforcement
│   ├── extension_manager.rs      ← MCP connection management
│   ├── manifest.rs (new)         ← Agent manifest generation (ACP format)
│   └── service_binding.rs (new)  ← Per-mode extension binding logic
├── agent_manager/
│   ├── service_broker.rs         ← Resolve manifest deps → MCP services
│   ├── acp_mcp_adapter.rs        ← Bidirectional protocol bridge
│   └── ...
```

---

## 6. Implementation Plan

### Phase 1: Add tool_groups to GooseAgent modes (LOW RISK)
- Add `tool_groups` to each `BuiltinMode` in `goose_agent.rs`
- This immediately scopes what tools each mode can see
- Zero behavior change for `assistant` mode (it gets `["mcp"]` = all)
- **Critical fix**: `judge` mode gets `[]` → no tool access

### Phase 2: Manifest-based extension binding
- Add `dependencies` field to `BuiltinMode` (maps to ACP manifest)
- ServiceBroker resolves mode dependencies when mode is activated
- Only load extensions needed for the active mode

### Phase 3: Migrate orchestrator extensions
- Move summon/extensionmanager/chatrecall/tom logic into OrchestratorAgent
- OrchestratorAgent runs TOM + chatrecall as pre-processing before delegation
- SummonClient's `delegate` → OrchestratorAgent's native delegation
- SummonClient's `load` → stays as MCP (knowledge loading is a service)

### Phase 4: Per-agent ExtensionManager instances
- Each agent gets its OWN ExtensionManager with only the extensions it needs
- Extensions are shared across agents (reference counting) but access is scoped
- This aligns with ACP: each agent session has its own service bindings

---

## 7. Open Questions

1. **Should `summon.load` stay as MCP?** It loads knowledge into context — this is arguably
   a service, not orchestration. Could be an MCP service that multiple agents use.

2. **TOM injection timing**: Currently TOM injects at every turn. Should it inject
   once at session start (orchestrator pre-processing) or every turn (agent-level)?

3. **Backward compatibility**: Users have `config.yaml` with extension configs.
   How to migrate without breaking existing setups?

4. **External agents**: When an external ACP agent declares dependencies, who resolves?
   Currently ServiceBroker does this at `connect_agent` time — is that enough?

---

## 8. Summary

| What | Current | Target | ACP Alignment |
|------|---------|--------|---------------|
| Extension loading | All loaded for all agents | Per-agent, per-mode via manifest | ✅ manifest.dependencies |
| Tool visibility | GooseAgent: all tools visible | Scoped by tool_groups per mode | ✅ mode.tool_groups |
| Orchestration tools | summon/extensionmgr as extensions | Part of OrchestratorAgent | ✅ orchestrator pattern |
| Context injection | TOM as platform extension | Orchestrator pre-processing | ✅ router agent pattern |
| Agent separation | All in goose crate | Module boundaries + manifest | ✅ can split to crate later |
| Service wiring | Static config.yaml | ServiceBroker resolves at runtime | ✅ registry-based discovery |
