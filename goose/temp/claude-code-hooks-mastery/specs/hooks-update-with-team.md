# Plan: Update Claude Code Hooks Mastery Codebase

## Task Description
Update the claude-code-hooks-mastery codebase to support all the latest Claude Code hooks, update Python implementations to include latest versions and input fields, add 5 new status line versions, and update documentation. The update includes creating new hooks for `SessionEnd`, `PermissionRequest`, `PostToolUseFailure`, `SubagentStart`, and `Setup`, while ensuring all existing hooks are updated with the latest JSON input fields. Validation will be performed using `claude -p '<prompt>'` commands with log file verification.

## Objective
Bring the hooks codebase to full parity with the latest Claude Code hooks documentation, including:
1. All hook types implemented with latest input schemas
2. 5 new creative status line implementations
3. Updated settings.json with all hook configurations
4. Comprehensive README.md documentation
5. Validated hooks through automated testing

## Problem Statement
The current codebase is missing several hook types documented in the latest Claude Code hooks specification:
- **SessionEnd** - Fires when session terminates
- **PermissionRequest** - Fires when permission dialog appears
- **PostToolUseFailure** - Fires after tool fails
- **SubagentStart** - Fires when spawning a subagent
- **Setup** - Fires with `--init`, `--init-only`, or `--maintenance` flags

Additionally, existing hooks may not include all the latest input fields from the documentation.

## Solution Approach
Deploy specialized builder/validator agent pairs for each hook type, plus dedicated agents for status lines and documentation. Each pair follows a build-then-validate pattern to ensure quality.

## Relevant Files
Use these files to complete the task:

### Documentation (Read for Reference)
- `ai_docs/claude_code_hooks_docs.md` - Complete hooks reference documentation
- `ai_docs/claude_code_hooks_getting_started.md` - Hooks quickstart guide
- `ai_docs/claude_code_status_lines_docs.md` - Status line configuration docs

### Existing Hooks (Update)
- `.claude/hooks/pre_tool_use.py` - PreToolUse hook implementation
- `.claude/hooks/post_tool_use.py` - PostToolUse hook implementation
- `.claude/hooks/stop.py` - Stop hook implementation
- `.claude/hooks/subagent_stop.py` - SubagentStop hook implementation
- `.claude/hooks/notification.py` - Notification hook implementation
- `.claude/hooks/session_start.py` - SessionStart hook implementation
- `.claude/hooks/user_prompt_submit.py` - UserPromptSubmit hook implementation
- `.claude/hooks/pre_compact.py` - PreCompact hook implementation

### New Files to Create
- `.claude/hooks/session_end.py` - NEW: SessionEnd hook
- `.claude/hooks/permission_request.py` - NEW: PermissionRequest hook
- `.claude/hooks/post_tool_use_failure.py` - NEW: PostToolUseFailure hook
- `.claude/hooks/subagent_start.py` - NEW: SubagentStart hook
- `.claude/hooks/setup.py` - NEW: Setup hook
- `.claude/status_lines/status_line_v5.py` - NEW: Cost tracking status line
- `.claude/status_lines/status_line_v6.py` - NEW: Context window usage status line
- `.claude/status_lines/status_line_v7.py` - NEW: Session duration timer status line
- `.claude/status_lines/status_line_v8.py` - NEW: Token usage with cache stats status line
- `.claude/status_lines/status_line_v9.py` - NEW: Minimal powerline-style status line

### Configuration
- `.claude/settings.json` - Hook configuration (update with all hooks)

### Documentation (Update)
- `README.md` - Main documentation (update with all hooks and new status lines)

## Implementation Phases

### Phase 1: Foundation
Build all missing hook implementations:
- SessionEnd, PermissionRequest, PostToolUseFailure, SubagentStart, Setup
- Ensure consistent structure with existing hooks
- Follow uv single-file script pattern

### Phase 2: Core Implementation
Update existing hooks and create new status lines:
- Update all existing hooks with latest input fields
- Create 5 new status line versions
- Update settings.json with all hook configurations

### Phase 3: Integration & Polish
Validation and documentation:
- Validate all hooks using `claude -p` commands
- Update README.md with comprehensive documentation
- Ensure all log files are properly generated

## Team Orchestration

- You operate as the team lead and orchestrate the team to execute the plan.
- You're responsible for deploying the right team members with the right context to execute the plan.
- IMPORTANT: You NEVER operate directly on the codebase. You use `Task` and `Task*` tools to deploy team members to to the building, validating, testing, deploying, and other tasks.
  - This is critical. You're job is to act as a high level director of the team, not a builder.
  - You're role is to validate all work is going well and make sure the team is on track to complete the plan.
  - You'll orchestrate this by using the Task* Tools to manage coordination between the team members.
  - Communication is paramount. You'll use the Task* Tools to communicate with the team members and ensure they're on track to complete the plan.
- Take note of the session id of each team member. This is how you'll reference them.

### Team Members

- Builder (SessionEnd Hook)
  - Name: session-end-builder
  - Role: Implement the SessionEnd hook with logging and cleanup capabilities
  - Agent Type: builder
  - Resume: true

- Validator (SessionEnd Hook)
  - Name: session-end-validator
  - Role: Validate SessionEnd hook works correctly using `claude -p` and log verification
  - Agent Type: validator
  - Resume: true

- Builder (PermissionRequest Hook)
  - Name: permission-request-builder
  - Role: Implement the PermissionRequest hook with allow/deny decision control
  - Agent Type: builder
  - Resume: true

- Validator (PermissionRequest Hook)
  - Name: permission-request-validator
  - Role: Validate PermissionRequest hook works correctly
  - Agent Type: validator
  - Resume: true

- Builder (PostToolUseFailure Hook)
  - Name: post-tool-failure-builder
  - Role: Implement the PostToolUseFailure hook for failed tool logging
  - Agent Type: builder
  - Resume: true

- Validator (PostToolUseFailure Hook)
  - Name: post-tool-failure-validator
  - Role: Validate PostToolUseFailure hook works correctly
  - Agent Type: validator
  - Resume: true

- Builder (SubagentStart Hook)
  - Name: subagent-start-builder
  - Role: Implement the SubagentStart hook for subagent spawning events
  - Agent Type: builder
  - Resume: true

- Validator (SubagentStart Hook)
  - Name: subagent-start-validator
  - Role: Validate SubagentStart hook works correctly
  - Agent Type: validator
  - Resume: true

- Builder (Setup Hook)
  - Name: setup-builder
  - Role: Implement the Setup hook for init/maintenance operations
  - Agent Type: builder
  - Resume: true

- Validator (Setup Hook)
  - Name: setup-validator
  - Role: Validate Setup hook works correctly using `claude --init` commands
  - Agent Type: validator
  - Resume: true

- Builder (Status Lines)
  - Name: status-lines-builder
  - Role: Create 5 new status line versions (v5-v9) with creative features
  - Agent Type: builder
  - Resume: true

- Validator (Status Lines)
  - Name: status-lines-validator
  - Role: Validate all new status lines work correctly and display properly
  - Agent Type: validator
  - Resume: true

- Builder (Documentation)
  - Name: docs-builder
  - Role: Update README.md with all hooks, status lines, and new features
  - Agent Type: builder
  - Resume: true

## Step by Step Tasks

- IMPORTANT: Execute every step in order, top to bottom. Each task maps directly to a `TaskCreate` call.
- Before you start, run `TaskCreate` to create the initial task list that all team members can see and execute.

### 1. Build SessionEnd Hook
- **Task ID**: build-session-end
- **Depends On**: none
- **Assigned To**: session-end-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `.claude/hooks/session_end.py` with proper uv script header
- Implement logging to `logs/session_end.json`
- Handle all reason types: `clear`, `logout`, `prompt_input_exit`, `other`
- Include optional cleanup tasks capability

### 2. Build PermissionRequest Hook
- **Task ID**: build-permission-request
- **Depends On**: none
- **Assigned To**: permission-request-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `.claude/hooks/permission_request.py` with proper uv script header
- Implement allow/deny decision control via JSON output
- Support `updatedInput` for modifying tool inputs
- Log all permission requests to `logs/permission_request.json`

### 3. Build PostToolUseFailure Hook
- **Task ID**: build-post-tool-failure
- **Depends On**: none
- **Assigned To**: post-tool-failure-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `.claude/hooks/post_tool_use_failure.py` with proper uv script header
- Log failed tool calls with error details to `logs/post_tool_use_failure.json`
- Include tool_name, tool_input, and error information

### 4. Build SubagentStart Hook
- **Task ID**: build-subagent-start
- **Depends On**: none
- **Assigned To**: subagent-start-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `.claude/hooks/subagent_start.py` with proper uv script header
- Log subagent spawn events with agent_id and agent_type
- Optional TTS announcement of subagent start

### 5. Build Setup Hook
- **Task ID**: build-setup
- **Depends On**: none
- **Assigned To**: setup-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `.claude/hooks/setup.py` with proper uv script header
- Handle both `init` and `maintenance` triggers
- Support environment variable persistence via CLAUDE_ENV_FILE
- Log setup events to `logs/setup.json`

### 6. Build Status Lines v5-v9
- **Task ID**: build-status-lines
- **Depends On**: none
- **Assigned To**: status-lines-builder
- **Agent Type**: builder
- **Parallel**: true
- Create `status_line_v5.py` - Cost tracking (total_cost_usd, lines added/removed)
- Create `status_line_v6.py` - Context window usage (used_percentage, remaining_percentage)
- Create `status_line_v7.py` - Session duration timer with start time tracking
- Create `status_line_v8.py` - Token usage with cache stats (cache_creation, cache_read)
- Create `status_line_v9.py` - Minimal powerline-style with Unicode separators

### 7. Validate SessionEnd Hook
- **Task ID**: validate-session-end
- **Depends On**: build-session-end
- **Assigned To**: session-end-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/session_end.py` exists and is valid Python
- Run `uv run python -m py_compile .claude/hooks/session_end.py`
- Test hook is triggered correctly (SessionEnd fires on exit)

### 8. Validate PermissionRequest Hook
- **Task ID**: validate-permission-request
- **Depends On**: build-permission-request
- **Assigned To**: permission-request-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/permission_request.py` exists and is valid Python
- Run `uv run python -m py_compile .claude/hooks/permission_request.py`
- Verify JSON output structure matches documentation

### 9. Validate PostToolUseFailure Hook
- **Task ID**: validate-post-tool-failure
- **Depends On**: build-post-tool-failure
- **Assigned To**: post-tool-failure-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/post_tool_use_failure.py` exists and is valid Python
- Run `uv run python -m py_compile .claude/hooks/post_tool_use_failure.py`

### 10. Validate SubagentStart Hook
- **Task ID**: validate-subagent-start
- **Depends On**: build-subagent-start
- **Assigned To**: subagent-start-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/subagent_start.py` exists and is valid Python
- Run `uv run python -m py_compile .claude/hooks/subagent_start.py`

### 11. Validate Setup Hook
- **Task ID**: validate-setup
- **Depends On**: build-setup
- **Assigned To**: setup-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/setup.py` exists and is valid Python
- Run `uv run python -m py_compile .claude/hooks/setup.py`

### 12. Validate Status Lines
- **Task ID**: validate-status-lines
- **Depends On**: build-status-lines
- **Assigned To**: status-lines-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify all v5-v9 status line files exist
- Run `uv run python -m py_compile` on each status line file
- Test each status line with mock JSON input

### 13. Update Settings.json
- **Task ID**: update-settings
- **Depends On**: validate-session-end, validate-permission-request, validate-post-tool-failure, validate-subagent-start, validate-setup
- **Assigned To**: session-end-builder
- **Agent Type**: builder
- **Parallel**: false
- Add SessionEnd hook configuration
- Add PermissionRequest hook configuration
- Add PostToolUseFailure hook configuration
- Add SubagentStart hook configuration
- Add Setup hook configuration
- Ensure all matchers are properly configured

### 14. Integration Test All Hooks
- **Task ID**: integration-test
- **Depends On**: update-settings
- **Assigned To**: setup-validator
- **Agent Type**: validator
- **Parallel**: false
- Run `claude -p 'echo hello'` to trigger basic hooks
- Verify logs are created in `logs/` directory
- Check log file contents for proper JSON structure

### 15. Update README Documentation
- **Task ID**: update-readme
- **Depends On**: integration-test, validate-status-lines
- **Assigned To**: docs-builder
- **Agent Type**: builder
- **Parallel**: false
- Add documentation for all new hooks (SessionEnd, PermissionRequest, PostToolUseFailure, SubagentStart, Setup)
- Update hook lifecycle diagram reference
- Add documentation for new status lines (v5-v9)
- Update Key Files section with new files
- Update Features Demonstrated section

### 16. Final Validation
- **Task ID**: validate-all
- **Depends On**: update-readme
- **Assigned To**: setup-validator
- **Agent Type**: validator
- **Parallel**: false
- Run all validation commands
- Verify README.md is properly formatted
- Confirm all acceptance criteria are met
- Generate final report

## Acceptance Criteria

1. **All 12 hook types implemented**: PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, UserPromptSubmit, Stop, SubagentStop, SubagentStart, PreCompact, SessionStart, SessionEnd, Setup
2. **5 new status lines created**: v5 (cost), v6 (context window), v7 (duration), v8 (tokens/cache), v9 (powerline)
3. **All Python files compile without errors**: `uv run python -m py_compile` passes
4. **Settings.json updated**: All hooks configured with proper matchers
5. **Log files generated**: Each hook creates proper JSON log files in `logs/`
6. **README.md updated**: Documents all hooks and status lines with examples
7. **Consistent code style**: All hooks follow existing uv single-file script pattern

## Validation Commands
Execute these commands to validate the task is complete:

- `uv run python -m py_compile .claude/hooks/session_end.py` - Compile new SessionEnd hook
- `uv run python -m py_compile .claude/hooks/permission_request.py` - Compile new PermissionRequest hook
- `uv run python -m py_compile .claude/hooks/post_tool_use_failure.py` - Compile new PostToolUseFailure hook
- `uv run python -m py_compile .claude/hooks/subagent_start.py` - Compile new SubagentStart hook
- `uv run python -m py_compile .claude/hooks/setup.py` - Compile new Setup hook
- `uv run python -m py_compile .claude/status_lines/status_line_v5.py` - Compile status line v5
- `uv run python -m py_compile .claude/status_lines/status_line_v6.py` - Compile status line v6
- `uv run python -m py_compile .claude/status_lines/status_line_v7.py` - Compile status line v7
- `uv run python -m py_compile .claude/status_lines/status_line_v8.py` - Compile status line v8
- `uv run python -m py_compile .claude/status_lines/status_line_v9.py` - Compile status line v9
- `cat .claude/settings.json | python -m json.tool` - Validate settings.json is valid JSON
- `ls -la logs/` - Verify log directory structure after hook testing

## Notes

- All hooks should use the `#!/usr/bin/env -S uv run --script` shebang pattern
- Include the PEP 723 inline script metadata for dependencies
- Use `python-dotenv` for environment variable loading
- Each hook should log to its own JSON file in `logs/` directory
- Follow the established error handling pattern: catch exceptions, exit gracefully
- Status lines must output to stdout and exit with code 0
- The `CLAUDE_ENV_FILE` environment variable is only available in SessionStart and Setup hooks
