# Document Editor Implementation Plan

## Current Status

### ✅ Completed
1. **Frontend Components**
   - `CollaborativeDocEditor.tsx` - Main editor with Tiptap
   - `window.gooseEditors` API exposed for programmatic control
   - Event-driven communication (`goose-doc-assist`, `populate-chat-input`)
   - Visual feedback for Goose editing state

2. **Chat Integration (Frontend)**
   - `ChatInput.tsx` enhanced to capture document context
   - Document metadata passed with messages (docId, content, selection)

3. **Backend Integration (Partial)**
   - `crates/goose-server/src/routes/reply.rs` - Parses `document_context` from requests
   - Document context injected as system message for AI
   - `crates/goose/src/agents/document_tools.rs` - Tool schema defined
   - `crates/goose/src/agents/agent.rs` - Tool registered in agent

4. **Electron IPC (Already in place)**
   - `ui/desktop/src/main.ts` - IPC handlers for document collaboration
   - `ui/desktop/src/preload.ts` - IPC exposed to renderer
   - Document state management in main process

### 🚧 In Progress
**Implement tool execution in Rust**

The `edit_document` tool is defined and registered, but needs execution logic.

## Implementation Approach

### Option 1: HTTP-based IPC (Recommended)
Since goose-server already runs an HTTP server that Electron communicates with, we can add a new endpoint for document editing.

**Pros:**
- Consistent with existing architecture
- No new IPC mechanism needed
- Easy to test and debug

**Steps:**
1. Add new route in `crates/goose-server/src/routes/` for document editing
2. Implement handler that sends IPC message to Electron via existing WebSocket/SSE connection
3. Update `dispatch_tool_call` in `agent.rs` to route to this handler

### Option 2: Direct WebSocket/SSE
Use the existing notification stream to send document edit commands.

**Pros:**
- Real-time, bidirectional
- Already set up for notifications

**Steps:**
1. Add document edit notification type
2. Frontend listens for these notifications
3. Execute `window.gooseEditors` methods on receipt

## Recommended Implementation

**Use Option 2 (WebSocket/SSE)** because:
- The infrastructure already exists
- Document edits are essentially "notifications" to the frontend
- No new routes needed
- Consistent with how other real-time updates work

## Next Steps

1. **Add document edit handling in `agent.rs`**
   ```rust
   else if tool_call.name == EDIT_DOCUMENT_TOOL_NAME {
       // Extract parameters
       // Send notification via existing notification stream
       // Return success response
   }
   ```

2. **Update `CollaborativeDocEditor.tsx` to listen for notifications**
   - Already has `window.gooseEditors` API
   - Just needs to listen for incoming edit commands

3. **Test end-to-end flow**
   - User opens document
   - User asks Goose to edit via chat
   - AI calls `edit_document` tool
   - Frontend receives notification and executes edit
   - User sees real-time update

## Architecture Overview

```
User Chat Input
    ↓
ChatInput.tsx (captures document context)
    ↓
goose-server/routes/reply.rs (receives context)
    ↓
Agent processes with AI
    ↓
AI calls edit_document tool
    ↓
agent.rs dispatches tool call
    ↓
Sends notification to frontend
    ↓
CollaborativeDocEditor.tsx receives notification
    ↓
Executes window.gooseEditors[docId].method()
    ↓
User sees real-time edit in document
```

## Files to Modify

1. `crates/goose/src/agents/document_tools.rs` - Export tool name constant
2. `crates/goose/src/agents/agent.rs` - Add tool execution in `dispatch_tool_call`
3. `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Listen for edit notifications
4. Test the full flow

## Testing Checklist

- [ ] Open document editor
- [ ] Document state tracked in main process
- [ ] Chat input captures document context
- [ ] Backend receives document context
- [ ] AI can call edit_document tool
- [ ] Frontend receives edit command
- [ ] Document updates in real-time
- [ ] Visual feedback shows Goose is editing
- [ ] Multiple edit actions work (insert, replace, format, etc.)
