# Matrix Session Fix - Implementation Guide

## Changes Made

### 1. Updated `ChatType` Interface
**File**: `ui/desktop/src/types/chat.ts`

Added Matrix-specific properties to the `ChatType` interface:
```typescript
export interface ChatType {
  // ... existing properties ...
  matrixRoomId?: string | null; // The Matrix room ID (e.g., !roomId:server.com)
  matrixRecipientId?: string | null; // The recipient user ID for Matrix rooms
  isMatrixTab?: boolean; // Flag to identify Matrix tabs
}
```

### 2. Created `generateMatrixSessionId()` Function
**File**: `ui/desktop/src/sessions.ts`

Added a new function to generate unique, stable session IDs for Matrix rooms:
```typescript
export function generateMatrixSessionId(matrixRoomId: string): string
```

This function:
- Creates a hash from the Matrix room ID for uniqueness
- Ensures the same Matrix room always gets the same session ID
- Uses format: `YYYYMMDD_matrix_<roomhash>`
- Makes Matrix sessions easily identifiable

## Where to Apply the Fix

### Step 1: Find Matrix Room Opening Code

You need to find where Matrix rooms are opened when clicked in a Space. This is likely in one of these locations:

**Option A: Direct component search**
```bash
# Search for room click handlers
grep -rn "onClick.*room\|handleClick.*room\|onRoomClick" ui/desktop/src/components

# Search for Space-related components
find ui/desktop/src/components -name "*Space*" -o -name "*Room*" -o -name "*Channel*"
```

**Option B: Follow the data flow**
1. Open DevTools in Goose Desktop
2. Go to the Channels/Spaces view
3. Right-click on a Matrix room and "Inspect Element"
4. Look at the React component tree to find the component name
5. Search for that component in the codebase

### Step 2: Modify the Room Opening Logic

Once you find where Matrix rooms are opened (likely a function like `handleOpenRoom`, `openMatrixRoom`, or `onRoomClick`), modify it like this:

**BEFORE (Current - Broken)**:
```typescript
const handleOpenRoom = async (room) => {
  // ❌ Problem: Uses regular session ID or reuses existing session
  const sessionId = generateSessionId(); // or getCurrentSessionId()
  
  // ❌ Problem: No Matrix metadata
  const chat = {
    id: sessionId,
    title: room.name,
    messages: []
  };
  
  // Open the tab/chat
  openTab(chat); // or navigate to pair view, etc.
};
```

**AFTER (Fixed)**:
```typescript
import { generateMatrixSessionId } from '../sessions'; // Add this import

const handleOpenRoom = async (room) => {
  // ✅ Solution: Use Matrix-specific session ID
  const sessionId = generateMatrixSessionId(room.roomId);
  
  console.log('[Matrix Room Open] Creating chat for room:', {
    roomId: room.roomId,
    sessionId,
    roomName: room.name
  });
  
  // ✅ Solution: Include Matrix metadata
  const chat: ChatType = {
    id: sessionId,
    title: room.name,
    messages: [],
    messageHistoryIndex: 0,
    matrixRoomId: room.roomId,  // ✅ Add this
    matrixRecipientId: room.recipientId || null,  // ✅ Add this
    isMatrixTab: true,  // ✅ Add this
    recipeConfig: null
  };
  
  // Open the tab/chat
  openTab(chat); // or navigate to pair view, etc.
};
```

### Step 3: Verify the Backend Session Creation

Ensure that when a Matrix room is opened, a backend session is created with the correct mapping. This might be in a separate service or API call.

Look for code that calls something like:
- `createSession()`
- `createBackendSession()`
- `sessionMappingService.createMapping()`
- API calls to `/api/sessions`

Make sure it's called with the Matrix-specific session ID:

```typescript
// After creating the chat object
await createBackendSession({
  session_id: sessionId,  // The Matrix-specific session ID
  description: `Matrix room: ${room.name}`,
  matrix_room_id: room.roomId  // Store the mapping
});
```

## Example Implementation Locations

Based on common patterns, the code is likely in one of these files:

### Pattern 1: Space Rooms View Component
```typescript
// File: ui/desktop/src/components/channels/SpaceRoomsView.tsx (or similar)

const SpaceRoomsView = ({ spaceId }) => {
  const handleOpenRoom = async (room) => {
    // THIS IS WHERE YOU APPLY THE FIX
    const sessionId = generateMatrixSessionId(room.roomId);
    
    const chat: ChatType = {
      id: sessionId,
      title: room.name,
      messages: [],
      messageHistoryIndex: 0,
      matrixRoomId: room.roomId,
      matrixRecipientId: room.recipientId,
      isMatrixTab: true,
      recipeConfig: null
    };
    
    // Navigate to pair view or open tab
    navigate('/pair', { state: { chat } });
  };
  
  return (
    <div>
      {rooms.map(room => (
        <RoomItem 
          key={room.roomId}
          room={room}
          onClick={() => handleOpenRoom(room)}
        />
      ))}
    </div>
  );
};
```

### Pattern 2: Matrix Service
```typescript
// File: ui/desktop/src/services/MatrixService.ts (or similar)

class MatrixService {
  async openRoom(roomId: string) {
    // THIS IS WHERE YOU APPLY THE FIX
    const sessionId = generateMatrixSessionId(roomId);
    
    const room = await this.client.getRoom(roomId);
    
    const chat: ChatType = {
      id: sessionId,
      title: room.name,
      messages: [],
      messageHistoryIndex: 0,
      matrixRoomId: roomId,
      matrixRecipientId: this.getRecipientId(room),
      isMatrixTab: true,
      recipeConfig: null
    };
    
    return chat;
  }
}
```

### Pattern 3: Tab Management Hook
```typescript
// File: ui/desktop/src/hooks/useMatrixTabs.ts (or similar)

export const useMatrixTabs = () => {
  const openMatrixRoom = async (roomId: string, roomName: string) => {
    // THIS IS WHERE YOU APPLY THE FIX
    const sessionId = generateMatrixSessionId(roomId);
    
    const chat: ChatType = {
      id: sessionId,
      title: roomName,
      messages: [],
      messageHistoryIndex: 0,
      matrixRoomId: roomId,
      isMatrixTab: true,
      recipeConfig: null
    };
    
    addTab(chat);
  };
  
  return { openMatrixRoom };
};
```

## Testing the Fix

After applying the fix:

### 1. Test Unique Session IDs
```bash
# Open DevTools Console
# Open Matrix Room A
# Look for log: [Matrix Room Open] Creating chat for room: {...}
# Note the sessionId

# Open Matrix Room B
# Look for log: [Matrix Room Open] Creating chat for room: {...}
# Note the sessionId

# Verify: sessionIds should be DIFFERENT
```

### 2. Test Message Routing
```bash
# In Room A tab, send message: "Test for Room A"
# Check console logs for [Matrix Message Send - ...]
# Verify sessionId matches Room A's session ID
# Verify message appears in Matrix Room A (not Room B)

# Switch to Room B tab, send message: "Test for Room B"
# Check console logs
# Verify sessionId matches Room B's session ID
# Verify message appears in Matrix Room B (not Room A)
```

### 3. Test Session Persistence
```bash
# Open Room A, note its sessionId
# Close the tab
# Reopen Room A
# Verify it has the SAME sessionId
# Verify message history loads correctly
```

## Verification Checklist

- [ ] Found the code that opens Matrix rooms
- [ ] Added `import { generateMatrixSessionId } from '../sessions'`
- [ ] Changed session ID generation to use `generateMatrixSessionId(room.roomId)`
- [ ] Added `matrixRoomId` to the chat object
- [ ] Added `isMatrixTab: true` to the chat object
- [ ] Added `matrixRecipientId` if available
- [ ] Verified backend session creation uses the Matrix session ID
- [ ] Tested with 2+ Matrix rooms - each has unique session ID
- [ ] Tested message sending - messages go to correct rooms
- [ ] Tested reopening rooms - same session ID is used
- [ ] Removed diagnostic logging once verified working

## Expected Console Output (After Fix)

```
[Matrix Room Open] Creating chat for room: {
  roomId: '!KxKXXYDKFfbKQXgDXO:tchncs.de',
  sessionId: '20251203_matrix_abc123',  // Unique per room!
  roomName: 'Room Alpha'
}

[Matrix Message Send - BaseChat] handleSubmit called: {
  chatId: '20251203_matrix_abc123',  // Matches room!
  chatTitle: 'Room Alpha',
  messagePreview: 'Test message...'
}

[Matrix Message Send - useMessageStream] Sending to backend: {
  session_id: '20251203_matrix_abc123',  // Consistent!
  ...
}
```

## Troubleshooting

### Issue: Can't find where rooms are opened
**Solution**: Use React DevTools to inspect the component tree when viewing a Space, or add `console.log` statements in likely files and click a room to see which fires.

### Issue: Session ID is still shared
**Solution**: Make sure you're calling `generateMatrixSessionId(room.roomId)` and not `generateSessionId()` or reusing an existing session.

### Issue: matrixRoomId is undefined
**Solution**: Check that `room.roomId` exists. You might need to use `room.id` or `room.room_id` depending on the data structure.

### Issue: Messages still go to wrong room
**Solution**: Verify the backend is using the session_id to look up the Matrix room mapping. This might require backend changes.

## Next Steps

1. **Find the code** using the search commands above
2. **Apply the fix** following the BEFORE/AFTER pattern
3. **Test thoroughly** using the testing steps
4. **Share results** - if it works, great! If not, share the code you modified and any error messages

The fix is straightforward once you find the right location - just ensure each Matrix room gets its own unique session ID with the proper metadata!
