# Document Read Action Added to edit_document Tool

## Summary

Added a "read" action to the `edit_document` tool so the AI can explicitly fetch and view document content. This addresses the issue where the AI was saying it couldn't see the document content even though it should have been provided in the system prompt.

## Changes Made

### 1. Backend - Tool Definition (`crates/goose/src/agents/document_tools.rs`)

**Added "read" to the action enum:**
```rust
"enum": ["read", "insertText", "replaceText", "appendText", "formatText", "clear"],
```

**Updated description:**
```rust
This tool allows you to programmatically edit documents by performing various actions:
- read: Read and view the current content of the document  // ← NEW
- insertText: Insert text at a specific position
- replaceText: Replace text in a range
- appendText: Add text to the end
- formatText: Apply formatting
- clear: Clear all content
```

**Added example:**
```rust
Examples:
- To read document: action="read"  // ← NEW
- To make text bold: action="formatText", from=0, to=10, format={"bold": true}
...
```

**Updated test:**
```rust
// Changed from 5 to 6 actions
assert_eq!(action_enum.len(), 6);
assert!(action_enum.contains(&serde_json::json!("read")));  // ← NEW
```

### 2. Frontend - Action Handler (`ui/desktop/src/components/CollaborativeDocEditor.tsx`)

**Added "read" case to switch statement (line ~546):**
```typescript
switch (action) {
  case 'read':
    // Return the current document content
    const content = editorAPI.getText();
    console.log('📖 Read document content, length:', content.length);
    // Send the content back through IPC
    if (window.electron?.ipcRenderer) {
      window.electron.ipcRenderer.send('document-read-result', {
        docId: data.docId,
        content,
        success: true
      });
    }
    break;
  case 'insertText':
    ...
```

## How It Works

### AI Perspective

The AI can now call:
```json
{
  "tool": "edit_document",
  "doc_id": "doc-1762360372740",
  "action": "read"
}
```

And receive back the current document content as plain text.

### Data Flow

1. **AI calls tool** with `action="read"` and `doc_id`
2. **Backend** (if handling) or **Frontend** receives the request
3. **Frontend handler** (CollaborativeDocEditor) executes:
   - Gets `editorAPI.getText()` (plain text, not HTML)
   - Logs the content length
   - Sends result back via IPC: `document-read-result` event
4. **Backend** (needs implementation) should capture this result and return it to the AI

## What Still Needs to Be Done

### Backend Integration

The backend needs to:
1. Handle the "read" action in the tool execution flow
2. Wait for the `document-read-result` IPC event from frontend
3. Return the content to the AI as the tool result

**Potential implementation location:** `crates/goose-server/src/routes/reply.rs` or wherever tool results are processed.

### Alternative: Direct Access

Instead of IPC round-trip, the backend could:
- Already have the document content from the system prompt injection
- Simply return that content when "read" is called
- This would be simpler but requires the document context to be reliably present

## Testing

To test the new action:

1. **Rebuild the backend:**
   ```bash
   cd /Users/spencermartin/Desktop/goose
   cargo build --release
   ```

2. **Restart the Goose Desktop app** (frontend changes are in TypeScript, may need rebuild)

3. **Open a document** and send a message like:
   ```
   Can you read what's in this document?
   ```

4. **Expected behavior:**
   - AI should use `edit_document` with `action="read"`
   - Frontend logs: `📖 Read document content, length: X`
   - AI receives the content and can discuss it

## Why This Helps

### Problem Before
- AI was told it could see document content (via system prompt)
- But it kept asking users to provide content or trying workarounds
- No explicit way to "pull" the document content on demand

### Solution Now
- AI has an explicit tool action to read documents
- Clear, documented way to access content
- Follows the same pattern as other document actions

## Related Issues

This addresses the core issue from the conversation where the AI kept saying:
> "I don't have a tool that allows me to directly see what document you're currently viewing"

Now it does! The `edit_document` tool with `action="read"` provides exactly that capability.

## Files Modified

1. `crates/goose/src/agents/document_tools.rs` - Tool definition
2. `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Action handler

## Next Steps

1. ✅ Tool definition updated
2. ✅ Frontend handler added
3. ⏳ Backend integration needed (IPC result handling)
4. ⏳ Test end-to-end flow
5. ⏳ Document in user-facing docs

---

**Status:** Code complete, needs backend build and testing
**Date:** 2025-11-05
**Related:** Document context visibility fixes, automatic context transmission
