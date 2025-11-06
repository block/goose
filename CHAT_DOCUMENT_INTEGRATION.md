# Chat + Document Integration

## Overview

The collaborative document editor now integrates with the chat system, allowing users to:
1. Ask Goose about their document via chat
2. See Goose respond by editing the document directly
3. Have a natural conversation while Goose collaborates in real-time

---

## User Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    Goose Desktop                                 │
├──────────────────────┬──────────────────────────────────────────┤
│                      │                                           │
│   Chat Panel         │   Document Editor                         │
│                      │                                           │
│  User: "Make this    │   "The quick brown fox..."                │
│   more exciting"     │                                           │
│                      │   🤖 Goose enabled                        │
│  Goose: "I'll make   │                                           │
│   it more dynamic!"  │   🤖 Goose is editing...                 │
│                      │                                           │
│  *Goose edits doc*   │   "The lightning-fast brown fox          │
│                      │    leaps gracefully..."                   │
│                      │                                           │
│  Goose: "Done! I've  │   ✅ Edit complete                       │
│   made it more       │                                           │
│   exciting."         │                                           │
│                      │                                           │
└──────────────────────┴──────────────────────────────────────────┘
```

---

## How It Works

### 1. User Clicks "Ask Goose"

When the user clicks the "Ask Goose" button in the document editor:

```typescript
// In CollaborativeDocEditor.tsx
const handleAskGoose = () => {
  const content = editor.getHTML();
  const plainText = editor.getText();
  const selectedText = editor.state.doc.textBetween(from, to);
  
  // Create context message
  const contextMessage = selectedText 
    ? `I'm working on a document and need help with this section:\n\n"${selectedText}"\n\n...`
    : `I'm working on a document and need help. Here's what I have so far:\n\n${plainText}...`;
  
  // Populate chat input
  window.dispatchEvent(new CustomEvent('populate-chat-input', {
    detail: {
      message: contextMessage,
      docId,
      metadata: {
        type: 'document-assist',
        docId,
        content,
        selectedText,
        selection: { from, to },
      }
    }
  }));
};
```

### 2. Chat Input Gets Populated

The chat component listens for this event and populates the input:

```typescript
// In ChatInput.tsx or similar
useEffect(() => {
  const handlePopulateInput = (e: CustomEvent) => {
    const { message, docId, metadata } = e.detail;
    
    // Set the input value
    setInputValue(message);
    
    // Store metadata for context
    setDocumentContext({
      docId,
      ...metadata
    });
    
    // Optionally auto-focus the input
    inputRef.current?.focus();
  };
  
  window.addEventListener('populate-chat-input', handlePopulateInput);
  return () => window.removeEventListener('populate-chat-input', handlePopulateInput);
}, []);
```

### 3. User Sends Message

User can:
- Edit the pre-filled message
- Add more context
- Send as-is

### 4. Goose Receives Context

When Goose receives the message, it has access to:
- The document ID
- Full document content
- Selected text (if any)
- Selection range

```python
# In Goose backend
def handle_message(message, metadata):
    if metadata.get('type') == 'document-assist':
        doc_id = metadata['docId']
        content = metadata['content']
        selected_text = metadata['selectedText']
        
        # Goose can now:
        # 1. Analyze the document
        # 2. Generate improvements
        # 3. Edit the document directly
        
        # Example: Improve selected text
        improved = improve_text(selected_text)
        edit_document(doc_id, 'replace', 
                     from_pos=metadata['selection']['from'],
                     to_pos=metadata['selection']['to'],
                     text=improved)
```

### 5. Goose Edits Document

Goose uses the document API to make edits:

```javascript
// Goose executes this
const editor = window.gooseEditors[docId];
editor.replaceText(from, to, improvedText);
```

User sees:
- Green "Goose is editing..." indicator
- Text changing in real-time
- Goose's response in chat

---

## Integration Points

### Event: `populate-chat-input`

**Dispatched by**: Document editor  
**Listened by**: Chat input component

**Payload**:
```typescript
{
  message: string,           // Pre-filled message
  docId: string,            // Document ID
  metadata: {
    type: 'document-assist',
    docId: string,
    content: string,        // Full HTML content
    selectedText: string,   // Selected text (if any)
    selection: {
      from: number,
      to: number
    }
  }
}
```

### Event: `goose-doc-assist`

**Dispatched by**: Document editor  
**Listened by**: Goose backend (optional)

**Payload**:
```typescript
{
  docId: string,
  content: string,
  selectedText: string,
  selection: { from: number, to: number },
  action: 'assist'
}
```

---

## Implementation Steps

### Step 1: Update Chat Input Component

```typescript
// In ChatInput.tsx or RichChatInput.tsx

const [documentContext, setDocumentContext] = useState<any>(null);

useEffect(() => {
  const handlePopulateInput = (e: CustomEvent) => {
    const { message, docId, metadata } = e.detail;
    
    // Set input value
    setInputValue(message);
    
    // Store context
    setDocumentContext({
      docId,
      ...metadata
    });
    
    // Focus input
    inputRef.current?.focus();
  };
  
  window.addEventListener('populate-chat-input', handlePopulateInput as EventListener);
  return () => window.removeEventListener('populate-chat-input', handlePopulateInput as EventListener);
}, []);

// When sending message, include document context
const handleSend = () => {
  sendMessage(inputValue, {
    documentContext: documentContext || undefined
  });
  
  // Clear context after sending
  setDocumentContext(null);
};
```

### Step 2: Update Message Handler

```typescript
// In message handling logic

const sendMessage = async (text: string, metadata?: any) => {
  const message = {
    role: 'user',
    content: text,
    metadata: metadata || {}
  };
  
  // Send to Goose
  const response = await gooseAPI.sendMessage(message);
  
  // If response includes document edits, they'll happen automatically
  // via the window.gooseEditors API
};
```

### Step 3: Goose Backend Integration

```python
# In Goose message handler

async def handle_message(message: str, metadata: dict):
    # Check if this is a document-related message
    if metadata.get('documentContext'):
        doc_context = metadata['documentContext']
        doc_id = doc_context['docId']
        
        # Process the request
        if doc_context.get('selectedText'):
            # User selected text and wants help
            result = await improve_text(doc_context['selectedText'])
            
            # Edit the document
            await edit_document(
                doc_id=doc_id,
                action='replace',
                from_pos=doc_context['selection']['from'],
                to_pos=doc_context['selection']['to'],
                text=result
            )
            
            return f"I've improved the selected text. Check the document!"
        else:
            # User wants general help with the document
            result = await analyze_document(doc_context['content'])
            return result
    
    # Regular message handling
    return await process_message(message)
```

---

## Example Conversations

### Example 1: Improve Selection

```
User: *selects "The quick brown fox"*
User: *clicks "Ask Goose"*

Chat Input: "I'm working on a document and need help with this section:

"The quick brown fox"

Full document context:
The quick brown fox jumps over the lazy dog..."

User: "Make this more exciting"
User: *sends*

Goose: "I'll make it more dynamic!"
Goose: *edits document*
  → "The lightning-fast brown fox"

Goose: "Done! I've made it more exciting by adding 'lightning-fast' to emphasize speed."
```

### Example 2: Add Content

```
User: *clicks "Ask Goose" (no selection)*

Chat Input: "I'm working on a document and need help. Here's what I have so far:

Introduction

The quick brown fox jumps over the lazy dog..."

User: "Add a conclusion paragraph"
User: *sends*

Goose: "I'll add a conclusion for you!"
Goose: *edits document*
  → Appends: "\n\nConclusion\n\nIn summary, the agility and speed of the fox demonstrate..."

Goose: "I've added a conclusion paragraph that summarizes the main points."
```

### Example 3: Format Document

```
User: "Can you format this document with proper headings?"

Goose: "I'll organize it with headings!"
Goose: *edits document*
  → Converts first line to H1
  → Adds H2 for sections
  → Creates bullet lists

Goose: "I've formatted your document with:
- H1 for the title
- H2 for main sections
- Bullet lists for key points"
```

### Example 4: Fix Grammar

```
User: *selects text with errors*
User: "Fix the grammar"

Goose: "I'll fix the grammar issues!"
Goose: *edits document*
  → Fixes typos
  → Corrects grammar
  → Improves punctuation

Goose: "Fixed! I corrected:
- 'benifits' → 'benefits'
- 'is' → 'are'
- Added missing commas"
```

---

## Visual Indicators

### In Document Editor

**Before Goose edits**:
```
┌────────────────────────────────────┐
│ Collaborative Document             │
│ ID: doc-123  🤖 Goose enabled     │
├────────────────────────────────────┤
│ [Toolbar]                          │
├────────────────────────────────────┤
│ The quick brown fox...             │
└────────────────────────────────────┘
```

**During Goose edits**:
```
┌────────────────────────────────────┐
│ Collaborative Document             │
│ ID: doc-123  🤖 Goose is editing...│ ⚡
├────────────────────────────────────┤
│ [Toolbar]                          │
├────────────────────────────────────┤
│ The lightning-fast brown fox...    │
└────────────────────────────────────┘
```

### In Chat Panel

**User message**:
```
┌────────────────────────────────────┐
│ 👤 You                             │
│ Make this more exciting            │
│                                    │
│ 📄 Working on: doc-123            │
└────────────────────────────────────┘
```

**Goose response**:
```
┌────────────────────────────────────┐
│ 🤖 Goose                           │
│ I'll make it more dynamic!         │
│                                    │
│ ✏️ Edited document: doc-123       │
└────────────────────────────────────┘
```

---

## Advanced Features

### 1. Multi-Turn Editing

```
User: "Make this more exciting"
Goose: *edits* "Done!"

User: "Actually, make it more professional"
Goose: *edits again* "Made it more professional!"

User: "Perfect, thanks!"
```

### 2. Iterative Refinement

```
User: "Write an introduction"
Goose: *writes intro*

User: "Make it shorter"
Goose: *shortens intro*

User: "Add a hook"
Goose: *adds hook to intro*
```

### 3. Explain Changes

```
User: "Improve this paragraph"
Goose: *edits*
Goose: "I made these changes:
1. Replaced 'very good' with 'excellent'
2. Added transition words
3. Split long sentence into two
4. Improved flow"
```

### 4. Undo Suggestions

```
User: "Make this more formal"
Goose: *edits*

User: "Actually, I liked it better before"
Goose: "No problem! Press Cmd+Z to undo, or I can revert it for you."
User: *presses Cmd+Z*
```

---

## Error Handling

### Document Not Found

```typescript
if (!window.gooseEditors[docId]) {
  return "I can't find that document. Make sure it's still open.";
}
```

### Goose Disabled

```typescript
if (!window.gooseEditors[docId].enabled) {
  return "Goose collaboration is disabled for this document. Click the toggle button to enable it.";
}
```

### Invalid Selection

```typescript
if (from > to || from < 0) {
  return "The selection range is invalid. Please select text again.";
}
```

---

## Testing

### Manual Test Flow

1. Open a document
2. Type some text
3. Select text
4. Click "Ask Goose"
5. Verify chat input is populated
6. Edit the message
7. Send
8. Watch Goose edit the document
9. Verify changes appear
10. Try Cmd+Z to undo

### Test Cases

```typescript
describe('Chat-Document Integration', () => {
  it('populates chat input when Ask Goose clicked', () => {
    // Click Ask Goose
    // Check chat input has document context
  });
  
  it('includes document metadata in message', () => {
    // Send message
    // Verify metadata includes docId, content, selection
  });
  
  it('Goose can edit document from chat', () => {
    // Simulate Goose response
    // Verify document is edited
  });
  
  it('shows typing indicator during edit', () => {
    // Start edit
    // Check for "Goose is editing..." badge
  });
  
  it('user can undo Goose edits', () => {
    // Goose edits
    // Press Cmd+Z
    // Verify edit is undone
  });
});
```

---

## Configuration

### Enable/Disable Integration

```typescript
// In CollaborativeDocEditor
const [chatIntegrationEnabled, setChatIntegrationEnabled] = useState(true);

// Only populate chat if enabled
if (chatIntegrationEnabled) {
  window.dispatchEvent(new CustomEvent('populate-chat-input', ...));
}
```

### Customize Context Message

```typescript
const contextMessageTemplate = (selectedText, fullText) => {
  if (selectedText) {
    return `Help me with: "${selectedText}"`;
  }
  return `Help me with my document: ${fullText.substring(0, 200)}...`;
};
```

---

## Summary

The chat-document integration creates a seamless experience where:

1. ✅ User can ask Goose about their document via chat
2. ✅ Chat input is pre-filled with document context
3. ✅ Goose receives full document information
4. ✅ Goose can edit the document directly
5. ✅ User sees real-time visual feedback
6. ✅ Everything is undoable
7. ✅ Natural conversation flow

**This transforms the document editor from a standalone tool into an integrated AI workspace!**

---

**Next Steps**:
1. Implement `populate-chat-input` listener in chat component
2. Add document context to message metadata
3. Update Goose backend to handle document edits
4. Test the full flow
5. Add visual indicators in chat

**Status**: ✅ Document editor ready, needs chat component integration

---

**Created**: November 5, 2025  
**Branch**: `spence/doceditor`
