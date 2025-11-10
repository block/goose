---
title: Subagents
sidebar_position: 1
sidebar_label: Subagents
---

Subagents are independent instances that execute tasks while keeping your main conversation clean and focused. They bring process isolation and context preservation by offloading work to separate instances. Think of them as temporary assistants that handle specific jobs without cluttering your chat with tool execution details.

:::warning
Subagents are an experimental feature in active development. Behavior and configuration may change in future releases.
:::

## How to Use Subagents

To use subagents, ask Goose to delegate tasks using natural language. Goose automatically decides when to spawn subagents and handles their lifecycle. You can:

1. **Request specialized help**: "Use a code reviewer to analyze this function for security issues"
2. **Reference specific recipes**: "Use the 'security-auditor' recipe to scan this endpoint"  
3. **Run parallel tasks**: "Create three HTML templates simultaneously"
4. **Delegate complex work**: "Research quantum computing developments and summarize findings"

You can run multiple subagents sequentially or in parallel.

| Type | Description | Trigger Keywords | Example |
|------|-------------|------------------|---------|
| **Sequential** (Default) | Tasks execute one after another | "first...then", "after" | `"First analyze the code, then generate documentation"` |
| **Parallel** | Tasks execute simultaneously | "parallel", "simultaneously", "at the same time", "concurrently" | `"Create three HTML templates in parallel"` |

:::info
If a subagent fails or times out (5-minute default), you will receive no output from that subagent. For parallel execution, if any subagent fails, you get results only from the successful ones.
:::


## Prerequisites
To use subagents, you need to enable alpha features first. You can do this by setting an [environment variable](/docs/guides/environment-variables#experimental-features) or adding it to your [config file](/docs/guides/config-file#experimental-features):

**Environment Variable:**
```bash
export ALPHA_FEATURES=true
```

**Config File** (`~/.config/goose/config.yaml`):
```yaml
ALPHA_FEATURES: true
```

## Internal Subagents

Internal subagents spawn Goose instances to handle tasks using your current session's context and extensions. There are two ways to configure and execute internal subagents:

1. **Direct Prompts** - Quick, one-off tasks using natural language instructions
2. **Recipes** - Reusable, structured configurations for specialized subagent behavior

### Direct Prompts
Direct prompts provided for one-off tasks using natural language prompts. The main agent automatically configures the subagent based on your request.

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

### Recipes
Use [recipe](/docs/guides/recipes/) files to define specific instructions, extensions, and behavior for subagents. Recipes provide reusable configurations that can be shared and referenced by name.

**Creating a Recipe File**

`code-reviewer.yaml`

```yaml
id: code-reviewer
version: 1.0.0
title: "Code Review Assistant"
description: "Specialized subagent for code quality and security analysis"
instructions: |
  You are a code review assistant. Analyze code and provide feedback on:
  - Code quality and readability
  - Security vulnerabilities
  - Performance issues
  - Best practices adherence
activities:
  - Analyze code structure
  - Check for security issues
  - Review performance patterns
extensions:
  - type: builtin
    name: developer
    display_name: Developer
    timeout: 300
    bundled: true
parameters:
  - key: focus_area
    input_type: string
    requirement: optional
    description: "Specific area to focus on (security, performance, readability, etc.)"
    default: "general"
prompt: |
  Please review the following code focusing on {{focus_area}} aspects.
  Provide specific, actionable feedback with examples.
```

**Place your recipe file where Goose can find it**
- Set [`GOOSE_RECIPE_PATH`](/docs/guides/recipes/recipe-reference#recipe-location) environment variable to your recipe directory
- Or place it in your current working directory

**Goose Prompt**
```
Use the "code-reviewer" recipe to analyze the authentication feature I implemented
```

**Goose Output**
```
I'll use your code-reviewer recipe to create a specialized subagent for this analysis.

🤖 Subagent created using code-reviewer recipe
💭 Analyzing authentication function for security issues...
🔧 Scanning code structure and patterns...
⚠️  Security vulnerabilities detected!

## Code Review Results

### Critical Issues Found:
1. **SQL Injection Vulnerability**: Direct string interpolation in SQL query
2. **Missing Password Hashing**: Plain text password comparison

### Recommendations:
- Use parameterized queries or ORM
- Implement proper password hashing (bcrypt, scrypt)
- Add input validation and sanitization
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

```md
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

## Configuration

Subagents are automatically have the following pre-configured settings, but you can override any defaults using natural language in your prompts.

### Default Settings
| Parameter | Default | Source |
|-----------|---------|--------|
| **Max Turns** | 10 | Built-in default |
| **Timeout** | 5 minutes | Built-in default |

### Customizing Settings in Prompts

You can override any default by including the setting in your natural language request:

**Examples:**

```
"Use subagents to write a test and documentation, but make them timeout after 7 minutes"
```

```
""Use subagents to analyze code, limit each to 5 turns""
```

## Security Constraints

Subagents operate with restricted tool access to ensure safe execution and prevent interference with the main session.

### Allowed Operations

Subagents have access to these safe operations:

- **Extension discovery**: Search for available extensions to understand what tools are available
- **Resource access**: Read and list resources from enabled extensions for context
- **Extension tools**: Use tools from extensions specified in recipes or inherited from the parent session

### Restricted Operations

The following operations are blocked to ensure subagents remain focused on their assigned tasks without affecting the broader system state:

- **Subagent spawning**: Cannot create additional subagents to prevent infinite recursion
- **Extension management**: Cannot enable, disable, or modify extensions to avoid conflicts with the main session
- **Schedule management**: Cannot create, modify, or delete scheduled tasks to prevent interference with parent workflows

:::info
Subagents can browse extensions for suggestions but cannot enable them to avoid modifying the parent session.
:::
