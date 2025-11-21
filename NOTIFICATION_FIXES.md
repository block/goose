# Notification System Fixes

## Summary
Fixed four major issues with the Matrix notification system:
1. **Goose assistant responses were triggering notifications** (spam)
2. **Notifications continued after closing Matrix tabs** (stale state)
3. **Wrong room notifications were suppressed** (legacy pair view bug)
4. **"Open Chat" created blank sessions** instead of opening existing chats

## Changes Made

### 1. MessageNotification.tsx - Suppress Goose Assistant Responses

**Problem**: Goose's AI responses in collaborative Matrix sessions were showing as notifications because they come from other users' Goose instances.

**Solution**: Parse the `goose-session-message:` content and check the `role` field. Only show notifications for `role: "user"` (human messages), suppress `role: "assistant"` (AI responses).

**Code Added** (lines ~108-135):
```typescript
// IMPORTANT: Don't show notifications for Goose assistant responses!
// These are AI responses, not human messages that need notification
// Parse the goose-session-message content to check the role
try {
  if (content.startsWith('goose-session-message:')) {
    const jsonContent = content.substring('goose-session-message:'.length);
    const parsed = JSON.parse(jsonContent);
    
    // Suppress notifications for assistant messages (Goose's responses)
    if (parsed.role === 'assistant') {
      console.log('🔕 Suppressing Goose assistant message notification (AI response, not human):', {
        roomId,
        sender,
        role: parsed.role
      });
      return;
    }
    
    // Only show notifications for user messages (human messages in collaborative sessions)
    console.log('🦆 Goose user message detected (human message in collab session):', {
      roomId,
      sender,
      role: parsed.role
    });
  }
} catch (error) {
  console.warn('Failed to parse goose-session-message content:', error);
  // If parsing fails, fall through to normal notification logic
}
```

### 2. MessageNotification.tsx - Fix Stale Tab State in Notification Suppression

**Problem**: When a Matrix tab is closed, the notification listener still had the old tab state captured in its closure, so it continued suppressing notifications for that room.

**Solution**: Changed from destructuring `shouldSuppressNotification` to storing the entire `activeSessionHook` object, then calling the method fresh each time to get current tab state.

**Code Changed** (lines ~34-35, ~59, ~144):
```typescript
// OLD:
const { shouldSuppressNotification } = useActiveSession();
const shouldSuppress = shouldSuppressNotification(roomId, sender);

// NEW:
const activeSessionHook = useActiveSession();
// CRITICAL: Call shouldSuppressNotification fresh each time to get current tab state
// Don't capture it in closure - this ensures we always check against current tabs
const shouldSuppress = activeSessionHook.shouldSuppressNotification(roomId, sender);
```

This ensures that every time a message arrives, the suppression check uses the **current** tab state from `TabContext`, not stale state from when the listener was created.

### 3. useActiveSession.ts - Remove Buggy Legacy Pair View Suppression

**Problem**: The legacy `/pair` view suppression was checking if `messageSenderId === currentRecipientId`, which incorrectly suppressed notifications from **different rooms** when the sender ID matched.

**Example Bug**:
- You're viewing room A with recipient `@spence:tchncs.de`
- Message arrives from room B, sent by `@spence:tchncs.de` (you)
- Legacy logic suppressed it because sender matched recipient, even though it's a different room!

**Solution**: Removed the buggy legacy `/pair` view suppression logic entirely. The Matrix room check (line ~169) is the correct way to suppress notifications.

**Code Removed** (lines ~210-220):
```typescript
// REMOVED:
if (currentView.path.startsWith('/pair') && 
    currentView.matrixRecipientId && 
    messageSenderId === currentView.matrixRecipientId) {
  console.log('🔕 Suppressing notification: message from current pair recipient (legacy)');
  return true;
}

// NOW: Only suppress based on room ID match, not sender ID
```

### 4. App.tsx - Fix "Open Chat" to Use Existing Tabs

**Problem**: Clicking "Open Chat" on a notification always navigated to `/pair`, which could cause the tab system to re-initialize and lose track of existing tabs. This resulted in creating a blank new session instead of opening the existing chat.

**Solution**: Check if we're already on the `/pair` route before navigating. If we are, dispatch the `create-matrix-tab` event immediately without navigation.

**Code Changed** (lines ~368-395):
```typescript
// OLD:
navigate('/pair');
setTimeout(() => {
  const event = new CustomEvent('create-matrix-tab', { detail: { roomId, senderId } });
  window.dispatchEvent(event);
}, 100);

// NEW:
const isOnPairRoute = location.pathname === '/pair' || location.pathname === '/tabs';

if (isOnPairRoute) {
  // Already on pair route - just dispatch the event immediately
  const event = new CustomEvent('create-matrix-tab', { detail: { roomId, senderId } });
  window.dispatchEvent(event);
} else {
  // Navigate to pair view first, then dispatch event
  navigate('/pair');
  setTimeout(() => {
    const event = new CustomEvent('create-matrix-tab', { detail: { roomId, senderId } });
    window.dispatchEvent(event);
  }, 100);
}
```

This ensures that `openMatrixChat` in `TabContext.tsx` can properly check for existing tabs and switch to them instead of creating duplicates.

## Testing

1. **Test Goose Assistant Suppression**:
   - Open a Matrix collaborative session
   - Send a message and wait for Goose to respond
   - ✅ You should NOT see a notification for Goose's response
   - ✅ You should only see notifications for human messages from collaborators

2. **Test Tab Close Behavior**:
   - Open a Matrix chat tab
   - While viewing it, messages should be suppressed (no notifications)
   - Close the tab
   - Send a message to that room from another device
   - ✅ You should now see a notification (not suppressed)

3. **Test Multi-Room Suppression**:
   - Open Matrix room A in a tab
   - Receive a message in room B (different room)
   - ✅ You should see a notification for room B
   - ✅ Room B notification should NOT be suppressed just because you're viewing room A

4. **Test "Open Chat" from Notification**:
   - Receive a notification for a Matrix room
   - Click "Open Chat" on the notification
   - ✅ If the room is already open in a tab, it should switch to that tab
   - ✅ If the room is not open, it should create a new tab with the existing conversation loaded
   - ✅ Should NOT create a blank new session

## Impact

- **Reduced notification spam**: No more constant Goose assistant response notifications
- **Correct suppression behavior**: Notifications work correctly after closing tabs and across multiple rooms
- **Better UX**: Users only get notified for actual human messages that need attention
- **Proper navigation**: "Open Chat" correctly opens existing conversations instead of blank sessions
