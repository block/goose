# Matrix Message Routing Diagnostics

## Changes Made

We've added diagnostic logging to track the message sending flow from the UI to the backend. This will help identify where messages are being incorrectly routed.

### Files Modified

1. **`ui/desktop/src/components/BaseChat.tsx`**
   - Added logging in `handleSubmit()` to show `chat.id` and `chat.title` when a message is submitted

2. **`ui/desktop/src/hooks/useChatEngine.ts`**
   - Added logging in `handleSubmit()` to show `chat.id` when the engine processes the message
   - Added `chat.id` to the dependency array to ensure it's tracked

3. **`ui/desktop/src/hooks/useMessageStream.ts`**
   - Added logging in `sendRequest()` to show the `session_id` being sent to the backend API

## How to Test

### Step 1: Open Multiple Matrix Rooms
1. Open the Goose Desktop app
2. Navigate to Channels/Spaces
3. Open 2-3 different Matrix rooms in separate tabs
4. Note the room names for each tab

### Step 2: Send Messages and Check Logs
1. Open the browser DevTools console (View → Toggle Developer Tools)
2. In **Tab A** (e.g., "Room Alpha"), type a message: "Test message for Room Alpha"
3. Before sending, note which tab is active
4. Send the message
5. Check the console logs - you should see three log entries:

```
[Matrix Message Send - BaseChat] handleSubmit called: {
  chatId: "some_session_id",
  chatTitle: "Room Alpha",
  messagePreview: "Test message for Room Alpha...",
  timestamp: "2025-12-03T..."
}

[Matrix Message Send - useChatEngine] Sending message: {
  chatId: "some_session_id",
  messagePreview: "Test message for Room Alpha...",
  timestamp: "2025-12-03T..."
}

[Matrix Message Send - useMessageStream] Sending to backend: {
  api: "/api/chat/reply",
  session_id: "some_session_id",
  messageCount: 1,
  timestamp: "2025-12-03T..."
}
```

6. **Verify the `chatId` / `session_id` is consistent across all three logs**
7. **Check which Matrix room the message actually appeared in**
8. Repeat for Tab B and Tab C

### Step 3: Analyze the Results

#### If messages are going to the wrong room:

**Check 1: Is the `session_id` the same for different rooms?**
- If YES: The problem is that multiple Matrix rooms are sharing the same session ID
- If NO: The problem is in the backend routing (session_id → Matrix room mapping)

**Check 2: Does the `chatId` match the room you're sending from?**
- Look at the `chatTitle` in the first log
- Compare with the tab you clicked in
- If they don't match, the active tab state is incorrect

**Check 3: Does the `session_id` change when you switch tabs?**
- Send a message from Tab A, note the `session_id`
- Switch to Tab B, send a message, note the `session_id`
- They should be DIFFERENT
- If they're the same, all tabs are sharing one session

### Step 4: Document Your Findings

Create a log file with your observations:

```
## Test Results - [Date/Time]

### Tab A: Room Alpha (!roomA:matrix.org)
- chatId: abc123
- session_id: abc123
- Message sent: "Test for Alpha"
- Message appeared in: [Which room?]

### Tab B: Room Beta (!roomB:matrix.org)
- chatId: def456
- session_id: def456
- Message sent: "Test for Beta"
- Message appeared in: [Which room?]

### Tab C: Room Gamma (!roomC:matrix.org)
- chatId: ghi789
- session_id: ghi789
- Message sent: "Test for Gamma"
- Message appeared in: [Which room?]

### Issues Found:
- [ ] Multiple rooms share the same session_id
- [ ] session_id doesn't match chatId
- [ ] chatTitle doesn't match active tab
- [ ] Messages appear in wrong rooms
- [ ] Other: _______________
```

## Expected Behavior

For correct routing, you should see:
1. **Each Matrix room tab has a unique `session_id`**
2. **The `session_id` is stable** (doesn't change when you switch tabs)
3. **The `session_id` matches across all three log points** (BaseChat → useChatEngine → useMessageStream)
4. **Messages appear in the correct Matrix room** (the one whose tab you sent from)

## Common Issues and Solutions

### Issue 1: All tabs share the same `session_id`
**Symptom:** All three log entries show the same `session_id` regardless of which tab you're in

**Cause:** The `chat` object is being shared across tabs, or tabs aren't creating unique sessions

**Solution:** Ensure each Matrix room creates its own unique session when opened. Check where Matrix rooms are opened (likely in `SpaceRoomsView` or similar) and verify session creation.

### Issue 2: `session_id` changes when switching tabs
**Symptom:** Sending from Tab A shows `session_id: abc123`, but switching to Tab A again later shows `session_id: xyz789`

**Cause:** The session isn't being persisted or the tab state is being reset

**Solution:** Check tab state management and ensure sessions persist across tab switches.

### Issue 3: `chatTitle` doesn't match the active tab
**Symptom:** You're in "Room Alpha" tab but logs show `chatTitle: "Room Beta"`

**Cause:** The `chat` object being passed to BaseChat is from the wrong tab

**Solution:** Check the tab container component and ensure it's passing the correct `chat` object for the active tab.

### Issue 4: `session_id` is correct but messages go to wrong room
**Symptom:** Logs show unique, consistent `session_id` values, but messages still appear in wrong rooms

**Cause:** Backend mapping between `session_id` and Matrix `room_id` is incorrect

**Solution:** This is a backend issue. Check:
- Session mapping service/database
- Backend message routing logic
- Matrix SDK integration

## Next Steps

Once you've identified the issue using these diagnostics:

1. **If it's a frontend issue** (session_id sharing or inconsistency):
   - Look at how Matrix rooms create/load sessions
   - Check tab state management
   - Verify `chat` object creation and passing

2. **If it's a backend issue** (routing with correct session_id):
   - Check backend logs for session → room mapping
   - Verify Matrix SDK message sending
   - Check database/state for session mappings

3. **Document your findings** and share the log output for further debugging

## Cleanup

Once the issue is fixed, you can remove the diagnostic logging by searching for:
```
[Matrix Message Send
```

And removing those `console.log()` statements from the three files mentioned above.
