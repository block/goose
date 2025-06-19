---
sidebar_position: 20
title: Environment Variables
sidebar_label: Environment Variables
---

Goose supports various environment variables that allow you to customize its behavior. This guide provides a comprehensive list of available environment variables grouped by their functionality.

## Model Configuration

These variables control the [language models](/docs/getting-started/providers) and their behavior.

### Basic Provider Configuration

These are the minimum required variables to get started with Goose.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_PROVIDER` | Specifies the LLM provider to use | [See available providers](/docs/getting-started/providers#available-providers) | None (must be [configured](/docs/getting-started/providers#configure-provider)) |
| `GOOSE_MODEL` | Specifies which model to use from the provider | Model name (e.g., "gpt-4", "claude-3.5-sonnet") | None (must be configured) |
| `GOOSE_TEMPERATURE` | Sets the [temperature](https://medium.com/@kelseyywang/a-comprehensive-guide-to-llm-temperature-%EF%B8%8F-363a40bbc91f) for model responses | Float between 0.0 and 1.0 | Model-specific default |

**Examples**

```bash
# Basic model configuration
export GOOSE_PROVIDER="anthropic"
export GOOSE_MODEL="claude-3.5-sonnet"
export GOOSE_TEMPERATURE=0.7
```

### Advanced Provider Configuration

These variables are needed when using custom endpoints, enterprise deployments, or specific provider implementations.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_PROVIDER__TYPE` | The specific type/implementation of the provider | [See available providers](/docs/getting-started/providers#available-providers) | Derived from GOOSE_PROVIDER |
| `GOOSE_PROVIDER__HOST` | Custom API endpoint for the provider | URL (e.g., "https://api.openai.com") | Provider-specific default |
| `GOOSE_PROVIDER__API_KEY` | Authentication key for the provider | API key string | None |

**Examples**

```bash
# Advanced provider configuration
export GOOSE_PROVIDER__TYPE="anthropic"
export GOOSE_PROVIDER__HOST="https://api.anthropic.com"
export GOOSE_PROVIDER__API_KEY="your-api-key-here"
```

### Lead/Worker Model Configuration

These variables configure a [lead/worker model pattern](/docs/tutorials/lead-worker) where a powerful lead model handles initial planning and complex reasoning, then switches to a faster/cheaper worker model for execution. The switch happens automatically based on your settings.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_LEAD_MODEL` | **Required to enable lead mode.** Name of the lead model | Model name (e.g., "gpt-4o", "claude-3.5-sonnet") | None |
| `GOOSE_LEAD_PROVIDER` | Provider for the lead model | [See available providers](/docs/getting-started/providers#available-providers) | Falls back to `GOOSE_PROVIDER` |
| `GOOSE_LEAD_TURNS` | Number of initial turns using the lead model before switching to the worker model | Integer | 3 |
| `GOOSE_LEAD_FAILURE_THRESHOLD` | Consecutive failures before fallback to the lead model | Integer | 2 |
| `GOOSE_LEAD_FALLBACK_TURNS` | Number of turns to use the lead model in fallback mode | Integer | 2 |

A _turn_ is one complete prompt-response interaction. Here's how it works with the default settings:
- Use the lead model for the first 3 turns
- Use the worker model starting on the 4th turn
- Fallback to the lead model if the worker model struggles for 2 consecutive turns
- Use the lead model for 2 turns and then switch back to the worker model

The lead model and worker model names are displayed at the start of the Goose CLI session. If you don't export a `GOOSE_MODEL` for your session, the worker model defaults to the `GOOSE_MODEL` in your [configuration file](/docs/guides/config-file).

**Examples**

```bash
# Basic lead/worker setup
export GOOSE_LEAD_MODEL="o4"

# Advanced lead/worker configuration
export GOOSE_LEAD_MODEL="claude4-opus"
export GOOSE_LEAD_PROVIDER="anthropic"
export GOOSE_LEAD_TURNS=5
export GOOSE_LEAD_FAILURE_THRESHOLD=3
export GOOSE_LEAD_FALLBACK_TURNS=2
```

### Planning Mode Configuration

These variables control Goose's [planning functionality](/docs/guides/creating-plans).

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_PLANNER_PROVIDER` | Specifies which provider to use for planning mode | [See available providers](/docs/getting-started/providers#available-providers) | Falls back to GOOSE_PROVIDER |
| `GOOSE_PLANNER_MODEL` | Specifies which model to use for planning mode | Model name (e.g., "gpt-4", "claude-3.5-sonnet")| Falls back to GOOSE_MODEL |

**Examples**

```bash
# Planning mode with different model
export GOOSE_PLANNER_PROVIDER="openai"
export GOOSE_PLANNER_MODEL="gpt-4"
```

## Session Management

These variables control how Goose manages conversation sessions and context.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_CONTEXT_STRATEGY` | Controls how Goose handles context limit exceeded situations | "summarize", "truncate", "clear", "prompt" | "prompt" (interactive), "summarize" (headless) |

**Examples**

```bash
# Automatically summarize when context limit is reached
export GOOSE_CONTEXT_STRATEGY=summarize

# Always prompt user to choose (default for interactive mode)
export GOOSE_CONTEXT_STRATEGY=prompt
```

## Tool Configuration

These variables control how Goose handles [tool permissions](/docs/guides/tool-permissions) and their execution.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_MODE` | Controls how Goose handles tool execution | "auto", "approve", "chat", "smart_approve" | "smart_approve" |
| `GOOSE_TOOLSHIM` | Enables/disables tool call interpretation | "1", "true" (case insensitive) to enable | false |
| `GOOSE_TOOLSHIM_OLLAMA_MODEL` | Specifies the model for [tool call interpretation](/docs/guides/experimental-features/#ollama-tool-shim) | Model name (e.g. llama3.2, qwen2.5) | System default |
| `GOOSE_CLI_MIN_PRIORITY` | Controls verbosity of [tool output](/docs/guides/adjust-tool-output) | Float between 0.0 and 1.0 | 0.0 |
| `GOOSE_CLI_TOOL_PARAMS_TRUNCATION_MAX_LENGTH` | Maximum length for tool parameter values before truncation in CLI output (not in debug mode) | Integer | 40 |

**Examples**

```bash
# Enable tool interpretation
export GOOSE_TOOLSHIM=true
export GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2
export GOOSE_MODE="auto"
export GOOSE_CLI_MIN_PRIORITY=0.2  # Show only medium and high importance output
export GOOSE_CLI_TOOL_PARAMS_MAX_LENGTH=100  # Show up to 100 characters for tool parameters in CLI output
```
## Security Configuration

These variables control security related features.

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `GOOSE_ALLOWLIST` | Controls which extensions can be loaded | URL for [allowed extensions](/docs/guides/allowlist) list | Unset |
| `GOOSE_DISABLE_KEYRING` | Disables the system keyring for secret storage | Set to any value (e.g., "1", "true", "yes") to disable. The actual value doesn't matter, only whether the variable is set. | Unset (keyring enabled) |

:::tip
When the keyring is disabled, secrets are stored here:

* macOS/Linux: `~/.config/goose/secrets.yaml`
* Windows: `%APPDATA%\Block\goose\config\secrets.yaml`
:::


## Langfuse Integration

These variables configure the [Langfuse integration for observability](/docs/tutorials/langfuse).

| Variable | Purpose | Values | Default |
|----------|---------|---------|---------|
| `LANGFUSE_PUBLIC_KEY` | Public key for Langfuse integration | String | None |
| `LANGFUSE_SECRET_KEY` | Secret key for Langfuse integration | String | None |
| `LANGFUSE_URL` | Custom URL for Langfuse service | URL String | Default Langfuse URL |
| `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` | Alternative public key for Langfuse | String | None |
| `LANGFUSE_INIT_PROJECT_SECRET_KEY` | Alternative secret key for Langfuse | String | None |


## Notes

- Environment variables take precedence over configuration files.
- For security-sensitive variables (like API keys), consider using the system keyring instead of environment variables.
- Some variables may require restarting Goose to take effect.
- When using the planning mode, if planner-specific variables are not set, Goose will fall back to the main model configuration.

