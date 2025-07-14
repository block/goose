---
title: Subagents
sidebar_position: 3
sidebar_label: Subagents
---

Subagents are independent instances of Goosse that execute tasks while keeping your main conversation clean and focused. They bring process isolation and context preservation by offloading work to separate instances. Think of them as temporary assistants that handle specific jobs without cluttering your chat with tool execution details.

:::warning
Subagents are an experimental feature in active development. Behavior and configuration may change in future releases.
:::

## Types of Subagents

### Internal Subagents
Internal subagents spawn Goose instances to handle tasks using your current session's context and extensions.

**Example: Creating HTML Files**
```
User: "Use 2 subagents to create hello.html with 'Hello World' content and goodbye.html with 'Goodbye World' content in parallel"

Tool Output:
{
  "execution_summary": {
    "total_tasks": 2,
    "successful_tasks": 2,
    "failed_tasks": 0,
    "execution_time_seconds": 16.2
  },
  "task_results": [
    {
      "task_id": "create_hello_html",
      "status": "success",
      "result": "Successfully created hello.html with Hello World content"
    },
    {
      "task_id": "create_goodbye_html", 
      "status": "success",
      "result": "Successfully created goodbye.html with Goodbye World content"
    }
  ]
}
```

### External Subagents
External subagents let you bring in AI agents from other providers and platforms, like Claude Code and OpenAI Codex, enabling Goose to coordinate and integrate your workflow with the broader AI ecosystem.

**Example: Code Analysis with Codex**

Goose Prompt:
```
"Use the codex subagent to analyze my codebase structure and identify the main components"
```

Goose Output:
```
Based on my analysis of your codebase, here are the main components:

1. **Core Agent System** (`crates/goose/src/agents/`)
   - Agent orchestration and session management
   - Tool execution framework
   - Extension system integration

2. **CLI Interface** (`crates/goose-cli/`)
   - Command-line interface and session handling
   - Configuration management

3. **Server Components** (`crates/goose-server/`)
   - HTTP API endpoints
   - WebSocket communication for real-time interaction

4. **Desktop UI** (`ui/desktop/`)
   - Electron-based desktop application
   - TypeScript frontend with React components

The architecture follows a modular design with clear separation between the core agent logic, interfaces, and UI components.
```

## Configuration

### Internal Subagents

#### Recipe Mode
Uses YAML recipe files that define specific instructions, extensions, and behavior.

**Example Recipe File** (`create-docs.yaml`):
```yaml
name: "Documentation Generator"
description: "Generate project documentation"
extensions:
  - developer
  - memory
instructions: |
  1. Scan the project structure
  2. Generate README.md with project overview
  3. Create API documentation from code comments
  4. Update CHANGELOG.md with recent changes
```

**Usage**

Goose Prompt:
```
"Run the create-docs recipe as a subagent"
```

#### Ad-hoc Mode
Direct instructions provided for one-off tasks.

**Example**

Goose Prompt: 

```
Use subagents to run the following tasks in parallel
    - "Create a simple landing page HTML file"
    - "Generate a Python script that processes CSV files"
    - "Write unit tests for the authentication module"
```

### External Subagents

**Codex Configuration** (in your Goose config):
```yaml
  subagent:
    args:
    - mcp
    bundled: true
    cmd: codex
    description: OpenAI Codex CLI Sub-agent
    enabled: true
    env_keys:
    - OPENAI_API_KEY
    envs: {}
    name: subagent
    timeout: 300
    type: stdio
```

**Codex CLI Configuration** (`~/.codex/config.toml`):
```toml
# Use fast model for quick responses
# model = "codex-mini-latest"
disable_response_storage = true

# Never prompt for approval - auto-execute
approval_policy = "never"

[sandbox]
mode = "workspace-write"
```

## Execution Types

### Sequential (Default)
Tasks execute one after another in order.

**How to trigger:**
- Default behavior - no special keywords needed
- Use phrases like "first do X, then do Y"
- Any request without parallel keywords

**Example:**
```
User: "First analyze the code, then generate documentation based on the analysis"
```

### Parallel
Tasks execute simultaneously.

**How to trigger:**
- Use keywords: "parallel", "simultaneously", "at the same time", "concurrently"
- Explicitly request concurrent execution

**Example:**
```
User: "Create three different HTML templates in parallel"
```

## Suggested Use Cases

**Simple, Independent Operations**
- Creating multiple files with similar structure
- Basic data processing tasks
- File transformations and generations

**Context Preservation**
- Complex analysis that generates lots of tool output
- Specialized tasks better handled by dedicated agents
- Keeping main conversation focused on high-level decisions

**Process Isolation**
- Tasks that might fail without affecting main workflow
- Operations requiring different configurations
- Experimental or exploratory work

## Lifecycle and Cleanup

Subagents are temporary instances that exist only for task execution. After the task is completed, no manual intervention is needed for cleanup.

:::info
If a subagent fails or times out (5-minute default), you receive no output from that subagent. For parallel execution, if any subagent fails, you get results only from the successful ones.
:::
