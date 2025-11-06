# 🎉 Phase 4 Complete: Frontend Tools for Document Collaboration

## What We Built

We successfully implemented **Phase 4: Frontend Tools** - a set of tools that allow Goose to interact with collaborative documents using the MCP (Model Context Protocol) Frontend extension type.

## Quick Summary

### Tools Created

1. **`document_view`** - Read document content
2. **`document_edit`** - Make edits (insert, replace, append, clear)
3. **`document_format`** - Apply formatting (bold, italic, headings, lists, etc.)
4. **`list_documents`** - List all open documents

### Files Created

- ✅ `ui/desktop/src/extensions/documentTools.ts` - Tool definitions and execution logic
- ✅ `ui/desktop/src/hooks/useDocumentTools.ts` - React hook for registration and handling

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         USER ASKS GOOSE                         │
│              "Help me write a blog post about React"            │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                    Goose AI (Backend)                           │
│  • Receives user message with document context                  │
│  • Decides to use document tools                                │
│  • Calls: list_documents()                                      │
└────────────────────────────────────────────────────────────────┘
                              ↓ Frontend Tool Call
┌────────────────────────────────────────────────────────────────┐
│              useDocumentTools Hook (Frontend)                   │
│  • Receives tool call via window.postMessage                    │
│  • Calls: executeDocumentTool('list_documents', {})             │
└────────────────────────────────────────────────────────────────┘
                              ↓ IPC
┌────────────────────────────────────────────────────────────────┐
│                   Electron Main Process                         │
│  • ipcMain.handle('list-documents')                             │
│  • Returns: Array.from(documentStore.values())                  │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│              useDocumentTools Hook (Frontend)                   │
│  • Formats result as MCP tool response                          │
│  • Returns via window.postMessage                               │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                    Goose AI (Backend)                           │
│  • Receives list of documents                                   │
│  • Sees: [{ docId: "doc-123", plainText: "I need to...", ... }]│
│  • Calls: document_view({ doc_id: "doc-123" })                  │
└────────────────────────────────────────────────────────────────┘
                              ↓ (repeat process)
┌────────────────────────────────────────────────────────────────┐
│              useDocumentTools Hook (Frontend)                   │
│  • Gets full document content from main process                 │
│  • Returns to Goose                                             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                    Goose AI (Backend)                           │
│  • Analyzes document content                                    │
│  • Decides to add an outline                                    │
│  • Calls: document_edit({                                       │
│      doc_id: "doc-123",                                         │
│      action: "appendText",                                      │
│      params: { text: "# React Hooks Guide\n\n..." }            │
│    })                                                           │
└────────────────────────────────────────────────────────────────┘
                              ↓ (repeat process)
┌────────────────────────────────────────────────────────────────┐
│              useDocumentTools Hook (Frontend)                   │
│  • Sends edit command via IPC                                   │
│  • Main process broadcasts to renderer                          │
│  • CollaborativeDocEditor executes edit                         │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                   USER SEES GOOSE'S EDITS                       │
│              Text appears with "Goose is editing..." badge      │
└────────────────────────────────────────────────────────────────┘
```

## Tool Specifications

### 1. `document_view`

**Purpose**: Read the current content of a document

**Parameters**:
```typescript
{
  doc_id: string  // e.g., "doc-12345"
}
```

**Returns**:
```json
{
  "content": [{
    "type": "text",
    "text": "{
      \"docId\": \"doc-12345\",
      \"content\": \"<p>Hello World</p>\",
      \"plainText\": \"Hello World\",
      \"selection\": { \"from\": 11, \"to\": 11 },
      \"timestamp\": 1699200000000,
      \"lastModified\": 1699200000000
    }"
  }]
}
```

**Example Usage**:
```javascript
// Goose calls this tool
await document_view({ doc_id: "doc-12345" });

// Gets back the full document state
// Can see what the user has written
// Can understand context before making edits
```

### 2. `document_edit`

**Purpose**: Make edits to a document

**Parameters**:
```typescript
{
  doc_id: string,
  action: 'insertText' | 'replaceText' | 'appendText' | 'clear',
  params: {
    text?: string,      // For insert, replace, append
    position?: number,  // For insertText
    from?: number,      // For replaceText
    to?: number         // For replaceText
  }
}
```

**Actions**:

**insertText** - Insert text at a specific position:
```javascript
await document_edit({
  doc_id: "doc-12345",
  action: "insertText",
  params: {
    text: "New text here",
    position: 10  // Insert at position 10
  }
});
```

**replaceText** - Replace a range of text:
```javascript
await document_edit({
  doc_id: "doc-12345",
  action: "replaceText",
  params: {
    from: 0,
    to: 5,
    text: "Replaced"
  }
});
```

**appendText** - Add text to the end:
```javascript
await document_edit({
  doc_id: "doc-12345",
  action: "appendText",
  params: {
    text: "\n\nNew paragraph at the end"
  }
});
```

**clear** - Clear all content:
```javascript
await document_edit({
  doc_id: "doc-12345",
  action: "clear",
  params: {}
});
```

### 3. `document_format`

**Purpose**: Apply formatting to text

**Parameters**:
```typescript
{
  doc_id: string,
  from: number,      // Start position
  to: number,        // End position
  format: string     // Format to apply
}
```

**Supported Formats**:
- **Text styles**: `bold`, `italic`, `underline`, `strikethrough`
- **Headings**: `heading1`, `heading2`, `heading3`
- **Lists**: `bulletList`, `orderedList`
- **Code**: `code`, `codeBlock`
- **Other**: `blockquote`

**Example Usage**:
```javascript
// Make text bold
await document_format({
  doc_id: "doc-12345",
  from: 0,
  to: 11,
  format: "bold"
});

// Convert to heading
await document_format({
  doc_id: "doc-12345",
  from: 0,
  to: 20,
  format: "heading1"
});

// Create a bullet list
await document_format({
  doc_id: "doc-12345",
  from: 50,
  to: 150,
  format: "bulletList"
});
```

### 4. `list_documents`

**Purpose**: List all currently open documents

**Parameters**: None

**Returns**:
```json
{
  "content": [{
    "type": "text",
    "text": "[
      {
        \"docId\": \"doc-12345\",
        \"plainText\": \"Hello World...\",
        \"lastModified\": 1699200000000
      },
      {
        \"docId\": \"doc-67890\",
        \"plainText\": \"Another document...\",
        \"lastModified\": 1699200001000
      }
    ]"
  }]
}
```

**Example Usage**:
```javascript
// See what documents are available
const docs = await list_documents();

// Goose can then choose which document to work with
// based on the user's request
```

## Implementation Details

### documentTools.ts

This file defines the tools and their execution logic:

```typescript
// Tool definitions (MCP format)
export const documentTools = [
  documentViewTool,
  documentEditTool,
  documentFormatTool,
  listDocumentsTool,
];

// Extension configuration
export const documentToolsExtension = {
  type: 'frontend' as const,
  name: 'document-tools',
  tools: documentTools,
  instructions: `...`, // Instructions for Goose on how to use the tools
};

// Tool execution function
export async function executeDocumentTool(
  toolName: string,
  args: Record<string, any>
): Promise<{ content: Array<{ type: string; text: string }> }> {
  // Handles tool execution by calling IPC methods
  // Returns results in MCP format
}
```

### useDocumentTools.ts

This React hook handles registration and tool execution:

```typescript
export function useDocumentTools(sessionId: string | null) {
  // 1. Register extension when session starts
  useEffect(() => {
    if (!sessionId) return;
    
    // POST to /extensions/add with documentToolsExtension
    await fetch(getApiUrl('/extensions/add'), {
      method: 'POST',
      body: JSON.stringify({
        session_id: sessionId,
        ...documentToolsExtension,
      }),
    });
  }, [sessionId]);

  // 2. Listen for tool execution requests
  useEffect(() => {
    const handleToolExecution = async (event: MessageEvent) => {
      if (event.data?.type !== 'execute-frontend-tool') return;
      
      const { toolName, args, requestId } = event.data;
      
      // Execute the tool
      const result = await executeDocumentTool(toolName, args);
      
      // Send result back
      window.postMessage({
        type: 'frontend-tool-result',
        requestId,
        result,
      }, '*');
    };
    
    window.addEventListener('message', handleToolExecution);
  }, []);

  return { isRegistered };
}
```

## How It Works

### Step 1: Registration

When a chat session starts, the `useDocumentTools` hook registers the extension:

```
App.tsx (or Chat component)
    ↓
useDocumentTools(sessionId)
    ↓
POST /extensions/add
    ↓
Goose backend receives extension config
    ↓
Tools are now available to Goose AI
```

### Step 2: Tool Execution

When Goose decides to use a tool:

```
Goose AI: "I need to see the document"
    ↓
Calls: document_view({ doc_id: "doc-123" })
    ↓
Backend sends message to frontend
    ↓
window.postMessage({ type: 'execute-frontend-tool', ... })
    ↓
useDocumentTools receives message
    ↓
Calls: executeDocumentTool('document_view', { doc_id: "doc-123" })
    ↓
Calls: window.electron.ipcRenderer.invoke('get-document-state', 'doc-123')
    ↓
Main process returns document state
    ↓
executeDocumentTool formats as MCP response
    ↓
window.postMessage({ type: 'frontend-tool-result', ... })
    ↓
Backend receives result
    ↓
Goose AI: "I can see the document now!"
```

### Step 3: Making Edits

When Goose wants to edit:

```
Goose AI: "I'll add an outline"
    ↓
Calls: document_edit({
  doc_id: "doc-123",
  action: "appendText",
  params: { text: "# Outline\n\n..." }
})
    ↓
(same flow as above)
    ↓
executeDocumentTool('document_edit', ...)
    ↓
window.electron.ipcRenderer.send('execute-document-edit', ...)
    ↓
Main process broadcasts to renderer
    ↓
CollaborativeDocEditor receives edit command
    ↓
Executes: window.gooseEditors['doc-123'].appendText(...)
    ↓
User sees text appear in document!
    ↓
"Goose is editing..." badge shows
```

## What's Next: Phase 5 - Integration

Now that the tools are created, we need to integrate them into the app:

### Tasks

1. **Add useDocumentTools to App**
   - Import the hook
   - Call it with the current sessionId
   - Tools will auto-register when session starts

2. **Test Tool Registration**
   - Start a chat session
   - Check console for: `[useDocumentTools] Document tools extension registered successfully`
   - Verify tools are available to Goose

3. **Test End-to-End Flow**
   - Create a document
   - Type some text
   - Ask Goose: "Help me write a blog post"
   - Goose should:
     - Call `list_documents` to see available docs
     - Call `document_view` to read the doc
     - Call `document_edit` to add content
     - User sees changes in real-time

4. **Handle Edge Cases**
   - What if no documents are open?
   - What if document ID is invalid?
   - What if edit fails?

## Testing Guide

### Test 1: Manual Tool Execution (Console)

```javascript
// In browser console
import { executeDocumentTool } from './extensions/documentTools';

// Test list_documents
const docs = await executeDocumentTool('list_documents', {});
console.log(docs);

// Test document_view
const view = await executeDocumentTool('document_view', { doc_id: 'doc-xxxxx' });
console.log(view);

// Test document_edit
const edit = await executeDocumentTool('document_edit', {
  doc_id: 'doc-xxxxx',
  action: 'appendText',
  params: { text: '\n\nTest from console!' }
});
console.log(edit);
```

### Test 2: Via Goose (Once Integrated)

```
User: "I need help writing a blog post about React hooks"

Expected Goose behavior:
1. Calls list_documents()
2. Sees: [{ docId: "doc-123", plainText: "I need to write...", ... }]
3. Calls document_view({ doc_id: "doc-123" })
4. Sees full content
5. Responds: "I can help! Let me create an outline for you."
6. Calls document_edit({
     doc_id: "doc-123",
     action: "appendText",
     params: { text: "\n\n# React Hooks Guide\n\n## Introduction\n..." }
   })
7. User sees outline appear in document
8. "Goose is editing..." badge shows
```

### Test 3: Formatting

```
User: "Make the title a heading and the list items bold"

Expected Goose behavior:
1. Calls document_view({ doc_id: "doc-123" })
2. Analyzes content to find title and list
3. Calls document_format({
     doc_id: "doc-123",
     from: 0,
     to: 20,
     format: "heading1"
   })
4. Calls document_format({
     doc_id: "doc-123",
     from: 50,
     to: 100,
     format: "bold"
   })
5. User sees formatting applied
```

## Success Criteria

### Phase 4 (Complete ✅)
- ✅ Tool definitions created (4 tools)
- ✅ Tool execution logic implemented
- ✅ MCP format responses
- ✅ IPC integration
- ✅ Error handling
- ✅ React hook for registration
- ✅ Message-based tool execution

### Phase 5 (Next)
- [ ] Hook integrated into App
- [ ] Tools registered on session start
- [ ] End-to-end flow works
- [ ] Goose can read documents
- [ ] Goose can edit documents
- [ ] User sees changes in real-time

## Key Design Decisions

### 1. Frontend Extension Type
**Decision**: Use Frontend extension instead of Stdio/SSE
**Rationale**: 
- No need for separate process
- Direct access to Electron IPC
- Simpler architecture
- Better performance

### 2. Message-Based Communication
**Decision**: Use window.postMessage for tool execution
**Rationale**:
- Standard browser API
- Works across contexts
- Easy to debug
- No additional dependencies

### 3. MCP Format Responses
**Decision**: Return results in MCP tool response format
**Rationale**:
- Compatible with Goose backend
- Standard format for tool results
- Easy to parse and display

### 4. Separate Hook for Registration
**Decision**: Create useDocumentTools hook
**Rationale**:
- Reusable across components
- Handles lifecycle automatically
- Clean separation of concerns
- Easy to test

## Debugging Tips

### Check Tool Registration

```javascript
// In browser console
// After session starts, check if extension is registered
console.log('Tools registered:', window.documentToolsRegistered);
```

### Monitor Tool Calls

```javascript
// Add logging to executeDocumentTool
console.log('[DocumentTools] Tool called:', toolName, args);
console.log('[DocumentTools] Result:', result);
```

### Test IPC Methods

```javascript
// Test IPC methods directly
await window.electron.ipcRenderer.invoke('list-documents');
await window.electron.ipcRenderer.invoke('get-document-state', 'doc-123');
```

### Check Main Process Logs

```bash
# Terminal where you ran `npm run dev`
# Look for:
[Main] Document updated: ...
[Main] Document state stored for: ...
[Main] Execute document edit: ...
```

## Conclusion

**Phase 4 is complete!** 🎉

We now have a complete set of tools that allow Goose to:
- ✅ List all open documents
- ✅ Read document content
- ✅ Make edits (insert, replace, append, clear)
- ✅ Apply formatting (bold, italic, headings, lists, etc.)

The tools are:
- ✅ Defined in MCP format
- ✅ Integrated with Electron IPC
- ✅ Ready to be registered
- ✅ Ready to handle execution

**Next step**: Integrate the `useDocumentTools` hook into the app so the tools are automatically registered when a session starts!

---

**Status**: ✅ Phase 4 Complete
**Next**: Phase 5 - Integration
**Branch**: `spence/doceditor`
**Ready for**: Tool registration and end-to-end testing
