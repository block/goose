[Skip to main content](https://code.claude.com/docs/en/hooks#content-area)

[Claude Code Docs home page![light logo](https://mintcdn.com/claude-code/o69F7a6qoW9vboof/logo/light.svg?fit=max&auto=format&n=o69F7a6qoW9vboof&q=85&s=536eade682636e84231afce2577f9509)![dark logo](https://mintcdn.com/claude-code/o69F7a6qoW9vboof/logo/dark.svg?fit=max&auto=format&n=o69F7a6qoW9vboof&q=85&s=0766b3221061e80143e9f300733e640b)](https://code.claude.com/docs)

![US](https://d3gk2c5xim1je2.cloudfront.net/flags/US.svg)

English

Search...

⌘KAsk AI

Search...

Navigation

Reference

Hooks reference

[Getting started](https://code.claude.com/docs/en/overview) [Build with Claude Code](https://code.claude.com/docs/en/sub-agents) [Deployment](https://code.claude.com/docs/en/third-party-integrations) [Administration](https://code.claude.com/docs/en/setup) [Configuration](https://code.claude.com/docs/en/settings) [Reference](https://code.claude.com/docs/en/cli-reference) [Resources](https://code.claude.com/docs/en/legal-and-compliance)

On this page

- [Hook lifecycle](https://code.claude.com/docs/en/hooks#hook-lifecycle)
- [Configuration](https://code.claude.com/docs/en/hooks#configuration)
- [Structure](https://code.claude.com/docs/en/hooks#structure)
- [Project-Specific Hook Scripts](https://code.claude.com/docs/en/hooks#project-specific-hook-scripts)
- [Plugin hooks](https://code.claude.com/docs/en/hooks#plugin-hooks)
- [Hooks in skills and agents](https://code.claude.com/docs/en/hooks#hooks-in-skills-and-agents)
- [Prompt-Based Hooks](https://code.claude.com/docs/en/hooks#prompt-based-hooks)
- [How prompt-based hooks work](https://code.claude.com/docs/en/hooks#how-prompt-based-hooks-work)
- [Configuration](https://code.claude.com/docs/en/hooks#configuration-2)
- [Response schema](https://code.claude.com/docs/en/hooks#response-schema)
- [Supported hook events](https://code.claude.com/docs/en/hooks#supported-hook-events)
- [Example: Intelligent Stop hook](https://code.claude.com/docs/en/hooks#example%3A-intelligent-stop-hook)
- [Example: SubagentStop with custom logic](https://code.claude.com/docs/en/hooks#example%3A-subagentstop-with-custom-logic)
- [Comparison with bash command hooks](https://code.claude.com/docs/en/hooks#comparison-with-bash-command-hooks)
- [Best practices](https://code.claude.com/docs/en/hooks#best-practices)
- [Hook Events](https://code.claude.com/docs/en/hooks#hook-events)
- [PreToolUse](https://code.claude.com/docs/en/hooks#pretooluse)
- [PermissionRequest](https://code.claude.com/docs/en/hooks#permissionrequest)
- [PostToolUse](https://code.claude.com/docs/en/hooks#posttooluse)
- [Notification](https://code.claude.com/docs/en/hooks#notification)
- [UserPromptSubmit](https://code.claude.com/docs/en/hooks#userpromptsubmit)
- [Stop](https://code.claude.com/docs/en/hooks#stop)
- [SubagentStop](https://code.claude.com/docs/en/hooks#subagentstop)
- [PreCompact](https://code.claude.com/docs/en/hooks#precompact)
- [Setup](https://code.claude.com/docs/en/hooks#setup)
- [SessionStart](https://code.claude.com/docs/en/hooks#sessionstart)
- [Persisting environment variables](https://code.claude.com/docs/en/hooks#persisting-environment-variables)
- [SessionEnd](https://code.claude.com/docs/en/hooks#sessionend)
- [Hook Input](https://code.claude.com/docs/en/hooks#hook-input)
- [PreToolUse Input](https://code.claude.com/docs/en/hooks#pretooluse-input)
- [Bash tool](https://code.claude.com/docs/en/hooks#bash-tool)
- [Write tool](https://code.claude.com/docs/en/hooks#write-tool)
- [Edit tool](https://code.claude.com/docs/en/hooks#edit-tool)
- [Read tool](https://code.claude.com/docs/en/hooks#read-tool)
- [PostToolUse Input](https://code.claude.com/docs/en/hooks#posttooluse-input)
- [Notification Input](https://code.claude.com/docs/en/hooks#notification-input)
- [UserPromptSubmit Input](https://code.claude.com/docs/en/hooks#userpromptsubmit-input)
- [Stop Input](https://code.claude.com/docs/en/hooks#stop-input)
- [SubagentStop Input](https://code.claude.com/docs/en/hooks#subagentstop-input)
- [PreCompact Input](https://code.claude.com/docs/en/hooks#precompact-input)
- [Setup Input](https://code.claude.com/docs/en/hooks#setup-input)
- [SessionStart Input](https://code.claude.com/docs/en/hooks#sessionstart-input)
- [SubagentStart Input](https://code.claude.com/docs/en/hooks#subagentstart-input)
- [SessionEnd Input](https://code.claude.com/docs/en/hooks#sessionend-input)
- [Hook Output](https://code.claude.com/docs/en/hooks#hook-output)
- [Simple: Exit Code](https://code.claude.com/docs/en/hooks#simple%3A-exit-code)
- [Exit Code 2 Behavior](https://code.claude.com/docs/en/hooks#exit-code-2-behavior)
- [Advanced: JSON Output](https://code.claude.com/docs/en/hooks#advanced%3A-json-output)
- [Common JSON Fields](https://code.claude.com/docs/en/hooks#common-json-fields)
- [PreToolUse Decision Control](https://code.claude.com/docs/en/hooks#pretooluse-decision-control)
- [PermissionRequest Decision Control](https://code.claude.com/docs/en/hooks#permissionrequest-decision-control)
- [PostToolUse Decision Control](https://code.claude.com/docs/en/hooks#posttooluse-decision-control)
- [UserPromptSubmit Decision Control](https://code.claude.com/docs/en/hooks#userpromptsubmit-decision-control)
- [Stop/SubagentStop Decision Control](https://code.claude.com/docs/en/hooks#stop%2Fsubagentstop-decision-control)
- [Setup Decision Control](https://code.claude.com/docs/en/hooks#setup-decision-control)
- [SessionStart Decision Control](https://code.claude.com/docs/en/hooks#sessionstart-decision-control)
- [SessionEnd Decision Control](https://code.claude.com/docs/en/hooks#sessionend-decision-control)
- [Exit Code Example: Bash Command Validation](https://code.claude.com/docs/en/hooks#exit-code-example%3A-bash-command-validation)
- [JSON Output Example: UserPromptSubmit to Add Context and Validation](https://code.claude.com/docs/en/hooks#json-output-example%3A-userpromptsubmit-to-add-context-and-validation)
- [JSON Output Example: PreToolUse with Approval](https://code.claude.com/docs/en/hooks#json-output-example%3A-pretooluse-with-approval)
- [Working with MCP Tools](https://code.claude.com/docs/en/hooks#working-with-mcp-tools)
- [MCP Tool Naming](https://code.claude.com/docs/en/hooks#mcp-tool-naming)
- [Configuring Hooks for MCP Tools](https://code.claude.com/docs/en/hooks#configuring-hooks-for-mcp-tools)
- [Examples](https://code.claude.com/docs/en/hooks#examples)
- [Security Considerations](https://code.claude.com/docs/en/hooks#security-considerations)
- [Disclaimer](https://code.claude.com/docs/en/hooks#disclaimer)
- [Security Best Practices](https://code.claude.com/docs/en/hooks#security-best-practices)
- [Configuration Safety](https://code.claude.com/docs/en/hooks#configuration-safety)
- [Hook Execution Details](https://code.claude.com/docs/en/hooks#hook-execution-details)
- [Debugging](https://code.claude.com/docs/en/hooks#debugging)
- [Basic Troubleshooting](https://code.claude.com/docs/en/hooks#basic-troubleshooting)
- [Advanced Debugging](https://code.claude.com/docs/en/hooks#advanced-debugging)
- [Debug Output Example](https://code.claude.com/docs/en/hooks#debug-output-example)

For a quickstart guide with examples, see [Get started with Claude Code hooks](https://code.claude.com/docs/en/hooks-guide).

## [​](https://code.claude.com/docs/en/hooks\#hook-lifecycle)  Hook lifecycle

Hooks fire at specific points during a Claude Code session.

![Hook lifecycle diagram showing the sequence of hooks from SessionStart through the agentic loop to SessionEnd](https://mintcdn.com/claude-code/z2YM37Ycg6eMbID3/images/hooks-lifecycle.png?fit=max&auto=format&n=z2YM37Ycg6eMbID3&q=85&s=5c25fedbc3db6f8882af50c3cc478c32)

| Hook | When it fires |
| --- | --- |
| `SessionStart` | Session begins or resumes |
| `UserPromptSubmit` | User submits a prompt |
| `PreToolUse` | Before tool execution |
| `PermissionRequest` | When permission dialog appears |
| `PostToolUse` | After tool succeeds |
| `PostToolUseFailure` | After tool fails |
| `SubagentStart` | When spawning a subagent |
| `SubagentStop` | When subagent finishes |
| `Stop` | Claude finishes responding |
| `PreCompact` | Before context compaction |
| `SessionEnd` | Session terminates |
| `Notification` | Claude Code sends notifications |

## [​](https://code.claude.com/docs/en/hooks\#configuration)  Configuration

Claude Code hooks are configured in your [settings files](https://code.claude.com/docs/en/settings):

- `~/.claude/settings.json` \- User settings
- `.claude/settings.json` \- Project settings
- `.claude/settings.local.json` \- Local project settings (not committed)
- Managed policy settings

Enterprise administrators can use `allowManagedHooksOnly` to block user, project, and plugin hooks. See [Hook configuration](https://code.claude.com/docs/en/settings#hook-configuration).

### [​](https://code.claude.com/docs/en/hooks\#structure)  Structure

Hooks are organized by matchers, where each matcher can have multiple hooks:

Copy

Ask AI

```
{
  "hooks": {
    "EventName": [\
      {\
        "matcher": "ToolPattern",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "your-command-here"\
          }\
        ]\
      }\
    ]
  }
}
```

- **matcher**: Pattern to match tool names, case-sensitive (only applicable for
`PreToolUse`, `PermissionRequest`, and `PostToolUse`)
  - Simple strings match exactly: `Write` matches only the Write tool
  - Supports regex: `Edit|Write` or `Notebook.*`
  - Use `*` to match all tools. You can also use empty string (`""`) or leave
    `matcher` blank.
- **hooks**: Array of hooks to execute when the pattern matches
  - `type`: Hook execution type - `"command"` for bash commands or `"prompt"` for LLM-based evaluation
  - `command`: (For `type: "command"`) The bash command to execute (can use `$CLAUDE_PROJECT_DIR` environment variable)
  - `prompt`: (For `type: "prompt"`) The prompt to send to the LLM for evaluation
  - `timeout`: (Optional) How long a hook should run, in seconds, before canceling that specific hook

For events like `UserPromptSubmit`, `Stop`, `SubagentStop`, and `Setup`
that don’t use matchers, you can omit the matcher field:

Copy

Ask AI

```
{
  "hooks": {
    "UserPromptSubmit": [\
      {\
        "hooks": [\
          {\
            "type": "command",\
            "command": "/path/to/prompt-validator.py"\
          }\
        ]\
      }\
    ]
  }
}
```

### [​](https://code.claude.com/docs/en/hooks\#project-specific-hook-scripts)  Project-Specific Hook Scripts

You can use the environment variable `CLAUDE_PROJECT_DIR` (only available when
Claude Code spawns the hook command) to reference scripts stored in your project,
ensuring they work regardless of Claude’s current directory:

Copy

Ask AI

```
{
  "hooks": {
    "PostToolUse": [\
      {\
        "matcher": "Write|Edit",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/check-style.sh"\
          }\
        ]\
      }\
    ]
  }
}
```

### [​](https://code.claude.com/docs/en/hooks\#plugin-hooks)  Plugin hooks

[Plugins](https://code.claude.com/docs/en/plugins) can provide hooks that integrate seamlessly with your user and project hooks. Plugin hooks are automatically merged with your configuration when plugins are enabled.**How plugin hooks work**:

- Plugin hooks are defined in the plugin’s `hooks/hooks.json` file or in a file given by a custom path to the `hooks` field.
- When a plugin is enabled, its hooks are merged with user and project hooks
- Multiple hooks from different sources can respond to the same event
- Plugin hooks use the `${CLAUDE_PLUGIN_ROOT}` environment variable to reference plugin files

**Example plugin hook configuration**:

Copy

Ask AI

```
{
  "description": "Automatic code formatting",
  "hooks": {
    "PostToolUse": [\
      {\
        "matcher": "Write|Edit",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/format.sh",\
            "timeout": 30\
          }\
        ]\
      }\
    ]
  }
}
```

Plugin hooks use the same format as regular hooks with an optional `description` field to explain the hook’s purpose.

Plugin hooks run alongside your custom hooks. If multiple hooks match an event, they all execute in parallel.

**Environment variables for plugins**:

- `${CLAUDE_PLUGIN_ROOT}`: Absolute path to the plugin directory
- `${CLAUDE_PROJECT_DIR}`: Project root directory (same as for project hooks)
- All standard environment variables are available

See the [plugin components reference](https://code.claude.com/docs/en/plugins-reference#hooks) for details on creating plugin hooks.

### [​](https://code.claude.com/docs/en/hooks\#hooks-in-skills-and-agents)  Hooks in skills and agents

In addition to settings files and plugins, hooks can be defined directly in [skills](https://code.claude.com/docs/en/skills) and [subagents](https://code.claude.com/docs/en/sub-agents) using frontmatter. These hooks are scoped to the component’s lifecycle and only run when that component is active.**Supported events**: `PreToolUse`, `PostToolUse`, and `Stop`**Example in a Skill**:

Copy

Ask AI

```
---
name: secure-operations
description: Perform operations with security checks
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/security-check.sh"
---
```

**Example in an agent**:

Copy

Ask AI

```
---
name: code-reviewer
description: Review code changes
hooks:
  PostToolUse:
    - matcher: "Edit|Write"
      hooks:
        - type: command
          command: "./scripts/run-linter.sh"
---
```

Component-scoped hooks follow the same configuration format as settings-based hooks but are automatically cleaned up when the component finishes executing.**Additional option for skills:**

- `once`: Set to `true` to run the hook only once per session. After the first successful execution, the hook is removed. Note: This option is currently only supported for skills, not for agents.

## [​](https://code.claude.com/docs/en/hooks\#prompt-based-hooks)  Prompt-Based Hooks

In addition to bash command hooks (`type: "command"`), Claude Code supports prompt-based hooks (`type: "prompt"`) that use an LLM to evaluate whether to allow or block an action. Prompt-based hooks are currently only supported for `Stop` and `SubagentStop` hooks, where they enable intelligent, context-aware decisions.

### [​](https://code.claude.com/docs/en/hooks\#how-prompt-based-hooks-work)  How prompt-based hooks work

Instead of executing a bash command, prompt-based hooks:

1. Send the hook input and your prompt to a fast LLM (Haiku)
2. The LLM responds with structured JSON containing a decision
3. Claude Code processes the decision automatically

### [​](https://code.claude.com/docs/en/hooks\#configuration-2)  Configuration

Copy

Ask AI

```
{
  "hooks": {
    "Stop": [\
      {\
        "hooks": [\
          {\
            "type": "prompt",\
            "prompt": "Evaluate if Claude should stop: $ARGUMENTS. Check if all tasks are complete."\
          }\
        ]\
      }\
    ]
  }
}
```

**Fields:**

- `type`: Must be `"prompt"`
- `prompt`: The prompt text to send to the LLM
  - Use `$ARGUMENTS` as a placeholder for the hook input JSON
  - If `$ARGUMENTS` is not present, input JSON is appended to the prompt
- `timeout`: (Optional) Timeout in seconds (default: 30 seconds)

### [​](https://code.claude.com/docs/en/hooks\#response-schema)  Response schema

The LLM must respond with JSON containing:

Copy

Ask AI

```
{
  "ok": true | false,
  "reason": "Explanation for the decision"
}
```

**Response fields:**

- `ok`: `true` allows the action, `false` prevents it
- `reason`: Required when `ok` is `false`. Explanation shown to Claude

### [​](https://code.claude.com/docs/en/hooks\#supported-hook-events)  Supported hook events

Prompt-based hooks work with any hook event, but are most useful for:

- **Stop**: Intelligently decide if Claude should continue working
- **SubagentStop**: Evaluate if a subagent has completed its task
- **UserPromptSubmit**: Validate user prompts with LLM assistance
- **PreToolUse**: Make context-aware permission decisions
- **PermissionRequest**: Intelligently allow or deny permission dialogs

### [​](https://code.claude.com/docs/en/hooks\#example:-intelligent-stop-hook)  Example: Intelligent Stop hook

Copy

Ask AI

```
{
  "hooks": {
    "Stop": [\
      {\
        "hooks": [\
          {\
            "type": "prompt",\
            "prompt": "You are evaluating whether Claude should stop working. Context: $ARGUMENTS\n\nAnalyze the conversation and determine if:\n1. All user-requested tasks are complete\n2. Any errors need to be addressed\n3. Follow-up work is needed\n\nRespond with JSON: {\"ok\": true} to allow stopping, or {\"ok\": false, \"reason\": \"your explanation\"} to continue working.",\
            "timeout": 30\
          }\
        ]\
      }\
    ]
  }
}
```

### [​](https://code.claude.com/docs/en/hooks\#example:-subagentstop-with-custom-logic)  Example: SubagentStop with custom logic

Copy

Ask AI

```
{
  "hooks": {
    "SubagentStop": [\
      {\
        "hooks": [\
          {\
            "type": "prompt",\
            "prompt": "Evaluate if this subagent should stop. Input: $ARGUMENTS\n\nCheck if:\n- The subagent completed its assigned task\n- Any errors occurred that need fixing\n- Additional context gathering is needed\n\nReturn: {\"ok\": true} to allow stopping, or {\"ok\": false, \"reason\": \"explanation\"} to continue."\
          }\
        ]\
      }\
    ]
  }
}
```

### [​](https://code.claude.com/docs/en/hooks\#comparison-with-bash-command-hooks)  Comparison with bash command hooks

| Feature | Bash Command Hooks | Prompt-Based Hooks |
| --- | --- | --- |
| **Execution** | Runs bash script | Queries LLM |
| **Decision logic** | You implement in code | LLM evaluates context |
| **Setup complexity** | Requires script file | Configure prompt |
| **Context awareness** | Limited to script logic | Natural language understanding |
| **Performance** | Fast (local execution) | Slower (API call) |
| **Use case** | Deterministic rules | Context-aware decisions |

### [​](https://code.claude.com/docs/en/hooks\#best-practices)  Best practices

- **Be specific in prompts**: Clearly state what you want the LLM to evaluate
- **Include decision criteria**: List the factors the LLM should consider
- **Test your prompts**: Verify the LLM makes correct decisions for your use cases
- **Set appropriate timeouts**: Default is 30 seconds, adjust if needed
- **Use for complex decisions**: Bash hooks are better for simple, deterministic rules

See the [plugin components reference](https://code.claude.com/docs/en/plugins-reference#hooks) for details on creating plugin hooks.

## [​](https://code.claude.com/docs/en/hooks\#hook-events)  Hook Events

### [​](https://code.claude.com/docs/en/hooks\#pretooluse)  PreToolUse

Runs after Claude creates tool parameters and before processing the tool call.**Common matchers:**

- `Task` \- Subagent tasks (see [subagents documentation](https://code.claude.com/docs/en/sub-agents))
- `Bash` \- Shell commands
- `Glob` \- File pattern matching
- `Grep` \- Content search
- `Read` \- File reading
- `Edit` \- File editing
- `Write` \- File writing
- `WebFetch`, `WebSearch` \- Web operations

Use [PreToolUse decision control](https://code.claude.com/docs/en/hooks#pretooluse-decision-control) to allow, deny, or ask for permission to use the tool.

### [​](https://code.claude.com/docs/en/hooks\#permissionrequest)  PermissionRequest

Runs when the user is shown a permission dialog.
Use [PermissionRequest decision control](https://code.claude.com/docs/en/hooks#permissionrequest-decision-control) to allow or deny on behalf of the user.Recognizes the same matcher values as PreToolUse.

### [​](https://code.claude.com/docs/en/hooks\#posttooluse)  PostToolUse

Runs immediately after a tool completes successfully.Recognizes the same matcher values as PreToolUse.

### [​](https://code.claude.com/docs/en/hooks\#notification)  Notification

Runs when Claude Code sends notifications. Supports matchers to filter by notification type.**Common matchers:**

- `permission_prompt` \- Permission requests from Claude Code
- `idle_prompt` \- When Claude is waiting for user input (after 60+ seconds of idle time)
- `auth_success` \- Authentication success notifications
- `elicitation_dialog` \- When Claude Code needs input for MCP tool elicitation

You can use matchers to run different hooks for different notification types, or omit the matcher to run hooks for all notifications.**Example: Different notifications for different types**

Copy

Ask AI

```
{
  "hooks": {
    "Notification": [\
      {\
        "matcher": "permission_prompt",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "/path/to/permission-alert.sh"\
          }\
        ]\
      },\
      {\
        "matcher": "idle_prompt",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "/path/to/idle-notification.sh"\
          }\
        ]\
      }\
    ]
  }
}
```

### [​](https://code.claude.com/docs/en/hooks\#userpromptsubmit)  UserPromptSubmit

Runs when the user submits a prompt, before Claude processes it. This allows you
to add additional context based on the prompt/conversation, validate prompts, or
block certain types of prompts.

### [​](https://code.claude.com/docs/en/hooks\#stop)  Stop

Runs when the main Claude Code agent has finished responding. Does not run if
the stoppage occurred due to a user interrupt.

### [​](https://code.claude.com/docs/en/hooks\#subagentstop)  SubagentStop

Runs when a Claude Code subagent (Task tool call) has finished responding.

### [​](https://code.claude.com/docs/en/hooks\#precompact)  PreCompact

Runs before Claude Code is about to run a compact operation.**Matchers:**

- `manual` \- Invoked from `/compact`
- `auto` \- Invoked from auto-compact (due to full context window)

### [​](https://code.claude.com/docs/en/hooks\#setup)  Setup

Runs when Claude Code is invoked with repository setup and maintenance flags (`--init`, `--init-only`, or `--maintenance`). Use this hook for operations you don’t want on every session—such as installing dependencies, running migrations, or periodic maintenance tasks.

Use **Setup** hooks for one-time or occasional operations (dependency installation, migrations, cleanup). Use **SessionStart** hooks for things you want on every session (loading context, setting environment variables). Setup hooks require explicit flags because running them automatically would slow down every session start.

**Matchers:**

- `init` \- Invoked from `--init` or `--init-only` flags
- `maintenance` \- Invoked from `--maintenance` flag

Setup hooks have access to the `CLAUDE_ENV_FILE` environment variable for persisting environment variables, similar to SessionStart hooks.

### [​](https://code.claude.com/docs/en/hooks\#sessionstart)  SessionStart

Runs when Claude Code starts a new session or resumes an existing session (which
currently does start a new session under the hood). Useful for loading development context like existing issues or recent changes to your codebase, or setting up environment variables.

For one-time operations like installing dependencies or running migrations, use [Setup hooks](https://code.claude.com/docs/en/hooks#setup) instead. SessionStart runs on every session, so keep these hooks fast.

**Matchers:**

- `startup` \- Invoked from startup
- `resume` \- Invoked from `--resume`, `--continue`, or `/resume`
- `clear` \- Invoked from `/clear`
- `compact` \- Invoked from auto or manual compact.

#### [​](https://code.claude.com/docs/en/hooks\#persisting-environment-variables)  Persisting environment variables

SessionStart hooks have access to the `CLAUDE_ENV_FILE` environment variable, which provides a file path where you can persist environment variables for subsequent bash commands.**Example: Setting individual environment variables**

Copy

Ask AI

```
#!/bin/bash

if [ -n "$CLAUDE_ENV_FILE" ]; then
  echo 'export NODE_ENV=production' >> "$CLAUDE_ENV_FILE"
  echo 'export API_KEY=your-api-key' >> "$CLAUDE_ENV_FILE"
  echo 'export PATH="$PATH:./node_modules/.bin"' >> "$CLAUDE_ENV_FILE"
fi

exit 0
```

**Example: Persisting all environment changes from the hook**When your setup modifies the environment (for example, `nvm use`), capture and persist all changes by diffing the environment:

Copy

Ask AI

```
#!/bin/bash

ENV_BEFORE=$(export -p | sort)

# Run your setup commands that modify the environment
source ~/.nvm/nvm.sh
nvm use 20

if [ -n "$CLAUDE_ENV_FILE" ]; then
  ENV_AFTER=$(export -p | sort)
  comm -13 <(echo "$ENV_BEFORE") <(echo "$ENV_AFTER") >> "$CLAUDE_ENV_FILE"
fi

exit 0
```

Any variables written to this file will be available in all subsequent bash commands that Claude Code executes during the session.

`CLAUDE_ENV_FILE` is only available for SessionStart hooks. Other hook types do not have access to this variable.

### [​](https://code.claude.com/docs/en/hooks\#sessionend)  SessionEnd

Runs when a Claude Code session ends. Useful for cleanup tasks, logging session
statistics, or saving session state.The `reason` field in the hook input will be one of:

- `clear` \- Session cleared with /clear command
- `logout` \- User logged out
- `prompt_input_exit` \- User exited while prompt input was visible
- `other` \- Other exit reasons

## [​](https://code.claude.com/docs/en/hooks\#hook-input)  Hook Input

Hooks receive JSON data via stdin containing session information and
event-specific data:

Copy

Ask AI

```
{
  // Common fields
  session_id: string
  transcript_path: string  // Path to conversation JSON
  cwd: string              // The current working directory when the hook is invoked
  permission_mode: string  // Current permission mode: "default", "plan", "acceptEdits", "dontAsk", or "bypassPermissions"

  // Event-specific fields
  hook_event_name: string
  ...
}
```

### [​](https://code.claude.com/docs/en/hooks\#pretooluse-input)  PreToolUse Input

The exact schema for `tool_input` depends on the tool. Here are examples for commonly hooked tools.

#### [​](https://code.claude.com/docs/en/hooks\#bash-tool)  Bash tool

The Bash tool is the most commonly hooked tool for command validation:

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Bash",
  "tool_input": {
    "command": "psql -c 'SELECT * FROM users'",
    "description": "Query the users table",
    "timeout": 120000
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

| Field | Type | Description |
| --- | --- | --- |
| `command` | string | The shell command to execute |
| `description` | string | Optional description of what the command does |
| `timeout` | number | Optional timeout in milliseconds |
| `run_in_background` | boolean | Whether to run the command in background |

#### [​](https://code.claude.com/docs/en/hooks\#write-tool)  Write tool

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

| Field | Type | Description |
| --- | --- | --- |
| `file_path` | string | Absolute path to the file to write |
| `content` | string | Content to write to the file |

#### [​](https://code.claude.com/docs/en/hooks\#edit-tool)  Edit tool

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "old_string": "original text",
    "new_string": "replacement text"
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

| Field | Type | Description |
| --- | --- | --- |
| `file_path` | string | Absolute path to the file to edit |
| `old_string` | string | Text to find and replace |
| `new_string` | string | Replacement text |
| `replace_all` | boolean | Whether to replace all occurrences (default: false) |

#### [​](https://code.claude.com/docs/en/hooks\#read-tool)  Read tool

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PreToolUse",
  "tool_name": "Read",
  "tool_input": {
    "file_path": "/path/to/file.txt"
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

| Field | Type | Description |
| --- | --- | --- |
| `file_path` | string | Absolute path to the file to read |
| `offset` | number | Optional line number to start reading from |
| `limit` | number | Optional number of lines to read |

### [​](https://code.claude.com/docs/en/hooks\#posttooluse-input)  PostToolUse Input

The exact schema for `tool_input` and `tool_response` depends on the tool.

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "PostToolUse",
  "tool_name": "Write",
  "tool_input": {
    "file_path": "/path/to/file.txt",
    "content": "file content"
  },
  "tool_response": {
    "filePath": "/path/to/file.txt",
    "success": true
  },
  "tool_use_id": "toolu_01ABC123..."
}
```

### [​](https://code.claude.com/docs/en/hooks\#notification-input)  Notification Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "Notification",
  "message": "Claude needs your permission to use Bash",
  "notification_type": "permission_prompt"
}
```

### [​](https://code.claude.com/docs/en/hooks\#userpromptsubmit-input)  UserPromptSubmit Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "UserPromptSubmit",
  "prompt": "Write a function to calculate the factorial of a number"
}
```

### [​](https://code.claude.com/docs/en/hooks\#stop-input)  Stop Input

`stop_hook_active` is true when Claude Code is already continuing as a result of
a stop hook. Check this value or process the transcript to prevent Claude Code
from running indefinitely.

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "Stop",
  "stop_hook_active": true
}
```

### [​](https://code.claude.com/docs/en/hooks\#subagentstop-input)  SubagentStop Input

Triggered when a subagent finishes. The `transcript_path` is the main session’s transcript, while `agent_transcript_path` is the subagent’s own transcript stored in a nested `subagents/` folder.

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../abc123.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "SubagentStop",
  "stop_hook_active": false,
  "agent_id": "def456",
  "agent_transcript_path": "~/.claude/projects/.../abc123/subagents/agent-def456.jsonl"
}
```

### [​](https://code.claude.com/docs/en/hooks\#precompact-input)  PreCompact Input

For `manual`, `custom_instructions` comes from what the user passes into
`/compact`. For `auto`, `custom_instructions` is empty.

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "permission_mode": "default",
  "hook_event_name": "PreCompact",
  "trigger": "manual",
  "custom_instructions": ""
}
```

### [​](https://code.claude.com/docs/en/hooks\#setup-input)  Setup Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "Setup",
  "trigger": "init"
}
```

The `trigger` field will be either `"init"` (from `--init` or `--init-only`) or `"maintenance"` (from `--maintenance`).

### [​](https://code.claude.com/docs/en/hooks\#sessionstart-input)  SessionStart Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "SessionStart",
  "source": "startup",
  "model": "claude-sonnet-4-20250514"
}
```

The `source` field indicates how the session started: `"startup"` for new sessions, `"resume"` for resumed sessions, `"clear"` after `/clear`, or `"compact"` after compaction. The `model` field contains the model identifier when available. If you start Claude Code with `claude --agent <name>`, an `agent_type` field contains the agent name.

### [​](https://code.claude.com/docs/en/hooks\#subagentstart-input)  SubagentStart Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "SubagentStart",
  "agent_id": "agent-abc123",
  "agent_type": "Explore"
}
```

Triggered when a subagent is spawned. The `agent_id` field contains the unique identifier for the subagent, and `agent_type` contains the agent name (built-in agents like `"Bash"`, `"Explore"`, `"Plan"`, or custom agent names).

### [​](https://code.claude.com/docs/en/hooks\#sessionend-input)  SessionEnd Input

Copy

Ask AI

```
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "SessionEnd",
  "reason": "exit"
}
```

## [​](https://code.claude.com/docs/en/hooks\#hook-output)  Hook Output

There are two mutually exclusive ways for hooks to return output back to Claude Code. The output
communicates whether to block and any feedback that should be shown to Claude
and the user.

### [​](https://code.claude.com/docs/en/hooks\#simple:-exit-code)  Simple: Exit Code

Hooks communicate status through exit codes, stdout, and stderr:

- **Exit code 0**: Success. `stdout` is shown to the user in verbose mode
(ctrl+o), except for `UserPromptSubmit` and `SessionStart`, where stdout is
added to the context. JSON output in `stdout` is parsed for structured control
(see [Advanced: JSON Output](https://code.claude.com/docs/en/hooks#advanced-json-output)).
- **Exit code 2**: Blocking error. Only `stderr` is used as the error message
and fed back to Claude. The format is `[command]: {stderr}`. JSON in `stdout`
is **not** processed for exit code 2. See per-hook-event behavior below.
- **Other exit codes**: Non-blocking error. `stderr` is shown to the user in verbose mode (ctrl+o) with
format `Failed with non-blocking status code: {stderr}`. If `stderr` is empty,
it shows `No stderr output`. Execution continues.

Reminder: Claude Code does not see stdout if the exit code is 0, except for
the `UserPromptSubmit` hook where stdout is injected as context.

#### [​](https://code.claude.com/docs/en/hooks\#exit-code-2-behavior)  Exit Code 2 Behavior

| Hook Event | Behavior |
| --- | --- |
| `PreToolUse` | Blocks the tool call, shows stderr to Claude |
| `PermissionRequest` | Denies the permission, shows stderr to Claude |
| `PostToolUse` | Shows stderr to Claude (tool already ran) |
| `Notification` | N/A, shows stderr to user only |
| `UserPromptSubmit` | Blocks prompt processing, erases prompt, shows stderr to user only |
| `Stop` | Blocks stoppage, shows stderr to Claude |
| `SubagentStop` | Blocks stoppage, shows stderr to Claude subagent |
| `PreCompact` | N/A, shows stderr to user only |
| `Setup` | N/A, shows stderr to user only |
| `SessionStart` | N/A, shows stderr to user only |
| `SessionEnd` | N/A, shows stderr to user only |

### [​](https://code.claude.com/docs/en/hooks\#advanced:-json-output)  Advanced: JSON Output

Hooks can return structured JSON in `stdout` for more sophisticated control.

JSON output is only processed when the hook exits with code 0. If your hook
exits with code 2 (blocking error), `stderr` text is used directly—any JSON in `stdout`
is ignored. For other non-zero exit codes, only `stderr` is shown to the user in verbose mode (ctrl+o).

#### [​](https://code.claude.com/docs/en/hooks\#common-json-fields)  Common JSON Fields

All hook types can include these optional fields:

Copy

Ask AI

```
{
  "continue": true, // Whether Claude should continue after hook execution (default: true)
  "stopReason": "string", // Message shown when continue is false

  "suppressOutput": true, // Hide stdout from transcript mode (default: false)
  "systemMessage": "string" // Optional warning message shown to the user
}
```

If `continue` is false, Claude stops processing after the hooks run.

- For `PreToolUse`, this is different from `"permissionDecision": "deny"`, which
only blocks a specific tool call and provides automatic feedback to Claude.
- For `PostToolUse`, this is different from `"decision": "block"`, which
provides automated feedback to Claude.
- For `UserPromptSubmit`, this prevents the prompt from being processed.
- For `Stop` and `SubagentStop`, this takes precedence over any
`"decision": "block"` output.
- In all cases, `"continue" = false` takes precedence over any
`"decision": "block"` output.

`stopReason` accompanies `continue` with a reason shown to the user, not shown
to Claude.

#### [​](https://code.claude.com/docs/en/hooks\#pretooluse-decision-control)  `PreToolUse` Decision Control

`PreToolUse` hooks can control whether a tool call proceeds.

- `"allow"` bypasses the permission system. `permissionDecisionReason` is shown
to the user but not to Claude.
- `"deny"` prevents the tool call from executing. `permissionDecisionReason` is
shown to Claude.
- `"ask"` asks the user to confirm the tool call in the UI.
`permissionDecisionReason` is shown to the user but not to Claude.

Additionally, hooks can modify tool inputs before execution using `updatedInput`:

- `updatedInput` modifies the tool’s input parameters before the tool executes
- Combine with `"permissionDecision": "allow"` to modify the input and auto-approve the tool call
- Combine with `"permissionDecision": "ask"` to modify the input and show it to the user for confirmation

Hooks can also provide context to Claude using `additionalContext`:

- `"hookSpecificOutput.additionalContext"` adds a string to Claude’s context before the tool executes.

Copy

Ask AI

```
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "permissionDecisionReason": "My reason here",
    "updatedInput": {
      "field_to_modify": "new value"
    },
    "additionalContext": "Current environment: production. Proceed with caution."
  }
}
```

The `decision` and `reason` fields are deprecated for PreToolUse hooks.
Use `hookSpecificOutput.permissionDecision` and
`hookSpecificOutput.permissionDecisionReason` instead. The deprecated fields
`"approve"` and `"block"` map to `"allow"` and `"deny"` respectively.

#### [​](https://code.claude.com/docs/en/hooks\#permissionrequest-decision-control)  `PermissionRequest` Decision Control

`PermissionRequest` hooks can allow or deny permission requests shown to the user.

- For `"behavior": "allow"` you can also optionally pass in an `"updatedInput"` that modifies the tool’s input parameters before the tool executes.
- For `"behavior": "deny"` you can also optionally pass in a `"message"` string that tells the model why the permission was denied, and a boolean `"interrupt"` which will stop Claude.

Copy

Ask AI

```
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow",
      "updatedInput": {
        "command": "npm run lint"
      }
    }
  }
}
```

#### [​](https://code.claude.com/docs/en/hooks\#posttooluse-decision-control)  `PostToolUse` Decision Control

`PostToolUse` hooks can provide feedback to Claude after tool execution.

- `"block"` automatically prompts Claude with `reason`.
- `undefined` does nothing. `reason` is ignored.
- `"hookSpecificOutput.additionalContext"` adds context for Claude to consider.

Copy

Ask AI

```
{
  "decision": "block" | undefined,
  "reason": "Explanation for decision",
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "Additional information for Claude"
  }
}
```

#### [​](https://code.claude.com/docs/en/hooks\#userpromptsubmit-decision-control)  `UserPromptSubmit` Decision Control

`UserPromptSubmit` hooks can control whether a user prompt is processed and add context.**Adding context (exit code 0):**
There are two ways to add context to the conversation:

1. **Plain text stdout** (simpler): Any non-JSON text written to stdout is added
as context. This is the easiest way to inject information.
2. **JSON with `additionalContext`** (structured): Use the JSON format below for
more control. The `additionalContext` field is added as context.

Both methods work with exit code 0. Plain stdout is shown as hook output in
the transcript; `additionalContext` is added more discretely.**Blocking prompts:**

- `"decision": "block"` prevents the prompt from being processed. The submitted
prompt is erased from context. `"reason"` is shown to the user but not added
to context.
- `"decision": undefined` (or omitted) allows the prompt to proceed normally.

Copy

Ask AI

```
{
  "decision": "block" | undefined,
  "reason": "Explanation for decision",
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "My additional context here"
  }
}
```

The JSON format isn’t required for simple use cases. To add context, you can print plain text to stdout with exit code 0. Use JSON when you need to
block prompts or want more structured control.

#### [​](https://code.claude.com/docs/en/hooks\#stop/subagentstop-decision-control)  `Stop`/`SubagentStop` Decision Control

`Stop` and `SubagentStop` hooks can control whether Claude must continue.

- `"block"` prevents Claude from stopping. You must populate `reason` for Claude
to know how to proceed.
- `undefined` allows Claude to stop. `reason` is ignored.

Copy

Ask AI

```
{
  "decision": "block" | undefined,
  "reason": "Must be provided when Claude is blocked from stopping"
}
```

#### [​](https://code.claude.com/docs/en/hooks\#setup-decision-control)  `Setup` Decision Control

`Setup` hooks allow you to load context and configure the environment during repository initialization or maintenance.

- `"hookSpecificOutput.additionalContext"` adds the string to the context.
- Multiple hooks’ `additionalContext` values are concatenated.
- Setup hooks have access to `CLAUDE_ENV_FILE` for persisting environment variables.

Copy

Ask AI

```
{
  "hookSpecificOutput": {
    "hookEventName": "Setup",
    "additionalContext": "Repository initialized with custom configuration"
  }
}
```

#### [​](https://code.claude.com/docs/en/hooks\#sessionstart-decision-control)  `SessionStart` Decision Control

`SessionStart` hooks allow you to load in context at the start of a session.

- `"hookSpecificOutput.additionalContext"` adds the string to the context.
- Multiple hooks’ `additionalContext` values are concatenated.

Copy

Ask AI

```
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "My additional context here"
  }
}
```

#### [​](https://code.claude.com/docs/en/hooks\#sessionend-decision-control)  `SessionEnd` Decision Control

`SessionEnd` hooks run when a session ends. They cannot block session termination
but can perform cleanup tasks.

#### [​](https://code.claude.com/docs/en/hooks\#exit-code-example:-bash-command-validation)  Exit Code Example: Bash Command Validation

Copy

Ask AI

```
#!/usr/bin/env python3
import json
import re
import sys

# Define validation rules as a list of (regex pattern, message) tuples
VALIDATION_RULES = [\
    (\
        r"\bgrep\b(?!.*\|)",\
        "Use 'rg' (ripgrep) instead of 'grep' for better performance and features",\
    ),\
    (\
        r"\bfind\s+\S+\s+-name\b",\
        "Use 'rg --files | rg pattern' or 'rg --files -g pattern' instead of 'find -name' for better performance",\
    ),\
]

def validate_command(command: str) -> list[str]:
    issues = []
    for pattern, message in VALIDATION_RULES:
        if re.search(pattern, command):
            issues.append(message)
    return issues

try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

tool_name = input_data.get("tool_name", "")
tool_input = input_data.get("tool_input", {})
command = tool_input.get("command", "")

if tool_name != "Bash" or not command:
    sys.exit(1)

# Validate the command
issues = validate_command(command)

if issues:
    for message in issues:
        print(f"• {message}", file=sys.stderr)
    # Exit code 2 blocks tool call and shows stderr to Claude
    sys.exit(2)
```

#### [​](https://code.claude.com/docs/en/hooks\#json-output-example:-userpromptsubmit-to-add-context-and-validation)  JSON Output Example: UserPromptSubmit to Add Context and Validation

For `UserPromptSubmit` hooks, you can inject context using either method:

- **Plain text stdout** with exit code 0: Simplest approach, prints text
- **JSON output** with exit code 0: Use `"decision": "block"` to reject prompts,
or `additionalContext` for structured context injection

Remember: Exit code 2 only uses `stderr` for the error message. To block using
JSON (with a custom reason), use `"decision": "block"` with exit code 0.

Copy

Ask AI

```
#!/usr/bin/env python3
import json
import sys
import re
import datetime

# Load input from stdin
try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

prompt = input_data.get("prompt", "")

# Check for sensitive patterns
sensitive_patterns = [\
    (r"(?i)\b(password|secret|key|token)\s*[:=]", "Prompt contains potential secrets"),\
]

for pattern, message in sensitive_patterns:
    if re.search(pattern, prompt):
        # Use JSON output to block with a specific reason
        output = {
            "decision": "block",
            "reason": f"Security policy violation: {message}. Please rephrase your request without sensitive information."
        }
        print(json.dumps(output))
        sys.exit(0)

# Add current time to context
context = f"Current time: {datetime.datetime.now()}"
print(context)

"""
The following is also equivalent:
print(json.dumps({
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": context,
  },
}))
"""

# Allow the prompt to proceed with the additional context
sys.exit(0)
```

#### [​](https://code.claude.com/docs/en/hooks\#json-output-example:-pretooluse-with-approval)  JSON Output Example: PreToolUse with Approval

Copy

Ask AI

```
#!/usr/bin/env python3
import json
import sys

# Load input from stdin
try:
    input_data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"Error: Invalid JSON input: {e}", file=sys.stderr)
    sys.exit(1)

tool_name = input_data.get("tool_name", "")
tool_input = input_data.get("tool_input", {})

# Example: Auto-approve file reads for documentation files
if tool_name == "Read":
    file_path = tool_input.get("file_path", "")
    if file_path.endswith((".md", ".mdx", ".txt", ".json")):
        # Use JSON output to auto-approve the tool call
        output = {
            "decision": "approve",
            "reason": "Documentation file auto-approved",
            "suppressOutput": True  # Don't show in verbose mode
        }
        print(json.dumps(output))
        sys.exit(0)

# For other cases, let the normal permission flow proceed
sys.exit(0)
```

## [​](https://code.claude.com/docs/en/hooks\#working-with-mcp-tools)  Working with MCP Tools

Claude Code hooks work seamlessly with
[Model Context Protocol (MCP) tools](https://code.claude.com/docs/en/mcp). When MCP servers
provide tools, they appear with a special naming pattern that you can match in
your hooks.

### [​](https://code.claude.com/docs/en/hooks\#mcp-tool-naming)  MCP Tool Naming

MCP tools follow the pattern `mcp__<server>__<tool>`, for example:

- `mcp__memory__create_entities` \- Memory server’s create entities tool
- `mcp__filesystem__read_file` \- Filesystem server’s read file tool
- `mcp__github__search_repositories` \- GitHub server’s search tool

### [​](https://code.claude.com/docs/en/hooks\#configuring-hooks-for-mcp-tools)  Configuring Hooks for MCP Tools

You can target specific MCP tools or entire MCP servers:

Copy

Ask AI

```
{
  "hooks": {
    "PreToolUse": [\
      {\
        "matcher": "mcp__memory__.*",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "echo 'Memory operation initiated' >> ~/mcp-operations.log"\
          }\
        ]\
      },\
      {\
        "matcher": "mcp__.*__write.*",\
        "hooks": [\
          {\
            "type": "command",\
            "command": "/home/user/scripts/validate-mcp-write.py"\
          }\
        ]\
      }\
    ]
  }
}
```

## [​](https://code.claude.com/docs/en/hooks\#examples)  Examples

For practical examples including code formatting, notifications, and file protection, see [More Examples](https://code.claude.com/docs/en/hooks-guide#more-examples) in the get started guide.

## [​](https://code.claude.com/docs/en/hooks\#security-considerations)  Security Considerations

### [​](https://code.claude.com/docs/en/hooks\#disclaimer)  Disclaimer

**USE AT YOUR OWN RISK**: Claude Code hooks execute arbitrary shell commands on
your system automatically. By using hooks, you acknowledge that:

- You are solely responsible for the commands you configure
- Hooks can modify, delete, or access any files your user account can access
- Malicious or poorly written hooks can cause data loss or system damage
- Anthropic provides no warranty and assumes no liability for any damages
resulting from hook usage
- You should thoroughly test hooks in a safe environment before production use

Always review and understand any hook commands before adding them to your
configuration.

### [​](https://code.claude.com/docs/en/hooks\#security-best-practices)  Security Best Practices

Here are some key practices for writing more secure hooks:

1. **Validate and sanitize inputs** \- Never trust input data blindly
2. **Always quote shell variables** \- Use `"$VAR"` not `$VAR`
3. **Block path traversal** \- Check for `..` in file paths
4. **Use absolute paths** \- Specify full paths for scripts (use
“$CLAUDE\_PROJECT\_DIR” for the project path)
5. **Skip sensitive files** \- Avoid `.env`, `.git/`, keys, etc.

### [​](https://code.claude.com/docs/en/hooks\#configuration-safety)  Configuration Safety

Direct edits to hooks in settings files don’t take effect immediately. Claude
Code:

1. Captures a snapshot of hooks at startup
2. Uses this snapshot throughout the session
3. Warns if hooks are modified externally
4. Requires review in `/hooks` menu for changes to apply

This prevents malicious hook modifications from affecting your current session.

## [​](https://code.claude.com/docs/en/hooks\#hook-execution-details)  Hook Execution Details

- **Timeout**: 60-second execution limit by default, configurable per command.
  - A timeout for an individual command does not affect the other commands.
- **Parallelization**: All matching hooks run in parallel
- **Deduplication**: Multiple identical hook commands are deduplicated automatically
- **Environment**: Runs in current directory with Claude Code’s environment
  - The `CLAUDE_PROJECT_DIR` environment variable is available and contains the
    absolute path to the project root directory (where Claude Code was started)
  - The `CLAUDE_CODE_REMOTE` environment variable indicates whether the hook is running in a remote (web) environment (`"true"`) or local CLI environment (not set or empty). Use this to run different logic based on execution context.
- **Input**: JSON via stdin
- **Output**:
  - PreToolUse/PermissionRequest/PostToolUse/Stop/SubagentStop: Progress shown in verbose mode (ctrl+o)
  - Notification/SessionEnd: Logged to debug only (`--debug`)
  - UserPromptSubmit/SessionStart/Setup: stdout added as context for Claude

## [​](https://code.claude.com/docs/en/hooks\#debugging)  Debugging

### [​](https://code.claude.com/docs/en/hooks\#basic-troubleshooting)  Basic Troubleshooting

If your hooks aren’t working:

1. **Check configuration** \- Run `/hooks` to see if your hook is registered
2. **Verify syntax** \- Ensure your JSON settings are valid
3. **Test commands** \- Run hook commands manually first
4. **Check permissions** \- Make sure scripts are executable
5. **Review logs** \- Use `claude --debug` to see hook execution details

Common issues:

- **Quotes not escaped** \- Use `\"` inside JSON strings
- **Wrong matcher** \- Check tool names match exactly (case-sensitive)
- **Command not found** \- Use full paths for scripts

### [​](https://code.claude.com/docs/en/hooks\#advanced-debugging)  Advanced Debugging

For complex hook issues:

1. **Inspect hook execution** \- Use `claude --debug` to see detailed hook
execution
2. **Validate JSON schemas** \- Test hook input/output with external tools
3. **Check environment variables** \- Verify Claude Code’s environment is correct
4. **Test edge cases** \- Try hooks with unusual file paths or inputs
5. **Monitor system resources** \- Check for resource exhaustion during hook
execution
6. **Use structured logging** \- Implement logging in your hook scripts

### [​](https://code.claude.com/docs/en/hooks\#debug-output-example)  Debug Output Example

Use `claude --debug` to see hook execution details:

Copy

Ask AI

```
[DEBUG] Executing hooks for PostToolUse:Write
[DEBUG] Getting matching hook commands for PostToolUse with query: Write
[DEBUG] Found 1 hook matchers in settings
[DEBUG] Matched 1 hooks for query "Write"
[DEBUG] Found 1 hook commands to execute
[DEBUG] Executing hook command: <Your command> with timeout 60000ms
[DEBUG] Hook command completed with status 0: <Your stdout>
```

Progress messages appear in verbose mode (ctrl+o) showing:

- Which hook is running
- Command being executed
- Success/failure status
- Output or error messages

Was this page helpful?

YesNo

[Checkpointing](https://code.claude.com/docs/en/checkpointing) [Plugins reference](https://code.claude.com/docs/en/plugins-reference)

⌘I