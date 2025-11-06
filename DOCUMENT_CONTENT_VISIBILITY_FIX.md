# Document Content Visibility Fix

## Issue
The AI was not recognizing that the document content was already provided in the system prompt. When users asked to "make the document better" or similar requests, the AI would:
1. Try to use `screen_capture` to view the document
2. Ask the user to provide the content
3. Not realize the content was already in the context

## Root Cause
The system prompt was injecting the document content, but it wasn't visually prominent enough. The AI was scanning through the context and missing the embedded content, treating it as just another part of the instructions rather than the actual document to work with.

## Solution
Enhanced the visual formatting of the document context injection with:

### 1. **Clear Visual Boundaries**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📄 DOCUMENT EDITOR CONTEXT - READ THIS FIRST
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 2. **Explicit Content Markers**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📝 CURRENT DOCUMENT CONTENT (BELOW THIS LINE):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[Document content here]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📝 END OF DOCUMENT CONTENT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### 3. **Step-by-Step Instructions**
```
⚠️ CRITICAL INSTRUCTIONS:

1. THE DOCUMENT CONTENT IS ALREADY VISIBLE ABOVE between the lines:
   "CURRENT DOCUMENT CONTENT (BELOW THIS LINE)" and "END OF DOCUMENT CONTENT"

2. DO NOT use screen_capture, list_windows, or any viewing tools
   → You already have the full document content!

3. When user asks to edit/improve/modify the document:
   → Read the content shown above
   → Describe what you see
   → Use edit_document tool to make changes
```

### 4. **Concrete Examples**
Provided both correct and incorrect workflow examples:

**✅ CORRECT:**
```
User: "Make this better"

You: "I can see your document currently contains: [quote the content shown above].
Let me expand this into a more comprehensive version..."

[Then call: edit_document with action=replaceText or appendText]
```

**❌ WRONG:**
```
User: "Make this better"

You: "Let me capture the screen to see the document..."
[Calls: screen_capture]

❌ THIS IS WRONG! The content is already provided above!
```

## Implementation Details

**File Modified:** `crates/goose-server/src/routes/reply.rs`

**Location:** In the `reply_handler` function, within the document context injection block (around line 230-280)

**Key Changes:**
1. Added visual separators using Unicode box-drawing characters
2. Made content boundaries extremely explicit with "BELOW THIS LINE" and "END OF DOCUMENT CONTENT" markers
3. Numbered the critical instructions for clarity
4. Provided concrete workflow examples showing correct vs. incorrect approaches
5. Used emojis and formatting to draw attention to key sections

## Expected Behavior After Fix

When a user has a document open and asks the AI to edit it:

1. ✅ AI immediately recognizes the document content in the context
2. ✅ AI quotes or references the current content
3. ✅ AI uses `edit_document` tool to make changes
4. ❌ AI does NOT try to use `screen_capture`
5. ❌ AI does NOT ask the user to provide the content

## Testing

To verify the fix works:

1. Open a new document in Goose Desktop
2. Add some content (e.g., "This is a test document")
3. Click "Ask Goose" or type in chat: "Make this document better"
4. **Expected:** AI should acknowledge the current content and use `edit_document` tool
5. **Not Expected:** AI should NOT try to capture screen or ask for content

## Related Fixes

This is the fourth in a series of document editing improvements:

1. **Context Injection** - Document content sent to AI
2. **Screen Capture Prevention** - Explicit warnings against screen capture
3. **Automatic Doc ID** - AI uses correct doc_id automatically
4. **Content Visibility** ← This fix - AI recognizes content is already provided

## Status

✅ **IMPLEMENTED** - Code changes complete
✅ **COMPILED** - Backend rebuilt successfully
⏳ **TESTING** - Ready for user verification

## Next Steps

1. Restart the Goose Desktop application (both backend and frontend)
2. Test the document editing workflow
3. Verify AI behavior matches expected outcomes
4. Monitor for any remaining edge cases

---

**Date:** 2025-11-05  
**Branch:** `spence/doceditor`  
**Commit Status:** Not committed (working locally)
