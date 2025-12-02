# Working Directory Update Limitation

## Current Status

The frontend successfully updates the session's working directory in the backend session manager, and the UI reflects this change. However, shell commands executed through the developer extension still use the original working directory.

## Technical Details

The issue stems from the architecture of the MCP (Model Context Protocol) extensions:

1. **MCP extensions run as separate processes**: The developer extension is started once and reused across all sessions and working directory changes.

2. **No per-request context**: The MCP protocol doesn't support passing session-specific metadata (like working directory) with each tool call.

3. **Environment variables are process-wide**: We can't dynamically change environment variables per request in a multi-session environment.

## Implementation Attempted

We've implemented the following:

### Frontend (TypeScript/React)
- `DirSwitcher.tsx`: Updates the session's working directory via the API without reloading the window
- Maintains local state to reflect the current directory in the UI
- Successfully calls the backend API to update the session

### Backend (Rust)
- `session.rs`: Added `update_session_working_dir` endpoint that updates the session's working_dir field
- Session manager correctly stores and retrieves the updated working directory

### Developer Extension (Rust/MCP)
- `rmcp_developer.rs`: Checks for `GOOSE_SESSION_WORKING_DIR` environment variable
- `shell.rs`: `configure_shell_command` accepts an optional working directory parameter

## The Gap

The missing piece is passing the session's current working directory to the developer extension when executing shell commands. Since:
- The MCP protocol doesn't support per-request metadata
- The developer extension is a long-running process serving multiple sessions
- Environment variables can't be changed dynamically per request

## Workarounds

Until a proper solution is implemented, users can:

1. **Use absolute paths**: When the working directory changes, use absolute paths in shell commands
2. **Prefix commands with cd**: Start shell commands with `cd /new/working/dir && ...`
3. **Restart the session**: Create a new session with the desired working directory

## Potential Solutions

1. **Modify MCP protocol**: Extend the protocol to support session metadata in tool calls (requires upstream changes)
2. **Per-session MCP processes**: Spawn a new developer extension for each session (resource intensive)
3. **Proxy layer**: Add a proxy between the agent and MCP that injects session context (complex)
4. **Tool parameter**: Add an optional `working_dir` parameter to the shell tool (breaks compatibility)

## Files Modified

- `ui/desktop/src/components/bottom_menu/DirSwitcher.tsx`
- `ui/desktop/src/main.ts`
- `ui/desktop/src/preload.ts`
- `crates/goose-server/src/routes/session.rs`
- `crates/goose-server/src/openapi.rs`
- `crates/goose-mcp/src/developer/rmcp_developer.rs`
- `crates/goose-mcp/src/developer/shell.rs`
- `crates/goose-server/tests/test_working_dir_update.rs`
