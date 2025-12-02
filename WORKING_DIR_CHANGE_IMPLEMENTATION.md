# Working Directory Change Implementation

## Summary
Implemented the ability to change the working directory mid-session in the Goose desktop application without launching a new window. The solution involves restarting the agent when the working directory changes, which ensures that all MCP extensions (especially the developer extension) use the new working directory for shell commands.

## Problem
- The UI could update the session's working directory, but the agent's shell commands (`pwd`, etc.) continued to execute in the original directory
- This was because MCP extensions are long-running processes that inherit their working directory when spawned
- The MCP protocol doesn't support passing session-specific context per tool call

## Solution
Restart the agent when the working directory changes:
1. Update the session's working directory in the backend
2. Stop the current agent (shutting down MCP extensions)
3. Create a new agent (spawning new MCP extensions with the updated directory)
4. Restore the agent's provider, extensions, and recipe configuration

## Implementation Details

### Backend Changes

#### 1. New API Endpoint (`/agent/restart`)
- **File**: `crates/goose-server/src/routes/agent.rs`
- Added `RestartAgentRequest` struct
- Added `restart_agent` handler that:
  - Removes the existing agent from the session
  - Creates a new agent using the session's updated working directory
  - Restores provider and extensions
  - Reapplies any recipe configuration
- Refactored into helper functions to avoid clippy "too_many_lines" warning:
  - `restore_agent_provider`: Restores the LLM provider configuration
  - `restore_agent_extensions`: Reloads all enabled extensions

#### 2. OpenAPI Specification
- **File**: `crates/goose-server/src/openapi.rs`
- Added `restart_agent` path to the API documentation
- Added `RestartAgentRequest` schema

#### 3. Session Working Directory Update
- **File**: `crates/goose-server/src/routes/session.rs`
- Previously implemented endpoint `/sessions/{session_id}/working_dir` (PUT)
- Updates the session's `working_dir` field persistently

### Frontend Changes

#### 1. Directory Switcher Component
- **File**: `ui/desktop/src/components/bottom_menu/DirSwitcher.tsx`
- Modified to call the restart agent API after updating the working directory
- Workflow:
  1. User clicks on the directory display
  2. Directory chooser dialog opens
  3. On selection:
     - Calls `updateSessionWorkingDir` to update the backend session
     - Calls `restartAgent` to restart the agent with the new directory
     - Updates local state to reflect the change immediately
     - Shows success toast notification

#### 2. API Client
- **Files**: `ui/desktop/src/api/sdk.gen.ts`, `ui/desktop/src/api/types.gen.ts`
- Auto-generated from OpenAPI specification
- Added `restartAgent` function and `RestartAgentRequest` type

### MCP Extension Changes

#### 1. Shell Command Configuration
- **File**: `crates/goose-mcp/src/developer/shell.rs`
- Modified `configure_shell_command` to accept an optional `working_dir` parameter
- Sets the working directory for spawned shell processes

#### 2. Developer Extension
- **File**: `crates/goose-mcp/src/developer/rmcp_developer.rs`
- Modified to check for `GOOSE_SESSION_WORKING_DIR` environment variable
- Passes the working directory to `configure_shell_command` if set

## Testing

### Backend Test
- **File**: `crates/goose-server/tests/test_working_dir_update.rs`
- Unit test verifying that `SessionManager::update_session` correctly updates the working directory

### Manual Testing Steps
1. Open the Goose desktop application
2. Start a chat session
3. Run `pwd` command to see current directory
4. Click on the directory display in the bottom menu
5. Select a new directory
6. Run `pwd` again - it should now show the new directory

## Benefits
- Users can change directories mid-session without losing chat context
- No need to open new windows for different projects
- Agent's shell commands correctly reflect the new working directory
- All file operations use the updated directory

## Limitations
- The agent is briefly unavailable during the restart (typically < 1 second)
- Any in-flight operations when the directory changes will be interrupted
- The conversation history is preserved, but the agent's internal state is reset

## Future Improvements
1. Consider implementing a more graceful handoff that doesn't require full agent restart
2. Add a loading indicator during the agent restart
3. Queue any messages sent during the restart period
4. Investigate MCP protocol enhancements to support dynamic working directory changes
