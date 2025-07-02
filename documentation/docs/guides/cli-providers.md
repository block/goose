---
sidebar_position: 25
title: CLI Providers
sidebar_label: CLI Providers
---

# CLI Providers

Goose supports two special "pass-through" providers that leverage existing CLI tools from Anthropic and Google. These providers allow you to use your existing subscriptions to Claude Code and Google Gemini CLI tools directly through Goose, providing a cost-effective way to access these excellent AI models and tools while maintaining the benefits of Goose's ecosystem, and working with multiple models and extensions.

## Overview

CLI providers are different from traditional LLM providers in several key ways:

- **Pass-through architecture**: Instead of making direct API calls, these providers drive the CLI commands
- **Subscription-based**: Use your existing Claude Code or Google Gemini CLI subscriptions instead of paying per token (usually a flat monthly fee)
- **Tool integration**: These providers work with their respective CLI tools' built-in capabilities while Goose manages sessions and extensions
- **Cost-effective**: Leverage unlimited or subscription-based pricing models instead of token-based billing

## Available CLI Providers

### Claude Code (`claude-code`)

The Claude Code provider integrates with Anthropic's [Claude CLI tool](https://claude.ai/cli), allowing you to use Claude models through your existing Claude Code subscription.

**Features:**
- Uses Claude's latest models
- 200,000 token context limit
- Automatic filtering of Goose extensions from system prompts (since Claude Code has its own tool ecosystem)
- JSON output parsing for structured responses

**Requirements:**
- Claude CLI tool installed and configured
- Active Claude Code subscription
- CLI tool authenticated with your Anthropic account

### Gemini CLI (`gemini-cli`)

The Gemini CLI provider integrates with Google's [Gemini CLI tool](https://ai.google.dev/gemini-api/docs), providing access to Gemini models through your Google AI subscription.

**Features:**
- 1,000,000 token context limit

**Requirements:**
- Gemini CLI tool installed and configured
- CLI tool authenticated with your Google account

## Setup Instructions

### Setting up Claude Code Provider

1. **Install Claude CLI Tool**
   
   Follow the installation instructions [here](https://docs.anthropic.com/en/docs/claude-code/overview) to install and configure the Claude CLI tool.

2. **Authenticate with Claude**
   
   Ensure your Claude CLI is authenticated and working

3. **Configure Goose**
   
   Set the provider environment variable:
   ```bash
   export GOOSE_PROVIDER=claude-code
   ```
   
   Or configure through the Goose CLI:
   ```bash
   goose configure
   # Select "Configure Providers"
   # Choose "Claude Code" from the list
   ```

### Setting up Gemini CLI Provider

1. **Install Gemini CLI Tool**
   
   Follow the installation instructions for [gemini cli]https://blog.google/technology/developers/introducing-gemini-cli-open-source-ai-agent/) to install and configure the Gemini CLI tool.

2. **Authenticate with Google**
   
   Ensure your Gemini CLI is authenticated and working.

3. **Configure Goose**
   
   Set the provider environment variable:
   ```bash
   export GOOSE_PROVIDER=gemini-cli
   ```
   
   Or configure through the Goose CLI:
   ```bash
   goose configure
   # Select "Configure Providers"
   # Choose "Gemini CLI" from the list
   ```

## Usage Examples

### Basic Usage

Once configured, use these providers just like any other Goose provider:

```bash
# Using Claude Code
GOOSE_PROVIDER=claude-code goose session start

# Using Gemini CLI
GOOSE_PROVIDER=gemini-cli goose session start
```

### Combining with Other Models

CLI providers work well in combination with other models using Goose's lead/worker pattern:

```bash
# Use Claude Code as lead model, GPT-4o as worker
export GOOSE_LEAD_PROVIDER=claude-code
export GOOSE_PROVIDER=openai
export GOOSE_MODEL=gpt-4o
export GOOSE_LEAD_MODEL=default

goose session start
```

## Configuration Options

### Claude Code Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GOOSE_PROVIDER` | Set to `claude-code` to use this provider | None |
| `CLAUDE_CODE_COMMAND` | Path to the Claude CLI command | `claude` |

### Gemini CLI Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `GOOSE_PROVIDER` | Set to `gemini-cli` to use this provider | None |

## How It Works

### System Prompt Filtering

Both CLI providers automatically filter out Goose's extension information from system prompts since these CLI tools have their own tool ecosystems. This prevents conflicts and ensures clean interaction with the underlying CLI tools.

### Message Translation

- **Claude Code**: Converts Goose messages to Claude's JSON message format, handling tool calls and responses appropriately
- **Gemini CLI**: Converts messages to simple text prompts with role prefixes (Human:/Assistant:)

### Response Processing

- **Claude Code**: Parses JSON responses to extract text content and usage information
- **Gemini CLI**: Processes plain text responses from the CLI tool

## Limitations and Considerations

### Tool Calling

These providers handle tool calling differently than standard API providers as they have their own tool integrations out of the box.

### Error Handling

CLI providers depend on external tools, so ensure:

- CLI tools are properly installed and in your PATH
- Authentication is maintained and valid
- Subscription limits are not exceeded


## Benefits

### Cost Effectiveness

- **Subscription-based pricing**: Use unlimited or fixed-price subscriptions instead of per-token billing
- **Existing subscriptions**: Leverage subscriptions you may already have

### Flexibility

- **Hybrid usage**: Combine CLI providers with API-based providers using lead/worker patterns
- **Session management**: Full Goose session management and extension system
- **Easy switching**: Switch between CLI and API providers as needed

## Best Practices

1. **Authentication Management**: Keep CLI tools authenticated and monitor subscription status
2. **Hybrid Approaches**: Consider using CLI providers for heavy workloads and API providers for quick tasks
3. **Backup Providers**: Configure fallback API providers in case CLI tools are unavailable

---

CLI providers offer a powerful way to integrate Goose with existing AI tool subscriptions while maintaining the benefits of Goose's session management and extension ecosystem. They're particularly valuable for users with existing subscriptions who want to maximize their investment while gaining access to Goose's capabilities.
