# Collaborative Document Editor - Implementation Status

## 🎯 Vision

A collaborative document editor where:
1. **User types** in a rich text document
2. **Goose sees** the document in real-time (like a second collaborator)
3. **Goose can offer help** proactively via chat
4. **User can ask Goose** questions about the document via chat
5. **Goose can edit** the document directly with visual feedback

## ✅ What's Implemented

### 1. Document Editor (CollaborativeDocEditor.tsx)
- ✅ Rich text editor using Tiptap
- ✅ Full formatting toolbar (bold, italic, headings, lists, etc.)
- ✅ "Goose enabled" badge showing collaboration is active
- ✅ "Goose is editing..." badge when Goose makes changes
- ✅ "Ask Goose" button to request help
- ✅ Toggle to enable/disable Goose collaboration

### 2. Real-time Document Awareness
- ✅ Document dispatches `document-updated` events on every change
- ✅ Events include: docId, content (HTML), plainText, selection, timestamp
- ✅ Goose can "see" what the user types in real-time

### 3. Programmatic API
- ✅ `window.gooseEditors[docId]` API exposed globally
- ✅ Methods available:
  - `insertText(text, position?)` - Insert text at position
  - `replaceText(from, to, text)` - Replace range with text
  - `appendText(text)` - Add text to end
  - `formatText(from, to, format)` - Apply formatting
  - `getContent()` - Get HTML content
  - `getText()` - Get plain text
  - `getSelection()` - Get current selection
  - `clear()` - Clear document

### 4. Chat Integration (Partial)
- ✅ "Ask Goose" button dispatches `populate-chat-input` event
- ✅ Chat input (`ChatInput.tsx`) listens for the event
- ✅ Chat input is populated with document context
- ✅ User can modify the message before sending
- ⏳ Document metadata not yet passed to backend
- ⏳ No visual badge showing document context in messages

### 5. Sidecar Integration
- ✅ Document editor opens from the "plus button" menu
- ✅ Opens in the bento box alongside chat
- ✅ Multiple documents can be open simultaneously
- ✅ Each document has a unique ID

## 🔄 What's In Progress

### Chat → Backend Integration
**Status**: Needs implementation

**What's needed**:
1. Store document metadata when "Ask Goose" is clicked
2. Pass metadata with the message when user sends it
3. Backend receives document context in message metadata

**Code location**: `ChatInput.tsx` - `performSubmit` function

**Example**:
```typescript
// In performSubmit, add:
const messageMetadata = documentContext ? {
  type: 'document-assist',
  docId: documentContext.docId,
  content: documentContext.content,
  selectedText: documentContext.selectedText,
  selection: documentContext.selection,
} : undefined;

// Pass to backend with message
```

## ⏳ What's Next

### Phase 3: Goose Backend Tools

**Goal**: Enable Goose to interact with documents programmatically.

**Components needed**:

#### 3.1 IPC Bridge (Electron)
- Listen for `document-updated` events from renderer
- Store document state in memory
- Forward to Goose backend when requested
- Receive edit commands from Goose
- Execute edits via `window.gooseEditors[docId]`

**Files to create/modify**:
- `ui/desktop/electron/main.ts` - Add IPC handlers
- `ui/desktop/electron/documentBridge.ts` (new) - Document state management

#### 3.2 Goose Backend Tools (Python)
- `document_view(docId)` - Get current document content
- `document_edit(docId, action, params)` - Edit document
- `document_format(docId, from, to, format)` - Format text

**Files to create/modify**:
- `goose/toolkit/document.py` (new) - Document tools
- `goose/cli/session.py` - Register tools
- `goose/cli/prompt/system.txt` - Add document context

### Phase 4: Proactive Assistance

**Goal**: Goose can offer help without being asked.

**Features**:
- Detect patterns (unformatted lists, long paragraphs, etc.)
- Send proactive messages to chat
- Offer specific actions ("Would you like me to format this?")

### Phase 5: Visual Enhancements

**Goal**: Make collaboration feel natural.

**Features**:
- Goose cursor showing where Goose is "looking"
- Animated edits (typing effect)
- Presence indicators
- Smooth transitions

## 🧪 How to Test (Current State)

### Test 1: Basic Document Creation
1. Click the "plus button" in the top right
2. Click "New Document"
3. Document editor opens in the bento box
4. Type some text
5. ✅ Should see "Goose enabled" badge

### Test 2: Real-time Updates
1. Open browser console (Cmd+Option+I)
2. Type in the document
3. ✅ Should see `document-updated` events in console
4. ✅ Events include docId, content, plainText, selection

### Test 3: Programmatic API
1. Open browser console
2. Create a new document
3. Find the docId (shown in the header)
4. Test API:
```javascript
// Get the editor
const editor = window.gooseEditors['doc-xxxxx'];

// Insert text
editor.insertText('Hello from Goose!');

// Get content
console.log(editor.getText());

// Format text
editor.formatText(0, 5, 'bold');
```
5. ✅ Should see changes in the document
6. ✅ Should see "Goose is editing..." badge

### Test 4: Chat Integration
1. Create a new document and type some text
2. Click "Ask Goose" button
3. ✅ Alert appears with document ID
4. ✅ Chat input is populated with document context
5. ✅ Can modify the message
6. ⏳ Send message (metadata not yet passed to backend)

## 📊 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    User Interface Layer                      │
├─────────────────────────────────────────────────────────────┤
│  CollaborativeDocEditor  │  Chat Panel (ChatInput)          │
│  - Tiptap Editor         │  - User messages                 │
│  - Toolbar               │  - Goose responses               │
│  - "Ask Goose" button ✅ │  - Populated from doc ✅         │
│  - Goose status ✅       │  - Metadata TODO ⏳              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ├─ Events ✅
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    Event System (Window Events)              │
├──────────────────────────────────────────────────────────────┤
│  • document-updated ✅ : Real-time content changes           │
│  • populate-chat-input ✅ : Pre-fill chat with context       │
│  • goose-doc-assist ✅ : Request Goose assistance            │
└──────────────────────────────────────────────────────────────┘
                            │
                            ├─ IPC ⏳ (NOT YET IMPLEMENTED)
                            │
┌───────────────────────────▼──────────────────────────────────┐
│                    Electron Main Process                      │
├───────────────────────────────────────────────────────────────┤
│  • Listen for document-updated events ⏳                      │
│  • Forward to Goose backend ⏳                                │
│  • Receive Goose edit commands ⏳                             │
│  • Execute via window.gooseEditors[docId] API ⏳              │
└───────────────────────────────────────────────────────────────┘
                            │
                            │
┌───────────────────────────▼───────────────────────────────────┐
│                    Goose Backend (AI Agent)                    │
├────────────────────────────────────────────────────────────────┤
│  Tools: ⏳ (NOT YET IMPLEMENTED)                               │
│  • document_view(docId): Get current document content         │
│  • document_edit(docId, action, params): Edit document        │
│  • document_format(docId, from, to, format): Format text      │
│                                                                 │
│  Capabilities: ⏳ (NOT YET IMPLEMENTED)                        │
│  • Monitor document changes in real-time                       │
│  • Offer proactive assistance                                 │
│  • Respond to user questions about document                   │
│  • Make edits based on user requests                          │
└─────────────────────────────────────────────────────────────────┘
```

## 🎬 User Flow Examples

### Example 1: User Asks for Help (Current State)

**What works now**:
1. User types: "I need to write a blog post about React hooks"
2. User clicks "Ask Goose" button
3. ✅ Alert shows document ID
4. ✅ Chat input is populated with context
5. ✅ User can modify message
6. User sends message
7. ⏳ Goose receives message (but no document context yet)

**What will work after Phase 3**:
1. User types: "I need to write a blog post about React hooks"
2. User clicks "Ask Goose" button
3. Chat input is populated with context
4. User sends message
5. ✅ Goose receives message WITH document context
6. ✅ Goose uses `document_view` to see full document
7. ✅ Goose responds: "I can help you with that! Let me create an outline."
8. ✅ Goose uses `document_edit` to append outline
9. ✅ User sees outline appear in document
10. ✅ "Goose is editing..." badge appears

### Example 2: Proactive Assistance (Phase 4)

**Future implementation**:
1. User types a long paragraph (500+ words) without breaks
2. ✅ Goose detects pattern via `document-updated` events
3. ✅ Goose sends proactive message: "I noticed you have a long paragraph. Would you like me to break it into smaller sections?"
4. User responds: "Yes, please!"
5. ✅ Goose analyzes content and adds paragraph breaks
6. ✅ User sees changes in real-time

## 📝 Key Files

### Frontend (TypeScript/React)
- ✅ `ui/desktop/src/components/CollaborativeDocEditor.tsx` - Document editor
- ✅ `ui/desktop/src/components/DocEditor.css` - Editor styling
- ✅ `ui/desktop/src/components/ChatInput.tsx` - Chat input with event listener
- ✅ `ui/desktop/src/components/Layout/MainPanelLayout.tsx` - Bento box integration
- ✅ `ui/desktop/src/components/Layout/SidecarInvoker.tsx` - Plus button menu

### Backend (To be implemented)
- ⏳ `ui/desktop/electron/main.ts` - IPC handlers
- ⏳ `ui/desktop/electron/documentBridge.ts` (new) - Document state
- ⏳ `goose/toolkit/document.py` (new) - Document tools
- ⏳ `goose/cli/session.py` - Tool registration
- ⏳ `goose/cli/prompt/system.txt` - System prompt updates

### Documentation
- ✅ `COLLABORATIVE_DOC_FULL_IMPLEMENTATION.md` - Complete implementation plan
- ✅ `COLLABORATIVE_EDITOR_STATUS.md` - This file
- ✅ `GOOSE_DOCUMENT_COLLABORATION.md` - API reference
- ✅ `CHAT_DOCUMENT_INTEGRATION.md` - Chat integration details
- ✅ `CONSOLE_TEST_COMMANDS.md` - Testing commands

## 🚀 Quick Start for Development

### 1. Start the App
```bash
cd /Users/spencermartin/Desktop/goose
source bin/activate-hermit
cd ui/desktop
npm run dev
```

### 2. Test Current Features
1. Open the app
2. Click "plus button" → "New Document"
3. Type some text
4. Open browser console
5. Test API: `window.gooseEditors['doc-xxxxx'].insertText('Test')`
6. Click "Ask Goose"
7. Verify chat input is populated

### 3. Next Development Steps
1. **Complete Chat Integration**:
   - Store document metadata in `ChatInput.tsx`
   - Pass metadata with message to backend
   - Add visual badge to messages with document context

2. **Implement IPC Bridge**:
   - Add IPC handlers in Electron main process
   - Create document state management
   - Test bidirectional communication

3. **Create Goose Tools**:
   - Implement `document_view` tool
   - Implement `document_edit` tool
   - Test end-to-end flow

## 🐛 Known Issues

1. **Alert on "Ask Goose" button**: Temporary debugging alert should be removed once chat integration is complete.
2. **No persistence**: Documents are not saved; content is lost on refresh.
3. **No document list**: No UI to browse or reopen previous documents.
4. **No export**: No functionality to export documents.

## 📚 Additional Resources

- [Tiptap Documentation](https://tiptap.dev/)
- [Tiptap Collaboration Guide](https://tiptap.dev/docs/editor/extensions/functionality/collaboration)
- [Electron IPC Documentation](https://www.electronjs.org/docs/latest/tutorial/ipc)

## 🎉 Success Criteria

### Phase 1 (Complete ✅)
- ✅ User can create a new document
- ✅ Document shows "Goose enabled" indicator
- ✅ User can type in the document
- ✅ Document dispatches real-time updates
- ✅ `window.gooseEditors` API is accessible
- ✅ API methods work correctly

### Phase 2 (Partial ✅)
- ✅ User can click "Ask Goose"
- ✅ Chat input is populated with document context
- ⏳ User can send message with document context
- ⏳ Message includes document metadata

### Phase 3 (Not Started ⏳)
- ⏳ Goose can view document content
- ⏳ Goose can edit document via tools
- ⏳ Edits appear in real-time with visual indicators

### Phase 4 (Not Started ⏳)
- ⏳ Goose can offer proactive assistance
- ⏳ Goose detects patterns and suggests improvements

### Phase 5 (Not Started ⏳)
- ⏳ Goose cursor visible in document
- ⏳ Animated edits
- ⏳ Presence indicators

## 📞 Support

For questions or issues:
1. Check the documentation files in the project root
2. Review the implementation plan in `COLLABORATIVE_DOC_FULL_IMPLEMENTATION.md`
3. Test the API using commands in `CONSOLE_TEST_COMMANDS.md`
4. Check the browser console for event logs

---

**Last Updated**: 2025-11-05
**Status**: Phase 1 Complete, Phase 2 Partial, Phases 3-5 Pending
