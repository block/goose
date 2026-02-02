<div align="center">

# goose

_a state-of-the-art enterprise AI agent platform that automates engineering tasks_

<p align="center">
  <a href="https://opensource.org/licenses/Apache-2.0">
    <img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg">
  </a>
  <a href="https://discord.gg/goose-oss">
    <img src="https://img.shields.io/discord/1287729918100246654?logo=discord&logoColor=white&label=Join+Us&color=blueviolet" alt="Discord">
  </a>
  <a href="https://github.com/block/goose/actions/workflows/ci.yml">
     <img src="https://img.shields.io/github/actions/workflow/status/block/goose/ci.yml?branch=main" alt="CI">
  </a>
  <img src="https://img.shields.io/badge/tests-672%20passing-brightgreen" alt="Tests">
  <img src="https://img.shields.io/badge/rust-1.75+-orange" alt="Rust">
</p>

**Phase 6: Advanced Agentic AI Complete** | LangGraph-Style Checkpointing | ReAct Reasoning | Self-Improvement via Reflexion

</div>

---

## Overview

Goose is a **sophisticated enterprise AI agent framework** built in Rust, featuring advanced multi-agent orchestration, specialist agents, LangGraph-style state persistence, advanced reasoning patterns (ReAct, CoT, ToT), self-improvement capabilities, and enterprise workflow automation.

Whether you're building enterprise applications, managing complex development pipelines, or coordinating multiple AI agents for large-scale projects, goose provides the sophisticated orchestration and autonomous execution needed for modern software development.

[![Watch the video](https://github.com/user-attachments/assets/ddc71240-3928-41b5-8210-626dfb28af7a)](https://youtu.be/D-DpDunrbpo)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              AGENTIC GOOSE                                       │
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │                         GOOSE CORE (Rust)                                 │   │
│  │                                                                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │   │
│  │  │ StateGraph  │  │  Reasoning  │  │  Reflexion  │  │  Observability  │  │   │
│  │  │ Engine      │  │  (ReAct/CoT)│  │  Agent      │  │  & Cost Tracker │  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │   │
│  │         │                │                │                   │           │   │
│  │  ┌──────┴────────────────┴────────────────┴───────────────────┴──────┐   │   │
│  │  │                    Checkpoint Manager (SQLite/Memory)              │   │   │
│  │  └───────────────────────────────────────────────────────────────────┘   │   │
│  │                                                                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │   │
│  │  │ Orchestrator│  │   Planner   │  │   Critic    │  │  Workflow       │  │   │
│  │  │ (Multi-Agent)│  │   System    │  │   System    │  │  Engine         │  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │   │
│  │         │                │                │                   │           │   │
│  └─────────┼────────────────┼────────────────┼───────────────────┼───────────┘   │
│            │                │                │                   │               │
│            ▼                ▼                ▼                   ▼               │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                        SPECIALIST AGENTS                                     ││
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐          ││
│  │   │  Code   │  │  Test   │  │ Deploy  │  │  Docs   │  │Security │          ││
│  │   │  Agent  │  │  Agent  │  │  Agent  │  │  Agent  │  │  Agent  │          ││
│  │   └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘          ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       │ MCP Protocol
                                       ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL MCP SERVERS                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Playwright  │  │  OpenHands   │  │  GitHub      │  │  60+ Other           │  │
│  │  Browser     │  │  SDK         │  │  Integration │  │  Extensions          │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Key Features

### 🧠 Phase 6: Advanced Agentic AI

| Feature | Description |
|---------|-------------|
| **LangGraph-Style Checkpointing** | Durable state persistence with SQLite, thread-based history, and branching support |
| **ReAct Reasoning** | Reasoning + Acting pattern with thought traces and action results |
| **Chain-of-Thought** | Step-by-step reasoning for complex problem decomposition |
| **Tree-of-Thoughts** | Branching exploration with parallel solution evaluation |
| **Reflexion Agent** | Self-improvement through episodic memory and verbal reinforcement |
| **Execution Observability** | Token tracking, cost estimation, model pricing, budget alerts |

```rust
// Example: ReAct Reasoning Pattern
let mut manager = ReasoningManager::react();
let trace = manager.start_trace("Fix authentication bug");
trace.add_thought("First, analyze the token validation logic", ThoughtType::Initial);
let action_id = trace.add_action("Read auth.rs", 0);
trace.record_action_result(action_id, ActionResult::success("Token validation found"));
trace.add_observation(action_id, "Token expiry not being checked");
```

### 🚀 Phase 5: Enterprise Multi-Agent Platform

| Feature | Description |
|---------|-------------|
| **AgentOrchestrator** | Coordinates multiple specialist AI agents with dependency resolution |
| **5 Specialist Agents** | Code, Testing, Deployment, Documentation, and Security specialists |
| **WorkflowEngine** | Pre-built enterprise pipelines (Full-Stack, Microservices, DevOps) |
| **Task Management** | Parallel execution with progress tracking and failure recovery |

```rust
// Example: Multi-Agent Workflow
let orchestrator = AgentOrchestrator::new(config).await?;
let workflow = orchestrator.create_workflow("build-feature", "Implement OAuth2")?;
workflow.add_task(AgentRole::Code, "Implement OAuth2 flow")?;
workflow.add_task(AgentRole::Test, "Write integration tests")?;
workflow.add_task(AgentRole::Security, "Security audit")?;
orchestrator.execute_workflow(workflow).await?;
```

### 🎯 Phase 4: Advanced Agent Capabilities

| Feature | Description |
|---------|-------------|
| **Planning System** | Multi-step plan creation with progress tracking |
| **Self-Critique** | Automated quality assessment with severity classification |
| **Execution Modes** | Freeform vs. Structured execution options |

### 🛡️ Phase 3: Core Autonomous Architecture

| Feature | Description |
|---------|-------------|
| **StateGraph Engine** | Self-correcting CODE → TEST → FIX loops |
| **Approval Policies** | SAFE, PARANOID, AUTOPILOT security presets |
| **Test Framework Integration** | Pytest, Jest, Cargo, Go test parsing |
| **Done Gate Verification** | Multi-stage validation before completion |

---

## Self-Correcting Development Loop

```
                    ┌─────────────────────────────────────┐
                    │         StateGraph Engine           │
                    │                                     │
                    │  ┌─────────────────────────────┐    │
                    │  │      Graph Definition       │    │
                    │  │                             │    │
                    │  │  entry_point: "code"        │    │
                    │  │  max_iterations: 10         │    │
                    │  │  success_condition: fn()    │    │
                    │  └─────────────────────────────┘    │
                    │                                     │
                    └─────────────────┬───────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    │                 │                 │
                    ▼                 ▼                 ▼
            ┌───────────┐     ┌───────────┐     ┌───────────┐
            │           │     │           │     │           │
            │   CODE    │────▶│   TEST    │────▶│    FIX    │
            │   NODE    │     │   NODE    │     │   NODE    │
            │           │     │           │     │           │
            └───────────┘     └─────┬─────┘     └─────┬─────┘
                                    │                 │
                                    │  tests fail     │
                                    │◀────────────────┘
                                    │
                                    │  tests pass
                                    ▼
                            ┌───────────────┐
                            │   VALIDATE    │──────▶ DONE ✓
                            │   NODE        │
                            └───────────────┘
```

---

## Reflexion: Self-Improvement Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    REFLEXION AGENT                           │
│                                                              │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐  │
│  │ Attempt │───▶│ Evaluate│───▶│ Reflect │───▶│  Store  │  │
│  │  Task   │    │ Outcome │    │ on Fail │    │ Memory  │  │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘  │
│       ▲                                            │        │
│       │                                            │        │
│       │         ┌─────────────────────────┐       │        │
│       └─────────│   Retrieve Relevant     │◀──────┘        │
│                 │   Past Reflections      │                 │
│                 └─────────────────────────┘                 │
│                                                              │
│  Episodic Memory:                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Task: "Fix auth bug"                                  │  │
│  │ Diagnosis: "Token expiry not checked"                 │  │
│  │ Lessons: ["Always validate token timestamps"]         │  │
│  │ Improvements: ["Add expiry check before validation"]  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Observability & Cost Tracking

```rust
// Track execution costs in real-time
let tracker = CostTracker::new(ModelPricing::claude_sonnet());
tracker.set_budget(10.0).await;  // $10 budget limit

// Record LLM calls
tracker.record_llm_call(&TokenUsage::new(1000, 500));
tracker.record_tool_call();

// Check budget
if tracker.is_over_budget().await {
    warn!("Budget exceeded!");
}

// Get summary
println!("{}", tracker.get_summary().await);
// Output: Tokens: 1000 in / 500 out | Cost: $0.0225 | Calls: 1 LLM, 1 tools
```

---

## Quick Start

### Installation

```bash
# Install via cargo
cargo install goose-cli

# Or build from source
git clone https://github.com/block/goose.git
cd goose
cargo build --release
```

### Basic Usage

```bash
# Start interactive session
goose run

# With specific approval policy
goose run --approval-policy paranoid --text "deploy to production"

# Structured execution mode
goose run --execution-mode structured --text "implement OAuth2 system"
```

### Configuration

```yaml
# ~/.config/goose/config.yaml
extensions:
  playwright:
    type: stdio
    cmd: npx
    args: ["-y", "@playwright/mcp@latest"]

  openhands:
    type: stdio
    cmd: python
    args: ["-m", "openhands.server.mcp"]
```

---

## Approval Policies

| Policy | Safe Commands | High-Risk Commands | Critical Commands |
|--------|---------------|-------------------|-------------------|
| **SAFE** | Auto-approve | User approval | Blocked |
| **PARANOID** | User approval | User approval | Blocked |
| **AUTOPILOT** | Auto-approve* | Auto-approve* | Auto-approve* |

*Autopilot only auto-approves in Docker sandbox environments

---

## Test Coverage

```
✅ 672 passing tests
✅ Zero compilation warnings
✅ Cross-platform (Windows/Linux/macOS)
✅ 54 new Phase 6 tests for:
   - LangGraph-style checkpointing
   - ReAct reasoning traces
   - Reflexion self-improvement
   - Cost tracking & observability
```

---

## Quick Links

- [Quickstart](https://block.github.io/goose/docs/quickstart)
- [Installation](https://block.github.io/goose/docs/getting-started/installation)
- [Tutorials](https://block.github.io/goose/docs/category/tutorials)
- [Documentation](https://block.github.io/goose/docs/category/getting-started)
- [Enterprise Integration Status](docs/AGENTIC_GOOSE_INTEGRATION_STATUS.md)
- [Phase 6 Roadmap](docs/PHASE_6_AGENTIC_ENHANCEMENT_ROADMAP.md)

## Need Help?

- [Diagnostics & Reporting](https://block.github.io/goose/docs/troubleshooting/diagnostics-and-reporting)
- [Known Issues](https://block.github.io/goose/docs/troubleshooting/known-issues)

---

## Community

<p align="center">
  <a href="https://discord.gg/goose-oss">Discord</a> •
  <a href="https://www.youtube.com/@goose-oss">YouTube</a> •
  <a href="https://www.linkedin.com/company/goose-oss">LinkedIn</a> •
  <a href="https://x.com/goose_oss">Twitter/X</a> •
  <a href="https://bsky.app/profile/opensource.block.xyz">Bluesky</a>
</p>

---

<div align="center">

### A little goose humor 🦢

> Why did the developer choose goose as their AI agent?
>
> Because it always helps them "migrate" their code to production! 🚀

**Built with ❤️ by the Goose community**

</div>
