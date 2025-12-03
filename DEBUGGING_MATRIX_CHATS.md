# Debugging Matrix Space Room Chats - Instructions

## Issue Summary

You're experiencing two main issues with Matrix Space room chats:

1. **History not populating** - When opening a Space room, historical messages don't show
2. **New messages not appearing** - When sending messages, they don't display in the chat thread

## Diagnostic Logging Added

I've added enhanced diagnostic logging to help identify the root cause:

### In `BaseChat2.tsx`:
- Logs the render state including `matrixRoomId`, `isMatrixTab`, `messagesCount`, etc.
- Look for: `🔍 BaseChat2 render state:`

### In `useChatStream.ts`:
- Logs when the Matrix message listener is being set up (or not)
- Look for: `🔍 useChatStream Matrix listener setup check:`
- Look for: `✅ useChatStream: Setting up Matrix message listener` (if successful)
- Look for: `🚫 useChatStream: Not a Matrix tab` (if failing)

## How to Debug

### Step 1: Open a Space Room Chat

1. Go to Channels view
2. Click on a Space
3. Click on a room within that Space
4. The chat should open in a new tab

### Step 2: Check Browser Console

Open the browser console (F12 or Cmd+Option+I) and look for the diagnostic logs.

#### What to Check:

**A. Is `matrixRoomId` defined in BaseChat2?**
```
🔍 BaseChat2 render state: {
  sessionId: "abc123...",
  matrixRoomId: "!KMDIEEnU...",  // ← Should be defined, not "undefined..."
  isMatrixTab: true,              // ← Should be true
  messagesCount: 0,
  ...
}
```

**B. Is the Matrix listener being set up?**
```
🔍 useChatStream Matrix listener setup check: {
  isMatrixTab: true,               // ← Should be true
  willSetupListener: true,         // ← Should be true
  ...
}

✅ useChatStream: Setting up Matrix message listener for Matrix tab
```

**C. Are messages being received?**
```
✅ useChatStream received SESSION-SPECIFIC matrix-message-received event
✅ Matrix message added to stream for Matrix tab
```

### Step 3: Identify the Problem

Based on the console logs, identify which condition is failing:

#### Problem A: `matrixRoomId` is undefined
**Symptom**: 
```
🔍 BaseChat2 render state: {
  matrixRoomId: "undefined...",
  isMatrixTab: false,
  ...
}
```

**Cause**: The `matrixRoomId` prop is not being passed to `BaseChat2` correctly.

**Fix Location**: Check `TabbedChatContainer.tsx` to ensure `matrixRoomId` is passed:
```typescript
<BaseChat2
  sessionId={activeTabState.tab.sessionId}
  matrixRoomId={activeTabState.tab.matrixRoomId}  // ← Must be defined
  // ...
/>
```

#### Problem B: Matrix listener not being set up
**Symptom**:
```
🚫 useChatStream: Not a Matrix tab, skipping Matrix message listener setup
```

**Cause**: `isMatrixTab` is false because `matrixRoomId` is undefined.

**Fix**: Same as Problem A - ensure `matrixRoomId` is passed correctly.

#### Problem C: Messages not being added to state
**Symptom**: Listener is set up, but messages aren't appearing.

**Possible Causes**:
1. Messages are being filtered out by session ID mismatch
2. Message deduplication is too aggressive
3. `setMessagesAndLog` is not updating state correctly

**Check for**:
```
🚫 useChatStream ignoring matrix message for different session
```

If you see this, the `targetSessionId` in the event doesn't match the `sessionId` in `useChatStream`.

### Step 4: Test Message Sending

1. Send a message in the Space room chat
2. Check console for:
   - `📤 Sending message to Matrix room:`
   - `✅ Message sent to Matrix successfully`
   - `✅ useChatStream received SESSION-SPECIFIC matrix-message-received event`
   - `✅ Matrix message added to stream`

### Step 5: Test History Loading

1. Open a Space room that has existing messages
2. Check console for:
   - `📜 Loading Matrix room history`
   - `📜 Fetched X messages from Matrix room`
   - `📝 Adding X messages from matrix-history`

## Common Issues & Solutions

### Issue 1: Tab not marked as Matrix type

**Check**: In `TabContext.openMatrixChat()`, verify the tab is created with:
```typescript
{
  type: 'matrix',
  matrixRoomId: roomId,
  sessionId: backendSessionId,
  // ...
}
```

### Issue 2: Session ID mismatch

**Check**: The `sessionId` in `useChatStream` should match the backend session ID, NOT the Matrix room ID.

**Verify**:
- `tab.sessionId` = backend session ID (e.g., `"20251203_123456"`)
- `tab.matrixRoomId` = Matrix room ID (e.g., `"!KMDIEEnUYlUgxXwjZo:tchncs.de"`)

### Issue 3: Message event not being dispatched

**Check**: In `BaseChat2.append()`, verify the event is being dispatched:
```typescript
const messageEvent = new CustomEvent('matrix-message-received', {
  detail: { 
    message,
    targetSessionId: sessionId,  // Should match useChatStream's sessionId
    timestamp: new Date().toISOString()
  }
});
window.dispatchEvent(messageEvent);
```

## Next Steps

1. Open a Space room chat
2. Copy all console logs that start with `🔍`, `✅`, or `🚫`
3. Share those logs so we can identify the exact issue
4. Based on the logs, we'll apply the appropriate fix

## Quick Test Commands

You can run these in the browser console to check state:

```javascript
// Check all tabs
window.__tabContext?.tabStates.map(ts => ({
  id: ts.tab.id,
  type: ts.tab.type,
  sessionId: ts.tab.sessionId?.substring(0, 8),
  matrixRoomId: ts.tab.matrixRoomId?.substring(0, 20),
  messagesCount: ts.chat.messages?.length
}))

// Check active tab
window.__tabContext?.getActiveTabState()

// Check session mappings
window.__sessionMappingService?.getAllMappings()
```

## Files Modified

- `ui/desktop/src/components/BaseChat2.tsx` - Added diagnostic logging
- `ui/desktop/src/hooks/useChatStream.ts` - Added diagnostic logging
- `MATRIX_CHAT_ROUTING_ISSUES.md` - Detailed analysis document
- `DEBUGGING_MATRIX_CHATS.md` - This file (debugging instructions)
