# Working Directory Handling Analysis Report

## Executive Summary

This report analyzes the evolution of working directory handling in the Goose codebase, comparing the old implementation (PR #6057, commit 9a01fcb7404) with the new implementation (commit aa356bd460f). The new approach provides per-session working directory isolation through MCP request metadata, making the old environment variable-based approach obsolete.

---

## 1. Architecture Comparison

### 1.1 OLD WAY (PR #6057 - January 8, 2026)

The original implementation used process-level environment variables and working directories:

**Flow:**
1. `Agent::add_extension` receives `working_dir` parameter (line 749)
2. Passes it to `ExtensionManager::add_extension` (line 780)
3. `add_extension` stores it as `effective_working_dir` (line 489-490)
4. For Stdio/InlinePython/Builtin extensions, passes it to `child_process_client` (lines 560, 596, 650)
5. `child_process_client` function:
   - Falls back to `GOOSE_WORKING_DIR` env var if not provided (line 220)
   - Sets the MCP process working directory via `command.current_dir(dir)` (line 225)
   - Sets `GOOSE_WORKING_DIR` environment variable for the child process (line 227)
6. MCP servers (e.g., rmcp_developer) read from `std::env::var("GOOSE_WORKING_DIR")` to determine working directory

**Key Code Locations (OLD WAY):**
- `/Users/douwe/proj/wt-goose-a/crates/goose/src/agents/agent.rs:749` - Extracts working_dir from session
- `/Users/douwe/proj/wt-goose-a/crates/goose/src/agents/extension_manager.rs:217-227` - Fallback and env var setting
- `/Users/douwe/proj/wt-goose-a/crates/goose-mcp/src/developer/shell.rs` - Old implementation used `GOOSE_WORKING_DIR` env var

### 1.2 NEW WAY (Commit aa356bd - February 3, 2026)

The new implementation uses MCP request metadata for per-call working directory context:

**Flow:**
1. `Agent::add_extension` still receives `working_dir` from session (line 749)
2. Passes it to `ExtensionManager::add_extension` (line 780)
3. `add_extension` uses it ONLY for setting the process working directory (lines 560, 596, 650)
4. During tool execution, `Agent` calls `dispatch_tool_call` with working_dir (line 584)
5. `ExtensionManager::dispatch_tool_call`:
   - Receives `working_dir: Option<&std::path::Path>` parameter (line 1182)
   - Converts to string and passes to `client.call_tool` (lines 1242, 1257)
6. `McpClient::call_tool`:
   - Receives `working_dir: Option<&str>` parameter (line 54, 551)
   - Calls `send_request_with_context` with working_dir (line 567)
7. `inject_session_context_into_request`:
   - Injects working_dir into MCP request Extensions._meta under "agent-working-dir" header (lines 670-675)
8. MCP servers extract from request metadata:
   - `extract_working_dir_from_meta(&context.meta)` (line 889 in rmcp_developer.rs)
   - Uses it for the specific tool call (line 904, 988)

**Key Code Locations (NEW WAY):**
- `/Users/douwe/proj/wt-goose-a/crates/goose/src/session_context.rs:4` - `WORKING_DIR_HEADER` constant
- `/Users/douwe/proj/wt-goose-a/crates/goose/src/agents/mcp_client.rs:646-679` - Metadata injection
- `/Users/douwe/proj/wt-goose-a/crates/goose-mcp/src/developer/rmcp_developer.rs:21-31` - Metadata extraction
- `/Users/douwe/proj/wt-goose-a/crates/goose/src/agents/extension_manager.rs:1178-1273` - dispatch_tool_call with working_dir

---

## 2. Benefits of the New Approach

### 2.1 Per-Session Isolation
- **OLD**: Process-level environment variable, potentially shared across sessions
- **NEW**: Per-request metadata, complete isolation between concurrent sessions

### 2.2 Cleaner Architecture
- **OLD**: Relies on environment variable inheritance, side effects
- **NEW**: Explicit parameter passing through MCP protocol

### 2.3 Docker Compatibility
- **OLD**: Environment variables set at process spawn time
- **NEW**: Working directory can be specified per-call, more flexible for containerized extensions

### 2.4 Testability
- **OLD**: Requires setting process environment variables in tests
- **NEW**: Simple parameter passing, easier to mock and test

---

## 3. Code That Can Be Removed/Simplified

### 3.1 HIGH PRIORITY: Remove GOOSE_WORKING_DIR Fallback

**File:** `/Users/douwe/proj/wt-goose-a/crates/goose/src/agents/extension_manager.rs`

**Lines 217-227:** The fallback to `GOOSE_WORKING_DIR` environment variable is now obsolete:

```rust
// REMOVE THIS SECTION (Lines 217-227):
    // Use explicitly passed working_dir, falling back to GOOSE_WORKING_DIR env var
    let effective_working_dir = working_dir
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::var("GOOSE_WORKING_DIR").ok().map(PathBuf::from));

    if let Some(ref dir) = effective_working_dir {
        if dir.exists() && dir.is_dir() {
            tracing::info!("Setting MCP process working directory: {:?}", dir);
            command.current_dir(dir);
            // Also set GOOSE_WORKING_DIR env var for the child process
            command.env("GOOSE_WORKING_DIR", dir);
```

**REPLACE WITH:**
```rust
    // Use explicitly passed working_dir for MCP process startup
    if let Some(dir) = working_dir {
        if dir.exists() && dir.is_dir() {
            tracing::info!("Setting MCP process working directory: {:?}", dir);
            command.current_dir(dir);
        } else {
            tracing::warn!(
                "Working directory doesn't exist or isn't a directory: {:?}",
                dir
            );
        }
    } else {
        tracing::info!("No working directory specified, using default");
    }
```

**Rationale:**
- The `GOOSE_WORKING_DIR` environment variable is no longer needed since working_dir is passed through MCP metadata
- The `.env("GOOSE_WORKING_DIR", dir)` call on line 227 is unnecessary
- The fallback logic on line 220 is obsolete

### 3.2 MEDIUM PRIORITY: Update Memory Extension

**File:** `/Users/douwe/proj/wt-goose-a/crates/goose-mcp/src/memory/mod.rs`

**Lines 185-187:** The memory extension still reads from `GOOSE_WORKING_DIR`:

```rust
// CURRENT CODE (Lines 185-187):
        let local_memory_dir = std::env::var("GOOSE_WORKING_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap())
            .join(".goose")
            .join("memory");
```

**ISSUE:** Memory is a builtin extension that runs in-process (see `/Users/douwe/proj/wt-goose-a/crates/goose-mcp/src/lib.rs:59`). Unlike external MCP servers, it doesn't receive request context metadata.

**OPTIONS:**

**Option A - Extract from request metadata (RECOMMENDED):**
If memory tools need working_dir, add a helper function similar to rmcp_developer:

```rust
// In memory/mod.rs, add at the top:
const WORKING_DIR_HEADER: &str = "agent-working-dir";

fn extract_working_dir_from_meta(meta: &Meta) -> Option<PathBuf> {
    meta.0
        .get(WORKING_DIR_HEADER)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

// Then in tool handlers that need working_dir:
let working_dir = extract_working_dir_from_meta(&context.meta)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
let local_memory_dir = working_dir.join(".goose").join("memory");
```

**Option B - Remove GOOSE_WORKING_DIR fallback (SIMPLER):**
If memory doesn't need session-specific working directories:

```rust
// SIMPLIFIED CODE:
let local_memory_dir = std::env::current_dir()
    .unwrap_or_else(|_| PathBuf::from("."))
    .join(".goose")
    .join("memory");
```

**Recommendation:** Implement Option A only if memory operations need to be session-aware. Otherwise, use Option B.

### 3.3 LOW PRIORITY: UI Code References

**File:** `/Users/douwe/proj/wt-goose-a/ui/desktop/src/utils/workingDir.ts`

**Lines 1-4:** Already cleaned up in commit aa356bd:

```typescript
// ALREADY SIMPLIFIED:
export const getInitialWorkingDir = (): string => {
  // Fall back to initial config from app startup
  return (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';
};
```

**No action needed** - this is appropriate for initial window setup.

**File:** `/Users/douwe/proj/wt-goose-a/ui/desktop/src/components/bottom_menu/DirSwitcher.tsx`

**Lines 46:** Already cleaned up in commit aa356bd - removed `setCurrentWorkingDir(newDir)` call.

**No action needed** - cleanup already complete.

---

## 4. Detailed Code Flow Analysis

### 4.1 Current State: Extension Startup

**Scenario:** Adding a Stdio extension

1. **Agent::add_extension** (`crates/goose/src/agents/agent.rs:733-792`)
   - Gets session from session_manager (lines 738-748)
   - Extracts `working_dir = Some(session.working_dir)` (line 749)
   - Calls `extension_manager.add_extension(extension, working_dir, container)` (line 780)

2. **ExtensionManager::add_extension** (`crates/goose/src/agents/extension_manager.rs:475-681`)
   - Resolves `effective_working_dir` (lines 488-490)
   - For Stdio extensions (lines 520-564):
     - Builds command with args and envs
     - Calls `child_process_client(command, timeout, provider, Some(&effective_working_dir), container)` (line 556-562)

3. **child_process_client** (`crates/goose/src/agents/extension_manager.rs:202-269`)
   - **ISSUE**: Lines 217-227 implement the OLD WAY
   - Falls back to `GOOSE_WORKING_DIR` env var (line 220)
   - Sets process working directory (line 225)
   - Sets `GOOSE_WORKING_DIR` env var for child process (line 227)
   - Spawns the MCP server process

### 4.2 Current State: Tool Execution

**Scenario:** Agent executes a Bash tool

1. **Agent::execute_tool** (`crates/goose/src/agents/agent.rs:575-587`)
   - Calls `extension_manager.dispatch_tool_call(&session.id, tool_call, Some(session.working_dir.as_path()), cancellation_token)` (line 581-586)

2. **ExtensionManager::dispatch_tool_call** (`crates/goose/src/agents/extension_manager.rs:1178-1273`)
   - Receives `working_dir: Option<&std::path::Path>` (line 1182)
   - Converts to string: `working_dir_str = working_dir.map(|p| p.to_string_lossy().to_string())` (line 1242)
   - Calls `client.call_tool(&session_id, &tool_name, arguments, working_dir_str.as_deref(), cancellation_token)` (lines 1253-1259)

3. **McpClient::call_tool** (`crates/goose/src/agents/mcp_client.rs:546-574`)
   - Creates CallToolRequest with parameters
   - Calls `send_request_with_context(session_id, working_dir, request, cancel_token)` (line 567)

4. **send_request_with_context** (`crates/goose/src/agents/mcp_client.rs:390-405`)
   - Calls `inject_session_context_into_request(request, Some(session_id), working_dir)` (line 396)

5. **inject_session_context_into_request** (`crates/goose/src/agents/mcp_client.rs:681-719`)
   - For CallToolRequest, calls `inject_session_context_into_extensions(req.extensions, session_id, working_dir)` (lines 702-704)

6. **inject_session_context_into_extensions** (`crates/goose/src/agents/mcp_client.rs:646-679`)
   - Inserts working_dir into meta_map under `WORKING_DIR_HEADER` (lines 670-675)
   - Returns updated extensions

7. **MCP Server Receives Request** (`crates/goose-mcp/src/developer/rmcp_developer.rs`)
   - Tool handler receives RequestContext with meta field (line 889)
   - Calls `extract_working_dir_from_meta(&context.meta)` (line 889)
   - Passes working_dir to `execute_shell_command` (line 904)

8. **execute_shell_command** (`crates/goose-mcp/src/developer/rmcp_developer.rs:983-1005`)
   - Receives `working_dir: Option<PathBuf>` (line 988)
   - Calls `configure_shell_command(&shell_config, command, working_dir.as_deref())` (line 1005)

9. **configure_shell_command** (`crates/goose-mcp/src/developer/shell.rs:108-118`)
   - If working_dir provided, sets `command_builder.current_dir(dir)` (lines 115-117)

---

## 5. Recommendations

### 5.1 Immediate Actions (High Priority)

1. **Remove GOOSE_WORKING_DIR Environment Variable Logic**
   - File: `crates/goose/src/agents/extension_manager.rs:217-227`
   - Remove fallback to `std::env::var("GOOSE_WORKING_DIR")`
   - Remove `command.env("GOOSE_WORKING_DIR", dir)` call
   - Simplify to only use explicitly passed working_dir parameter
   - **Impact**: Low risk - the new metadata-based approach is already active
   - **Test**: Verify that MCP tools still receive correct working directory through metadata

2. **Document the New Approach**
   - Add code comments explaining the metadata-based approach
   - Update any developer documentation about working directory handling
   - Document the `WORKING_DIR_HEADER` constant and its usage pattern

### 5.2 Short-term Actions (Medium Priority)

3. **Update Memory Extension**
   - File: `crates/goose-mcp/src/memory/mod.rs:185-187`
   - Determine if memory needs session-specific working directories
   - If yes: Implement metadata extraction like rmcp_developer
   - If no: Remove `GOOSE_WORKING_DIR` fallback, use `std::env::current_dir()` only
   - **Impact**: Medium - affects where memory files are stored
   - **Test**: Verify memory operations work correctly across multiple sessions

4. **Audit Other Builtin Extensions**
   - Check if other builtin extensions (autovisualiser, computercontroller, tutorial) need working_dir
   - If they do, implement the same metadata extraction pattern
   - Ensure consistency across all extensions

### 5.3 Long-term Considerations (Low Priority)

5. **Consider Removing working_dir from add_extension**
   - Currently `add_extension` sets the process working directory at spawn time
   - With per-call metadata, this may be unnecessary
   - **Risk**: Process startup working directory might matter for initialization
   - **Recommendation**: Keep for now, but document that runtime working_dir comes from metadata

6. **Enhance Error Handling**
   - Add validation for working_dir paths before injection into metadata
   - Log warnings when working_dir is not provided or invalid
   - Consider falling back to session working_dir in dispatch_tool_call if needed

7. **Performance Optimization**
   - The string conversion on line 1242 happens for every tool call
   - Consider caching or optimizing if this becomes a bottleneck

---

## 6. Potential Issues and Edge Cases

### 6.1 Builtin Extensions Without Metadata Support

**Issue:** Builtin extensions that don't use rmcp may not have access to request context metadata.

**Affected Extensions:**
- Memory (confirmed - line 185 in memory/mod.rs)
- Potentially others in goose-mcp/src/

**Mitigation:**
- Audit all builtin extensions for working_dir usage
- Implement metadata extraction where needed
- Document the pattern for future extension developers

### 6.2 Backward Compatibility

**Issue:** External MCP servers might still expect `GOOSE_WORKING_DIR` environment variable.

**Risk Assessment:** Low
- External servers are independent processes
- If they need working_dir, they should read from metadata
- Setting process working dir at spawn time (line 225) still works

**Mitigation:**
- Document the new metadata-based approach for extension developers
- Keep backward compatibility for a deprecation period if needed
- Remove env var setting only after confirming no extensions depend on it

### 6.3 Docker Container Extensions

**Issue:** Docker containers spawn in a different environment.

**Current State:**
- Lines 533-554: Docker exec commands don't set working_dir
- Lines 574-600: Builtin extensions in Docker call child_process_client

**Recommendation:**
- Verify Docker container extensions receive working_dir correctly
- Test with `docker exec` to ensure working directory propagation

### 6.4 Multiple Sessions in Same Process

**Issue:** In-process builtin extensions running multiple sessions concurrently.

**Current State:**
- Old way: `std::env::set_var("GOOSE_WORKING_DIR")` was removed (see commit aa356bd, no longer in codebase)
- New way: Each request has its own metadata

**Status:** ✅ RESOLVED
- The commit aa356bd specifically removed the problematic `std::env::set_var` call
- Per-request metadata ensures proper isolation
- No action needed

### 6.5 Tool Calls from Server Routes

**Issue:** `goose-server/src/routes/agent.rs:947-955` calls dispatch_tool_call with `working_dir: None`.

**Risk Assessment:** Medium
- This is for direct API calls to tools
- Without working_dir, tools may use incorrect paths

**Recommendation:**
- Investigate if server route tool calls should include working_dir
- If yes, extract from session context or request parameters
- If no, document why it's intentionally None

---

## 7. Testing Recommendations

### 7.1 Unit Tests

1. **Test metadata injection:**
   - Verify `inject_session_context_into_extensions` adds WORKING_DIR_HEADER correctly
   - Test with None, Some(""), and Some("/valid/path")
   - Verify existing session-id tests still pass

2. **Test metadata extraction:**
   - Verify `extract_working_dir_from_meta` handles all cases
   - Test with missing header, empty string, valid path

3. **Test dispatch_tool_call:**
   - Mock McpClient and verify working_dir is passed correctly
   - Test with None and Some(path) working_dir values

### 7.2 Integration Tests

1. **Multi-session isolation:**
   - Create two sessions with different working directories
   - Execute tools concurrently
   - Verify each tool uses the correct working directory

2. **Extension types:**
   - Test with Stdio extension (external process)
   - Test with Builtin extension (in-process)
   - Test with Docker container extension

3. **Real MCP tools:**
   - Test Bash tool with different working directories
   - Verify file operations use correct paths
   - Test with relative and absolute paths

### 7.3 Regression Tests

1. **Verify no dependency on GOOSE_WORKING_DIR env var:**
   - Unset environment variable in tests
   - Verify tools still work correctly
   - Confirm working_dir comes from metadata only

2. **Backward compatibility:**
   - Test with older session data
   - Verify graceful handling of missing working_dir

---

## 8. Migration Checklist

- [ ] **Code Changes**
  - [ ] Remove GOOSE_WORKING_DIR fallback in extension_manager.rs (lines 217-227)
  - [ ] Update memory extension to use metadata or current_dir (memory/mod.rs:185-187)
  - [ ] Audit other builtin extensions for working_dir usage
  - [ ] Review server routes for working_dir in dispatch_tool_call

- [ ] **Testing**
  - [ ] Add unit tests for metadata injection/extraction
  - [ ] Add integration tests for multi-session isolation
  - [ ] Test all extension types (Stdio, Builtin, Docker)
  - [ ] Verify real MCP tools (Bash, etc.) work correctly

- [ ] **Documentation**
  - [ ] Document the new metadata-based approach
  - [ ] Add code comments explaining WORKING_DIR_HEADER usage
  - [ ] Update extension developer guide
  - [ ] Document any deprecations

- [ ] **Validation**
  - [ ] Run full test suite
  - [ ] Manual testing with CLI
  - [ ] Manual testing with UI (Desktop app)
  - [ ] Test with multiple concurrent sessions

- [ ] **Deployment**
  - [ ] Code review
  - [ ] Create PR with detailed description
  - [ ] Monitor for issues post-deployment
  - [ ] Update changelog

---

## 9. Conclusion

The new metadata-based working directory approach (commit aa356bd) provides significant improvements over the environment variable approach (PR #6057):

- **Better isolation**: Per-request metadata vs. process-level env vars
- **Cleaner architecture**: Explicit parameter passing
- **More flexible**: Working directory can change per tool call
- **Easier to test**: No global state modifications

**Key cleanup actions:**
1. Remove GOOSE_WORKING_DIR fallback in extension_manager.rs (HIGH PRIORITY)
2. Update memory extension (MEDIUM PRIORITY)
3. Audit other builtin extensions (MEDIUM PRIORITY)

**Risk level:** LOW - The new approach is already implemented and working. Cleanup removes obsolete code without changing behavior.

**Next steps:** Follow the migration checklist in Section 8, starting with high-priority code changes and their associated tests.

---

## Appendix A: File Reference

### Modified Files (commit aa356bd)
- `crates/goose-cli/src/scenario_tests/mock_client.rs` - Added working_dir parameter to call_tool
- `crates/goose-mcp/src/developer/rmcp_developer.rs` - Added metadata extraction
- `crates/goose-server/src/routes/agent.rs` - Updated dispatch_tool_call signature
- `crates/goose/src/agents/agent.rs` - Pass working_dir to dispatch_tool_call
- `crates/goose/src/agents/extension_manager.rs` - Updated dispatch_tool_call signature
- `crates/goose/src/agents/mcp_client.rs` - Added working_dir to call_tool and metadata injection
- `crates/goose/src/session_context.rs` - Added WORKING_DIR_HEADER constant
- `ui/desktop/src/utils/workingDir.ts` - Removed runtime working_dir tracking

### Files Needing Cleanup
- `crates/goose/src/agents/extension_manager.rs:217-227` - Remove GOOSE_WORKING_DIR logic
- `crates/goose-mcp/src/memory/mod.rs:185-187` - Update or remove GOOSE_WORKING_DIR usage

### Reference Commits
- **NEW**: aa356bd460fa039d3babf8b012d554f1df88075e - "fix: per session working dir isolation"
- **OLD**: 9a01fcb74048b3b61f4657b87f8aecdeb58924f4 - "Add support for changing working dir and extensions"

---

*Report generated: 2026-02-04*
*Analyzed commits: aa356bd (new) vs 9a01fcb7404 (old)*
*Codebase: /Users/douwe/proj/wt-goose-a*
