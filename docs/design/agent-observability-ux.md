# Goose Agent Observability — UX/UI Design Document

**Author:** UX/UI Design Review  
**Date:** 2026-02-13  
**Status:** Proposal  
**Scope:** Desktop (Electron), CLI, Web (SSE consumers)

---

## Executive Summary

End users of Goose cannot currently see which model produced a response, which
extension provided a tool, or how the agent reasoned about a task. The data
flows through the entire backend pipeline but is **discarded at the UI layer**.

This document proposes incremental changes across all three interfaces (Desktop,
CLI, Web) to surface agent observability using data that already exists.

---

## 1. Problem Analysis

### 1.1 The Data Pipeline (What Already Works)

```
Agent (Rust)                    Server (SSE)              UI (React/CLI)
─────────────                   ────────────              ─────────────
AgentEvent::Message         →   MessageEvent::Message     → ✅ Rendered
AgentEvent::ModelChange     →   MessageEvent::ModelChange → ❌ IGNORED
AgentEvent::McpNotification →   MessageEvent::Notification→ ✅ Progress bars
AgentEvent::HistoryReplaced →   MessageEvent::UpdateConv  → ✅ Applied
```

### 1.2 The Critical Gap

**Desktop `useChatStream.ts` line 290:**
```typescript
case 'ModelChange': {
  break;  // ← Event received from server and thrown away
}
```

**CLI `session/mod.rs` line 1057:**
```rust
Some(Ok(AgentEvent::ModelChange { model, mode })) => {
    if self.debug {  // ← Only visible in debug mode
        eprintln!("Model changed to {} in {} mode", model, mode);
    }
}
```

### 1.3 Current Observability Matrix

| Signal                 | Desktop           | CLI               | Gap                    |
|------------------------|-------------------|-------------------|------------------------|
| Which model answered   | ❌ Ignored        | ❌ Debug-only     | **Critical**           |
| Which provider         | ❌ Not shown      | ❌ Not shown      | **Critical**           |
| Extension name         | ⚠️ Tooltip only   | ⚠️ Prefix only    | Not prominent          |
| Tool call status       | ✅ Status dot     | ✅ Inline markers | Good                   |
| Tool arguments         | ✅ Expandable     | ✅ Per-tool render | Good                   |
| Tool duration          | ⚠️ Client guess   | ❌ None           | No server timing       |
| Reasoning/thinking     | ✅ Collapsible    | ⚠️ Env var opt-in  | CLI default off        |
| Subagent delegation    | ✅ Notifications  | ✅ Notifications  | Good                   |
| Progress               | ✅ Progress bars  | ✅ Progress bars  | Good                   |
| Token count            | ✅ In state       | ✅ End of turn    | Not per-message        |
| Cost                   | ⚠️ Feature flag   | ⚠️ Config opt-in  | Hidden by default      |
| Tool call sequence     | ⚠️ Flat list      | ⚠️ Flat list      | No visual timeline     |

---

## 2. Design Principles

1. **Progressive Disclosure** — Essential info visible by default, details on
   demand (click/hover/expand)
2. **Non-intrusive Attribution** — Model/provider visible but subordinate to
   the actual response content
3. **Consistent Data Model** — All interfaces consume the same event stream;
   differences are only in rendering
4. **Accessibility** — Never rely on color alone; use text labels, icons, ARIA
   attributes alongside visual indicators

---

## 3. Desktop UI Design

### 3.1 Response Attribution Badge

**Location:** GooseMessage footer, inline with existing timestamp.

```
┌──────────────────────────────────────────────────┐
│  Here's the file content you requested...        │
│                                                  │
│  \`\`\`python                                      │
│  def hello(): ...                                │
│  \`\`\`                                             │
│                                                  │
│  2:34 PM · gpt-4o · auto                         │
│           ↑ model    ↑ mode                      │
│  [hover tooltip: "openai / gpt-4o / auto mode"]  │
└──────────────────────────────────────────────────┘
```

**Design rationale:**
- Same visual weight as the existing timestamp — does not compete with content
- Model name is the most useful identifier (providers have few models each)
- Mode (auto/chat/agent) indicates the agent's behavior style
- Full provider info available on hover via existing TooltipWrapper component

**When model info is unavailable** (e.g., replaying old sessions without
metadata), gracefully degrade to showing only the timestamp.

**Implementation — 3 changes needed:**

**1. `useChatStream.ts` — Track model per-message:**
```typescript
// Add to streamFromResponse():
let currentModelInfo: { model: string; mode: string } | null = null;

case 'ModelChange': {
  currentModelInfo = { model: event.model, mode: event.mode };
  break;
}

case 'Message': {
  const msg = event.message;
  // Attach current model info to assistant messages
  if (msg.role === 'assistant' && currentModelInfo) {
    (msg as any)._modelInfo = { ...currentModelInfo };
  }
  currentMessages = pushMessage(currentMessages, msg);
  // ... rest of existing logic
}
```

**2. `GooseMessage.tsx` — Show in footer:**
```tsx
// Replace timestamp-only footer (line ~162):
<div className="text-xs font-mono text-text-muted pt-1">
  {timestamp}
  {message._modelInfo && (
    <>
      <span className="mx-1 opacity-50">·</span>
      <span>{message._modelInfo.model}</span>
      <span className="mx-1 opacity-50">·</span>
      <span>{message._modelInfo.mode}</span>
    </>
  )}
</div>
```

**3. For persisted sessions** (Phase 3): Add `model`/`provider`/`mode`
optional fields to the Rust Message struct so attribution survives reload.

### 3.2 Tool Call Header Enhancement

**Current:**
```
┌─────────────────────────────┐
│ 🔧 shell                    │  ← Extension name hidden in tooltip
│    running ls -la            │
└─────────────────────────────┘
```

**Proposed:**
```
┌──────────────────────────────────────┐
│ 🔧 developer › shell          ✅ 0.3s│
│    running ls -la                     │
│    ▸ Output                           │
└──────────────────────────────────────┘
```

Changes to `ToolCallWithResponse.tsx`:
- Show `extensionName › toolName` as the primary label (data already parsed
  via `getExtensionTooltip()` / `getToolName()`)
- Show duration aligned right (client-side `startTime` state already exists
  in ToolCallView, line 485)
- Duration format: "<1s", "1.2s", "12s", "1m 03s"

### 3.3 Tool Call Timeline Connector

When multiple tool calls appear consecutively (detected by the existing
`identifyConsecutiveToolCalls()` in toolCallChaining.ts), render a vertical
connector line between them:

```
┌─ 🔧 developer › shell ─────────── ✅ 0.3s ─┐
│  $ ls -la                                    │
│  ▸ Output                                    │
├─ 🔧 developer › text_editor ──── ✅ 0.1s ──┤
│  reading /src/main.rs                        │
│  ▸ Output                                    │
├─ 🔧 developer › shell ─────────── ✅ 1.2s ─┤
│  $ cargo build                               │
│  ▸ Output                                    │
└──────────────────────────────────────────────┘
         Total: 3 tool calls · 1.6s
```

Uses the existing `isInChain()` utility from toolCallChaining.ts.

### 3.4 Thinking/Reasoning Display

**Current implementation is already excellent:**
```tsx
{cotText && (
  <details className="bg-background-muted border rounded p-2 mb-2">
    <summary>Show thinking</summary>
    <MarkdownContent content={cotText} />
  </details>
)}
```

✅ **No change needed.** Collapsible progressive disclosure is correct.

### 3.5 Observability Panel (Power Users)

A slide-out panel accessible via keyboard shortcut (Ctrl+Shift+D) or a
debug icon in the bottom bar:

```
┌─────────── Session Debug ───────────────┐
│                                         │
│ Model:     openai / gpt-4o              │
│ Mode:      auto                         │
│ Session:   20260213_003831              │
│                                         │
│ ── Token Usage ────────────────────── │
│ Input:     12,450 tokens                │
│ Output:     3,200 tokens                │
│ Context:   ████████░░ 78% (15.6K/20K)   │
│ Est. Cost: $0.0234                      │
│                                         │
│ ── Active Extensions ─────────────── │
│ • developer (built-in)                  │
│ • memory (built-in)                     │
│ • github (user)                         │
│                                         │
│ ── Event Log ─────────────────────── │
│ 00:38:34 ModelChange → gpt-4o (auto)   │
│ 00:38:35 ToolRequest → developer/shell  │
│ 00:38:35 ToolResponse → ✅ (0.3s)       │
│ 00:38:36 Message → "Here's the..."     │
└─────────────────────────────────────────┘
```

Data sources (all already available):
- TokenState from useChatStream
- ModelAndProviderContext for model/provider
- NotificationEvent[] from stream state
- Extension list from listApps() API

---

## 4. CLI Design

### 4.1 Response Attribution Line

**Current:** Model info only shown with `--debug` flag.

**Proposed:** Dim attribution line before each agent response:

```
( Nesting ideas... )

─── gpt-4o · auto ─────────────────────────────

Here's the file content you requested...

Context: ████████░░ 78% (15,650/20,000 tokens)
Cost: $0.0023 USD (1250 tokens: in 980, out 270)
```

**Implementation in `session/mod.rs` line 1057:**
```rust
Some(Ok(AgentEvent::ModelChange { model, mode })) => {
    if is_stream_json_mode {
        emit_stream_event(&StreamEvent::ModelChange {
            model: model.clone(), mode: mode.clone()
        });
    } else if !is_json_mode && interactive {
        println!("{}", style(format!("─── {} · {} ───", model, mode)).dim());
    }
}
```

### 4.2 Tool Call Enhancement

**Proposed:**
```
  ┌ [1/3] developer › shell
  │ $ ls -la
  │ ✓ 0.3s
  ├ [2/3] developer › text_editor
  │ reading /src/main.rs
  │ ✓ 0.1s
  └ [3/3] developer › shell
    $ cargo build
    ✓ 1.2s
```

Elements:
- Sequence number [N/total] for multi-tool turns
- Extension prefix before tool name
- Box-drawing characters for visual grouping
- Duration per tool call

### 4.3 Reasoning Visibility

**Current:** Requires `GOOSE_CLI_SHOW_THINKING=1` environment variable.

**Proposed:** Show a hint when thinking content is present:
```
💭 Reasoning used (set GOOSE_CLI_SHOW_THINKING=1 to display)
```

---

## 5. Web / SSE API

The server SSE endpoint already emits all necessary events. No changes
needed for web clients:

```
data: {"type":"ModelChange","model":"gpt-4o","mode":"auto"}
data: {"type":"Message","message":{...},"token_state":{...}}
data: {"type":"Notification","request_id":"...","message":{...}}
data: {"type":"Finish","reason":"endTurn","token_state":{...}}
```

---

## 6. Data Model Changes

### 6.1 Message-Level Attribution (Recommended for Phase 3)

```rust
// In crates/goose/src/conversation/message.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttribution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

// Add to Message struct:
#[serde(skip_serializing_if = "Option::is_none")]
pub attribution: Option<MessageAttribution>,
```

Benefits: persists across session reload, enables benchmarking analysis,
backward compatible via Option + skip_serializing_if.

### 6.2 Tool Call Timing (Nice-to-Have)

```rust
// On ToolRequest:
#[serde(skip_serializing_if = "Option::is_none")]
pub started_at: Option<i64>,

// On ToolResponse:
#[serde(skip_serializing_if = "Option::is_none")]
pub completed_at: Option<i64>,
```

Replaces the inaccurate client-side `Date.now()` tracking.

---

## 7. Implementation Roadmap

### Phase 1: Quick Wins (1-2 days)

| # | Change                                          | Files                              | Effort |
|---|-------------------------------------------------|------------------------------------|--------|
| 1 | Handle ModelChange in useChatStream, tag msgs   | `useChatStream.ts`                 | 1h     |
| 2 | Show model + mode in GooseMessage footer        | `GooseMessage.tsx`                 | 1h     |
| 3 | Remove debug gate on CLI ModelChange display    | `cli/session/mod.rs` L1057        | 15min  |
| 4 | Show extension name in tool call header         | `ToolCallWithResponse.tsx`         | 30min  |

### Phase 2: Enhanced Tool Display (2-3 days)

| # | Change                                          | Files                              | Effort |
|---|-------------------------------------------------|------------------------------------|--------|
| 5 | Show tool duration in UI                        | `ToolCallWithResponse.tsx`         | 1h     |
| 6 | Tool call timeline connector for chains         | `ToolCallWithResponse.tsx`, CSS   | 2h     |
| 7 | CLI numbered tool calls with connectors         | `cli/session/output.rs`           | 2h     |
| 8 | CLI thinking hint message                       | `cli/session/output.rs`           | 30min  |

### Phase 3: Persistent Attribution (3-4 days)

| # | Change                                          | Files                              | Effort |
|---|-------------------------------------------------|------------------------------------|--------|
| 9  | Add MessageAttribution to Rust Message struct  | `message.rs`                      | 1h     |
| 10 | Populate attribution in agent reply stream     | `agent.rs`                        | 1h     |
| 11 | Regenerate OpenAPI spec                        | `just generate-openapi`           | 15min  |
| 12 | Use persisted attribution in GooseMessage      | `GooseMessage.tsx`                | 30min  |

### Phase 4: Power User Features (1 week)

| # | Change                                          | Files                              | Effort |
|---|-------------------------------------------------|------------------------------------|--------|
| 13 | Observability debug panel                      | New component                      | 4h     |
| 14 | Server-side tool timing                        | `message.rs`, `tool_execution.rs` | 3h     |
| 15 | Default cost display to on                     | Config changes                     | 30min  |

---

## 8. Existing Infrastructure to Leverage

| Component                         | Location                        | Purpose                          |
|-----------------------------------|---------------------------------|----------------------------------|
| `AgentEvent::ModelChange`         | agent.rs:143                    | Emits model/mode changes         |
| `MessageEvent::ModelChange`       | reply.rs:137                    | SSE event to client              |
| `getToolName()`                   | ToolCallWithResponse.tsx:417    | Extracts tool name               |
| `getExtensionTooltip()`           | ToolCallWithResponse.tsx:425    | Extracts extension name          |
| `identifyConsecutiveToolCalls()`  | toolCallChaining.ts             | Groups chained tool calls        |
| `ToolCallStatusIndicator`         | ToolCallStatusIndicator.tsx     | Status dots (green/red/yellow)   |
| `splitChainOfThought()`           | GooseMessage.tsx:51             | Parses `<think>` tags           |
| `useCostTracking`                 | useCostTracking.ts              | Token/cost accumulation          |
| `ModelAndProviderContext`          | ModelAndProviderContext.tsx      | Current model/provider state     |
| `TokenState`                      | useChatStream.ts                | Per-turn token counts            |
| `display_context_usage()`         | cli/output.rs:969               | CLI context bar                  |
| `display_cost_usage()`            | cli/output.rs:1024              | CLI cost display                 |
| `ThinkingIndicator`               | cli/output.rs                   | Spinner with goose messages      |
| `ProgressBars`                    | cli/output.rs:1061              | CLI progress tracking            |
| `TooltipWrapper`                  | TooltipWrapper.tsx              | Reusable hover tooltip           |

---

## 9. Open Questions

1. **Model vs. provider in attribution?** Model names are usually unique enough.
   Recommendation: model + mode by default, provider on hover.

2. **Cost per-message or per-session?** Per-session exists in bottom bar.
   Recommendation: per-session is sufficient for now.

3. **Observability panel — dev-only?** Recommendation: hidden by default,
   discoverable via keyboard shortcut or settings toggle.

4. **CLI thinking default?** Recommendation: show hint line, keep full content
   as opt-in via env var.
