# Phase 5: Backend Integration - Progress Report

## 🎉 What We've Accomplished

### 1. Document Context Structures ✅
**File**: `crates/goose-server/src/routes/reply.rs`

Added three new structs to handle document context:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentSelection {
    from: usize,
    to: usize,
    text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentContext {
    doc_id: String,
    content: String,
    selection: Option<DocumentSelection>,
    timestamp: i64,
}
```

### 2. ChatRequest Enhancement ✅
Extended the `ChatRequest` struct to include optional document context:

```rust
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatRequest {
    messages: Vec<Message>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
    document_context: Option<DocumentContext>,  // NEW!
}
```

### 3. Document Context Logging ✅
Added logging when document context is received:

```rust
// Log document context if present
if let Some(ref doc_ctx) = document_context {
    tracing::info!(
        doc_id = %doc_ctx.doc_id,
        content_length = doc_ctx.content.len(),
        has_selection = doc_ctx.selection.is_some(),
        "Document context received"
    );
}
```

### 4. AI Prompt Enhancement ✅
Inject document context as an agent-only system message:

```rust
// If document context is present, inject it as a system message
if let Some(doc_ctx) = document_context.clone() {
    let mut context_message = format!(
        "You are assisting the user with a document they are editing.\n\n\
        Document ID: {}\n\
        Current Content:\n```\n{}\n```\n",
        doc_ctx.doc_id,
        doc_ctx.content
    );

    if let Some(selection) = doc_ctx.selection {
        context_message.push_str(&format!(
            "\nUser has selected text from position {} to {}:\n\"{}\"\n",
            selection.from, selection.to, selection.text
        ));
    }

    context_message.push_str(
        "\nYou can edit this document using the edit_document tool with these actions:\n\
        - insertText(text, position?) - Insert text at a position (or at cursor)\n\
        - replaceText(from, to, text) - Replace text in a range\n\
        - appendText(text) - Add text to the end\n\
        - formatText(from, to, format) - Apply formatting to a range\n\
        - clear() - Clear all content\n\n\
        Always explain what you're doing before making edits."
    );

    // Add as an agent-only system message
    let system_message = Message::user()
        .with_text(context_message)
        .agent_only();

    messages.push(system_message);
}
```

## 📊 Data Flow (Current State)

```
Frontend (Complete) ✅
    ↓
ChatInput dispatches message with documentContext
    ↓
Axios POST to /reply endpoint
    ↓
Backend (Partial) 🔄
    ↓
reply_handler receives ChatRequest
    ↓
Parses documentContext ✅
    ↓
Logs document context ✅
    ↓
Injects as system message ✅
    ↓
AI receives enhanced prompt ✅
    ↓
AI calls edit_document tool ⏳ (TO BE IMPLEMENTED)
    ↓
Tool handler ⏳ (TO BE IMPLEMENTED)
    ↓
IPC to Electron ⏳ (TO BE IMPLEMENTED)
    ↓
Execute window.gooseEditors[docId].method() ⏳ (TO BE IMPLEMENTED)
    ↓
Document updates ⏳ (TO BE IMPLEMENTED)
```

## 🧪 Testing the Current Implementation

### Test 1: Verify Backend Compiles
```bash
cd /Users/spencermartin/Desktop/goose
source bin/activate-hermit
cargo check -p goose-server
# Should compile successfully ✅
```

### Test 2: Verify Document Context is Received
```bash
# 1. Start the backend
cargo run --bin goose-server

# 2. In the frontend:
# - Open a document
# - Click "Ask Goose"
# - Send a message

# 3. Check backend logs for:
# "Document context received" with doc_id, content_length, has_selection
```

### Test 3: Verify System Message Injection
```bash
# The AI should now receive the document context in its prompt
# You can verify this by checking the conversation messages
# The system message should be agent_visible=true, user_visible=false
```

## 🔜 What's Next

### Step 1: Create edit_document Tool
We need to create a tool that the AI can call to edit documents. This tool should:

1. Accept parameters: `doc_id`, `action`, `text`, `position`, `from`, `to`, `format`
2. Validate the parameters
3. Call the IPC bridge to execute the edit

**Location**: Need to find where tools are registered (likely in `crates/goose/src/agents/`)

### Step 2: Create IPC Bridge
We need to create an IPC channel in Electron that:

1. Receives edit requests from the Rust backend
2. Executes JavaScript in the renderer process
3. Calls `window.gooseEditors[docId].method(...args)`
4. Returns the result to the backend

**Location**: `ui/desktop/src/main/index.ts` or similar

### Step 3: Register the Tool
The `edit_document` tool needs to be registered with the agent so the AI knows it's available.

### Step 4: End-to-End Testing
Once all pieces are in place:
1. User opens document
2. User clicks "Ask Goose"
3. User sends: "Make this text bold"
4. AI receives document context
5. AI calls `edit_document` tool
6. Tool executes via IPC
7. Document updates in real-time
8. Visual feedback shows "Goose is editing..."

## 📝 Files Modified

### Backend ✅
- `crates/goose-server/src/routes/reply.rs`
  - Added `DocumentSelection` struct
  - Added `DocumentContext` struct
  - Modified `ChatRequest` to include `document_context`
  - Added document context logging
  - Added system message injection

### Frontend ✅ (from previous phases)
- `ui/desktop/src/components/ChatInput.tsx`
  - Added `documentContext` state
  - Enhanced message submission with context

### To Be Modified 🔄
- Tool registration (Rust)
- IPC bridge (Electron/TypeScript)
- Tool implementation (Rust)

## 🎯 Success Criteria

- [x] Backend can receive document context
- [x] Backend logs document context
- [x] AI receives enhanced prompt with document info
- [ ] AI can call edit_document tool
- [ ] Tool executes via IPC
- [ ] Document updates in real-time
- [ ] Visual feedback works
- [ ] Multi-turn conversations maintain context

## 💡 Key Insights

1. **Backward Compatibility**: The `document_context` field is optional, so existing messages work normally.

2. **Agent-Only Messages**: We use `.agent_only()` to inject the document context as a system message that's visible to the AI but not shown to the user in the UI.

3. **Serialization**: Using `#[serde(rename_all = "camelCase")]` ensures the Rust structs match the JavaScript camelCase naming convention.

4. **Logging**: Comprehensive logging helps debug the data flow and verify that document context is being received correctly.

## 🚀 Next Developer Actions

1. **Find Tool Registration System**
   ```bash
   cd crates/goose
   rg "register.*tool" -A 5
   ```

2. **Create edit_document Tool**
   - Define tool schema
   - Implement tool handler
   - Register with agent

3. **Create IPC Bridge**
   - Add IPC handler in Electron main process
   - Execute JavaScript in renderer
   - Return results to backend

4. **Test Integration**
   - Verify tool is called
   - Verify IPC communication
   - Verify document updates

## 📚 Related Documentation

- `PHASE_5_COMPLETE_SUMMARY.md` - Frontend implementation
- `PHASE_5_BACKEND_INTEGRATION.md` - Original backend plan
- `GOOSE_DOCUMENT_COLLABORATION.md` - API reference
- `COMPLETE_FEATURE_SUMMARY.md` - Full feature overview

## 🎊 Current Status

**Backend**: 50% Complete
- ✅ Document context parsing
- ✅ Logging
- ✅ AI prompt enhancement
- ⏳ Tool creation
- ⏳ IPC bridge
- ⏳ Tool registration

**Overall Project**: 75% Complete
- ✅ Frontend (100%)
- 🔄 Backend (50%)

The foundation is solid! We've successfully connected the frontend to the backend and enhanced the AI's prompt with document context. The next step is to create the tool that allows the AI to actually edit the document.
