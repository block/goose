# Plan: Subagent TTS Summary with Queue System

## Task Description
Build a subagent stop hook that provides natural language task summaries via TTS when subagents complete. The system must handle concurrent subagent completions by implementing a queue system that ensures subagents "speak one at a time" - preventing audio overlap and planning conflicts.

## Objective
When a subagent completes (SubagentStop hook fires), the system will:
1. Extract the subagent's task context from session data
2. Use Anthropic Haiku 4.5 to generate a natural language summary of what the subagent accomplished
3. Queue the TTS announcement to prevent multiple subagents from speaking simultaneously
4. Deliver the audio summary to the user via ElevenLabs TTS

## Problem Statement
Currently, the `subagent_stop.py` hook only says a fixed "Subagent Complete" message via TTS. When multiple subagents run in parallel (which is common with the team orchestration pattern), they can complete at nearly the same time, causing:
- Audio overlap (multiple announcements playing simultaneously)
- No context about what each subagent actually accomplished
- Poor user experience when managing multiple concurrent agents

## Solution Approach
1. **Session-based context extraction**: Leverage existing `.claude/data/sessions/{session_id}.json` files to get the subagent's task/prompt
2. **LLM summarization**: Use Anthropic Haiku 4.5 (fast, cost-effective) with a dedicated prompt to generate personalized, conversational summaries
3. **File-based queue system**: Implement a simple file-lock queue at `.claude/data/tts_queue/` that ensures only one TTS can play at a time
4. **Integration with existing TTS**: Reuse the `elevenlabs_tts.py` infrastructure

## Relevant Files
Use these files to complete the task:

- `.claude/hooks/subagent_stop.py` - Current hook implementation to enhance with summarization and queue
- `.claude/hooks/utils/tts/elevenlabs_tts.py` - Existing TTS utility to reuse for audio playback
- `.claude/hooks/utils/llm/anth.py` - Existing Anthropic LLM utility to extend for task summarization
- `.claude/data/sessions/*.json` - Session files containing subagent prompts/context
- `.claude/settings.json` - Hook configuration (SubagentStop already configured)
- `.claude/output-styles/tts-summary.md` - Reference for the communication style we want
- `.claude/agents/work-completion-summary.md` - Reference agent showing TTS summary pattern
- `logs/subagent_stop.json` - Log data showing SubagentStop event structure including `agent_id` and `agent_transcript_path`

### New Files
- `.claude/hooks/utils/tts/tts_queue.py` - Queue management utility for serializing TTS playback
- `.claude/hooks/utils/llm/task_summarizer.py` - Dedicated LLM summarization prompt for subagent completion
- `.claude/data/tts_queue/` - Directory for queue lock files and pending items

## Implementation Phases

### Phase 1: Foundation
- Create the TTS queue system (`tts_queue.py`) with file-based locking
- Design the queue data structure and lock mechanism
- Implement queue entry/exit functions with timeout handling

### Phase 2: Core Implementation
- Create the task summarizer LLM utility (`task_summarizer.py`) using Haiku 4.5
- Design the summarization prompt following tts-summary.md style
- Integrate context extraction from session data and transcript path
- Update `subagent_stop.py` to:
  - Extract subagent context
  - Call the summarizer
  - Queue the TTS announcement

### Phase 3: Integration & Polish
- Test with parallel subagents using `claude -p` commands
- Verify queue serialization works correctly
- Handle edge cases (timeout, missing session data, API failures)
- Add logging for debugging queue behavior

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

- Builder
  - Name: queue-builder
  - Role: Implement the TTS queue system with file-based locking mechanism
  - Agent Type: builder
  - Resume: true

- Builder
  - Name: summarizer-builder
  - Role: Create the Haiku 4.5 task summarization LLM utility
  - Agent Type: builder
  - Resume: true

- Builder
  - Name: hook-builder
  - Role: Integrate summarization and queue into subagent_stop.py
  - Agent Type: builder
  - Resume: true

- Validator
  - Name: queue-validator
  - Role: Validate queue system works correctly with file locking
  - Agent Type: validator
  - Resume: true

- Validator
  - Name: summarizer-validator
  - Role: Validate the summarizer generates appropriate summaries
  - Agent Type: validator
  - Resume: true

- Validator
  - Name: integration-validator
  - Role: End-to-end validation with parallel subagent testing
  - Agent Type: validator
  - Resume: true

## Step by Step Tasks

- IMPORTANT: Execute every step in order, top to bottom. Each task maps directly to a `TaskCreate` call.
- Before you start, run `TaskCreate` to create the initial task list that all team members can see and execute.

### 1. Create TTS Queue Directory Structure
- **Task ID**: create-queue-dir
- **Depends On**: none
- **Assigned To**: queue-builder
- **Agent Type**: builder
- **Parallel**: false
- Create `.claude/data/tts_queue/` directory
- Create empty `.gitkeep` to track directory in git

### 2. Validate Queue Directory Exists
- **Task ID**: validate-queue-dir
- **Depends On**: create-queue-dir
- **Assigned To**: queue-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/data/tts_queue/` directory exists
- Verify `.gitkeep` file exists

### 3. Build TTS Queue Manager
- **Task ID**: build-queue-manager
- **Depends On**: validate-queue-dir
- **Assigned To**: queue-builder
- **Agent Type**: builder
- **Parallel**: false
- Create `.claude/hooks/utils/tts/tts_queue.py` with:
  - `acquire_tts_lock(agent_id: str, timeout: int = 30) -> bool` - Acquire exclusive TTS lock using file locking
  - `release_tts_lock(agent_id: str) -> None` - Release the TTS lock
  - `is_tts_locked() -> bool` - Check if TTS is currently locked
  - `cleanup_stale_locks(max_age_seconds: int = 60) -> None` - Remove locks older than max age
- Use a single lock file `.claude/data/tts_queue/tts.lock` with fcntl file locking
- Store agent_id and timestamp in lock file for debugging
- Implement retry logic with configurable timeout
- Follow uv script pattern with dependencies

### 4. Validate TTS Queue Manager
- **Task ID**: validate-queue-manager
- **Depends On**: build-queue-manager
- **Assigned To**: queue-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify file exists at `.claude/hooks/utils/tts/tts_queue.py`
- Verify script has proper uv script header
- Verify all required functions exist: `acquire_tts_lock`, `release_tts_lock`, `is_tts_locked`, `cleanup_stale_locks`
- Run `uv run python -m py_compile .claude/hooks/utils/tts/tts_queue.py` to verify syntax

### 5. Build Task Summarizer LLM Utility
- **Task ID**: build-task-summarizer
- **Depends On**: validate-queue-manager
- **Assigned To**: summarizer-builder
- **Agent Type**: builder
- **Parallel**: false
- Create `.claude/hooks/utils/llm/task_summarizer.py` with:
  - `summarize_subagent_task(task_description: str, agent_name: str = None) -> str` - Generate a natural language summary
- Use `claude-haiku-4-5-20250414` model (Haiku 4.5)
- Design prompt following tts-summary.md style:
  - Address user as "Dan" directly
  - Keep summaries under 20 words
  - Focus on outcomes and value delivered
  - Be conversational and personalized
  - Use phrases like "Dan, {agent_name} finished..." or "{agent_name} completed..."
- Include fallback message if API fails
- Follow uv script pattern matching anth.py structure

### 6. Validate Task Summarizer
- **Task ID**: validate-task-summarizer
- **Depends On**: build-task-summarizer
- **Assigned To**: summarizer-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify file exists at `.claude/hooks/utils/llm/task_summarizer.py`
- Verify script has proper uv script header with anthropic dependency
- Verify `summarize_subagent_task` function exists with correct signature
- Run `uv run python -m py_compile .claude/hooks/utils/llm/task_summarizer.py` to verify syntax
- Run `uv run .claude/hooks/utils/llm/task_summarizer.py "Implemented user authentication"` to test generation

### 7. Update Subagent Stop Hook
- **Task ID**: update-subagent-hook
- **Depends On**: validate-task-summarizer
- **Assigned To**: hook-builder
- **Agent Type**: builder
- **Parallel**: false
- Update `.claude/hooks/subagent_stop.py` to:
  - Import and use `tts_queue` for lock management
  - Import and use `task_summarizer` for LLM summarization
  - Extract task context from session data (check `.claude/data/sessions/{session_id}.json` for prompts)
  - If session data unavailable, fall back to parsing agent_transcript_path for context
  - Acquire TTS lock before speaking
  - Generate summary using Haiku 4.5
  - Call TTS with generated summary
  - Release TTS lock after completion
  - Add `--summarize` flag (default on when `--notify` is used)
  - Keep backward compatibility with existing `--notify` flag behavior
  - Handle errors gracefully (release lock on exception)

### 8. Validate Subagent Stop Hook Update
- **Task ID**: validate-hook-update
- **Depends On**: update-subagent-hook
- **Assigned To**: integration-validator
- **Agent Type**: validator
- **Parallel**: false
- Verify `.claude/hooks/subagent_stop.py` has been updated
- Verify imports for tts_queue and task_summarizer are present
- Verify lock acquire/release logic exists
- Run `uv run python -m py_compile .claude/hooks/subagent_stop.py` to verify syntax
- Verify backward compatibility (--notify flag still works)

### 9. Integration Test - Single Subagent
- **Task ID**: test-single-subagent
- **Depends On**: validate-hook-update
- **Assigned To**: integration-validator
- **Agent Type**: validator
- **Parallel**: false
- Run a single subagent test using Task tool: `claude -p "Use the Task tool with subagent_type 'general-purpose' and prompt 'Say hello and list the current directory' - this will spawn a subagent"`
- Verify SubagentStop hook fires (check `logs/subagent_stop.json` for new entry)
- Verify TTS announcement plays with summarized message (not just "Subagent Complete")
- Verify lock was acquired and released properly (no stale lock files)

### 10. Integration Test - Parallel Subagents
- **Task ID**: test-parallel-subagents
- **Depends On**: test-single-subagent
- **Assigned To**: integration-validator
- **Agent Type**: validator
- **Parallel**: false
- Run parallel subagent test: `claude -p "Use the Task tool to spawn THREE subagents in PARALLEL (in a single message with multiple Task tool calls): one to list files, one to show current date, one to echo hello. Use subagent_type 'general-purpose' for all."`
- Verify all three SubagentStop events logged in `logs/subagent_stop.json`
- Verify announcements play one at a time (not overlapping) - listen for sequential audio
- Verify all subagents eventually get announced (three distinct TTS messages)
- Document expected behavior for team reference

### 11. Final Validation
- **Task ID**: validate-all
- **Depends On**: test-parallel-subagents
- **Assigned To**: integration-validator
- **Agent Type**: validator
- **Parallel**: false
- Run all validation commands listed in Acceptance Criteria
- Verify all files exist and compile correctly
- Verify queue, summarizer, and hook integration works end-to-end
- Provide final validation report

## Acceptance Criteria
- [ ] `.claude/data/tts_queue/` directory exists for queue management
- [ ] `.claude/hooks/utils/tts/tts_queue.py` implements file-based lock with acquire/release functions
- [ ] `.claude/hooks/utils/llm/task_summarizer.py` uses Haiku 4.5 to generate natural language summaries
- [ ] Summarizer follows tts-summary.md style (addresses Dan, under 20 words, conversational)
- [ ] `.claude/hooks/subagent_stop.py` integrates queue and summarizer
- [ ] Only one TTS plays at a time when multiple subagents complete simultaneously
- [ ] Subagent summaries include context about what the agent accomplished
- [ ] System gracefully handles API failures with fallback messages
- [ ] All scripts compile without syntax errors
- [ ] Lock files are cleaned up properly (no stale locks)

## Validation Commands
Execute these commands to validate the task is complete:

- `ls -la .claude/data/tts_queue/` - Verify queue directory exists
- `uv run python -m py_compile .claude/hooks/utils/tts/tts_queue.py` - Verify queue manager syntax
- `uv run python -m py_compile .claude/hooks/utils/llm/task_summarizer.py` - Verify summarizer syntax
- `uv run python -m py_compile .claude/hooks/subagent_stop.py` - Verify hook syntax
- `uv run .claude/hooks/utils/llm/task_summarizer.py "Built authentication system"` - Test summarizer generates output
- `claude -p "Use the Task tool with subagent_type 'general-purpose' and prompt 'echo hello' to spawn a subagent"` - Test SubagentStop hook fires with TTS summary (requires Task tool to spawn actual subagent)

## Notes
- **Model Selection**: Using `claude-haiku-4-5-20250414` for cost-effectiveness and speed. Summarization is a simple task well-suited for Haiku.
- **Queue Implementation**: File-based locking with fcntl is chosen over database/Redis for simplicity and zero external dependencies. The `.claude/data/` directory is already used for session data, so this fits the existing pattern.
- **Lock Timeout**: Default 30 seconds prevents indefinite blocking if a TTS gets stuck. Cleanup function removes locks older than 60 seconds.
- **Fallback Strategy**: If LLM summarization fails, fall back to a generic "Subagent completed" message rather than failing silently.
- **Session Data**: The prompts array in session files provides task context. If unavailable, the agent_transcript_path in SubagentStop events can be parsed for context.
- **Testing Subagents**: SubagentStop only fires when the **Task tool** spawns a subagent that completes. Running `claude -p "some command"` starts a main agent (triggers Stop hook, not SubagentStop). To test, prompt Claude to use the Task tool explicitly: `claude -p "Use the Task tool with subagent_type 'general-purpose' to..."`. Parallel testing requires prompting Claude to spawn multiple Task tool calls in a single message.
