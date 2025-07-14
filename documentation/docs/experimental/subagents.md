---
title: Subagents
sidebar_position: 3
sidebar_label: Subagents
---

Subagents are independent instances that execute tasks while keeping your main conversation clean and focused. They bring process isolation and context preservation by offloading work to separate instances. Think of them as temporary assistants that handle specific jobs without cluttering your chat with tool execution details.

:::warning
Subagents are an experimental feature in active development. Behavior and configuration may change in future releases.
:::

:::info Prerequisites
To use subagents, you need to enable alpha features first. You can do this by setting an [environment variable](/docs/guides/environment-variables#experimental-features) or adding it to your [config file](/docs/guides/config-file#experimental-features):

**Environment Variable:**
```bash
export ALPHA_FEATURES=true
```

**Config File** (`~/.config/goose/config.yaml`):
```yaml
ALPHA_FEATURES: true
```
:::

## Execution Types

You can run multiple subagents sequentially or in parallel.

| Type | Description | Trigger Keywords | Example |
|------|-------------|------------------|---------|
| **Sequential** (Default) | Tasks execute one after another | "first...then", "after" | `"First analyze the code, then generate documentation"` |
| **Parallel** | Tasks execute simultaneously | "parallel", "simultaneously", "at the same time", "concurrently" | `"Create three HTML templates in parallel"` |

## Internal Subagents

Internal subagents spawn Goose instances to handle tasks using your current session's context and extensions.

### Direct Instruction
Direct instructions provided for one-off tasks using natural language prompts. The main agent automatically configures the subagent based on your request.

**Goose Prompt:**
```
"Use 2 subagents to create hello.html with 'Hello World' content and goodbye.html with 'Goodbye World' content in parallel"
```

**Tool Output:**
```json
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

### Recipe Configuration
Use [recipe](/docs/guides/recipes/) files to define specific instructions, extensions, and behavior for the subagent.

**Recipe File**: (`create-docs.yaml`)
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

**Command:**
```bash
goose run --recipe create-docs.yaml
```

**Tool Output:**
```json
{
  "execution_summary": {
    "total_tasks": 1,
    "successful_tasks": 1,
    "failed_tasks": 0,
    "execution_time_seconds": 45.8
  },
  "task_results": [
    {
      "task_id": "create_docs_recipe",
      "status": "success",
      "result": "Successfully generated project documentation: README.md updated, API docs created, CHANGELOG.md refreshed"
    }
  ]
}
```

## External Subagents

External subagents let you bring in AI agents from other providers and platforms, enabling Goose to coordinate and integrate your workflow with the broader ecosystem. In the below example, we use Codex as a subagent by running it as an MCP server:

**[Goose Configuration File](/docs/guides/config-file)** (`.~/.config/goose/config.yaml `):
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

**External Tool Configuration** (`~/.codex/config.toml`):
```toml
# Use fast model for quick responses
# model = "codex-mini-latest"
disable_response_storage = true

# Never prompt for approval - auto-execute
approval_policy = "never"

[sandbox]
mode = "workspace-write"
```

**Goose Prompt:**
```
"Use the codex subagent to analyze my codebase structure and identify the main components"
```

**Goose Output:**
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

## Suggested Use Cases

**Independent Operations**
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

## Configuration

Goose automatically configures subagents by looking at environment variables, user prompts, and recipe files to determine the best settings for each task.


| Parameter | Description | Default | Example |
|-----------|-------------|---------|---------|
| **Instructions** | Task-specific behavior and context | Auto-generated from user request | `"You are a code reviewer focusing on security"` |
| **Max Turns** | Conversation limit before auto-completion | 10 | Set higher for complex tasks |
| **Timeout** | Maximum execution time | 5 minutes | Prevents runaway processes |
| **Extensions** | Available tools and capabilities | Inherits from main session | Recipe can specify subset |

