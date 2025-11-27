---
sidebar_position: 45
title: CLI Providers
sidebar_label: CLI Providers
description: Use Claude Code, Cursor Agent, or Gemini CLI subscriptions in goose
---

# CLI Providers

goose can make use of pass-through providers that integrate with existing CLI tools from Anthropic, Cursor, and Google. These providers allow you to use your existing Claude Code, Cursor Agent, and Google Gemini CLI subscriptions through goose's interface, adding session management, persistence, and workflow integration capabilities to these tools.

:::warning Limitations
These providers don’t fully support all goose features, may have platform or capability limitations, and can sometimes require advanced debugging if issues arise. They’re included here purely as a convenience.
:::

## Why Use CLI Providers?

CLI providers are useful if you:

- already have a Claude Code, Cursor, or Google Gemini CLI subscription and want to use it through goose instead of paying per token
- need session persistence to save, resume, and export conversation history
- want to use goose recipes and scheduled tasks to create repeatable workflows
- prefer unified commands across different AI providers
- want to [use multiple models together](#combining-with-other-models) in your tasks 

### Benefits

#### Session Management
- **Persistent conversations**: Save and resume sessions across restarts
- **Export capabilities**: Export conversation history and artifacts
- **Session organization**: Manage multiple conversation threads

#### Workflow Integration  
- **Recipe compatibility**: Use CLI providers in automated goose recipes
- **Scheduling support**: Include in scheduled tasks and workflows
- **Hybrid configurations**: Combine with LLM providers using lead/worker patterns

#### Interface Consistency
- **Unified commands**: Use the same `goose session` interface across all providers
- **Consistent configuration**: Manage all providers through goose's configuration system

:::warning Extensions
CLI providers do **not** give you access to goose's extension ecosystem (MCP servers, third-party integrations, etc.). They use their own built-in tools to prevent conflicts. If you need goose's extensions, use standard [API providers](/docs/getting-started/providers#available-providers) instead.
:::


## Available CLI Providers

### Claude Code

The Claude Code provider integrates with Anthropic's [Claude CLI tool](https://claude.ai/cli), allowing you to use Claude models through your existing Claude Code subscription.

**Features:**
- Uses Claude's latest models
- 200,000 token context limit
- Automatic filtering of goose extensions from system prompts (since Claude Code has its own tool ecosystem)
- JSON output parsing for structured responses

**Requirements:**
- Claude CLI tool installed and configured
- Active Claude Code subscription
- CLI tool authenticated with your Anthropic account

### Cursor Agent

The Cursor provider integrates with Cursor's [CLI agent](https://docs.cursor.com/en/cli/installation), providing access to through your existing subscription.

**Features:**

- integrates with Cursor Agent CLI coding tasks.
- ideal for code-related workflows and file interactions.

**Requirements:**

- cursor-agent tool installed and configured.
- CLI tool authenticated.

### Gemini CLI

The Gemini CLI provider integrates with Google's [Gemini CLI tool](https://ai.google.dev/gemini-api/docs), providing access to Gemini models through your Google AI subscription.

**Features:**
- 1,000,000 token context limit

**Requirements:**
- Gemini CLI tool installed and configured
- CLI tool authenticated with your Google account

## Setup Instructions

### Claude Code

1. **Install Claude CLI Tool**
   
   Follow the [installation instructions for Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) to install and configure the Claude CLI tool.

2. **Authenticate with Claude**
   
   Ensure your Claude CLI is authenticated and working

3. **Configure goose**
   
   Set the provider environment variable:
   ```bash
   export GOOSE_PROVIDER=claude-code
   ```
   
   Or configure through the goose CLI using `goose configure`:

   ```bash
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◇  Which model provider should we use?
   │  Claude Code 
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```
## Cursor Agent

1. **Install Cursor agent Tool**

   Follow the [installation instructions for Cursor Agent](https://docs.cursor.com/en/cli/installation) to install and configure the cursor agent tool.

2. **Authenticate with Cursor**

   Ensure your Cursor Agent is authenticated and working

3. **Configure goose**

   Set the provider environment variable:

   ```bash
   export goose_provider=cursor-agent
   ```

   Or configure through the goose CLI using `goose configure`:

   ```bash
   ┌   goose-configure
   │
   ◇  What would you like to configure?
   │  configure providers
   │
   ◇  Which model provider should we use?
   │  Cursor Agent
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```

### Gemini CLI

1. **Install Gemini CLI Tool**
   
   Follow the [installation instructions for Gemini CLI](https://blog.google/technology/developers/introducing-gemini-cli-open-source-ai-agent/) to install and configure the Gemini CLI tool.

2. **Authenticate with Google**
   
   Ensure your Gemini CLI is authenticated and working.

3. **Configure goose**
   
   Set the provider environment variable:
   ```bash
   export GOOSE_PROVIDER=gemini-cli
   ```
   
   Or configure through the goose CLI using `goose configure`:

   ```bash
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Configure Providers 
   │
   ◇  Which model provider should we use?
   │  Gemini CLI 
   │
   ◇  Model fetch complete
   │
   ◇  Enter a model from that provider:
   │  default
   ```

## Usage Examples

### Basic Usage

Once configured, you can start a goose session using these providers just like any others:

```bash
goose session
```

### Combining with Other Models

CLI providers work well in combination with other models using goose's [lead/worker pattern](/docs/tutorials/lead-worker):

```bash
# Use Claude Code as lead model, GPT-4o as worker
export GOOSE_LEAD_PROVIDER=claude-code
export GOOSE_PROVIDER=openai
export GOOSE_MODEL=gpt-4o
export GOOSE_LEAD_MODEL=default

goose session
```

## Configuration Options

### Claude Code Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GOOSE_PROVIDER` | Set to `claude-code` to use this provider | None |
| `CLAUDE_CODE_COMMAND` | Path to the Claude CLI command | `claude` |


### Cursor Agent Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GOOSE_PROVIDER` | Set to `cursor-agent` to use this provider | None |
| `CURSOR_AGENT_COMMAND` | Path to the Cursor Agent command | `cursor-agent` |

### Gemini CLI Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GOOSE_PROVIDER` | Set to `gemini-cli` to use this provider | None |
| `GEMINI_CLI_COMMAND` | Path to the Gemini CLI command | `gemini` |

## How It Works

### System Prompt Filtering

The CLI providers automatically filter out goose's extension information from system prompts since these CLI tools have their own tool ecosystems. This prevents conflicts and ensures clean interaction with the underlying CLI tools.

### Message Translation

- **Claude Code**: Converts goose messages to Claude's JSON message format, handling tool calls and responses appropriately
- **Cursor Agent**: Converts goose messages to Cursor's JSON message format, handling tool calls and responses appropriately
- **Gemini CLI**: Converts messages to simple text prompts with role prefixes (Human:/Assistant:)

### Response Processing

- **Claude Code**: Parses JSON responses to extract text content and usage information
- **Cursor Agent**: Parses JSON responses to extract text content and usage information
- **Gemini CLI**: Processes plain text responses from the CLI tool

## Error Handling

CLI providers depend on external tools, so ensure:

- CLI tools are properly installed and in your PATH
- Authentication is maintained and valid
- Subscription limits are not exceeded


---

CLI providers offer a way to use existing AI tool subscriptions through goose's interface, adding session management and workflow integration capabilities. They're particularly valuable for users with existing CLI subscriptions who want unified session management and recipe integration.
