# Goose Web Interface - Cancel Button Implementation

## What's New

The `goose web` interface now has a **Cancel button** that works just like Ctrl+C in the CLI!

## How It Works

1. **During Processing**: When Goose is processing a request, the "Send" button changes to a red "Cancel" button
2. **Click to Cancel**: Click the Cancel button to stop the current operation
3. **Immediate Response**: The operation is cancelled and you'll see a "Operation cancelled" message
4. **Ready for Next**: The interface returns to normal state, ready for your next message

## Visual Changes

- **Send Button** (normal): Blue button that says "Send"
- **Cancel Button** (during processing): Red button that says "Cancel"
- **Automatic Reset**: Button returns to "Send" when operation completes or is cancelled

## Technical Details

### Backend
- Uses Tokio's `AbortHandle` to cancel async tasks
- Tracks active operations per session
- Handles cleanup when operations are cancelled

### Frontend
- Tracks processing state with `isProcessing` flag
- Changes button appearance and behavior based on state
- Sends cancel message through WebSocket

### Message Flow
1. User sends message → Button changes to "Cancel"
2. User clicks "Cancel" → Sends cancel message to server
3. Server aborts the task → Sends "cancelled" confirmation
4. UI shows cancellation message → Button returns to "Send"

## Usage

```bash
# Start the web server
goose web

# While Goose is processing:
# - Click the red "Cancel" button to stop the operation
# - Just like pressing Ctrl+C in the CLI!
```

## What Gets Cancelled

- Long-running tool executions
- Model responses being generated
- Any active processing for that session

The cancel functionality provides the same interruption capability as the CLI, making the web interface fully featured for interactive use!