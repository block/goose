# Collaborative Document Editor - Complete Implementation Plan

## Overview
This document outlines the complete implementation for a collaborative document editor where Goose acts as a second collaborator, able to see document changes in real-time, offer proactive assistance, and respond to user prompts about the document.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Interface Layer                      │
├─────────────────────────────────────────────────────────────┤
│  CollaborativeDocEditor  │  Chat Panel (ChatInput)          │
│  - Tiptap Editor         │  - User messages                 │
│  - Toolbar               │  - Goose responses               │
│  - "Ask Goose" button    │  - Document context badge        │
│  - Goose status          │                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ├─ Events ─┐
                            │          │
┌───────────────────────────▼──────────▼───────────────────────┐
│                    Event System (Window Events)               │
├───────────────────────────────────────────────────────────────┤
│  • document-updated: Real-time content changes                │
│  • populate-chat-input: Pre-fill chat with context           │
│  • goose-doc-assist: Request Goose assistance                │
└───────────────────────────────────────────────────────────────┘
                            │
                            ├─ IPC ─┐
                            │       │
┌───────────────────────────▼───────▼───────────────────────────┐
│                    Electron Main Process                       │
├────────────────────────────────────────────────────────────────┤
│  • Listen for document-updated events                          │
│  • Forward to Goose backend                                    │
│  • Receive Goose edit commands                                 │
│  • Execute via window.gooseEditors[docId] API                  │
└────────────────────────────────────────────────────────────────┘
                            │
                            │
┌───────────────────────────▼────────────────────────────────────┐
│                    Goose Backend (AI Agent)                     │
├─────────────────────────────────────────────────────────────────┤
│  Tools:                                                          │
│  • document_view(docId): Get current document content           │
│  • document_edit(docId, action, params): Edit document          │
│  • document_format(docId, from, to, format): Format text        │
│                                                                  │
│  Capabilities:                                                   │
│  • Monitor document changes in real-time                         │
│  • Offer proactive assistance                                   │
│  • Respond to user questions about document                     │
│  • Make edits based on user requests                            │
└──────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Real-time Document Awareness ✅ (DONE)

**Status**: Implemented in CollaborativeDocEditor.tsx

**Features**:
- ✅ Document editor dispatches `document-updated` events on every change
- ✅ Events include: docId, content (HTML), plainText, selection, timestamp
- ✅ Visual indicators for Goose presence ("Goose enabled" badge)
- ✅ Visual indicators for Goose activity ("Goose is editing..." badge)
- ✅ Programmatic API exposed via `window.gooseEditors[docId]`

**API Methods**:
```javascript
window.gooseEditors[docId] = {
  insertText(text, position?),
  replaceText(from, to, text),
  appendText(text),
  formatText(from, to, format),
  getContent(), // Returns HTML
  getText(), // Returns plain text
  getSelection(), // Returns { from, to, text }
  clear()
}
```

### Phase 2: Chat Integration (IN PROGRESS)

**Goal**: Connect the document editor with the chat panel so users can ask questions about their document.

**Components to Update**:

#### 2.1 ChatInput.tsx
Add event listener for `populate-chat-input` event:

```typescript
useEffect(() => {
  const handlePopulateChatInput = (event: CustomEvent) => {
    const { message, docId, metadata } = event.detail;
    
    // Update input value with document context
    setInputValue(message);
    
    // Store metadata for when message is sent
    setDocumentContext({
      docId,
      metadata,
    });
    
    // Focus the input
    inputRef.current?.focus();
  };
  
  window.addEventListener('populate-chat-input', handlePopulateChatInput as EventListener);
  
  return () => {
    window.removeEventListener('populate-chat-input', handlePopulateChatInput as EventListener);
  };
}, []);
```

#### 2.2 Message Metadata
When sending a message, include document context:

```typescript
const handleSendMessage = () => {
  const message = {
    role: 'user',
    content: inputValue,
    metadata: documentContext ? {
      type: 'document-assist',
      docId: documentContext.docId,
      ...documentContext.metadata,
    } : undefined,
  };
  
  onSendMessage(message);
  setDocumentContext(null); // Clear after sending
};
```

#### 2.3 Visual Indicators
Add a badge to messages that have document context:

```tsx
{message.metadata?.type === 'document-assist' && (
  <div className="flex items-center gap-1 px-2 py-1 bg-blue-50 dark:bg-blue-900/20 rounded text-xs text-blue-600 dark:text-blue-400">
    <FileEdit className="w-3 h-3" />
    <span>Document: {message.metadata.docId}</span>
  </div>
)}
```

### Phase 3: Goose Backend Tools (NEXT)

**Goal**: Create tools for Goose to interact with documents.

#### 3.1 IPC Bridge (Electron Main Process)

```typescript
// Listen for document updates from renderer
ipcMain.on('document-updated', (event, data) => {
  const { docId, content, plainText, selection, timestamp } = data;
  
  // Store current document state
  documentStore.set(docId, {
    content,
    plainText,
    selection,
    timestamp,
    lastModified: Date.now(),
  });
  
  // Notify Goose backend if monitoring this document
  if (gooseMonitoredDocs.has(docId)) {
    gooseBackend.notifyDocumentUpdate(docId, data);
  }
});

// Handle Goose edit commands
ipcMain.on('goose-edit-document', (event, data) => {
  const { docId, action, params } = data;
  
  // Send to renderer to execute via window.gooseEditors
  mainWindow.webContents.send('execute-document-edit', {
    docId,
    action,
    params,
  });
});
```

#### 3.2 Renderer IPC Handler

```typescript
// In CollaborativeDocEditor or a global handler
useEffect(() => {
  const handleExecuteEdit = (event: any, data: any) => {
    const { docId, action, params } = data;
    const editor = (window as any).gooseEditors?.[docId];
    
    if (!editor) {
      console.warn('Editor not found for docId:', docId);
      return;
    }
    
    switch (action) {
      case 'insertText':
        editor.insertText(params.text, params.position);
        break;
      case 'replaceText':
        editor.replaceText(params.from, params.to, params.text);
        break;
      case 'appendText':
        editor.appendText(params.text);
        break;
      case 'formatText':
        editor.formatText(params.from, params.to, params.format);
        break;
      case 'clear':
        editor.clear();
        break;
    }
  };
  
  window.electron?.ipcRenderer?.on('execute-document-edit', handleExecuteEdit);
  
  return () => {
    window.electron?.ipcRenderer?.off('execute-document-edit', handleExecuteEdit);
  };
}, []);
```

#### 3.3 Goose Backend Tools (Python)

```python
# goose/toolkit/document.py

class DocumentTool:
    """Tools for Goose to interact with collaborative documents."""
    
    def document_view(self, doc_id: str) -> dict:
        """
        Get the current content of a document.
        
        Args:
            doc_id: The document identifier
            
        Returns:
            dict with keys: content (HTML), plainText, selection, timestamp
        """
        # Request document state from Electron
        return self.ipc_bridge.get_document_state(doc_id)
    
    def document_edit(self, doc_id: str, action: str, **params) -> bool:
        """
        Edit a document.
        
        Args:
            doc_id: The document identifier
            action: One of 'insertText', 'replaceText', 'appendText', 'clear'
            **params: Action-specific parameters
            
        Returns:
            True if successful
        """
        return self.ipc_bridge.send_edit_command(doc_id, action, params)
    
    def document_format(self, doc_id: str, from_pos: int, to_pos: int, format: str) -> bool:
        """
        Format text in a document.
        
        Args:
            doc_id: The document identifier
            from_pos: Start position
            to_pos: End position
            format: Format to apply (bold, italic, heading1, etc.)
            
        Returns:
            True if successful
        """
        return self.ipc_bridge.send_edit_command(
            doc_id, 
            'formatText', 
            {'from': from_pos, 'to': to_pos, 'format': format}
        )
```

### Phase 4: Proactive Assistance

**Goal**: Goose can initiate conversations to offer help.

#### 4.1 Document Analysis
Goose monitors document changes and analyzes content:

```python
def analyze_document_for_assistance(doc_id: str, content: str):
    """Analyze document and offer proactive help."""
    
    # Detect patterns that might need assistance
    patterns = {
        'unformatted_list': r'^\d+\.\s+.+\n\d+\.\s+',  # Numbered list as plain text
        'code_block': r'```[\s\S]*?```',  # Code blocks
        'long_paragraph': lambda text: len(text.split('\n\n')) == 1 and len(text) > 500,
        'spelling_errors': lambda text: len(spell_check(text)) > 5,
    }
    
    suggestions = []
    
    if re.search(patterns['unformatted_list'], content):
        suggestions.append({
            'type': 'formatting',
            'message': 'I noticed you have a numbered list. Would you like me to format it as a proper ordered list?',
            'action': 'format_as_list',
        })
    
    if len(suggestions) > 0:
        # Send suggestion to chat
        send_proactive_message(doc_id, suggestions[0]['message'])
```

#### 4.2 Proactive Chat Messages
Goose can send messages to the chat:

```typescript
// Renderer receives proactive message from backend
window.electron?.ipcRenderer?.on('goose-proactive-message', (event, data) => {
  const { docId, message, suggestedActions } = data;
  
  // Add Goose message to chat
  addMessage({
    role: 'assistant',
    content: message,
    metadata: {
      type: 'proactive-assistance',
      docId,
      suggestedActions,
    },
  });
});
```

### Phase 5: Visual Enhancements

**Goal**: Make collaboration feel more natural and intuitive.

#### 5.1 Goose Cursor
Show where Goose is "looking" in the document:

```tsx
// Add to CollaborativeDocEditor
const [gooseCursorPosition, setGooseCursorPosition] = useState<number | null>(null);

// Render Goose cursor
{gooseCursorPosition !== null && (
  <div 
    className="absolute w-0.5 h-5 bg-blue-500 animate-pulse"
    style={{
      left: calculateCursorPosition(gooseCursorPosition).x,
      top: calculateCursorPosition(gooseCursorPosition).y,
    }}
  >
    <div className="absolute -top-6 left-0 flex items-center gap-1 px-2 py-1 bg-blue-500 text-white text-xs rounded whitespace-nowrap">
      <Bot className="w-3 h-3" />
      <span>Goose</span>
    </div>
  </div>
)}
```

#### 5.2 Edit Animations
Animate Goose's edits:

```typescript
const gooseInsertText = (text: string, position?: number) => {
  setGooseIsTyping(true);
  
  // Animate character by character
  let currentPos = position ?? editor.state.doc.content.size;
  let charIndex = 0;
  
  const typeInterval = setInterval(() => {
    if (charIndex < text.length) {
      editor.chain().focus().insertContentAt(currentPos, text[charIndex]).run();
      currentPos++;
      charIndex++;
    } else {
      clearInterval(typeInterval);
      setGooseIsTyping(false);
    }
  }, 50); // Type at 20 chars/second
};
```

#### 5.3 Collaboration Presence
Show active collaborators:

```tsx
<div className="flex items-center gap-2">
  <div className="flex items-center gap-1 px-2 py-1 bg-gray-100 dark:bg-gray-800 rounded text-xs">
    <User className="w-3 h-3" />
    <span>You</span>
  </div>
  {gooseEnabled && (
    <div className="flex items-center gap-1 px-2 py-1 bg-blue-100 dark:bg-blue-900 rounded text-xs">
      <Bot className="w-3 h-3" />
      <span>Goose</span>
      {gooseIsTyping && (
        <div className="flex gap-0.5">
          <div className="w-1 h-1 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
          <div className="w-1 h-1 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
          <div className="w-1 h-1 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
        </div>
      )}
    </div>
  )}
</div>
```

## User Flow Examples

### Example 1: User Asks for Help

1. User types in document: "I need to write a blog post about React hooks"
2. User clicks "Ask Goose" button
3. Chat input is populated with: "I'm working on a document and need help. Here's what I have so far: I need to write a blog post about React hooks"
4. User sends message (or modifies it first)
5. Goose receives message with document context
6. Goose responds in chat: "I can help you with that! Let me create an outline for your blog post."
7. Goose uses `document_edit` to append:
   ```
   # React Hooks: A Comprehensive Guide
   
   ## Introduction
   - What are React Hooks?
   - Why were they introduced?
   
   ## Core Hooks
   - useState
   - useEffect
   - useContext
   
   ## Advanced Hooks
   - useReducer
   - useMemo
   - useCallback
   ```

### Example 2: Proactive Assistance

1. User types a long paragraph (500+ words) without breaks
2. Goose detects this pattern
3. Goose sends proactive message in chat: "I noticed you have a long paragraph. Would you like me to break it into smaller, more readable sections?"
4. User responds: "Yes, please!"
5. Goose analyzes content and adds paragraph breaks at logical points

### Example 3: Formatting Help

1. User types:
   ```
   1. First item
   2. Second item
   3. Third item
   ```
2. User selects the text and clicks "Ask Goose"
3. User types: "Format this as a proper list"
4. Goose uses `document_format` to convert to ordered list
5. Document now shows properly formatted list with bullets

## Testing Plan

### Unit Tests
- [ ] Test `document-updated` event dispatching
- [ ] Test `populate-chat-input` event handling
- [ ] Test `window.gooseEditors` API methods
- [ ] Test IPC message passing

### Integration Tests
- [ ] Test full flow: User types → Event → Backend → Edit → UI update
- [ ] Test chat integration: Ask Goose → Populate input → Send → Response → Edit
- [ ] Test proactive assistance: Pattern detection → Message → User response → Action

### E2E Tests
- [ ] Create new document
- [ ] Type content
- [ ] Ask Goose for help
- [ ] Verify Goose makes edits
- [ ] Verify visual indicators appear
- [ ] Test with multiple documents open

## Next Steps

1. **Implement Chat Integration** (Phase 2)
   - Add event listener to ChatInput.tsx
   - Add document context to messages
   - Add visual indicators

2. **Set up IPC Bridge** (Phase 3)
   - Add IPC handlers in Electron main process
   - Add renderer IPC handlers
   - Test bidirectional communication

3. **Create Goose Backend Tools** (Phase 3)
   - Implement document_view tool
   - Implement document_edit tool
   - Implement document_format tool
   - Add to Goose's available tools

4. **Test End-to-End** (All Phases)
   - Manual testing of full workflow
   - Fix any issues
   - Optimize performance

5. **Add Visual Enhancements** (Phase 5)
   - Goose cursor
   - Edit animations
   - Presence indicators

## Files to Modify

### Frontend (TypeScript/React)
- ✅ `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Real-time updates
- [ ] `ui/desktop/src/components/ChatInput.tsx` - Event listener
- [ ] `ui/desktop/src/components/RichChatInput.tsx` - Event listener (if used)
- [ ] `ui/desktop/src/components/Message.tsx` - Document context badge
- [ ] `ui/desktop/src/App.tsx` or global handler - IPC receiver for edits

### Backend (Electron Main Process)
- [ ] `ui/desktop/electron/main.ts` - IPC handlers for document events
- [ ] `ui/desktop/electron/documentBridge.ts` (new) - Document state management

### Backend (Goose AI)
- [ ] `goose/toolkit/document.py` (new) - Document tools
- [ ] `goose/cli/session.py` - Register document tools
- [ ] `goose/cli/prompt/system.txt` - Add document collaboration context

## Success Criteria

✅ User can create a new document from the plus button
✅ Document shows "Goose enabled" indicator
✅ User can type in the document
✅ Document dispatches real-time updates
✅ `window.gooseEditors` API is accessible
- [ ] User can click "Ask Goose" and chat input is populated
- [ ] User can send message with document context
- [ ] Goose can view document content
- [ ] Goose can edit document via tools
- [ ] Edits appear in real-time with visual indicators
- [ ] Goose can offer proactive assistance
- [ ] Multiple documents can be open simultaneously
- [ ] Performance is smooth with real-time updates
