---
sidebar_position: 2
title: Recipe Reference Guide
description: Complete technical reference for creating and customizing recipes in Goose via the CLI.
---

Recipes are reusable Goose configurations that package up a specific setup so it can be easily shared and launched by others.

## Recipe File Format

Recipes can be defined in either:
- `.yaml` files (recommended)
- `.json` files 

Files should be named either:
- `recipe.yaml`/`recipe.json` 
- `<recipe_name>.yaml`/`<recipe_name>.json`

After creating recipe files, you can use [`goose` CLI commands](/docs/guides/goose-cli-commands) to run or validate the files and to manage recipe sharing.

### CLI and Desktop Formats

The Goose CLI supports CLI and Desktop recipe formats:

- **CLI Format**: Recipe fields (like `title`, `description`, `instructions`) are at the root level of the YAML/JSON file
- **Desktop Format**: Recipe fields are nested inside a `recipe` object, with additional metadata fields at the root level

The CLI automatically detects and handles both formats when running `goose run --recipe <file>` and `goose recipe` commands.

<details>
<summary>Format Examples</summary>

**CLI Format:**
```yaml
version: "1.0.0"
title: "Code Review Assistant"
description: "Automated code review with best practices"
instructions: "You are a code reviewer..."
prompt: "Review the code in this repository"
extensions: []
```

**Desktop Format:**
```yaml
name: "Code Review Assistant"
recipe:
  version: "1.0.0"
  title: "Code Review Assistant"
  description: "Automated code review with best practices"
  instructions: "You are a code reviewer..."
  prompt: "Review the code in this repository"
  extensions: []
isGlobal: true
lastModified: 2025-07-02T03:46:46.778Z
isArchived: false
```

:::note
Goose automatically adds metadata fields to recipes saved from the Desktop app.
:::

</details>

## Recipe Structure

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | String | The recipe format version (e.g., "1.0.0") |
| `title` | String | A short title describing the recipe |
| `description` | String | A detailed description of what the recipe does |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `instructions` | String | Template instructions that can include parameter substitutions |
| `prompt` | String | A template prompt that can include parameter substitutions; required in headless (non-interactive) mode |
| `parameters` | Array | List of parameter definitions |
| `extensions` | Array | List of extension configurations |
| `settings` | Object | Configuration for model provider, model name, and other settings |
| `sub_recipes` | Array | List of sub-recipes |
| `response` | Object | Configuration for structured output validation |
| `retry` | Object | Configuration for automated retry logic with success validation |

### Desktop Format Metadata Fields

When recipes are saved from Goose Desktop, additional metadata fields are included at the top level (outside the `recipe` key). These fields are used by the Desktop app for organization and management but are ignored by CLI operations.

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Display name used in Desktop Recipe Library |
| `isGlobal` | Boolean | Whether the recipe is available globally or locally to a project |
| `lastModified` | String | ISO timestamp of when the recipe was last modified |
| `isArchived` | Boolean | Whether the recipe is archived in the Desktop interface |

## Parameters

Each parameter in the `parameters` array has the following structure:

### Required Parameter Fields

| Field | Type | Description |
|-------|------|-------------|
| `key` | String | Unique identifier for the parameter |
| `input_type` | String | Type of input (e.g., "string") |
| `requirement` | String | One of: "required", "optional", or "user_prompt" |
| `description` | String | Human-readable description of the parameter |

### Optional Parameter Fields

| Field | Type | Description |
|-------|------|-------------|
| `default` | String | Default value for optional parameters |

### Parameter Requirements

- `required`: Parameter must be provided when using the recipe
- `optional`: Can be omitted if a default value is specified
- `user_prompt`: Will interactively prompt the user for input if not provided

The `required` and `optional` parameters work best for recipes opened in Goose Desktop. If a value isn't provided for a `user_prompt` parameter, the parameter won't be substituted and may appear as literal `{{ parameter_name }}` text in the recipe output.

:::important
- Optional parameters MUST have a default value specified
- Required parameters cannot have default values
- Parameter keys must match any template variables used in instructions or prompt
:::

## Extensions

The `extensions` field allows you to specify which Model Context Protocol (MCP) servers and other extensions the recipe needs to function. Each extension in the array has the following structure:

### Extension Fields

| Field | Type | Description |
|-------|------|-------------|
| `type` | String | Type of extension (e.g., "stdio") |
| `name` | String | Unique name for the extension |
| `cmd` | String | Command to run the extension |
| `args` | Array | List of arguments for the command |
| `env_keys` | Array | (Optional) Names of environment variables required by the extension |
| `timeout` | Number | Timeout in seconds |
| `bundled` | Boolean | (Optional) Whether the extension is bundled with Goose |
| `description` | String | Description of what the extension does |

### Example Extension Configuration

```yaml
extensions:
  - type: stdio
    name: codesearch
    cmd: uvx
    args:
      - mcp_codesearch@latest
    timeout: 300
    bundled: true
    description: "Query https://codesearch.sqprod.co/ directly from goose"
  
  - type: stdio
    name: presidio
    timeout: 300
    cmd: uvx
    args:
      - 'mcp_presidio@latest'

  - type: stdio
    name: github-mcp
    cmd: github-mcp-server
    args: []
    env_keys:
      - GITHUB_PERSONAL_ACCESS_TOKEN
    timeout: 60
    description: "GitHub MCP extension for repository operations"
```

### Extension Secrets

This feature is only available through the CLI.

If a recipe uses an extension that requires a secret, Goose can prompt users to provide the secret when running the recipe:

1. When a recipe is loaded, Goose scans all extensions (including those in sub-recipes) for `env_keys` fields
2. If any required environment variables are missing from the secure keyring, Goose prompts the user to enter them
3. Values are stored securely in the system keyring and reused for subsequent runs

To update a stored secret, remove it from the system keyring and run the recipe again to be re-prompted.

:::info
This feature is designed to prompt for and securely store secrets (such as API keys), but `env_keys` can include any environment variable needed by the extension (such as API endpoints, configuration values, etc.).

Users can press `ESC` to skip entering a variable if it's optional for the extension. 
:::

## Settings

The `settings` field allows you to configure the AI model and provider settings for the recipe. This overrides the default configuration when the recipe is executed.

### Settings Fields

| Field | Type | Description |
|-------|------|-------------|
| `goose_provider` | String | (Optional) The AI provider to use (e.g., "anthropic", "openai") |
| `goose_model` | String | (Optional) The specific model name to use |
| `temperature` | Number | (Optional) The temperature setting for the model (typically 0.0-1.0) |

### Example Settings Configuration

```yaml
settings:
  goose_provider: "anthropic"
  goose_model: "claude-3-5-sonnet-latest"
  temperature: 0.7
```

```yaml
settings:
  goose_provider: "openai"
  goose_model: "gpt-4o"
  temperature: 0.3
```

:::note
Settings specified in a recipe will override your default Goose configuration when that recipe is executed. If no settings are specified, Goose will use your configured defaults.
:::

## Sub-Recipes

The `sub_recipes` field specifies the [sub-recipes](/docs/guides/recipes/sub-recipes) that the main recipe calls to perform specific tasks. Each sub-recipe in the array has the following structure:

### Sub-Recipe Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Unique identifier for the sub-recipe |
| `path` | String | Relative or absolute path to the sub-recipe file |
| `values` | Object | (Optional) Pre-configured parameter values that are passed to the sub-recipe |
| `sequential_when_repeated` | Boolean | (Optional) Forces sequential execution of multiple sub-recipe instances. See [Running Sub-Recipes In Parallel](/docs/tutorials/sub-recipes-in-parallel) for details |

### Example Sub-Recipe Configuration

```yaml
sub_recipes:
  - name: "security_scan"
    path: "./sub-recipes/security-analysis.yaml"
    values:  # in key-value format: {parameter_name}: {parameter_value}
      scan_level: "comprehensive"
      include_dependencies: "true"
  
  - name: "quality_check"
    path: "./sub-recipes/quality-analysis.yaml"
```

## Automated Retry with Success Validation

The `retry` field enables recipes to automatically retry execution if success criteria are not met. This is useful for recipes that might need multiple attempts to achieve their goal, or for implementing automated validation and recovery workflows.

### Retry Configuration Fields

| Field | Type | Description |
|-------|------|-------------|
| `max_retries` | Number | Maximum number of retry attempts (required) |
| `timeout_seconds` | Number | (Optional) Timeout for success check commands (default: 300 seconds) |
| `on_failure_timeout_seconds` | Number | (Optional) Timeout for on_failure commands (default: 600 seconds) |
| `checks` | Array | List of success check configurations (required) |
| `on_failure` | String | (Optional) Shell command to run when a retry attempt fails |

### Success Check Configuration

Each success check in the `checks` array has the following structure:

| Field | Type | Description |
|-------|------|-------------|
| `type` | String | Type of check - currently only "shell" is supported |
| `command` | String | Shell command to execute for validation (must exit with code 0 for success) |

### How Retry Logic Works

1. **Recipe Execution**: The recipe runs normally with the provided instructions
2. **Success Validation**: After completion, all success checks are executed in order
3. **Retry Decision**: If any success check fails and retry attempts remain:
   - Execute the on_failure command (if configured)
   - Reset the agent's message history to initial state
   - Increment retry counter and restart execution
4. **Completion**: Process stops when either:
   - All success checks pass (success)
   - Maximum retry attempts are reached (failure)

### Basic Retry Example

```yaml
version: "1.0.0"
title: "Counter Increment Task"
description: "Increment a counter until it reaches target value"
prompt: "Increment the counter value in /tmp/counter.txt by 1."

retry:
  max_retries: 5
  timeout_seconds: 10
  checks:
    - type: shell
      command: "test $(cat /tmp/counter.txt 2>/dev/null || echo 0) -ge 3"
  on_failure: "echo 'Counter is at:' $(cat /tmp/counter.txt 2>/dev/null || echo 0) '(need 3 to succeed)'"
```

### Advanced Retry Example

```yaml
version: "1.0.0"
title: "Service Health Check"
description: "Start service and verify it's running properly"
prompt: "Start the web service and verify it responds to health checks"

retry:
  max_retries: 3
  timeout_seconds: 30
  on_failure_timeout_seconds: 60
  checks:
    - type: shell
      command: "curl -f http://localhost:8080/health"
    - type: shell  
      command: "pgrep -f 'web-service' > /dev/null"
  on_failure: "systemctl stop web-service || killall web-service"
```

### Environment Variables

You can configure retry behavior globally using environment variables:

- `GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS`: Global timeout for success check commands
- `GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS`: Global timeout for on_failure commands

These environment variables are overridden by recipe-specific timeout configurations.

## Structured Output with `response`

The `response` field enables recipes to enforce a final structured JSON output from Goose. When you specify a `json_schema`, Goose will:

1. **Validate the output**: Validates the output JSON against your JSON schema with basic JSON schema validations
2. **Final structured output**: Ensure the final output of the agent is a response matching your JSON structure

This **enables automation** by returning consistent, parseable results for scripts and workflows. Recipes can produce structured output when run from either the Goose CLI or Goose Desktop. See [use cases and ideas for automation workflows](/docs/guides/recipes/session-recipes#structured-output-for-automation).

### Basic Structure

```yaml
response:
  json_schema:
    type: object
    properties:
      # Define your fields here, with their type and description
    required:
      # List required field names
```

### Simple Example

```yaml
version: "1.0.0"
title: "Task Summary"
description: "Summarize completed tasks"
prompt: "Summarize the tasks you completed"
response:
  json_schema:
    type: object
    properties:
      summary:
        type: string
        description: "Brief summary of work done"
      tasks_completed:
        type: number
        description: "Number of tasks finished"
      next_steps:
        type: array
        items:
          type: string
        description: "Recommended next actions"
    required:
      - summary
      - tasks_completed
```

## Template Support

Recipes support Jinja-style template syntax in both `instructions` and `prompt` fields:

```yaml
instructions: "Follow these steps with {{ parameter_name }}"
prompt: "Your task is to {{ action }}"
```

Advanced template features include:
- Template inheritance using `{% extends "parent.yaml" %}`
- Blocks that can be defined and overridden:
  ```yaml
  {% block content %}
  Default content
  {% endblock %}
  ```
- `indent()` template filter

### indent() Filter For Multi-Line Values

Use the `indent()` filter to ensure multi-line parameter values are properly indented and can be resolved as valid JSON or YAML format. This example uses `{{ raw_data | indent(2) }}` to specify an indentation of two spaces when passing data to a sub-recipe:

```yaml
sub_recipes:
  - name: "analyze"
    path: "./analyze.yaml"
    values:
      content: |
        {{ raw_data | indent(2) }}
```

## Built-in Parameters

| Parameter | Description |
|-----------|-------------|
| `recipe_dir` | Automatically set to the directory containing the recipe file |

## Complete Recipe Example

```yaml
version: "1.0.0"
title: "Example Recipe"
description: "A sample recipe demonstrating the format"
instructions: "Follow these steps with {{ required_param }} and {{ optional_param }}"
prompt: "Your task is to use {{ required_param }}"
parameters:
  - key: required_param
    input_type: string
    requirement: required
    description: "A required parameter example"
  
  - key: optional_param
    input_type: string
    requirement: optional
    default: "default value"
    description: "An optional parameter example"
  
  - key: interactive_param
    input_type: string
    requirement: user_prompt
    description: "Will prompt user if not provided"

extensions:
  - type: stdio
    name: codesearch
    cmd: uvx
    args:
      - mcp_codesearch@latest
    timeout: 300
    bundled: true
    description: "Query codesearch directly from goose"

settings:
  goose_provider: "anthropic"
  goose_model: "claude-3-5-sonnet-latest"
  temperature: 0.7

retry:
  max_retries: 3
  timeout_seconds: 30
  checks:
    - type: shell
      command: "echo 'Task validation check passed'"
  on_failure: "echo 'Retry attempt failed, cleaning up...'"

response:
  json_schema:
    type: object
    properties:
      result:
        type: string
        description: "The main result of the task"
      details:
        type: array
        items:
          type: string
        description: "Additional details of steps taken"
    required:
      - result
      - status
```

## Template Inheritance

Parent recipe (`parent.yaml`):
```yaml
version: "1.0.0"
title: "Parent Recipe"
description: "Base recipe template"
prompt: |
  {% block prompt %}
  Default prompt text
  {% endblock %}
```

Child recipe:
```yaml
{% extends "parent.yaml" %}
{% block prompt %}
Modified prompt text
{% endblock %}
```

## Recipe Location

Recipes can be loaded from:

1. Local filesystem:
   - Current directory
   - Directories specified in `GOOSE_RECIPE_PATH` environment variable
   
2. GitHub repositories:
   - Configure using `GOOSE_RECIPE_GITHUB_REPO` configuration key
   - Requires GitHub CLI (`gh`) to be installed and authenticated

## Validation Rules

The following rules are enforced when loading recipes:

1. All template variables must have corresponding parameter definitions
2. Optional parameters must have default values
3. Parameter keys must be unique
4. Recipe files must be valid YAML or JSON
5. Required fields (version, title, description) must be present

## Error Handling

Common errors to watch for:

- Missing required parameters
- Optional parameters without default values
- Template variables without parameter definitions
- Invalid YAML/JSON syntax
- Missing required fields
- Invalid extension configurations
- Invalid retry configuration (missing required fields, invalid shell commands)

When these occur, Goose will provide helpful error messages indicating what needs to be fixed.

### Retry-Specific Errors

- **Invalid success checks**: Shell commands that cannot be executed or have syntax errors
- **Timeout errors**: Success checks or on_failure commands that exceed their timeout limits
- **Max retries exceeded**: When all retry attempts are exhausted without success
- **Missing required retry fields**: When `max_retries` or `checks` are not specified

## Learn More
Check out the [Goose Recipes](/docs/guides/recipes) guide for more docs, tools, and resources to help you master Goose recipes.