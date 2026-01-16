# Testing Guide: Session Name UI Sync Fix

This guide will help you test the session name UI synchronization fix on your local Goose Desktop app.

## Prerequisites

Make sure you have:
- Rust and Cargo installed
- Node.js and npm installed (via nvm recommended)
- Hermit activated (if using): `source ./bin/activate-hermit`

## Option 1: Quick Test (Recommended)

This uses the `just` command to build and run everything:

```bash
cd ~/Development/goose-session-name-fix
just run-ui
```

This will:
1. Build the Rust backend in release mode
2. Install npm dependencies
3. Generate the API client from OpenAPI spec
4. Start the Electron app

## Option 2: Manual Build

If you prefer to build step-by-step or don't have `just` installed:

### Step 1: Build the Rust Backend

```bash
cd ~/Development/goose-session-name-fix
cargo build --release -p goose-server
```

### Step 2: Start the UI

```bash
cd ui/desktop
npm install
npm run start-gui
```

## Option 3: Debug Mode (For Development)

If you want to debug the server separately:

### Terminal 1: Start the Rust server

```bash
cd ~/Development/goose-session-name-fix
export GOOSE_SERVER__SECRET_KEY=test
cargo run --package goose-server --bin goosed -- agent
```

The server will start on port 3000.

### Terminal 2: Start the UI in debug mode

```bash
cd ~/Development/goose-session-name-fix
just debug-ui
```

This connects the UI to your locally running server, allowing you to set breakpoints in your IDE.

## Testing the Fix

Once the app is running, follow these steps to verify the fix works:

### Test Case 1: New Session Auto-Naming

1. **Start a new session** in Goose Desktop
2. **Check the window title** - it should show "New session 1" (or similar number)
3. **Send your first message** - e.g., "Help me debug a Python script"
4. **Wait for the response to complete**
5. **Send your second message** - e.g., "What are common debugging techniques?"
6. **Wait for the response to complete**
7. **Send your third message** - e.g., "Show me an example"
8. **Wait for the response to complete**
9. **Check the window title** - it should now show a descriptive name like "Python debugging assistance" or similar

### Expected Behavior

✅ **Before this fix**: Window title stays "New session 1" until you close and reopen the session

✅ **After this fix**: Window title updates automatically after the 3rd message completes

### Test Case 2: Verify No Ongoing Overhead

1. **Continue the conversation** from Test Case 1
2. **Send 4-5 more messages**
3. **Open Developer Tools** (View → Toggle Developer Tools)
4. **Go to Network tab**
5. **Send another message**
6. **Verify**: You should NOT see any `GET /sessions/{session_id}` calls after the name has been updated

This confirms the optimization is working - it stops checking once the name is set.

### Test Case 3: Multiple Sessions

1. **Start Session 1** and send 3 messages (wait for name to update)
2. **Start Session 2** (File → New Session or Cmd+N)
3. **Switch between sessions** using the sidebar
4. **Verify**: Each session shows its own descriptive name in the window title

## Debugging Tips

### If the app doesn't start:

```bash
# Clean build
cd ~/Development/goose-session-name-fix
cargo clean
cd ui/desktop
rm -rf node_modules
npm install
just run-ui
```

### If you see "New session X" after 3 messages:

1. Open Developer Tools (View → Toggle Developer Tools)
2. Check the Console tab for errors
3. Check the Network tab - look for `GET /sessions/{session_id}` calls
4. Verify the response includes a `name` field that's different from "New session X"

### View the session in the database:

```bash
sqlite3 ~/.local/share/goose/sessions/sessions.db \
  "SELECT id, name, user_set_name FROM sessions ORDER BY updated_at DESC LIMIT 5;"
```

This shows you what the backend actually stored.

## Comparing with Production

To compare the behavior with the current production version:

1. **Test with production Goose** (your installed app)
   - Start a session, send 3 messages
   - Note: Window title stays "New session X"
   
2. **Test with your build** (from this branch)
   - Start a session, send 3 messages
   - Note: Window title updates automatically

## Performance Verification

To verify the performance claims in the PR:

1. **Open Developer Tools** → Network tab
2. **Filter for**: `/sessions/`
3. **Start a new session** and send 3 messages
4. **Count the `GET /sessions/{session_id}` calls** - should be 3-4 maximum
5. **Send 5 more messages** - should be 0 additional calls
6. **Check response size** - should be ~500 bytes per call

## Cleanup

When you're done testing, you can:

```bash
# Stop the app (Cmd+Q or close the window)

# Optional: Clean up test data
rm -rf ~/.local/share/goose/sessions/sessions.db
```

## Troubleshooting

### Port already in use

If you see "port 3000 already in use":

```bash
# Find and kill the process
lsof -ti:3000 | xargs kill -9
```

### TypeScript errors

If you see TypeScript compilation errors:

```bash
cd ~/Development/goose-session-name-fix/ui/desktop
npm run generate-api
npm run typecheck
```

### Backend crashes

Check the logs in the terminal where you ran the server. Common issues:
- Missing environment variables
- Database locked (close other Goose instances)
- Port conflicts

## Need Help?

If you run into issues:
1. Check the [CONTRIBUTING.md](./CONTRIBUTING.md) guide
2. Look at existing issues on GitHub
3. Ask on the Goose Discord

## Next Steps

Once you've verified the fix works:
1. Review the code changes in `ui/desktop/src/hooks/useChatStream.ts`
2. Review the PR description in `PR_DESCRIPTION.md`
3. When ready, create the PR using the instructions in the main README
