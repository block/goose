# Document Context Flow Fix - CRITICAL

## The Real Problem

The AI was not seeing document content because **the document context was never actually being sent to the backend**!

### The Broken Flow

1. ✅ `ChatInput` creates `messageData` with `documentContext`
2. ✅ `ChatInput` dispatches event with `documentContext`
3. ❌ `BaseChat.handleSubmit` extracts only `value`, **ignores `documentContext`**
4. ❌ `useChatEngine.handleSubmit` receives only string, **no context**
5. ❌ `useMessageStream.sendRequest` sends to backend **without `document_context`**
6. ❌ Backend never receives document context
7. ❌ AI never sees document content

### Why Previous Fixes Didn't Work

- **Backend formatting fix**: Made the prompt prettier, but there was no content to format!
- **Automatic context tracking**: Frontend tracked context perfectly, but never sent it!
- **Both fixes were necessary but not sufficient** - we needed the complete flow

## The Solution

### 1. Extract Document Context in BaseChat (`BaseChat.tsx`)

```typescript
// Handle submit
const handleSubmit = (e: React.FormEvent) => {
  const customEvent = e as unknown as CustomEvent;
  const combinedTextFromInput = customEvent.detail?.value || '';
  const documentContext = customEvent.detail?.documentContext;  // ← EXTRACT THIS

  // ... existing code ...

  // Pass both text and document context to engine
  engineHandleSubmit(combinedTextFromInput, undefined, documentContext);  // ← PASS IT
};
```

### 2. Accept and Forward Context in useChatEngine (`useChatEngine.ts`)

```typescript
const handleSubmit = useCallback(
  (combinedTextFromInput: string, onSummaryReset?: () => void, documentContext?: any) => {  // ← ACCEPT IT
    if (combinedTextFromInput.trim()) {
      // ... existing code ...

      const userMessage = createUserMessage(combinedTextFromInput.trim());

      // If document context is provided, update the message stream body
      if (documentContext) {
        console.log('📄 useChatEngine: Updating message stream body with document context:', documentContext);
        updateMessageStreamBody({ document_context: documentContext });  // ← FORWARD IT
      }

      // ... existing code ...
    }
  },
  [append, onMessageSent, stopPowerSaveBlocker, updateMessageStreamBody]
);
```

### 3. Send Context to Backend (Already Working in `useMessageStream.ts`)

The `updateMessageStreamBody` function updates `extraMetadataRef.current.body`, which is then sent in the fetch request:

```typescript
body: JSON.stringify({
  messages: expandedMessages,
  ...extraMetadataRef.current.body,  // ← Includes document_context now!
}),
```

## The Complete Flow (Fixed)

1. ✅ `ChatInput` creates `messageData` with `documentContext`
2. ✅ `ChatInput` dispatches event with `documentContext`
3. ✅ **`BaseChat.handleSubmit` extracts `documentContext`**
4. ✅ **`useChatEngine.handleSubmit` receives `documentContext`**
5. ✅ **`useChatEngine` calls `updateMessageStreamBody({ document_context })`**
6. ✅ **`useMessageStream.sendRequest` includes `document_context` in body**
7. ✅ **Backend receives `document_context` in `ChatRequest`**
8. ✅ **Backend injects document content into system prompt**
9. ✅ **AI sees and uses document content**

## Files Changed

### Frontend
1. **`ui/desktop/src/components/BaseChat.tsx`**
   - Extract `documentContext` from event detail
   - Pass it to `engineHandleSubmit`

2. **`ui/desktop/src/hooks/useChatEngine.ts`**
   - Add `documentContext` parameter to `handleSubmit`
   - Call `updateMessageStreamBody` with document context
   - Add `updateMessageStreamBody` to dependencies

### Backend (Already Fixed)
3. **`crates/goose-server/src/routes/reply.rs`**
   - Already has `DocumentContext` struct
   - Already deserializes `document_context` from request
   - Already injects into system prompt with enhanced formatting

## Testing

To verify the fix works:

1. **Restart the Goose Desktop app** (frontend changes require restart)
2. Open a new document
3. Type some content in the document
4. Type a message directly in chat (e.g., "make this better")
5. **Check console logs** for:
   ```
   📄 useChatEngine: Updating message stream body with document context: {...}
   ```
6. **Expected**: AI should immediately see and reference the document content
7. **Not Expected**: AI should NOT ask for content or try to screen capture

## Why This Was Hard to Find

The issue was a **silent failure** in the data flow:
- No errors were thrown
- Frontend logging showed context was being tracked
- Backend logging showed requests were being received
- But the context was being **dropped** between frontend components

This required tracing the entire data flow from `ChatInput` → `BaseChat` → `useChatEngine` → `useMessageStream` → Backend to find where the context was lost.

## Status

✅ **IMPLEMENTED** - All code changes complete  
✅ **TESTED** - Flow verified through console logging  
⏳ **USER TESTING** - Ready for end-to-end verification  

---

**Date:** 2025-11-05  
**Branch:** `spence/doceditor`  
**Commit Status:** Not committed (working locally)
