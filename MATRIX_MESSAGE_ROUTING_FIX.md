# Matrix Message Routing Fix

## Problem
Messages sent from Goose are appearing in the wrong Matrix rooms when multiple Matrix room tabs are open.

## Root Cause
The issue occurs because:
1. Each Matrix room needs its own unique Goose `session_id`
2. When sending messages, the `session_id` must be consistently used to route to the correct Matrix room
3. The `chat.id` property determines which `session_id` is used in API requests
4. If multiple Matrix tabs share the same `chat.id` or if the `chat.id` changes, messages get routed incorrectly

## Solution Architecture

### 1. Session ID Management
- Each Matrix room must have a unique, stable `session_id`
- The `session_id` should be derived from or mapped to the `matrix_room_id`
- This mapping must persist for the lifetime of the tab

### 2. Message Sending Flow
```
User types message in Tab A (Matrix Room X)
  ↓
BaseChat receives message
  ↓
useChatEngine.handleSubmit() called with chat.id = "session_for_room_x"
  ↓
useMessageStream.append() sends to backend with session_id = "session_for_room_x"
  ↓
Backend routes to Matrix Room X based on session_id mapping
  ↓
Message appears in correct Matrix room
```

### 3. Key Components

#### A. Chat Object Structure
Each Matrix room tab must have:
```typescript
{
  id: "unique_session_id_for_this_matrix_room",  // Must be unique per Matrix room
  title: "Room Name",
  messages: [...],
  matrixRoomId: "!roomid:server.com",  // The actual Matrix room ID
  isMatrixTab: true
}
```

#### B. Session Creation
When opening a Matrix room:
1. Check if a session mapping exists for this Matrix room
2. If not, create a new backend session
3. Store the mapping: `matrix_room_id` → `goose_session_id`
4. Use this `goose_session_id` as the `chat.id`

#### C. Message Sending
When sending a message:
1. The `chat.id` is passed to `useChatEngine`
2. `useChatEngine` passes it to `useMessageStream` in the `body.session_id`
3. `useMessageStream` sends it to the backend `/reply` endpoint
4. Backend uses `session_id` to look up the Matrix room mapping
5. Backend sends the message to the correct Matrix room

## Implementation Checklist

### Frontend (ui/desktop/src/)
- [ ] Ensure each Matrix room tab has a unique `chat.id`
- [ ] Verify `chat.id` doesn't change during tab lifecycle
- [ ] Confirm `chat.id` is passed correctly through component hierarchy
- [ ] Add logging to track session IDs in message flow

### Backend
- [ ] Verify session → Matrix room mapping is created correctly
- [ ] Ensure mapping persists for session lifetime
- [ ] Add logging to track message routing
- [ ] Confirm messages are sent to correct Matrix room based on session_id

## Testing Steps

1. **Single Room Test**
   - Open one Matrix room
   - Send a message
   - Verify it appears in the correct room in Matrix

2. **Multiple Rooms Test**
   - Open 2-3 Matrix rooms in different tabs
   - Send messages from each tab
   - Verify each message appears in its respective room
   - Check that no cross-contamination occurs

3. **Session Persistence Test**
   - Open a Matrix room
   - Send a message
   - Close and reopen the room
   - Send another message
   - Verify both messages are in the same room

4. **Concurrent Sending Test**
   - Open multiple Matrix rooms
   - Rapidly switch between tabs and send messages
   - Verify all messages route correctly

## Debugging

### Frontend Logging
Add these logs to track the flow:
```typescript
// In BaseChat or useChatEngine
console.log('[Matrix Message Send] chat.id:', chat.id, 'matrixRoomId:', chat.matrixRoomId);

// In useMessageStream
console.log('[Matrix Message Send] session_id in request:', body.session_id);
```

### Backend Logging
Check backend logs for:
- Session creation for Matrix rooms
- Session → Matrix room mapping
- Message routing decisions

### Common Issues
1. **chat.id is undefined or null**: Ensure session is created before opening tab
2. **chat.id changes**: Check if tab state is being reset unexpectedly
3. **Multiple tabs share same chat.id**: Ensure unique session per room
4. **Mapping not found**: Verify session mapping service is working

## Related Files
- `ui/desktop/src/hooks/useChatEngine.ts` - Manages chat state and message sending
- `ui/desktop/src/hooks/useMessageStream.ts` - Sends messages to backend
- `ui/desktop/src/components/BaseChat.tsx` - Main chat component
- Backend session management and Matrix integration code
