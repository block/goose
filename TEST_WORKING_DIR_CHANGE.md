# Testing Working Directory Change Feature

## Setup
1. Build the backend: `cargo build -p goose-server`
2. Build the frontend: `cd ui/desktop && npm run typecheck`
3. Start the application: `just run-ui`

## Test Steps

### 1. Initial Setup
1. Open the Goose desktop application
2. Start a new chat session
3. Note the current working directory displayed in the bottom menu

### 2. Verify Initial Working Directory
1. In the chat, type: `pwd`
2. Verify the output matches the displayed directory

### 3. Change Working Directory
1. Click on the directory path in the bottom menu
2. Select a different directory from the dialog
3. Wait for the success toast: "Working directory changed to [path] and agent restarted"

### 4. Verify Directory Change
1. In the chat, type: `pwd` again
2. **Expected Result**: The output should show the NEW directory path
3. Type: `ls` to list files
4. **Expected Result**: Files from the NEW directory should be listed

### 5. Check Logs (for debugging)
Open the browser developer console (Cmd+Option+I on Mac) and look for:
- `[DirSwitcher] Starting directory change process`
- `[DirSwitcher] New directory selected: "/path/to/new/dir"`
- `[DirSwitcher] Restarting agent to apply new working directory...`
- `[DirSwitcher] Agent restarted successfully`
- `[DirSwitcher] Working directory updated and agent restarted`

In the backend logs (terminal where goosed is running), look for:
- `=== UPDATE SESSION WORKING DIR START ===`
- `Session ID: [id]`
- `Requested working_dir: /path/to/new/dir`
- `Verification SUCCESS: Session [id] working_dir is now: "/path/to/new/dir"`
- `=== UPDATE SESSION WORKING DIR COMPLETE ===`
- `=== RESTART AGENT START ===`
- `Setting GOOSE_WORKING_DIR environment variable to: "/path/to/new/dir"`
- `Setting MCP process working directory from GOOSE_WORKING_DIR: "/path/to/new/dir"`
- `=== RESTART AGENT COMPLETE ===`

## What Was Fixed

### The Problem
- The UI could update the session's working directory
- But the agent's shell commands (`pwd`, etc.) continued to execute in the original directory
- This was because MCP extensions are long-running processes that inherit their working directory when spawned

### The Solution
1. **Backend API** (`/agent/restart`):
   - Stops the current agent (shutting down MCP extensions)
   - Sets `GOOSE_WORKING_DIR` environment variable
   - Creates a new agent with the updated working directory
   - Restores all configurations (provider, extensions, recipes)

2. **Frontend Integration**:
   - `DirSwitcher.tsx` calls both `updateSessionWorkingDir` and `restartAgent`
   - Shows success notification
   - Updates UI immediately without page reload

3. **MCP Extension Manager**:
   - Checks for `GOOSE_WORKING_DIR` environment variable
   - Sets it as the working directory for spawned MCP processes
   - Logs the directory being used

## Benefits
- Users can change directories mid-session without losing chat context
- No need to open new windows for different projects
- Agent's shell commands correctly reflect the new working directory
- All file operations use the updated directory

## Troubleshooting

If `pwd` still shows the old directory:
1. Check browser console for any error messages
2. Check backend logs for restart confirmation
3. Verify the directory exists and is accessible
4. Try changing to a simple path like `/tmp` to rule out permission issues

## Code Changes Summary
- `crates/goose-server/src/routes/agent.rs`: Added `restart_agent` endpoint
- `crates/goose-server/src/routes/session.rs`: Enhanced logging in `update_session_working_dir`
- `crates/goose/src/agents/extension_manager.rs`: Modified `child_process_client` to use `GOOSE_WORKING_DIR`
- `ui/desktop/src/components/bottom_menu/DirSwitcher.tsx`: Added agent restart after directory change
- `crates/goose-server/src/openapi.rs`: Added new endpoint to OpenAPI spec
