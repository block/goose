# Quick Test Guide - Collaborative Document Editor

## 🚀 Testing the "Ask Goose" Button

### Step 1: Open a Document

1. Start the app: `npm run start-gui`
2. Hover over the **right edge** of the window
3. Click the **plus button** (+)
4. Select **"New Document"**

You should see:
- Document editor opens
- Blue badge: "🤖 Goose enabled"
- "Ask Goose" button in the header

### Step 2: Click "Ask Goose"

1. Type some text in the document (e.g., "Hello world")
2. Click the **"Ask Goose"** button
3. You should see an alert with the document ID

### Step 3: Check the Console

Open browser DevTools (Cmd+Option+I or F12) and check the console. You should see:

```
🔍 Ask Goose button clicked!
📄 Document info: { docId: "doc-...", contentLength: ..., ... }
💬 Context message: I'm working on a document...
📤 Dispatching populate-chat-input event
📤 Dispatching goose-doc-assist event
✅ Events dispatched successfully
```

### Step 4: Test the API

In the browser console, try these commands:

```javascript
// 1. List available editors
console.log(Object.keys(window.gooseEditors));

// 2. Get your editor (use the ID from the alert)
const editor = window.gooseEditors['doc-1730818800000']; // Replace with your ID

// 3. Test inserting text
editor.insertText('Hello from the API! ');

// 4. Test appending text
editor.appendText('\n\nThis was added by the API!');

// 5. Test formatting
editor.formatText(0, 5, 'bold'); // Makes first 5 characters bold

// 6. Get content
console.log(editor.getText());

// 7. Get HTML
console.log(editor.getContent());
```

---

## 🎯 What Should Happen

### When You Click "Ask Goose"

✅ **Alert appears** with document ID  
✅ **Console shows** debug logs  
✅ **Events are dispatched** (populate-chat-input, goose-doc-assist)

### When You Test the API

✅ **Text appears** in the document  
✅ **Green "Goose is editing..." badge** shows briefly  
✅ **Changes are visible** immediately  
✅ **You can undo** with Cmd+Z

---

## 🐛 Troubleshooting

### Button Does Nothing

**Check**:
1. Is the document open? (You should see the editor)
2. Is Goose enabled? (Blue badge should show)
3. Open console - any errors?

**Fix**:
- Make sure you're using `CollaborativeDocEditor`, not `DocEditor`
- Check that `enableGooseCollaboration={true}` is set
- Verify the button has `onClick={handleAskGoose}`

### No API Available

**Check**:
```javascript
console.log(window.gooseEditors);
```

**Should show**:
```javascript
{
  'doc-1730818800000': { insertText: [Function], ... }
}
```

**If undefined**:
- Document might not be using `CollaborativeDocEditor`
- Check console for registration errors

### API Methods Don't Work

**Check**:
```javascript
const editor = window.gooseEditors['doc-123'];
console.log(typeof editor.insertText); // Should be 'function'
```

**Try**:
```javascript
// Test each method
editor.insertText('test');
editor.appendText('test');
editor.getContent();
```

---

## 🎬 Full Demo Script

```javascript
// 1. Get the editor
const docId = Object.keys(window.gooseEditors)[0];
const editor = window.gooseEditors[docId];

// 2. Clear and start fresh
editor.clear();

// 3. Add a title
editor.insertText('My Test Document\n\n');
editor.formatText(0, 17, 'heading1');

// 4. Add some content
editor.appendText('This is a test paragraph. ');
editor.appendText('It demonstrates the collaborative editing API.\n\n');

// 5. Add a list
editor.appendText('Key features:\n');
editor.appendText('- Real-time editing\n');
editor.appendText('- Visual feedback\n');
editor.appendText('- Full undo support\n');

// 6. Format the list
const text = editor.getText();
const listStart = text.indexOf('Key features:');
const listEnd = text.length;
editor.formatText(listStart, listEnd, 'bulletList');

// 7. Get the result
console.log('Final document:');
console.log(editor.getText());
```

---

## 🧪 Advanced Testing

### Test 1: Typing Animation

Simulate Goose typing word by word:

```javascript
const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
const words = "The quick brown fox jumps over the lazy dog".split(' ');

words.forEach((word, i) => {
  setTimeout(() => {
    editor.appendText(word + ' ');
  }, i * 300); // 300ms between words
});
```

### Test 2: Format Existing Text

```javascript
const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];

// Make first line a heading
editor.formatText(0, 20, 'heading1');

// Make next paragraph bold
editor.formatText(21, 50, 'bold');

// Make last paragraph a quote
const text = editor.getText();
editor.formatText(text.length - 100, text.length, 'blockquote');
```

### Test 3: Replace Text

```javascript
const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];

// Replace first word
editor.replaceText(0, 5, 'Hello');

// Replace a phrase
const text = editor.getText();
const start = text.indexOf('quick');
const end = start + 5;
editor.replaceText(start, end, 'lightning-fast');
```

---

## 📊 Expected Results

### Console Output

```
🔍 Ask Goose button clicked!
📄 Document info: {
  docId: "doc-1730818800000",
  contentLength: 45,
  plainTextLength: 42,
  selectedText: "",
  selection: { from: 0, to: 0 }
}
💬 Context message: I'm working on a document and need help. Here's what I have so far:

Hello world...
📤 Dispatching populate-chat-input event
📤 Dispatching goose-doc-assist event
✅ Events dispatched successfully
```

### Visual Feedback

When testing API methods:
1. Green "🤖 Goose is editing..." badge appears
2. Text changes in real-time
3. Badge fades after 1 second
4. Changes are in undo history (Cmd+Z works)

---

## ✅ Success Checklist

- [ ] Document opens successfully
- [ ] "Ask Goose" button is visible
- [ ] Clicking button shows alert
- [ ] Console shows debug logs
- [ ] `window.gooseEditors` is populated
- [ ] API methods work (insertText, etc.)
- [ ] Visual indicators appear
- [ ] Can undo API changes
- [ ] Multiple documents work independently

---

## 🎯 Next Steps

Once basic testing works:

1. **Test with selection**:
   - Select some text
   - Click "Ask Goose"
   - Check that selectedText is in the event

2. **Test multiple documents**:
   - Open 2-3 documents
   - Each should have its own entry in `window.gooseEditors`
   - Test API on each independently

3. **Test formatting**:
   - Try all format types: bold, italic, heading1, heading2, heading3, bulletList, orderedList, code, codeBlock, blockquote

4. **Test edge cases**:
   - Empty document
   - Very long document
   - Special characters
   - Multiple rapid edits

---

## 🚨 Known Issues

1. **Chat integration not complete**: The "Ask Goose" button dispatches events, but there's no listener yet in the chat component. This is expected - chat integration is the next step.

2. **Alert is temporary**: The alert is just for testing. Once chat integration is complete, it will be removed.

3. **No persistence**: Documents aren't saved yet. Refresh will lose content. This is a TODO.

---

## 📞 Getting Help

If something doesn't work:

1. **Check console** for errors
2. **Verify document type**: Should be `CollaborativeDocEditor`
3. **Check props**: `enableGooseCollaboration={true}`
4. **Test simple case**: Just `editor.insertText('test')`

---

**Status**: ✅ Ready for testing  
**Branch**: `spence/doceditor`  
**Last Updated**: November 5, 2025
