# Next Step: Complete Document Edit Tool Implementation

## Current Status ✅

### Completed:
1. **Tool Definition** (`crates/goose/src/agents/document_tools.rs`)
   - Tool schema defined with all actions (insertText, replaceText, appendText, formatText, clear)
   - Tool registered in agent (`agent.rs`)
   - Marker constant `DOCUMENT_EDIT_MARKER` defined for frontend detection

2. **Frontend Components**
   - `CollaborativeDocEditor.tsx` with `window.gooseEditors` API
   - IPC handlers in `main.ts` for document state management
   - IPC exposed in `preload.ts`

3. **Backend Integration**
   - Document context parsing in `reply.rs`
   - Context injected as system message for AI

## What's Left: Tool Execution Handler

### Add Handler in `agent.rs` (after line 600, after TODO_WRITE_TOOL_NAME handler)

```rust
} else if tool_call.name == EDIT_DOCUMENT_TOOL_NAME {
    // Handle document editing tool
    let doc_id = tool_call
        .arguments
        .get("doc_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let action = tool_call
        .arguments
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    if doc_id.is_empty() || action.is_empty() {
        ToolCallResult::from(Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "doc_id and action are required".to_string(),
            None,
        )))
    } else {
        // Serialize the entire tool call as JSON to pass to frontend
        let command_json = serde_json::to_string(&tool_call.arguments)
            .unwrap_or_else(|_| "{}".to_string());
        
        // Return a special marker that the frontend will recognize
        // The frontend will parse this and execute the appropriate window.gooseEditors method
        let response_text = format!(
            "{} {}",
            DOCUMENT_EDIT_MARKER,
            command_json
        );
        
        info!("Document edit command: action={}, doc_id={}", action, doc_id);
        
        ToolCallResult::from(Ok(vec![Content::text(response_text)]))
    }
```

### Frontend Detection (already in place via IPC)

The Electron main process already has handlers for:
- `document-updated` - Receives document state from renderer
- `get-document-state` - Returns document state to backend
- `execute-document-edit` - Sends edit commands to renderer

### Testing Flow

1. Open document editor
2. Click "Ask Goose" button
3. Type: "Make the first paragraph bold"
4. AI should call `edit_document` tool
5. Backend returns marker + JSON
6. Frontend (via IPC or message parsing) executes `window.gooseEditors[docId].formatText(...)`
7. User sees real-time update

## Alternative Simpler Approach (if above doesn't work)

Instead of using the marker in the tool response, we can use the existing IPC:

1. When `edit_document` tool is called in Rust
2. Send IPC message directly to Electron main process
3. Main process forwards to renderer via `execute-document-edit` event
4. Renderer executes `window.gooseEditors` method

This requires adding HTTP client or IPC bridge in Rust, which is more complex.

## Recommended: Use the Marker Approach

The marker approach is simpler because:
- No new IPC needed
- Frontend can parse tool responses
- Works with existing message flow
- Easy to debug

## Files to Modify

1. `crates/goose/src/agents/agent.rs` - Add handler (line ~600)
2. `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Add response parser (if not using IPC)

## Implementation Steps

1. Add the handler code above to `agent.rs` after the `TODO_WRITE_TOOL_NAME` handler
2. Compile and test: `cargo check -p goose`
3. Test end-to-end with the desktop app
4. If marker approach doesn't work, fall back to IPC bridge

## Success Criteria

- [ ] AI can call `edit_document` tool
- [ ] Tool execution doesn't error
- [ ] Frontend receives edit command
- [ ] Document updates in real-time
- [ ] Visual feedback shows "Goose is editing..."
