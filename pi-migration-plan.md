# Pi Backend Migration Plan

This document outlines the plan to improve Pi integration in Goose Desktop, moving toward feature parity with the Goose backend.

## Architecture Overview

### How Pi is Integrated

Pi is used as a **direct library** (not RPC) that runs in Electron's main process:

```
┌─────────────────────────────────────────────────────┐
│                 Electron Main Process               │
│  ┌─────────────────────────────────────────────┐   │
│  │  Pi Agent (in-process, same Node.js runtime) │   │
│  │  - createAgentSession()                      │   │
│  │  - session.prompt()                          │   │
│  │  - session.subscribe()                       │   │
│  └─────────────────────────────────────────────┘   │
│                        ↑                            │
│                   IPC (Electron)                    │
│                        ↓                            │
│  ┌─────────────────────────────────────────────┐   │
│  │           Renderer Process (React UI)        │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

**Why in-process (not RPC)?**
- Pi's RPC mode is designed for external process integration (e.g., VS Code spawning `pi` CLI)
- For Electron, the direct library API (`createAgentSession`) is simpler:
  - Same memory space, no serialization overhead
  - Direct function calls with typed APIs
  - Events via callbacks, not stdin/stdout parsing
- IPC between Electron main/renderer is async, so the renderer doesn't block
- Main process uses Node.js event loop, so LLM calls (network I/O) don't block

**Package used:** `@mariozechner/pi-coding-agent` v0.51.6
- Full coding agent with tools (read, bash, edit, write, grep, find, ls)
- Includes session management, compaction, extensions
- Depends on `@mariozechner/pi-agent-core` (lower-level agent loop) and `@mariozechner/pi-ai` (LLM API)

### Key Files

- `ui/desktop/src/pi/index.ts` - Main process integration (session management, IPC handlers)
- `ui/desktop/src/pi/eventTranslator.ts` - Translates Pi events to Goose message format (bidirectional)
- `ui/desktop/src/hooks/usePiChat.ts` - React hook for Pi chat
- `ui/desktop/src/hooks/useAgentChat.ts` - Router that delegates to usePiChat or useChatStream
- `ui/desktop/src/sessions.ts` - Session creation helpers (calls Pi IPC with goosedUrl)
- `ui/desktop/src/components/GooseSidebar/AppSidebar.tsx` - Sidebar session list

### Session Storage

Pi sessions are stored in **goosed's SQLite database** (same as Goose sessions):

```
Session Creation:
1. UI calls createPiSession() with goosedUrl
2. Pi module calls POST /sessions to create record in SQLite
3. Pi agent runs with SessionManager.inMemory() (no JSONL files)
4. After each turn, conversation saved to SQLite via PUT /sessions/{id}/conversation

Session Resume:
1. UI calls resumePiSession(sessionId, goosedUrl)
2. Pi module loads from SQLite via GET /sessions/{id}
3. Creates fresh Pi AgentSession
4. Injects history via agent.replaceMessages(translateGooseMessagesToPi(messages))
5. Pi now knows the full conversation context
```

**goosed endpoints used:**
- `POST /sessions` - Create session record (without starting Goose agent)
- `GET /sessions/{id}` - Load session with conversation
- `PUT /sessions/{id}/conversation` - Update conversation messages

## Current State

### What Works ✅
- Basic chat with Pi
- Tool calls displayed in UI
- Thinking content display
- **Session persistence in goosed SQLite** (just completed)
- **Session resume with conversation injection** (just completed)
- Session history in sidebar (uses goosed API)

### What's Missing
- Real token usage tracking
- Provider/model configuration
- MCP Extensions support
- Cost tracking
- Tool confirmation dialogs
- Message editing/forking
- Elicitation

---

## Phase 1: Token Usage & Costs

**Goal:** Show real token usage and costs in the UI footer.

### Pi's Token/Usage API

Pi events include usage data in assistant messages:
```typescript
interface PiAssistantMessage {
  role: 'assistant';
  content: PiAssistantContent[];
  timestamp: number;
  usage?: {
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
    totalTokens: number;
    cost: {
      input: number;
      output: number;
      cacheRead: number;
      cacheWrite: number;
      total: number;
    };
  };
}
```

### Implementation

1. **Update `eventTranslator.ts`** to extract usage from Pi messages
2. **Update `PiSession` interface** to track real token counts
3. **Emit token state with messages** via IPC (match Goose's `TokenState` format)
4. **Update `usePiChat.ts`** to handle token state updates
5. **Session costs** - accumulate and display in footer

### Files to Modify
- `ui/desktop/src/pi/eventTranslator.ts`
- `ui/desktop/src/pi/index.ts`
- `ui/desktop/src/hooks/usePiChat.ts`

---

## Phase 2: Provider & Model Configuration

**Goal:** Allow users to configure Pi's provider and model from Goose settings UI.

### Pi's Provider System

Pi supports many providers (see README):
- **Subscriptions:** Claude Pro/Max, ChatGPT Plus/Pro, GitHub Copilot, Gemini CLI
- **API Keys:** Anthropic, OpenAI, Azure, Google, Bedrock, Mistral, Groq, etc.

Pi reads provider config from:
- `~/.pi/agent/models.json` - Custom providers
- `~/.pi/agent/auth.json` - API keys and OAuth tokens
- CLI flags: `--provider <name> --model <id>`

### Implementation Options

**Option A: Share Goose's Provider Config**
- Map Goose providers to Pi providers
- Use Goose's stored API keys
- Pro: Single source of truth
- Con: Provider names/models may not match 1:1

**Option B: Separate Pi Provider Settings**
- Add Pi-specific provider settings in Goose UI
- Store in separate config (e.g., `~/.pi/agent/` or app data)
- Pro: Full Pi flexibility
- Con: Duplicate configuration

**Recommended: Option A with fallback**
1. Map common providers (Anthropic, OpenAI) from Goose config
2. For unmapped providers, allow Pi-specific config

### Files to Modify
- `ui/desktop/src/pi/index.ts` - Add provider/model selection to session creation
- `ui/desktop/src/components/settings/` - Add Pi model selector
- New: `ui/desktop/src/pi/providerMapping.ts` - Map Goose providers to Pi

---

## Phase 3: MCP Extensions Support

**Goal:** Allow Pi to use MCP servers configured in Goose.

### Pi's MCP Support

Pi supports MCP via the `pi-mcp-adapter` package:
```bash
pi install npm:pi-mcp-adapter
```

Configuration in `.pi/mcp.json` or `~/.pi/agent/mcp.json`:
```json
{
  "mcpServers": {
    "harness": {
      "command": "node",
      "args": ["/path/to/server.js"],
      "lifecycle": "eager",
      "env": {}
    }
  },
  "settings": {
    "toolPrefix": "none"
  }
}
```

### Implementation

1. **Read Goose's MCP config** from `extensions` in goose config
2. **Generate Pi's mcp.json** with equivalent servers
3. **Install pi-mcp-adapter** automatically if not present
4. **Write config** before creating Pi session

### Architecture Decision: Where to Run MCP Servers

**Current Goose:** MCP servers run inside goosed process
**Pi:** MCP servers are spawned by pi-mcp-adapter

**Options:**
1. **Let Pi manage MCP** - Simpler, but servers restart with each session
2. **Share MCP with goosed** - Complex, would need IPC bridge
3. **Electron main manages MCP** - Electron spawns servers, Pi connects

**Recommended: Option 1** (Pi manages MCP)
- Simpler implementation
- Pi's `lifecycle: "eager"` keeps servers running during session
- Trade-off: Servers not shared between Pi and Goose sessions

### Files to Modify
- `ui/desktop/src/pi/index.ts` - Generate mcp.json before session
- New: `ui/desktop/src/pi/mcpConfig.ts` - Translate Goose extensions to Pi MCP config

---

## Phase 4: Architecture Improvements

### Current Architecture Issues

1. **Pi runs in Electron main process**
   - Blocking operations can freeze UI
   - No isolation from app crashes

2. **Session format mismatch**
   - Goose: Server-side sessions with REST API
   - Pi: Local JSONL files, custom JSON in Electron

3. **No goosed integration**
   - Pi bypasses goosed entirely
   - Can't use goosed's session management, auth, etc.

### Proposed Architecture Options

**Option A: Keep Pi in Electron Main (Current)**
```
Electron Main Process
├── Pi SDK (createAgentSession)
├── Session storage (JSON files)
└── IPC to renderer
```
- Pros: Simple, working
- Cons: No isolation, main process blocking

**Option B: Pi in Worker Thread**
```
Electron Main Process
├── Worker Thread
│   └── Pi SDK
├── Session storage
└── IPC bridge
```
- Pros: Non-blocking, crash isolation
- Cons: More complex IPC

**Option C: Pi via goosed (Long-term)**
```
Electron Main Process
└── IPC to goosed

goosed
├── Pi SDK (as alternative agent)
├── Unified session management
├── Shared MCP servers
└── REST API to Electron
```
- Pros: Unified architecture, shared sessions/MCP
- Cons: Significant goosed changes needed

**Recommendation:**
- Short-term: Keep Option A, fix any blocking issues
- Medium-term: Consider Option B if main process becomes a bottleneck
- Long-term: Option C for full integration

---

## Phase 5: MCP UI & Apps Support

**Goal:** Support MCP UI resources and MCP Apps in Pi tool responses.

### How Goose Handles MCP UI

Goose supports two types of MCP UI content in tool responses:

#### 1. MCP UI Resources (Inline)
Embedded HTML/UI in tool results, rendered inline in the chat via `@mcp-ui/client`:

```typescript
// In ToolCallWithResponse.tsx
import { UIResourceRenderer } from '@mcp-ui/client';

// Tool result contains embedded resource
interface EmbeddedResource {
  resource: {
    uri: string;
    mimeType: string;  // e.g., "text/html"
    text?: string;     // HTML content
    blob?: string;     // Base64 binary
  }
}

// Rendered via UIResourceRenderer with action handlers
<UIResourceRenderer
  resource={content.resource}
  onUIAction={handleUIAction}  // Handles tool, prompt, link, notify, intent actions
/>
```

#### 2. MCP Apps (Full Apps)
Standalone apps served via `ui://` scheme, rendered in sandboxed iframe:

```typescript
// McpAppResource type
interface McpAppResource {
  uri: string;           // Must use ui:// scheme
  mimeType: string;      // "text/html;profile=mcp-app"
  name: string;
  text?: string;         // HTML content
  blob?: string;         // Base64 binary
  _meta?: {
    ui?: {
      resourceUri?: string;  // Triggers MCP App rendering
      csp?: CspMetadata;
      permissions?: PermissionsMetadata;
    }
  }
}

// Detected via _meta.ui.resourceUri in tool request or response
const hasMcpAppResourceURI = Boolean(
  toolRequest._meta?.ui?.resourceUri || 
  toolResponse?.toolResult?.value?._meta?.ui?.resourceUri
);
```

### MCP App Bridge Methods

MCP Apps can call back to Goose via postMessage bridge:

| Method | Description |
|--------|-------------|
| `ui/open-link` | Open external URL |
| `ui/message` | Send message to chat |
| `tools/call` | Call another MCP tool |
| `resources/read` | Read MCP resource |
| `notifications/message` | Log notification |
| `ping` | Health check |

### Implementation for Pi

The UI components (`McpAppRenderer`, `MCPUIResourceRenderer`, `ToolCallWithResponse`) are **frontend-only** and work regardless of backend. The key requirement is that Pi's tool results are translated to match Goose's expected format.

#### What Already Works

- `McpAppRenderer` - Renders HTML in sandboxed iframe
- `MCPUIResourceRenderer` - Renders inline MCP UI via `@mcp-ui/client`
- `ToolCallWithResponse` - Detects `_meta.ui.resourceUri` and renders appropriately

#### What Needs to Happen

**1. Translate Pi tool results to include MCP UI metadata**

Pi tool results need to be translated with proper `_meta` structure:

```typescript
// In eventTranslator.ts - translate Pi tool result to Goose format
function translateToolResult(piResult: PiToolResultMessage): GooseMessage {
  // Pi's tool result content may include MCP UI resources
  // Need to preserve _meta.ui.resourceUri if present
  
  const content = piResult.content.map(c => {
    if (c.type === 'text') {
      return { type: 'text', text: c.text };
    }
    // Handle embedded resources with _meta
    if ((c as any).resource) {
      return {
        type: 'resource',
        resource: (c as any).resource,
        // Preserve _meta for MCP UI detection
      };
    }
    return c;
  });
  
  return {
    role: 'user',
    content: [{
      type: 'toolResponse',
      id: piResult.toolCallId,
      toolResult: {
        status: piResult.isError ? 'error' : 'success',
        value: { content },  // UI looks for _meta here
      }
    }]
  };
}
```

**2. MCP App bridge callbacks**

MCP Apps can call back via bridge methods. These need Pi equivalents:

| Method | Goose Implementation | Pi Implementation Needed |
|--------|---------------------|-------------------------|
| `ui/open-link` | `window.electron.openExternal` | Same (frontend) |
| `ui/message` | `append()` prop | Same (frontend) |
| `tools/call` | REST to goosed | IPC to Pi main process |
| `resources/read` | REST to goosed | IPC to Pi main process |
| `notifications/message` | `console.log` | Same (frontend) |
| `ping` | Returns `{}` | Same (frontend) |

Only `tools/call` and `resources/read` need Pi-specific IPC handlers.

**3. Add IPC handlers for MCP App tool/resource calls**

```typescript
// In pi/index.ts
ipcMain.handle('pi:callTool', async (_event, { sessionId, name, args }) => {
  // Call tool via Pi session
  // This may require Pi extension or direct tool invocation
});

ipcMain.handle('pi:readResource', async (_event, { sessionId, uri, extensionName }) => {
  // Read resource via Pi's MCP adapter
});
```

**4. Route MCP App requests based on backend**

In `McpAppRenderer`, detect if using Pi backend and route accordingly:

```typescript
// Could check session type or add backend prop
const handleMcpRequest = async (method, params) => {
  if (method === 'tools/call' && isPiSession) {
    return window.electron.invoke('pi:callTool', { sessionId, ...params });
  }
  // ... existing goosed implementation
};
```

### Files to Modify

- `ui/desktop/src/pi/eventTranslator.ts` - Preserve `_meta` in tool result translation
- `ui/desktop/src/pi/index.ts` - Add `pi:callTool`, `pi:readResource` IPC handlers
- `ui/desktop/src/components/McpApps/McpAppRenderer.tsx` - Route requests based on backend
- `ui/desktop/src/preload.ts` - Expose new IPC methods

---

## Phase 6: Additional Feature Parity

### Tool Confirmation
- Pi's extensions API supports custom confirmation flows
- Need to implement an extension or hook into Pi's event system
- Lower priority - Pi is designed for autonomous operation

### Message Editing/Forking
- Pi has `/tree` and `/fork` commands
- Session format supports branching (entries have `id` and `parentId`)
- Could expose via IPC: `pi:branch`, `pi:fork`

### Elicitation
- Pi doesn't have built-in elicitation
- Could implement via Pi extension
- Lower priority

---

## Implementation Priority

1. ~~**Phase 7: Session Persistence** (CRITICAL)~~ ✅ COMPLETED
2. **Phase 1: Token Usage** (High) - Users need to see costs
3. **Phase 3: MCP Extensions** (High) - Core functionality  
4. **Phase 5: MCP UI** (High) - Tool responses need UI support
5. **Phase 2: Provider Config** (Medium) - Currently hardcoded
6. **Phase 4: Architecture** (Low) - Current setup works, in-process approach is appropriate
7. **Phase 6: Feature Parity** (Low) - Nice to have

### Next Steps

1. **Test the full flow** - Create Pi session, chat, close app, resume, verify context preserved
2. **Remove legacy JSON storage** - Once goosed integration verified, delete fallback code
3. **Phase 1: Token Usage** - Extract usage from Pi's AssistantMessage.usage

---

## Phase 7: Session Persistence - ✅ COMPLETED

**Status: COMPLETED** - Sessions now persist to goosed SQLite.

### The Problem (was)

```typescript
// In pi/index.ts - line 287-288
// TODO: Replay conversation history to Pi if it supports it
// For now, Pi starts fresh but we show the old messages in UI
```

When you resume a Pi session:
1. We load our custom JSON file
2. We create a **fresh** Pi AgentSession (no memory)
3. We show old messages in UI
4. **Pi has NO idea what was discussed before**

This means session resume is just cosmetic - Pi can't continue conversations.

### Current (Broken) Storage

```
~/Library/Application Support/goose-app/pi-sessions/
├── 20250205_143000.json  # Custom JSON format
├── 20250205_150000.json
└── ...
```

Each file contains:
```json
{
  "id": "20250205_143000",
  "conversation": [...goose format messages...],
  "input_tokens": 0,  // Always zero - never tracked
  "output_tokens": 0,
  ...
}
```

### Goose's Session Storage

Goose uses **SQLite** at `~/.local/share/goose/sessions/sessions.db`:

```sql
-- sessions table
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  name TEXT,
  session_type TEXT,  -- 'user', 'scheduled', 'sub_agent', 'hidden', 'terminal'
  working_dir TEXT,
  total_tokens INTEGER,
  ...
);

-- messages table  
CREATE TABLE messages (
  session_id TEXT REFERENCES sessions(id),
  role TEXT,
  content_json TEXT,
  ...
);
```

### The Solution: Use Goose's Session Storage ✅ IMPLEMENTED

**Architecture (now implemented):**

```
Pi Session Creation:
1. Call goosed API: POST /sessions (creates SQLite record)
2. Use Pi's SessionManager.inMemory() (Pi doesn't persist)
3. On each Pi message event, call goosed API to persist

Pi Session Resume:
1. Call goosed API: GET /sessions/{id} (load from SQLite)
2. Create Pi AgentSession
3. Inject conversation history into Pi's agent state
4. Pi now knows the full conversation context
```

### Implementation (completed)

**Step 1: Add SessionType for Pi (optional but clean)**

In `crates/goose/src/session/session_manager.rs`:
```rust
pub enum SessionType {
    User,
    Scheduled,
    SubAgent,
    Hidden,
    Terminal,
    Pi,  // NEW
}
```

**Step 2: Create Pi sessions via goosed API**

```typescript
// In pi/index.ts

export async function createPiSession(options: CreateSessionOptions): Promise<string> {
  // Create session record in Goose's SQLite via REST API
  const response = await fetch(`${goosedUrl}/agent/start`, {
    method: 'POST',
    body: JSON.stringify({
      working_dir: options.workingDir,
      // Maybe add session_type: 'pi' if we add that
    }),
  });
  const session = await response.json();  // Returns Session with id

  // Create Pi agent with in-memory session (Pi does all the AI work)
  const piSessionManager = SessionManager.inMemory(options.workingDir);
  const { session: piAgentSession } = await createAgentSession({
    sessionManager: piSessionManager,
  });

  // Store mapping: goose session ID -> Pi agent session
  activePiSessions.set(session.id, piAgentSession);

  return session.id;
}
```

**Step 3: Save conversation after Pi completes**

Pi does all the work. We just save the result:

```typescript
// When Pi finishes a turn, save the conversation to SQLite
session.subscribe((event) => {
  if (event.type === 'agent_end') {
    // Get all messages from Pi's agent
    const messages = piAgentSession.messages;
    
    // Save to Goose's SQLite (may need new endpoint or use existing)
    await saveConversationToGoose(gooseSessionId, messages);
  }
});
```

**Step 4: Resume with conversation injection**

```typescript
export async function resumePiSession(sessionId: string): Promise<void> {
  // Load conversation from Goose's SQLite
  const response = await fetch(`${goosedUrl}/sessions/${sessionId}`);
  const { conversation, working_dir } = await response.json();

  // Create fresh Pi agent
  const piSessionManager = SessionManager.inMemory(working_dir);
  const { session: piAgentSession } = await createAgentSession({
    sessionManager: piSessionManager,
  });

  // THE KEY FIX: Inject conversation history into Pi
  const piMessages = conversation.messages.map(translateGooseMessageToPi);
  piAgentSession.agent.replaceMessages(piMessages);

  // Now when user sends next message, Pi knows the full history!
  activePiSessions.set(sessionId, piAgentSession);
}
```

**Step 5: Remove custom JSON storage**

Delete all the broken custom session code:
- `saveSession()`, `loadSession()`, `listAllSessions()`, `deleteSessionFile()`
- `getSessionsDir()`, `getSessionPath()`
- `PiSession` interface (use Goose's `Session` type)

### Benefits ✅

1. **Unified session list** - Pi and Goose sessions in one place
2. **Real resume** - Pi actually knows the conversation
3. **Token tracking** - Use Goose's existing token fields
4. **No duplicate storage** - Single source of truth
5. **Session features** - Export, import, fork work for Pi too

### What Was Implemented

**Goosed server changes:**
- `POST /sessions` - Creates session record without starting Goose agent
- `PUT /sessions/{id}/conversation` - Updates conversation messages
- Added `CreateSessionRequest` and `UpdateConversationRequest` types
- Registered in OpenAPI spec

**Pi module changes (`ui/desktop/src/pi/index.ts`):**
- `createGoosedSession()` - Creates session via goosed API
- `saveConversationToGoosed()` - Saves conversation to SQLite
- `loadGoosedSession()` - Loads session from SQLite
- `createPiSession()` - Now creates in goosed when goosedUrl provided
- `resumePiSession()` - Loads from goosed and injects history via `agent.replaceMessages()`
- `promptPi()` - Saves conversation to goosed after each turn

**Event translator changes (`ui/desktop/src/pi/eventTranslator.ts`):**
- `translateGooseMessageToPi()` - Converts Goose messages back to Pi format for injection

**UI changes:**
- `sessions.ts` - `createPiSession()` passes goosedUrl
- `usePiChat.ts` - Session loading passes goosedUrl
- `AppSidebar.tsx` - Uses goosed API for session listing when goosedUrl available

### Remaining Cleanup (Optional)

Legacy JSON storage code can be removed once goosed integration is verified:
- `getSessionsDir()`, `getSessionPath()`, `generateSessionId()`
- `saveSession()`, `loadSession()`, `listAllSessions()`, `deleteSessionFile()`

These currently serve as fallback when goosedUrl is not provided.

---

## RPC Mode Reference (Not Used)

Pi can run in RPC mode for process integration. We evaluated this but chose the direct library approach instead.

**Why we don't use RPC mode:**
- RPC is designed for spawning Pi as a subprocess (e.g., VS Code extension)
- For Electron, direct library calls are simpler and more efficient
- No benefit to process isolation when Pi runs in Electron's main process anyway
- IPC between main/renderer is already async, so no blocking concerns

### Starting RPC Mode

```bash
pi --mode rpc --provider anthropic --model claude-sonnet-4-20250514 [options]
```

Options:
- `--provider <name>` - LLM provider
- `--model <id>` - Model ID
- `--no-session` - Disable persistence
- `--session-dir <path>` - Custom session directory

### Key RPC Commands

| Command | Description |
|---------|-------------|
| `prompt` | Send user message |
| `set_model` | Change model: `{"type":"set_model","provider":"anthropic","modelId":"claude-sonnet-4"}` |
| `get_state` | Get session state (model, streaming status, etc.) |
| `get_session_stats` | Get token usage and costs |
| `new_session` | Start fresh session |
| `switch_session` | Load different session file |
| `compact` | Compact context |
| `abort` | Cancel current operation |

### RPC Events

| Event | Description |
|-------|-------------|
| `agent_start` / `agent_end` | Agent lifecycle |
| `message_start` / `message_update` / `message_end` | Message streaming |
| `tool_execution_start` / `tool_execution_update` / `tool_execution_end` | Tool execution |

### Example: Set Model via RPC

```json
// stdin
{"type": "set_model", "provider": "anthropic", "modelId": "claude-sonnet-4-20250514"}

// stdout response
{"type": "response", "command": "set_model", "success": true, "data": {...model object...}}
```

### SDK vs RPC for Goose Desktop

**Current approach: SDK (createAgentSession)** ✅ CHOSEN
- Direct Node.js integration
- Type-safe
- Full control over session
- Simpler architecture for Electron

**Alternative: RPC Mode** (not used)
- Spawn `pi --mode rpc` as subprocess
- JSON protocol over stdin/stdout
- Process isolation
- Useful for external integrations, not Electron

**Decision:** Using SDK. RPC would add complexity without benefit for our use case.

---

## Reference: Pi SDK Usage

From `~/Documents/code/ablation/suite/src/runner.ts`:

```typescript
// Pi config directory
const PI_CONFIG_DIR = join(import.meta.dirname, "../.pi-root");

// Generate models.json for custom providers (e.g., ollama)
function generatePiModelsConfig(model: ModelConfig): object {
  if (model.provider !== "ollama") {
    return { providers: {} };
  }
  return {
    providers: {
      ollama: {
        baseUrl: "http://localhost:11434/v1",
        api: "openai-completions",
        apiKey: "ollama",
        models: [{ id: model.model, name: model.name, ... }]
      }
    }
  };
}

// MCP config for pi-mcp-adapter
const mcpConfig = {
  mcpServers: {
    harness: {
      command: "node",
      args: ["/path/to/server.js"],
      lifecycle: "eager",
      env: { MCP_HARNESS_LOG: "..." }
    }
  },
  settings: { toolPrefix: "none" }
};
writeFileSync(join(workdir, ".pi/mcp.json"), JSON.stringify(mcpConfig));

// Run Pi with env override
execSync(cmd, {
  env: {
    ...process.env,
    PI_CODING_AGENT_DIR: PI_CONFIG_DIR,  // Override config dir
  }
});
```

### Pi SDK (from index.d.ts)

```typescript
import { 
  createAgentSession, 
  AuthStorage, 
  ModelRegistry, 
  SessionManager 
} from "@mariozechner/pi-coding-agent";

const { session } = await createAgentSession({
  sessionManager: SessionManager.inMemory(),
  authStorage: new AuthStorage(),
  modelRegistry: new ModelRegistry(authStorage),
  // ... other options
});

await session.prompt("What files are in the current directory?");
```

Key exports:
- `createAgentSession` - Create agent session with SDK
- `AuthStorage` - Manage API keys and OAuth tokens
- `ModelRegistry` - Manage available models
- `SessionManager` - Manage session persistence
- `AgentSession` - The running agent session
- `AgentSessionEvent` - Event types emitted during execution

---

## Notes

- Pi is designed to be minimal and extensible
- No built-in sub-agents, plan mode, or permission popups
- MCP support via pi-mcp-adapter package
- Sessions stored as JSONL with tree structure (branching support)
- Provider config in `~/.pi/agent/models.json`
- Auth in `~/.pi/agent/auth.json`
