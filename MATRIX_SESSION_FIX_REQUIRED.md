# Matrix Session Fix Required

## Problem Identified

Based on the diagnostic logs, we've identified the root cause of messages being sent to the wrong Matrix rooms:

### Symptoms
1. **Multiple Matrix room tabs share the same `sessionId`** (e.g., `20251203_22`)
2. **Matrix room metadata is missing** from tabs opened from Spaces (`tabMatrixRoomId: null`)
3. **Wrong session is used when sending** (sends with `20251128_62` instead of `20251203_22`)

### Example from Logs

**Tab 1 (Space channel):**
```javascript
{
  sessionId: '20251203_22',
  tabMatrixRoomId: '!KxKXXYDKFfbKQXgDXO:tchncs.de',  // ✓ Has room ID
  isExplicitMatrixTab: true
}
```

**Tab 2 (Where you're typing):**
```javascript
{
  sessionId: '20251203_22',  // ❌ Same session ID!
  tabMatrixRoomId: null,      // ❌ Missing room ID!
  isExplicitMatrixTab: false  // ❌ Not recognized as Matrix tab!
}
```

**When sending:**
```javascript
// Uses completely different session!
sessionId: "20251128_62"  // ❌ Where did this come from?
roomId: "!KxKXXYDKFfbKQXgDXO:tchncs.de"  // ❌ Wrong room!
```

## Root Cause

When opening a Matrix room from a Space, the system is:
1. **Reusing an existing session** instead of creating a new one
2. **Not attaching Matrix room metadata** to the new tab
3. **Falling back to an old session mapping** when sending messages

## Required Fix

### 1. Unique Session Per Matrix Room

Each Matrix room must have its own unique session ID. The session ID should be:
- **Derived from the Matrix room ID** (e.g., `matrix_${roomId}`)
- **Created when the room is first opened**
- **Persisted for the lifetime of the tab**

### 2. Matrix Room Metadata

When creating a tab for a Matrix room, the `chat` object must include:
```typescript
{
  id: "unique_session_for_this_room",  // Unique per room
  title: "Room Name",
  messages: [],
  matrixRoomId: "!roomId:server.com",  // The actual Matrix room ID
  matrixRecipientId: "@user:server.com",  // The recipient user ID
  isMatrixTab: true,
  // ... other properties
}
```

### 3. Session → Room Mapping

The backend must maintain a mapping:
```
session_id → matrix_room_id
```

When a message is sent with `session_id: "abc123"`, the backend should:
1. Look up the mapping: `abc123 → !roomX:server.com`
2. Send the message to Matrix room `!roomX:server.com`

## Implementation Steps

### Step 1: Find Where Matrix Rooms Are Opened

Search for where Matrix rooms are opened from Spaces. This is likely in:
- `SpaceRoomsView.tsx`
- A tab management component
- A Matrix integration component

Look for functions like:
- `handleOpenRoom()`
- `openMatrixRoom()`
- `addTab()` with Matrix room data

### Step 2: Modify Room Opening Logic

When opening a Matrix room:

```typescript
// BEFORE (Wrong - reuses existing session)
const chat = {
  id: existingSessionId,  // ❌ Wrong!
  title: room.name,
  messages: []
};

// AFTER (Correct - creates unique session)
const chat = {
  id: await createMatrixRoomSession(room.roomId),  // ✓ Unique session
  title: room.name,
  messages: [],
  matrixRoomId: room.roomId,  // ✓ Attach room ID
  matrixRecipientId: room.recipientId,  // ✓ Attach recipient
  isMatrixTab: true  // ✓ Mark as Matrix tab
};
```

### Step 3: Create Session Mapping

When creating the session:

```typescript
async function createMatrixRoomSession(matrixRoomId: string): Promise<string> {
  // Check if session already exists for this room
  const existingSession = await getSessionForMatrixRoom(matrixRoomId);
  if (existingSession) {
    return existingSession.id;
  }
  
  // Create new backend session
  const newSession = await createBackendSession({
    description: `Matrix room: ${matrixRoomId}`,
    matrixRoomId: matrixRoomId
  });
  
  // Store the mapping
  await storeSessionMapping(newSession.id, matrixRoomId);
  
  return newSession.id;
}
```

### Step 4: Verify Message Sending

When sending a message, verify the flow:

```typescript
// In ChatInput or message sending logic
const sessionId = chat.id;  // Should be unique per Matrix room
const matrixRoomId = chat.matrixRoomId;  // Should be present

console.log('Sending message:', {
  sessionId,
  matrixRoomId,
  expectedRoom: chat.title
});

// Backend should use sessionId to look up matrixRoomId
// and send to the correct room
```

## Testing Checklist

After implementing the fix:

- [ ] Open Matrix Room A from a Space
  - Verify `sessionId` is unique (e.g., `matrix_roomA_xxx`)
  - Verify `matrixRoomId` is set
  - Verify `isMatrixTab` is true

- [ ] Open Matrix Room B from a Space
  - Verify `sessionId` is different from Room A
  - Verify `matrixRoomId` is set to Room B's ID
  - Verify `isMatrixTab` is true

- [ ] Send message from Room A
  - Verify console shows correct `sessionId`
  - Verify message appears in Room A (not Room B)

- [ ] Send message from Room B
  - Verify console shows correct `sessionId`
  - Verify message appears in Room B (not Room A)

- [ ] Close and reopen Room A
  - Verify same `sessionId` is used
  - Verify message history loads correctly

## Files to Check

Based on the summary, these files are likely involved:

1. **`ui/desktop/src/components/channels/SpaceRoomsView.tsx`**
   - Handles opening rooms from Spaces
   - Look for `handleOpenRoom()` or similar

2. **Tab management component** (TabbedChatContainer or similar)
   - Manages tab creation and state
   - Look for `addTab()` or tab state management

3. **Session mapping service**
   - Handles session → Matrix room mapping
   - May need to be created if it doesn't exist

4. **Backend session creation**
   - API endpoint for creating sessions
   - Should accept Matrix room metadata

## Quick Debug Commands

To find the relevant code:

```bash
# Find where rooms are opened from Spaces
grep -rn "handleOpenRoom\|openMatrixRoom" ui/desktop/src/components

# Find tab creation logic
grep -rn "addTab\|createTab.*matrix" ui/desktop/src

# Find session creation
grep -rn "createSession\|newSession.*matrix" ui/desktop/src

# Find Matrix room metadata usage
grep -rn "matrixRoomId.*chat\|chat.*matrixRoomId" ui/desktop/src
```

## Expected Result

After the fix:

```javascript
// Tab A (Matrix Room Alpha)
{
  sessionId: 'matrix_roomAlpha_unique',
  tabMatrixRoomId: '!roomAlpha:server.com',
  isExplicitMatrixTab: true
}

// Tab B (Matrix Room Beta)  
{
  sessionId: 'matrix_roomBeta_unique',  // Different!
  tabMatrixRoomId: '!roomBeta:server.com',  // Different!
  isExplicitMatrixTab: true
}

// Sending from Tab A
sessionId: 'matrix_roomAlpha_unique'  // Correct!
roomId: '!roomAlpha:server.com'  // Correct!
```

## Priority

This is a **HIGH PRIORITY** fix because:
1. Messages are being sent to wrong rooms (data integrity issue)
2. Users can't reliably use Matrix integration
3. The issue affects all Matrix Space rooms

## Next Action

**Find the code that opens Matrix rooms from Spaces** and modify it to create unique sessions with proper metadata.
