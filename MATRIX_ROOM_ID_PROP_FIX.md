# Matrix Room ID Prop Fix

## Problem
The diagnostic logs showed that `ChatInput` was not receiving the `matrixRoomId` prop:

```
🔍 ChatInput Matrix room detection (STRICT TAB-CENTRIC): {
  sessionId: '20251128_62', 
  isNewSession: false, 
  tabMatrixRoomId: null,  // <-- NULL!
  tabMatrixRecipientId: null, 
  isExplicitMatrixTab: false
}
```

This meant that `ChatInput` couldn't determine which Matrix room to send messages to, leading to messages being sent to incorrect rooms when multiple Matrix tabs were open.

## Root Cause
The `matrixRoomId` was not being passed through the component hierarchy:
- `pair.tsx` → `BaseChat` → `ChatInput`

While the `chat` object contained `matrixRoomId`, it wasn't being explicitly passed to `ChatInput` as a prop.

## Solution

### 1. Updated `ChatInput.tsx`
Added `matrixRoomId` to the `ChatInputProps` interface:

```typescript
interface ChatInputProps {
  // ... existing props ...
  matrixRoomId?: string | null; // Matrix room ID for this specific chat tab
}
```

Extracted the prop in the function signature:

```typescript
export default function ChatInput({
  // ... existing props ...
  matrixRoomId,
}: ChatInputProps) {
  // DIAGNOSTIC: Log the matrixRoomId prop when ChatInput renders
  console.log('[ChatInput] Received matrixRoomId prop:', {
    matrixRoomId,
    timestamp: new Date().toISOString(),
  });
  // ... rest of component ...
}
```

### 2. Updated `pair.tsx`
Modified `customChatInputProps` to pass the `matrixRoomId`:

```typescript
const customChatInputProps = {
  // Pass initial message from Hub or recipe prompt
  initialValue,
  // Pass Matrix room ID if this is a Matrix chat
  matrixRoomId: chat.matrixRoomId || null,
};
```

## Expected Behavior

After this fix:

1. **ChatInput receives matrixRoomId**: The diagnostic log should show:
   ```
   [ChatInput] Received matrixRoomId prop: {
     matrixRoomId: '!KxKXXYDKFfbKQXgDXO:tchncs.de',
     timestamp: '2025-12-03T...'
   }
   ```

2. **Tab-specific Matrix room detection**: `ChatInput` can now correctly identify which Matrix room it belongs to, ensuring messages are sent to the correct room.

3. **Multiple Matrix tabs work correctly**: Each `ChatInput` instance will have its own unique `matrixRoomId`, preventing cross-contamination of messages between tabs.

## Testing

1. **Restart Goose Desktop** to ensure all new code is loaded.
2. **Open 2-3 different Matrix rooms** from Spaces.
3. **Check the console logs** - you should see `[ChatInput] Received matrixRoomId prop` logs with the correct room IDs.
4. **Send messages from each Matrix room tab** and verify they appear in the correct Matrix room.
5. **Verify the diagnostic logs** show the correct `tabMatrixRoomId` (not null).

## Files Modified

1. `ui/desktop/src/components/ChatInput.tsx`
   - Added `matrixRoomId` prop to interface
   - Extracted prop in function signature
   - Added diagnostic logging

2. `ui/desktop/src/components/pair.tsx`
   - Updated `customChatInputProps` to include `matrixRoomId`

## Related Work

This fix complements the earlier work:
- Matrix Room Interceptor (`matrixRoomInterceptor.ts`)
- Matrix Session ID generation (`generateMatrixSessionId`)
- Matrix Room Helper (`matrixRoomHelper.ts`)
- Extended `ChatType` interface with Matrix metadata

Together, these changes ensure that:
1. Matrix chats have unique, stable session IDs
2. Matrix metadata is preserved during navigation
3. Each `ChatInput` knows which Matrix room it belongs to
4. Messages are sent to the correct Matrix room

## Date
2025-12-03
