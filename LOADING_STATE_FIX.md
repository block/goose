# Loading State for Matrix Chat Initialization

## Summary
Added a prominent loading state when opening Matrix chats from notifications to provide visual feedback while the conversation is being initialized and message history is being loaded.

## Problem
When clicking "Open Chat" on a Matrix notification, the chat would appear empty for a few seconds while:
1. Backend session was being created or retrieved
2. Matrix room history was being loaded
3. Session mapping was being established

This made it look like the chat was broken or empty, even though it was just loading.

## Solution
Implemented a two-phase loading state:

### Phase 1: Create Temporary Tab with Loading State
When `openMatrixChat` is called, immediately create a tab with:
- `loadingChat: true` - Shows loading indicator
- Temporary session ID (`temp_matrix_${timestamp}`)
- Tab is immediately visible to user

### Phase 2: Update with Real Session
Asynchronously:
- Get or create backend session mapping
- Update tab with real backend session ID
- Set `loadingChat: false` - Removes loading indicator
- Message history loads via `useChatStream`

## Changes Made

### 1. TabContext.tsx - Add Loading State to `openMatrixChat`

**File**: `ui/desktop/src/contexts/TabContext.tsx`

**Change**: Modified `openMatrixChat` to create a temporary loading tab immediately, then update it with the real session ID asynchronously.

**Before**:
```typescript
const openMatrixChat = useCallback(async (roomId: string, senderId: string) => {
  // Check for existing tab...
  
  // Get or create backend session (blocking)
  let backendSessionId = sessionMappingService.getGooseSessionId(roomId);
  // ... create session if needed ...
  
  // Create tab with real session ID
  const newTab = createNewTab({ sessionId: backendSessionId, ... });
  const newTabState = { tab: newTab, chat: {...}, loadingChat: false };
  
  setTabStates(prev => [...prev, newTabState]);
  setActiveTabId(newTab.id);
}, [tabStates]);
```

**After**:
```typescript
const openMatrixChat = useCallback(async (roomId: string, senderId: string) => {
  // Check for existing tab...
  
  // Create temporary tab with loading state IMMEDIATELY
  const tempTab = createNewTab({
    sessionId: `temp_matrix_${Date.now()}`,
    title: `Chat with ${senderName}`,
    type: 'matrix',
    matrixRoomId: roomId,
    matrixRecipientId: senderId,
    isActive: true
  });
  
  const tempTabState = {
    tab: tempTab,
    chat: {...},
    loadingChat: true // Show loading indicator
  };
  
  // Add loading tab immediately for instant feedback
  setTabStates(prev => [...prev, tempTabState]);
  setActiveTabId(tempTab.id);
  
  // Get or create backend session (async, doesn't block UI)
  let backendSessionId = sessionMappingService.getGooseSessionId(roomId);
  // ... create session if needed ...
  
  // Update tab with real session ID and remove loading state
  setTabStates(prev => prev.map(ts => 
    ts.tab.id === tempTab.id
      ? {
          ...ts,
          tab: { ...ts.tab, sessionId: backendSessionId },
          chat: { ...ts.chat, sessionId: backendSessionId },
          loadingChat: false // Remove loading indicator
        }
      : ts
  ));
}, [tabStates]);
```

### 2. BaseChat2.tsx - Improve Loading Indicator Visibility

**File**: `ui/desktop/src/components/BaseChat2.tsx`

**Change**: Made the loading indicator more prominent and centered.

**Before**:
```typescript
{loadingChat && (
  <div className="px-6 py-4">
    <LoadingGoose
      message="loading conversation..."
      chatState={ChatState.Idle}
    />
  </div>
)}
```

**After**:
```typescript
{loadingChat && (
  <div className="flex items-center justify-center h-full min-h-[400px]">
    <div className="text-center">
      <LoadingGoose
        message="Loading conversation..."
        chatState={ChatState.Idle}
      />
      <p className="text-text-muted text-sm mt-4">
        Fetching message history...
      </p>
    </div>
  </div>
)}
```

## User Experience Flow

### Before:
1. User clicks "Open Chat" on notification
2. **[2-3 second delay with empty chat]**
3. Messages suddenly appear

### After:
1. User clicks "Open Chat" on notification
2. **Tab opens immediately with loading indicator**
3. Loading message: "Loading conversation... Fetching message history..."
4. Messages load and loading indicator disappears

## Benefits

1. **Instant Feedback**: Tab opens immediately, no perceived delay
2. **Clear Communication**: User knows the app is working, not frozen
3. **Better UX**: Loading state prevents confusion about empty chats
4. **Non-Blocking**: Backend session creation doesn't block UI rendering
5. **Smooth Transition**: Loading indicator smoothly transitions to loaded messages

## Testing

1. **Test New Matrix Chat**:
   - Click "Open Chat" on a notification for a room you haven't opened yet
   - ✅ Tab should open immediately with loading indicator
   - ✅ Loading message should be visible and centered
   - ✅ After 1-2 seconds, messages should load and loading indicator disappears

2. **Test Existing Matrix Chat**:
   - Click "Open Chat" on a notification for a room already open in a tab
   - ✅ Should switch to existing tab immediately (no loading state)

3. **Test Slow Network**:
   - Simulate slow network conditions
   - ✅ Loading indicator should remain visible until session is ready
   - ✅ User should never see an empty chat without explanation

## Technical Details

- **Loading State Prop**: `loadingChat: boolean` in `TabState`
- **Temporary Session ID**: `temp_matrix_${timestamp}` format
- **Session Update**: Uses `setTabStates` to update specific tab without re-creating
- **Loading Component**: `LoadingGoose` with custom message
- **Minimum Height**: `min-h-[400px]` ensures loading indicator is visible even in small windows
