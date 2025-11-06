# Automatic Document Context Fix

## Issue
The AI was not receiving document content when users typed messages directly in chat (without clicking "Ask Goose"). The document context was only being sent when explicitly clicking the "Ask Goose" button, which meant:

1. Users had to click "Ask Goose" every time they wanted to interact with the document
2. The AI couldn't see document updates between "Ask Goose" clicks
3. The workflow was clunky and not intuitive

## Root Cause
The document context was only being captured and sent when the "Ask Goose" button was clicked. When users typed messages directly in the chat input, no document context was attached to those messages.

## Solution
Implemented **automatic document context tracking** that continuously updates the chat input with the latest document state:

### 1. **Continuous Context Updates** (`CollaborativeDocEditor.tsx`)
```typescript
// Send document updates to Goose backend in real-time
// AND keep chat input updated with latest document context
useEffect(() => {
  if (!editor || !docId || !gooseEnabled) return;

  const handleUpdate = () => {
    const content = editor.getHTML();
    const plainText = editor.getText();
    const { from, to } = editor.state.selection;
    const selectedText = editor.state.doc.textBetween(from, to);
    
    const updateData = {
      docId,
      content,
      plainText,
      selection: selectedText ? { from, to, text: selectedText } : undefined,
      timestamp: Date.now(),
    };
    
    // CRITICAL: Update the active document context for chat
    // This ensures ANY message sent while document is open includes context
    window.dispatchEvent(new CustomEvent('set-active-document-context', {
      detail: updateData
    }));
  };

  // Listen to editor updates
  editor.on('update', handleUpdate);
  editor.on('selectionUpdate', handleUpdate);
  
  // Send initial context when editor loads
  handleUpdate();

  return () => {
    editor.off('update', handleUpdate);
    editor.off('selectionUpdate', handleUpdate);
    
    // Clear active document context when editor unmounts
    window.dispatchEvent(new CustomEvent('set-active-document-context', {
      detail: null
    }));
  };
}, [editor, docId, gooseEnabled]);
```

### 2. **Chat Input Listener** (`ChatInput.tsx`)
```typescript
// Listen for active document context updates (automatic, not from "Ask Goose" button)
const handleSetActiveDocumentContext = (event: CustomEvent) => {
  const contextData = event.detail;
  
  if (contextData === null) {
    // Document closed, clear context
    console.log('📄 Active document context cleared');
    setDocumentContext(null);
  } else {
    // Document updated, store latest context
    console.log('📄 Active document context updated:', {
      docId: contextData.docId,
      contentLength: contextData.content?.length || contextData.plainText?.length,
      hasSelection: !!contextData.selection
    });
    
    setDocumentContext({
      docId: contextData.docId,
      content: contextData.content || contextData.plainText || '',
      selection: contextData.selection,
      timestamp: contextData.timestamp
    });
  }
};

window.addEventListener('set-active-document-context', handleSetActiveDocumentContext as EventListener);
```

### 3. **Automatic Context Inclusion**
The existing `performSubmit` function in `ChatInput.tsx` already includes document context with every message:

```typescript
// Include document context if available
if (documentContext) {
  messageData.documentContext = {
    docId: documentContext.docId,
    content: documentContext.content,
    selection: documentContext.selection,
    timestamp: documentContext.timestamp
  };
  console.log('📄 Including document context in message:', messageData.documentContext);
}
```

## How It Works

1. **Document Opens**: When a document editor is mounted, it immediately sends the initial context via `set-active-document-context` event
2. **Document Updates**: Every time the user types or selects text in the document, the context is automatically updated
3. **Message Sent**: When the user sends ANY message (whether via "Ask Goose" button or direct typing), the latest document context is automatically included
4. **Document Closes**: When the document editor unmounts, it sends a `null` context to clear the stored context

## Benefits

✅ **Seamless UX**: Users can type directly in chat without clicking "Ask Goose"  
✅ **Always Up-to-Date**: AI always has the latest document content and selection  
✅ **Automatic**: No manual intervention required  
✅ **Clean Separation**: "Ask Goose" button still works for pre-filling chat with context message  
✅ **Smart Cleanup**: Context is automatically cleared when document closes  

## Testing

To verify the fix works:

1. Open a new document in Goose Desktop
2. Type some content in the document
3. **Without clicking "Ask Goose"**, type a message directly in chat like "make this better"
4. **Expected**: AI should immediately see and reference the document content
5. **Not Expected**: AI should NOT ask for the content or try to screen capture

## Implementation Files

- **`ui/desktop/src/components/CollaborativeDocEditor.tsx`**: Sends continuous context updates
- **`ui/desktop/src/components/ChatInput.tsx`**: Listens for and stores active document context
- **`crates/goose-server/src/routes/reply.rs`**: Backend already handles document context (no changes needed)

## Status

✅ **IMPLEMENTED** - Code changes complete  
⏳ **TESTING** - Ready for user verification  

## Next Steps

1. Restart the Goose Desktop application (frontend only - no backend rebuild needed)
2. Test the automatic context inclusion
3. Verify AI can see document content without clicking "Ask Goose"

---

**Date:** 2025-11-05  
**Branch:** `spence/doceditor`  
**Commit Status:** Not committed (working locally)
