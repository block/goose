---
sidebar_position: 15
title: Configuration File
sidebar_label: Configuration File
---

# Configuration File

Goose uses a YAML configuration file to manage settings and extensions. This file is located at:

* macOS/Linux: `~/.config/goose/config.yaml`
* Windows: `%APPDATA%\Block\goose\config\config.yaml`

The configuration file allows you to set default behaviors, configure language models, and manage extensions. While many settings can also be set using [environment variables](/docs/guides/environment-variables), the config file provides a persistent way to maintain your preferences.

## Global Settings

The following settings can be configured at the root level of your config.yaml file:

| Setting | Purpose | Values | Default | Required |
|---------|---------|---------|---------|-----------|
| `GOOSE_PROVIDER` | Primary [LLM provider](/docs/getting-started/providers) | "anthropic", "openai", etc. | None | Yes |
| `GOOSE_MODEL` | Default model to use | Model name (e.g., "claude-3.5-sonnet", "gpt-4") | None | Yes |
| `GOOSE_TEMPERATURE` | Model response randomness | Float between 0.0 and 1.0 | Model-specific | No |
| `GOOSE_MODE` | Tool execution behavior | "auto", "approve", "chat", "smart_approve" | "smart_approve" | No |
| `GOOSE_MAX_TURNS` | [Maximum number of turns](/docs/guides/smart-context-management#maximum-turns) allowed without user input | Integer (e.g., 10, 50, 100) | 1000 | No |
| `GOOSE_LEAD_PROVIDER` | Provider for lead model in [lead/worker mode](/docs/guides/environment-variables#leadworker-model-configuration) | Same as `GOOSE_PROVIDER` options | Falls back to `GOOSE_PROVIDER` | No |
| `GOOSE_LEAD_MODEL` | Lead model for lead/worker mode | Model name | None | No |
| `GOOSE_PLANNER_PROVIDER` | Provider for [planning mode](/docs/guides/creating-plans) | Same as `GOOSE_PROVIDER` options | Falls back to `GOOSE_PROVIDER` | No |
| `GOOSE_PLANNER_MODEL` | Model for planning mode | Model name | Falls back to `GOOSE_MODEL` | No |
| `GOOSE_TOOLSHIM` | Enable tool interpretation | true/false | false | No |
| `GOOSE_TOOLSHIM_OLLAMA_MODEL` | Model for tool interpretation | Model name (e.g., "llama3.2") | System default | No |
| `GOOSE_CLI_MIN_PRIORITY` | Tool output verbosity | Float between 0.0 and 1.0 | 0.0 | No |
| `GOOSE_CLI_THEME` | [Theme](/docs/guides/goose-cli-commands#themes) for CLI response  markdown | "light", "dark", "ansi" | "dark" | No |
| `GOOSE_ALLOWLIST` | URL for allowed extensions | Valid URL | None | No |
| `GOOSE_RECIPE_GITHUB_REPO` | GitHub repository for recipes | Format: "org/repo" | None | No |

## Experimental Features

These settings enable experimental features that are in active development. These may change or be removed in future releases.

| Setting | Purpose | Values | Default | Required |
|---------|---------|---------|---------|-----------|
| `ALPHA_FEATURES` | Enables experimental alpha features like [subagents](/docs/experimental/subagents) | true/false | false | No |

Additional [environment variables](/docs/guides/environment-variables) may also be supported in config.yaml.

## Example Configuration

Here's a basic example of a config.yaml file:

```yaml
# Model Configuration
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-3.5-sonnet"
GOOSE_TEMPERATURE: 0.7

# Planning Configuration
GOOSE_PLANNER_PROVIDER: "openai"
GOOSE_PLANNER_MODEL: "gpt-4"

# Tool Configuration
GOOSE_MODE: "smart_approve"
GOOSE_TOOLSHIM: true
GOOSE_CLI_MIN_PRIORITY: 0.2

# Recipe Configuration
GOOSE_RECIPE_GITHUB_REPO: "block/goose-recipes"

# Experimental Features
ALPHA_FEATURES: true

# Extensions Configuration
extensions:
  developer:
    bundled: true
    enabled: true
    name: developer
    timeout: 300
    type: builtin
  
  memory:
    bundled: true
    enabled: true
    name: memory
    timeout: 300
    type: builtin
```

## Extensions Configuration

Extensions are configured under the `extensions` key. Each extension can have the following settings:

```yaml
extensions:
  extension_name:
    bundled: true/false        # Whether it's included with Goose
    display_name: "Name"       # Human-readable name (optional)
    enabled: true/false        # Whether the extension is active
    name: "extension_name"     # Internal name
    timeout: 300              # Operation timeout in seconds
    type: "builtin"/"stdio"   # Extension type
    
    # Additional settings for stdio extensions:
    cmd: "command"            # Command to execute
    args: ["arg1", "arg2"]    # Command arguments
    description: "text"       # Extension description
    env_keys: []             # Required environment variables
    envs: {}                 # Environment values
```

## Configuration Priority

Settings are applied in the following order of precedence:

1. Environment variables (highest priority)
2. Config file settings
3. Default values (lowest priority)

## Security Considerations

- Avoid storing sensitive information (API keys, tokens) in the config file
- Use the system keyring for storing secrets
- If keyring is disabled, secrets are stored in a separate `secrets.yaml` file

## Updating Configuration

Changes to the config file require restarting Goose to take effect. You can verify your current configuration using:

```bash
goose info -v
```

This will show all active settings and their current values.

## See Also

- [Environment Variables](./environment-variables.md) - For environment variable configuration
- [Using Extensions](/docs/getting-started/using-extensions.md) - For more details on extension configuration
- [Creating Plans](./creating-plans.md) - For information about planning mode configuration