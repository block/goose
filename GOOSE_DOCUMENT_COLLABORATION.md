# Goose Document Collaboration

## Overview

Goose can now collaborate with you on documents in real-time! The collaborative document editor exposes an API that allows Goose to make edits, suggestions, and improvements to your documents as you work.

---

## How It Works

### 1. **Global Editor Registry**

Each collaborative document registers itself in a global registry:

```typescript
window.gooseEditors = {
  'doc-123456': {
    editor: TiptapEditor,
    insertText: (text, position?) => void,
    replaceText: (from, to, text) => void,
    appendText: (text) => void,
    formatText: (from, to, format) => void,
    getContent: () => string,
    getText: () => string,
    getSelection: () => { from, to, text },
    clear: () => void,
  }
}
```

### 2. **Goose API Methods**

Goose can call these methods to edit documents:

#### Insert Text
```javascript
window.gooseEditors['doc-123456'].insertText('Hello, world!', 0);
```

#### Replace Text
```javascript
window.gooseEditors['doc-123456'].replaceText(0, 5, 'Hi');
```

#### Append Text
```javascript
window.gooseEditors['doc-123456'].appendText('\n\nAdded by Goose!');
```

#### Format Text
```javascript
// Make text bold
window.gooseEditors['doc-123456'].formatText(0, 5, 'bold');

// Convert to heading
window.gooseEditors['doc-123456'].formatText(0, 10, 'heading1');

// Create list
window.gooseEditors['doc-123456'].formatText(0, 50, 'bulletList');
```

#### Get Content
```javascript
const html = window.gooseEditors['doc-123456'].getContent();
const text = window.gooseEditors['doc-123456'].getText();
```

#### Get Selection
```javascript
const { from, to, text } = window.gooseEditors['doc-123456'].getSelection();
```

---

## Available Formats

Goose can apply these formats:
- `bold`
- `italic`
- `heading1`, `heading2`, `heading3`
- `bulletList`
- `orderedList`
- `code`
- `codeBlock`
- `blockquote`

---

## User Interface

### Goose Status Indicator

When Goose is enabled, you'll see:
- **Blue badge**: "Goose enabled" - Goose can edit this document
- **Green badge** (animated): "Goose is editing..." - Goose is currently making changes

### Ask Goose Button

Click the "Ask Goose" button to:
- Get suggestions for the selected text
- Ask Goose to improve the document
- Request formatting or restructuring

### Toggle Goose

Click the Bot/User icon to enable/disable Goose collaboration.

---

## Events

### Goose Document Assist Event

When you click "Ask Goose", this event is dispatched:

```typescript
window.dispatchEvent(new CustomEvent('goose-doc-assist', {
  detail: {
    docId: 'doc-123456',
    content: '<p>Full HTML content</p>',
    selectedText: 'Selected text',
    selection: { from: 0, to: 10 },
    action: 'assist',
  }
}));
```

Goose can listen for this event and respond by editing the document.

---

## Example: Goose Tool Implementation

Here's how to create a tool for Goose to edit documents:

```python
# In a Goose extension/tool

def edit_document(doc_id: str, action: str, **kwargs):
    """
    Edit a collaborative document.
    
    Args:
        doc_id: Document ID
        action: 'insert', 'replace', 'append', 'format', 'get'
        **kwargs: Action-specific parameters
    """
    
    if action == 'insert':
        text = kwargs.get('text', '')
        position = kwargs.get('position')
        return f"window.gooseEditors['{doc_id}'].insertText('{text}', {position})"
    
    elif action == 'replace':
        from_pos = kwargs.get('from', 0)
        to_pos = kwargs.get('to', 0)
        text = kwargs.get('text', '')
        return f"window.gooseEditors['{doc_id}'].replaceText({from_pos}, {to_pos}, '{text}')"
    
    elif action == 'append':
        text = kwargs.get('text', '')
        return f"window.gooseEditors['{doc_id}'].appendText('{text}')"
    
    elif action == 'format':
        from_pos = kwargs.get('from', 0)
        to_pos = kwargs.get('to', 0)
        format_type = kwargs.get('format', 'bold')
        return f"window.gooseEditors['{doc_id}'].formatText({from_pos}, {to_pos}, '{format_type}')"
    
    elif action == 'get':
        return f"window.gooseEditors['{doc_id}'].getContent()"
```

---

## Use Cases

### 1. **Grammar and Style Improvements**

User: "Goose, can you improve the grammar in this paragraph?"

Goose:
1. Gets the selected text
2. Processes it with an LLM
3. Replaces the text with improved version

```javascript
const { from, to, text } = window.gooseEditors['doc-123456'].getSelection();
const improved = await improveGrammar(text);
window.gooseEditors['doc-123456'].replaceText(from, to, improved);
```

### 2. **Content Generation**

User: "Goose, add a conclusion paragraph"

Goose:
1. Gets the current content
2. Generates a conclusion
3. Appends it to the document

```javascript
const content = window.gooseEditors['doc-123456'].getText();
const conclusion = await generateConclusion(content);
window.gooseEditors['doc-123456'].appendText('\n\n' + conclusion);
```

### 3. **Formatting Assistance**

User: "Goose, make this a bulleted list"

Goose:
1. Gets the selection
2. Applies bullet list format

```javascript
const { from, to } = window.gooseEditors['doc-123456'].getSelection();
window.gooseEditors['doc-123456'].formatText(from, to, 'bulletList');
```

### 4. **Document Restructuring**

User: "Goose, add headings to organize this document"

Goose:
1. Analyzes the content
2. Identifies sections
3. Applies heading formats

```javascript
const sections = identifySections(content);
sections.forEach(section => {
  window.gooseEditors['doc-123456'].formatText(
    section.from, 
    section.to, 
    'heading2'
  );
});
```

### 5. **Real-time Suggestions**

As you type, Goose can:
- Suggest completions
- Fix typos
- Improve phrasing
- Add relevant information

---

## Implementation in Goose Backend

### Option 1: JavaScript Execution

If Goose can execute JavaScript in the Electron context:

```python
def goose_edit_document(doc_id, action, **params):
    js_code = generate_edit_code(doc_id, action, params)
    execute_in_electron(js_code)
```

### Option 2: IPC Bridge

Create an IPC bridge between Goose and the editor:

```typescript
// In main.ts
ipcMain.handle('goose-edit-document', async (event, docId, action, params) => {
  // Forward to renderer
  mainWindow.webContents.send('goose-edit-document', docId, action, params);
});

// In renderer
window.electron.onGooseEdit((docId, action, params) => {
  const editor = window.gooseEditors[docId];
  if (editor) {
    switch (action) {
      case 'insert':
        editor.insertText(params.text, params.position);
        break;
      // ... other actions
    }
  }
});
```

### Option 3: Tool Calling

Create a tool that Goose can call:

```yaml
# tool definition
name: edit_document
description: Edit a collaborative document
parameters:
  doc_id:
    type: string
    description: Document ID
  action:
    type: string
    enum: [insert, replace, append, format, get]
  text:
    type: string
    description: Text to insert/replace
  from:
    type: integer
    description: Start position
  to:
    type: integer
    description: End position
  format:
    type: string
    description: Format to apply
```

---

## Safety and Permissions

### User Consent

Before Goose can edit documents:
1. User must enable Goose collaboration (toggle button)
2. User can see when Goose is editing (status indicator)
3. User can undo Goose's changes (Cmd+Z)

### Rate Limiting

To prevent overwhelming the user:
- Goose edits are rate-limited
- Visual indicator shows when Goose is editing
- User can disable Goose at any time

### Undo/Redo

All Goose edits go through Tiptap's history:
- User can undo Goose's changes
- Redo works normally
- History is preserved

---

## Future Enhancements

### 1. **Cursor Tracking**

Show where Goose is editing with a colored cursor:

```typescript
// Add Goose cursor indicator
editor.commands.setGooseCursor(position);
```

### 2. **Change Highlighting**

Highlight Goose's changes temporarily:

```typescript
// Highlight Goose's edits
editor.commands.highlightChange(from, to, 'goose-edit');
```

### 3. **Suggestion Mode**

Instead of direct edits, Goose can suggest changes:

```typescript
// Show suggestion
editor.commands.addSuggestion({
  from, to,
  oldText, newText,
  author: 'goose'
});
```

### 4. **Voice Collaboration**

User speaks commands, Goose edits:

```
User: "Goose, make this bold"
Goose: *applies bold formatting*
```

### 5. **Multi-Agent Collaboration**

Multiple AI agents can collaborate:
- Goose for general editing
- Specialized agents for code, math, etc.

---

## Testing

### Manual Testing

1. Open a collaborative document
2. Open browser console
3. Try these commands:

```javascript
// List available editors
console.log(Object.keys(window.gooseEditors));

// Get a specific editor
const editor = window.gooseEditors['doc-1730818800000'];

// Insert text
editor.insertText('Hello from Goose!');

// Get content
console.log(editor.getText());

// Format text
editor.formatText(0, 5, 'bold');
```

### Automated Testing

```typescript
describe('Goose Document Collaboration', () => {
  it('registers editor in global registry', () => {
    const docId = 'test-doc';
    // ... render editor
    expect(window.gooseEditors[docId]).toBeDefined();
  });

  it('allows Goose to insert text', () => {
    const editor = window.gooseEditors['test-doc'];
    editor.insertText('Test');
    expect(editor.getText()).toContain('Test');
  });

  it('shows typing indicator when Goose edits', () => {
    const editor = window.gooseEditors['test-doc'];
    editor.insertText('Test');
    // Check for typing indicator
    expect(screen.getByText(/Goose is editing/i)).toBeInTheDocument();
  });
});
```

---

## API Reference

### `window.gooseEditors[docId]`

#### Methods

##### `insertText(text: string, position?: number): void`
Insert text at position (or at cursor if no position).

##### `replaceText(from: number, to: number, text: string): void`
Replace text in range with new text.

##### `appendText(text: string): void`
Append text to the end of the document.

##### `formatText(from: number, to: number, format: string): void`
Apply formatting to text range.

##### `getContent(): string`
Get full HTML content.

##### `getText(): string`
Get plain text content.

##### `getSelection(): { from: number, to: number, text: string }`
Get current selection.

##### `clear(): void`
Clear all content.

---

## Conclusion

The collaborative document editor provides a powerful API for Goose to assist with writing, editing, and formatting documents. By exposing the editor through a global registry, Goose can seamlessly collaborate with users in real-time.

**Key Benefits:**
- ✅ Real-time collaboration
- ✅ Non-intrusive (user can disable)
- ✅ Fully undoable
- ✅ Visual feedback
- ✅ Flexible API

**Next Steps:**
1. Implement Goose tool for document editing
2. Add IPC bridge for backend communication
3. Test with real use cases
4. Add suggestion mode
5. Implement cursor tracking

---

**Created**: November 5, 2025  
**Branch**: `spence/doceditor`  
**Status**: ✅ Ready for integration
