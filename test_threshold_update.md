# Testing Auto-Compact Threshold Update

## Test Steps

1. **Start the application**
   ```bash
   just run-ui
   ```

2. **Open the chat interface**
   - Start a new chat session
   - Send a few messages to populate the context

3. **Test the threshold update**
   - Look at the bottom of the chat input where the context window indicator is shown
   - Click on the model selector dropdown to see the alerts
   - Find the "Auto summarize at X%" text with the pencil icon
   - Click the pencil icon to edit
   - Change the value (e.g., from 80% to 70%)
   - Click the save icon or press Enter
   - The threshold should save without reloading the page

4. **Verify the update**
   - The UI should immediately reflect the new threshold value
   - The threshold indicator dot on the progress bar should move to the new position
   - No page reload should occur
   - The session should remain intact with all messages preserved

## Expected Behavior

- ✅ Threshold value saves to backend via `/config/upsert` API
- ✅ UI updates immediately without page reload
- ✅ Custom event `autoCompactThresholdChanged` is dispatched
- ✅ ChatInput component receives the event and updates its local state
- ✅ Session remains intact - no restart or data loss

## Implementation Details

The solution uses a custom event system:
1. `AlertBox` dispatches `autoCompactThresholdChanged` event when saving
2. `ChatInput` listens for this event and updates its local state
3. No page reload is needed - the UI updates reactively

This approach avoids the complexity of prop drilling through the alert system while maintaining a smooth user experience.
