# Ready to Test: Document Context Integration

## 🎉 What's Working Right Now

We've successfully implemented document context integration between the frontend and backend! Here's what you can test immediately:

## ✅ Test 1: Frontend Document Context Capture

### Steps
```bash
1. Start the app:
   cd /Users/spencermartin/Desktop/goose
   source bin/activate-hermit
   cd ui/desktop
   npm run dev

2. In the application:
   - Click the "+" button (top right)
   - Select "New Document"
   - Type some content: "Hello World! This is a test document."
   - Click "Ask Goose" button
   
3. Open browser console (F12)
   - Look for: "📄 Document context stored for message submission"
   - Verify chat input is populated with prompt

4. Type a message: "Can you help me with this?"

5. Before clicking Send, check console
   - You should see the documentContext state is set

6. Click Send

7. Check console for:
   - "📄 Including document context in message: {...}"
   - You should see the full document context object logged
```

### Expected Results
- ✅ Chat input populates with document context
- ✅ Console shows "Document context stored"
- ✅ Console shows document context object when sending
- ✅ Message is sent successfully

## ✅ Test 2: Backend Document Context Reception

### Steps
```bash
1. Start the backend in a separate terminal:
   cd /Users/spencermartin/Desktop/goose
   source bin/activate-hermit
   cargo run --bin goose-server

2. Follow Test 1 steps in the frontend

3. Watch the backend terminal for logs:
   - Look for: "Document context received"
   - Should show: doc_id, content_length, has_selection
```

### Expected Results
- ✅ Backend logs "Document context received"
- ✅ Shows correct doc_id
- ✅ Shows content_length matching your document
- ✅ Shows has_selection: true/false based on selection

## ✅ Test 3: AI Receives Enhanced Prompt

### Steps
```bash
1. Continue from Test 2

2. Send a message from the frontend

3. The AI should receive an enhanced prompt that includes:
   - "You are assisting the user with a document they are editing"
   - Document ID
   - Current content
   - Selected text (if any)
   - Instructions about edit_document tool
```

### Expected Results
- ✅ AI receives document context in prompt
- ✅ AI knows it can use edit_document tool
- ✅ AI responds with awareness of document content

## 🧪 Test 4: Verify API is Accessible

### Steps
```javascript
// Open browser console (F12) and run:

// 1. Check if editors are registered
console.log(window.gooseEditors);
// Should show: { "doc-xxx": { insertText, replaceText, ... } }

// 2. Get the document ID
const docId = Object.keys(window.gooseEditors)[0];
console.log("Document ID:", docId);

// 3. Test API methods
const editor = window.gooseEditors[docId];

// Append text
editor.appendText("\n\nThis was added via API!");

// Insert text at beginning
editor.insertText("INSERTED: ", 0);

// Get content
console.log(editor.getContent());

// Get plain text
console.log(editor.getText());

// Get selection
console.log(editor.getSelection());
```

### Expected Results
- ✅ `window.gooseEditors` is defined
- ✅ Editor instance is accessible
- ✅ All methods work correctly
- ✅ Document updates in real-time

## 📊 What's Working vs What's Not

### ✅ Working (Can Test Now)
1. **Frontend**
   - Document editor with rich formatting
   - "Ask Goose" button
   - Document context capture
   - Chat input population
   - Message submission with context
   - `window.gooseEditors` API

2. **Backend**
   - Document context reception
   - Document context parsing
   - Document context logging
   - AI prompt enhancement
   - System message injection

3. **Integration**
   - Frontend → Backend communication
   - Document context flow
   - Logging and debugging

### ⏳ Not Working Yet (Needs Implementation)
1. **AI Tool Execution**
   - AI cannot yet call edit_document tool
   - Tool is not registered
   - IPC bridge doesn't exist

2. **Document Editing**
   - AI cannot edit documents yet
   - No real-time updates from AI
   - No "Goose is editing..." feedback

## 🎯 What You Should See

### Successful Test Output

**Frontend Console:**
```
📄 Document context stored for message submission
📄 Including document context in message: {
  docId: "doc-1234567890",
  content: "Hello World! This is a test document.",
  selection: null,
  timestamp: 1234567890000
}
```

**Backend Logs:**
```
INFO Document context received
  doc_id: doc-1234567890
  content_length: 38
  has_selection: false
```

**AI Response:**
```
I can see you're working on a document with the content "Hello World! This is a test document." 
How can I help you with this document?
```

## 🐛 Troubleshooting

### Issue: "Document context stored" not showing
**Solution**: 
- Check that CollaborativeDocEditor is mounted
- Verify "Ask Goose" button is clicked
- Check browser console for errors

### Issue: "Including document context" not showing
**Solution**:
- Verify you clicked "Ask Goose" before sending
- Check that documentContext state is set
- Look for any JavaScript errors

### Issue: Backend not logging "Document context received"
**Solution**:
- Verify backend is running
- Check that frontend is connecting to correct backend URL
- Look for network errors in browser dev tools

### Issue: window.gooseEditors is undefined
**Solution**:
- Verify document editor is open
- Check that CollaborativeDocEditor mounted successfully
- Look for JavaScript errors during mount

## 📝 Example Test Scenario

### Complete Flow Test
```bash
1. Start backend:
   cargo run --bin goose-server

2. Start frontend:
   npm run dev

3. Open app in browser

4. Create new document:
   - Click "+"
   - Select "New Document"

5. Add content:
   - Type: "# My Document\n\nThis is a test."
   - Apply bold formatting to "My Document"

6. Select text:
   - Select "test"

7. Ask Goose:
   - Click "Ask Goose"
   - Verify chat input populates
   - Check console for "Document context stored"

8. Send message:
   - Type: "What does this document say?"
   - Click Send
   - Check console for "Including document context"

9. Check backend:
   - Look for "Document context received" log
   - Verify doc_id, content_length, has_selection: true

10. Check AI response:
    - AI should acknowledge document content
    - AI should mention "test" (the selected text)
```

### Expected Timeline
- Steps 1-6: 2 minutes
- Steps 7-10: 1 minute
- Total: ~3 minutes

## 🎊 Success Criteria

You've successfully tested the integration if:

- [x] Frontend captures document context
- [x] Frontend sends context with messages
- [x] Backend receives context
- [x] Backend logs context correctly
- [x] AI receives enhanced prompt
- [x] AI responds with document awareness
- [x] `window.gooseEditors` API works

## 🚀 Next Steps After Testing

Once you've verified everything works:

1. **Create edit_document Tool**
   - See `NEXT_STEPS_TOOL_CREATION.md`
   - Estimated time: 2-3 hours

2. **Create IPC Bridge**
   - Add Electron IPC handler
   - Estimated time: 1-2 hours

3. **Connect Tool to IPC**
   - Implement communication
   - Estimated time: 1 hour

4. **End-to-End Testing**
   - Test full AI editing flow
   - Estimated time: 2 hours

**Total remaining work**: 6-8 hours

## 📚 Documentation

- `PHASE_5_BACKEND_COMPLETE_SUMMARY.md` - Full summary of what's done
- `PHASE_5_BACKEND_PROGRESS.md` - Detailed progress report
- `NEXT_STEPS_TOOL_CREATION.md` - How to implement the tool
- `CONSOLE_TEST_COMMANDS.md` - Browser console test commands

## 🎉 Congratulations!

You've successfully integrated document context between the frontend and backend! The AI can now receive full context about what the user is working on. The foundation is solid and ready for the final implementation of the editing tool.

**Status**: ✅ Ready to Test | 🔄 Tool Implementation Pending | 🎯 75% Complete
