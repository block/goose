# Phase 5: Backend Integration Plan

## Overview

This phase connects the collaborative document editor with Goose's AI backend, enabling Goose to receive document context from chat messages and programmatically edit documents.

## Current Status ✅

### Frontend (Completed)
1. **CollaborativeDocEditor** - Exposes programmatic API via `window.gooseEditors[docId]`
2. **Chat Input Listener** - Already listening for `populate-chat-input` events (ChatInput.tsx lines 1171-1188)
3. **Event System** - Two custom events implemented:
   - `populate-chat-input` - Pre-fills chat with document context
   - `goose-doc-assist` - Signals document assistance request

### API Methods Available
```javascript
window.gooseEditors[docId] = {
  insertText(text, position?),
  replaceText(from, to, text),
  appendText(text),
  formatText(from, to, format),
  getContent(),
  getText(),
  getSelection(),
  clear()
}
```

## What's Needed

### 1. Message Metadata Enhancement

**Location**: `ui/desktop/src/components/ChatInput.tsx`

Currently, when the user sends a message, we need to attach document context metadata if it came from a document editor.

**Implementation**:
```typescript
// In ChatInput.tsx, add state to track document context
const [documentContext, setDocumentContext] = useState<{
  docId: string;
  content: string;
  selection?: { from: number; to: number; text: string };
  timestamp: number;
} | null>(null);

// Update the populate-chat-input listener (lines 1171-1188)
const handlePopulateChatInput = (event: CustomEvent) => {
  const { message, docId, metadata } = event.detail;
  
  setDisplayValue(message);
  setValue(message);
  
  // Store the document context
  setDocumentContext({
    docId,
    content: metadata.content,
    selection: metadata.selection,
    timestamp: Date.now()
  });
  
  textAreaRef.current?.focus();
};

// In performSubmit function, include document context in message
const performSubmit = useCallback((text?: string) => {
  // ... existing code ...
  
  // Create message object with document context
  const messageData = {
    value: textToSend,
    documentContext: documentContext ? {
      docId: documentContext.docId,
      content: documentContext.content,
      selection: documentContext.selection,
      timestamp: documentContext.timestamp
    } : undefined
  };
  
  handleSubmit(
    new CustomEvent('submit', { detail: messageData }) as unknown as React.FormEvent
  );
  
  // Clear document context after sending
  setDocumentContext(null);
  
  // ... rest of existing code ...
}, [documentContext, /* other deps */]);
```

### 2. Backend Message Handler

**Location**: Rust backend (likely `crates/goose-server/` or `crates/goose-cli/`)

The backend needs to:
1. Receive messages with optional `documentContext` metadata
2. Pass this context to the AI agent
3. Provide a tool/function for the AI to call `window.gooseEditors` methods

**Conceptual Implementation** (Rust):
```rust
// Message structure
struct Message {
    content: String,
    document_context: Option<DocumentContext>,
}

struct DocumentContext {
    doc_id: String,
    content: String,
    selection: Option<Selection>,
    timestamp: i64,
}

// Tool definition for AI
fn register_document_tools() {
    // Tool: edit_document
    // Description: Edit a collaborative document that the user is working on
    // Parameters:
    //   - doc_id: string (required)
    //   - action: "insert" | "replace" | "append" | "format" | "clear"
    //   - text: string (for insert/replace/append)
    //   - position: number (optional, for insert)
    //   - from: number (optional, for replace/format)
    //   - to: number (optional, for replace/format)
    //   - format: object (optional, for format)
    
    register_tool("edit_document", |params| {
        // Execute JavaScript in the renderer process
        execute_js_in_renderer(&format!(
            "window.gooseEditors['{}'].{}({})",
            params.doc_id,
            params.action,
            serialize_params(params)
        ))
    });
}
```

### 3. AI Prompt Enhancement

When a message includes document context, the system prompt should be enhanced:

```
You are assisting the user with a document they are editing. 

Document ID: {docId}
Current Content:
```
{content}
```

{if selection exists}
User has selected text from position {from} to {to}:
"{selectedText}"
{endif}

The user's request: {userMessage}

You can edit this document using the edit_document tool with these actions:
- insertText(text, position?) - Insert text at a position (or at cursor)
- replaceText(from, to, text) - Replace text in a range
- appendText(text) - Add text to the end
- formatText(from, to, format) - Apply formatting to a range
- clear() - Clear all content

Always explain what you're doing before making edits.
```

### 4. Renderer IPC Bridge

**Location**: `ui/desktop/src/main/` (Electron main process)

Create an IPC channel for the backend to execute document edits:

```typescript
// In main process
ipcMain.handle('execute-document-edit', async (event, docId, method, args) => {
  // Send to renderer process
  const window = BrowserWindow.getFocusedWindow();
  if (window) {
    return window.webContents.executeJavaScript(`
      (function() {
        const editor = window.gooseEditors['${docId}'];
        if (editor && typeof editor.${method} === 'function') {
          return editor.${method}(${JSON.stringify(args)});
        }
        return { error: 'Editor not found or method not available' };
      })()
    `);
  }
});
```

### 5. Visual Feedback System

**Already Implemented** ✅
- `gooseIsTyping` state in CollaborativeDocEditor
- Visual badge showing "Goose is editing..."
- Toggle to enable/disable Goose collaboration

## Testing Strategy

### Phase 5A: Message Metadata (Frontend Only)
1. Open a document editor
2. Click "Ask Goose" button
3. Verify chat input is populated
4. Open browser console
5. Before sending, check that `documentContext` state is set
6. Send message
7. Verify message includes document context in the submit event

### Phase 5B: Backend Tool Registration
1. Start Goose with document tool enabled
2. Send a message: "Add a heading that says 'Hello World'"
3. Backend should recognize document context
4. AI should call `edit_document` tool
5. Verify tool execution reaches the IPC bridge

### Phase 5C: End-to-End Integration
1. Open document, type some content
2. Select text
3. Click "Ask Goose", ask: "Make this text bold"
4. Goose should:
   - Receive document context with selection
   - Call `formatText(from, to, { bold: true })`
   - Update the document
   - Show "Goose is editing..." indicator

### Phase 5D: Multi-Turn Conversation
1. Open document
2. Ask Goose: "Write a short poem about coding"
3. Goose inserts poem
4. Ask: "Make the first line bold"
5. Goose should remember document context and format correctly
6. Ask: "Add a new stanza about debugging"
7. Goose should append new content

## Implementation Order

1. ✅ **Frontend API** - Already complete
2. ✅ **Event System** - Already complete
3. ✅ **Chat Input Listener** - Already complete
4. **Message Metadata** - Add document context to messages (Frontend)
5. **IPC Bridge** - Create Electron IPC channel for document edits
6. **Backend Tool** - Register `edit_document` tool in Rust backend
7. **AI Integration** - Enhance prompts with document context
8. **Testing** - End-to-end testing with real documents

## Next Steps

To continue Phase 5, we need to:

1. **Modify ChatInput.tsx** to store and pass document context with messages
2. **Identify the correct Rust backend files** to add the document editing tool
3. **Create the IPC bridge** in Electron main process
4. **Test the integration** with console commands first, then with AI

## Files to Modify

### Frontend
- ✅ `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Complete
- 🔄 `ui/desktop/src/components/ChatInput.tsx` - Add document context state and metadata
- 🔄 `ui/desktop/src/main/index.ts` (or similar) - Add IPC handler for document edits

### Backend (To be determined)
- 🔄 `crates/goose-server/src/???` - Add document editing tool
- 🔄 `crates/goose/src/???` - Enhance message handling with document context

## Questions to Resolve

1. **Where is the message handling code in the Rust backend?**
   - Need to find the file that processes incoming chat messages
   - Need to understand the tool/function calling system

2. **How does the Electron app communicate with the Rust backend?**
   - Is it HTTP? IPC? WebSocket?
   - Need to trace the path from `handleSubmit` to backend

3. **What is the tool registration system?**
   - How are tools/functions exposed to the AI?
   - What's the format for tool definitions?

## Success Criteria

- [ ] User can click "Ask Goose" in document editor
- [ ] Chat input is populated with document context
- [ ] User sends message with document-related request
- [ ] Goose receives document context in backend
- [ ] Goose can call document editing methods
- [ ] Document updates in real-time
- [ ] Visual feedback shows "Goose is editing..."
- [ ] Multi-turn conversations maintain document context

## Documentation

Once complete, create:
- `GOOSE_DOCUMENT_INTEGRATION_GUIDE.md` - For developers
- `DOCUMENT_COLLABORATION_USER_GUIDE.md` - For end users
- Update `README_DOCEDITOR.md` with collaboration features
