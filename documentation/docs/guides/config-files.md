---
sidebar_position: 85
title: Configuration Files
sidebar_label: Configuration Files
---

# Configuration Overview

goose uses YAML [configuration files](#configuration-files) to manage settings and extensions. The primary config file is located at:

* macOS/Linux: `~/.config/goose/config.yaml`
* Windows: `%APPDATA%\Block\goose\config\config.yaml`

The configuration files allow you to set default behaviors, configure language models, set tool permissions, and manage extensions. While many settings can also be set using [environment variables](/docs/guides/environment-variables), the config files provide a persistent way to maintain your preferences.

## Configuration Files

- **config.yaml** - Provider, model, extensions, and general settings
- **permission.yaml** - Tool permission levels configured via `goose configure`
- **secrets.yaml** - API keys and secrets (only when keyring is disabled)
- **permissions/tool_permissions.json** - Runtime permission decisions (auto-managed)

## Global Settings

The following settings can be configured at the root level of your config.yaml file:

| Setting | Purpose | Values | Default | Required |
|---------|---------|---------|---------|-----------|
| `GOOSE_PROVIDER` | Primary [LLM provider](/docs/getting-started/providers) | "anthropic", "openai", etc. | None | Yes |
| `GOOSE_MODEL` | Default model to use | Model name (e.g., "claude-3.5-sonnet", "gpt-4") | None | Yes |
| `GOOSE_TEMPERATURE` | Model response randomness | Float between 0.0 and 1.0 | Model-specific | No |
| `GOOSE_MODE` | [Tool execution behavior](/docs/guides/goose-permissions) | "auto", "approve", "chat", "smart_approve" | "auto" | No |
| `GOOSE_MAX_TURNS` | [Maximum number of turns](/docs/guides/sessions/smart-context-management#maximum-turns) allowed without user input | Integer (e.g., 10, 50, 100) | 1000 | No |
| `GOOSE_LEAD_PROVIDER` | Provider for lead model in [lead/worker mode](/docs/guides/environment-variables#leadworker-model-configuration) | Same as `GOOSE_PROVIDER` options | Falls back to `GOOSE_PROVIDER` | No |
| `GOOSE_LEAD_MODEL` | Lead model for lead/worker mode | Model name | None | No |
| `GOOSE_PLANNER_PROVIDER` | Provider for [planning mode](/docs/guides/creating-plans) | Same as `GOOSE_PROVIDER` options | Falls back to `GOOSE_PROVIDER` | No |
| `GOOSE_PLANNER_MODEL` | Model for planning mode | Model name | Falls back to `GOOSE_MODEL` | No |
| `GOOSE_TOOLSHIM` | Enable tool interpretation | true/false | false | No |
| `GOOSE_TOOLSHIM_OLLAMA_MODEL` | Model for tool interpretation | Model name (e.g., "llama3.2") | System default | No |
| `GOOSE_CLI_MIN_PRIORITY` | Tool output verbosity | Float between 0.0 and 1.0 | 0.0 | No |
| `GOOSE_CLI_THEME` | [Theme](/docs/guides/goose-cli-commands#themes) for CLI response  markdown | "light", "dark", "ansi" | "dark" | No |
| `GOOSE_CLI_SHOW_COST` | Show estimated cost for token use in the CLI | true/false | false | No |
| `GOOSE_ALLOWLIST` | URL for allowed extensions | Valid URL | None | No |
| `GOOSE_RECIPE_GITHUB_REPO` | GitHub repository for recipes | Format: "org/repo" | None | No |
| `GOOSE_AUTO_COMPACT_THRESHOLD` | Set the percentage threshold at which goose [automatically summarizes your session](/docs/guides/sessions/smart-context-management#automatic-compaction). | Float between 0.0 and 1.0 (disabled at 0.0)| 0.8 | No |
| `otel_exporter_otlp_endpoint` | OTLP endpoint URL for [observability](/docs/guides/environment-variables#opentelemetry-protocol-otlp) | URL (e.g., `http://localhost:4318`) | None | No |
| `otel_exporter_otlp_timeout` | Export timeout in milliseconds for [observability](/docs/guides/environment-variables#opentelemetry-protocol-otlp) | Integer (ms) | 10000 | No |
| `SECURITY_PROMPT_ENABLED` | Enable [prompt injection detection](/docs/guides/security/prompt-injection-detection) to identify potentially harmful commands | true/false | false | No |
| `SECURITY_PROMPT_THRESHOLD` | Sensitivity threshold for [prompt injection detection](/docs/guides/security/prompt-injection-detection) (higher = stricter) | Float between 0.01 and 1.0 | 0.7 | No |

## Experimental Features

These settings enable experimental features that are in active development. These may change or be removed in future releases.

| Setting | Purpose | Values | Default | Required |
|---------|---------|---------|---------|-----------|
| `ALPHA_FEATURES` | Enables access to experimental alpha features&mdash;check the feature docs to see if this flag is required | true/false | false | No |

Additional [environment variables](/docs/guides/environment-variables) may also be supported in config.yaml.

## Example Configuration

Here's a basic example of a config.yaml file:

```yaml
# Model Configuration
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-4.5-sonnet"
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

# Search Path Configuration
GOOSE_SEARCH_PATHS:
  - "/usr/local/bin"
  - "~/custom/tools"
  - "/opt/homebrew/bin"

# Observability (OpenTelemetry)
otel_exporter_otlp_endpoint: "http://localhost:4318"
otel_exporter_otlp_timeout: 20000

# Security Configuration
SECURITY_PROMPT_ENABLED: true

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
    bundled: true/false       # Whether it's included with goose
    display_name: "Name"      # Human-readable name (optional)
    enabled: true/false       # Whether the extension is active
    name: "extension_name"    # Internal name
    timeout: 300              # Operation timeout in seconds
    type: "builtin"/"stdio"   # Extension type
    
    # Additional settings for stdio extensions:
    cmd: "command"            # Command to execute
    args: ["arg1", "arg2"]    # Command arguments
    description: "text"       # Extension description
    env_keys: []              # Required environment variables
    envs: {}                  # Environment values
```

## Search Path Configuration

Extensions may need to execute external commands or tools. By default, goose uses your system's PATH environment variable. You can add additional search directories in your config file:

```yaml
GOOSE_SEARCH_PATHS:
  - "/usr/local/bin"
  - "~/custom/tools"
  - "/opt/homebrew/bin"
```

These paths are prepended to the system PATH when running extension commands, ensuring your custom tools are found without modifying your global PATH.

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

Changes to config files require restarting goose to take effect. You can verify your current configuration using:

```bash
goose info -v
```

This will show all active settings and their current values.

## See Also

- **[Multi-Model Configuration](/docs/guides/multi-model/)** - For multiple model-selection strategies
- **[Environment Variables](./environment-variables.md)** - For environment variable configuration
- **[Using Extensions](/docs/getting-started/using-extensions.md)** - For more details on extension configuration
