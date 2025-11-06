# Quick Start Guide - Document Editing Feature

## 🚀 Start Testing in 3 Steps

### 1. Start the Application
```bash
cd /Users/spencermartin/Desktop/goose
source bin/activate-hermit
npm run dev
```

### 2. Open a Document
- Click the **"+"** button in the sidecar (left side)
- Select **"New Document"**
- Document editor opens in the main area

### 3. Ask Goose to Edit
Type in chat:
```
Add "Hello World" at the top of the document
```

✅ **Expected Result**:
- Text appears in document
- Green success badge in chat
- Message: "Document Updated"

---

## 💡 Try These Commands

### Basic Edits
```
"Add 'My First Document' as a title"
"Append 'The End' at the bottom"
"Clear the document"
```

### Formatting
```
"Make the first 5 characters bold"
"Highlight important text in yellow"
"Make all headings larger"
```

### Content Generation
```
"Write a short blog post about AI"
"Create a to-do list for today"
"Draft an email about the new feature"
```

### Complex Operations
```
"Replace 'Hello' with 'Goodbye' everywhere"
"Add bullet points to this list"
"Format this as a professional document"
```

---

## 🐛 Troubleshooting

### Nothing Happens?
1. **Check Console** (F12 or Cmd+Option+I)
   - Look for: `✅ Document edit executed` or `❌ Document edit failed`

2. **Verify Editor Exists**
   ```javascript
   // In browser console:
   console.log(window.gooseEditors);
   ```
   Should show: `{ 'doc-...': { insertText: fn, ... } }`

3. **Check Backend**
   - Terminal should show: `Received document context: ...`

### Error Badge Appears?
- **"Document editor not found"**: Document was closed, open a new one
- **"requires text parameter"**: AI didn't provide required parameters
- **Parse error**: Backend response format issue (check backend logs)

---

## 📊 What to Look For

### Success Indicators
- ✅ Green badge in tool output
- ✅ "Document Updated" message
- ✅ Text appears/changes in document
- ✅ Console log: `✅ Document edit executed`

### Error Indicators
- ❌ Red badge in tool output
- ❌ "Edit Failed" message
- ❌ Console log: `❌ Document edit failed`
- ❌ Error details shown

---

## 🎯 Key Features to Test

### 1. Real-time Editing
- Ask Goose to add text
- Watch it appear instantly
- No page refresh needed

### 2. Visual Feedback
- Success: Green badge with checkmark
- Error: Red badge with X
- Details: Hover/expand for more info

### 3. Context Awareness
- Goose knows document content
- Can reference existing text
- Makes intelligent edits

### 4. Multiple Actions
- Insert, replace, append
- Format (bold, italic, color)
- Clear document

---

## 📝 Example Session

```
User: "Create a new document"
[Clicks + button, selects New Document]

User: "Add a title 'My Notes'"
Goose: ✓ Document Updated - Inserted text at position 0
[Document shows: "# My Notes"]

User: "Add a list of 3 things to do today"
Goose: ✓ Document Updated - Appended text to document
[Document shows list below title]

User: "Make the title bold"
Goose: ✓ Document Updated - Applied formatting from 0 to 9
[Title becomes bold]

User: "Add 'Done!' at the end"
Goose: ✓ Document Updated - Appended text to document
[Text appears at bottom]
```

---

## 🔍 Debug Commands

### Check Editor Registry
```javascript
// Browser console
console.log(window.gooseEditors);
// Should show object with document IDs as keys
```

### Test Direct Edit
```javascript
// Browser console
const editor = window.gooseEditors['doc-123']; // Use actual ID
editor.insertText('Test', 0);
// Should insert text immediately
```

### Check Document Context
```javascript
// Browser console
const editor = window.gooseEditors['doc-123'];
console.log(editor.getContent());
console.log(editor.getText());
```

---

## 📚 Documentation

For more details, see:
- `COMPLETE_IMPLEMENTATION_SUMMARY.md` - Full technical overview
- `FRONTEND_DOCUMENT_EDIT_COMPLETE.md` - Frontend implementation
- `EDIT_DOCUMENT_TOOL_HANDLER_COMPLETE.md` - Backend implementation

---

## 🎉 Success!

If you see:
- ✅ Document opens
- ✅ Goose responds to edit requests
- ✅ Text appears/changes in real-time
- ✅ Success badges show up

**Congratulations! The feature is working!** 🎊

---

## 🆘 Need Help?

1. Check console for errors
2. Verify backend is running
3. Confirm document is open
4. Try simple commands first
5. Review error messages

---

**Happy Testing!** 🚀
