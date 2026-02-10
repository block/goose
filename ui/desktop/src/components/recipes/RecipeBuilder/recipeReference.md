<!--
  Shared recipe schema reference.
  Source of truth derived from: documentation/docs/guides/recipes/recipe-reference.md
  Used by: recipeBuilderRecipe.ts (AI system prompt)
  Keep in sync when the recipe schema changes.
-->

## Recipe YAML Reference

### Core Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | String | Yes | Short, descriptive name (3-100 chars) |
| `description` | String | Yes | Brief explanation of what the recipe does (10-500 chars) |
| `instructions` | String | Yes* | The system prompt — detailed instructions telling the AI how to behave. This is the most important field. |
| `prompt` | String | Yes* | Initial user message that starts the conversation. Without it, the user types their own first message. |
| `parameters` | Array | No | Input values users provide when launching the recipe |
| `extensions` | Array | No | MCP extensions the recipe needs |
| `settings` | Object | No | Model provider, model name, and temperature |
| `version` | String | No | Recipe format version, defaults to "1.0.0" |

*At least one of `instructions` or `prompt` must be provided.

### Parameters

Parameters make recipes dynamic. Users provide values when launching the recipe, and they get substituted into instructions and prompt using `{{ parameter_name }}` syntax.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `key` | String | Yes | Unique identifier, used in `{{ key }}` template syntax |
| `input_type` | String | Yes | One of: `string`, `number`, `boolean`, `date`, `file`, `select` |
| `requirement` | String | Yes | One of: `required`, `optional`, `user_prompt` |
| `description` | String | Yes | Human-readable label shown to the user |
| `default` | String | No | Default value (required for optional parameters) |
| `options` | Array | No | List of choices (required for `select` type) |

**Input types explained:**
- `string`: Free text input (default)
- `number`: Numeric input with validation
- `boolean`: True/false dropdown
- `date`: Date input
- `file`: User provides a file path — the **file contents** are substituted, not the path itself
- `select`: Dropdown with predefined options (requires `options` field)

**Requirement types:**
- `required`: Must be provided when running the recipe
- `optional`: Can be omitted if a `default` value is specified
- `user_prompt`: Interactively prompts the user for input if not provided

**Parameter example:**
```yaml
parameters:
  - key: language
    input_type: string
    requirement: required
    description: "Programming language to review"
  - key: max_files
    input_type: number
    requirement: optional
    default: "10"
    description: "Maximum files to process"
  - key: output_format
    input_type: select
    requirement: required
    description: "Choose output format"
    options:
      - json
      - markdown
      - csv
  - key: enable_debug
    input_type: boolean
    requirement: optional
    default: "false"
    description: "Enable debug mode"
  - key: source_code
    input_type: file
    requirement: required
    description: "Path to the source code file to analyze"
```

Then reference them in instructions or prompt:
```yaml
instructions: |
  Review {{ language }} code. Process up to {{ max_files }} files.
  Output in {{ output_format }} format. Debug: {{ enable_debug }}.
  Source code to review:
  {{ source_code }}
```

### Extensions

Extensions specify which MCP (Model Context Protocol) servers the recipe needs.

| Field | Type | Description |
|-------|------|-------------|
| `type` | String | `stdio`, `builtin`, `sse`, or `streamable_http` |
| `name` | String | Unique name for the extension |
| `cmd` | String | Command to run (for stdio type) |
| `args` | Array | Arguments for the command |
| `env_keys` | Array | Environment variables required by the extension |
| `timeout` | Number | Timeout in seconds |
| `bundled` | Boolean | Whether the extension is bundled with goose |
| `description` | String | What the extension does |

**Extension examples:**
```yaml
extensions:
  # A bundled extension (built into goose)
  - type: builtin
    name: developer
    display_name: Developer
    timeout: 300
    bundled: true

  # An external MCP server via stdio
  - type: stdio
    name: github-mcp
    cmd: github-mcp-server
    args: []
    env_keys:
      - GITHUB_PERSONAL_ACCESS_TOKEN
    timeout: 60
    description: "GitHub MCP extension for repository operations"

  # An MCP server installed via uvx
  - type: stdio
    name: weatherserver
    cmd: uvx
    args:
      - weather-mcp-server
    timeout: 300
    description: "Weather data lookup"
```

### Settings

Override default model and provider configuration for a specific recipe.

```yaml
settings:
  goose_provider: "anthropic"     # AI provider (e.g., "anthropic", "openai")
  goose_model: "claude-sonnet-4-5-20250929"  # Specific model
  temperature: 0.7                # Creativity (0.0-1.0, higher = more creative)
```

### Validation Rules
- All `{{ parameter_name }}` template variables must have a matching parameter definition
- All defined parameters must be used somewhere in instructions or prompt
- Optional parameters must have a default value
- File parameters cannot have default values
- Select parameters must have an `options` list
- At least one of `instructions` or `prompt` must be provided

### Output Format
Always output recipes in a YAML code block:

```yaml
version: "1.0.0"
title: "Recipe Title"
description: "What this recipe does"
instructions: |
  Detailed instructions for the AI here.
  Reference parameters with {{ parameter_name }} syntax.
prompt: "Optional initial prompt with {{ parameter_name }}"
parameters:
  - key: "parameter_name"
    input_type: "string"
    requirement: "required"
    description: "What this parameter is for"
```

### Complete Example: Code Review Recipe

```yaml
version: "1.0.0"
title: "Code Review for PR"
description: "Automated code review with configurable focus areas and language support"
instructions: |
  You are a code reviewer specialized in {{ language }} development.

  Review the code with a focus on {{ focus }}. Apply these standards:
  - Complexity threshold: {{ complexity_threshold }}
  - Required test coverage: {{ test_coverage }}%
  - Style guide: {{ style_guide }}

  For each issue found, provide:
  1. The file and line number
  2. A description of the issue
  3. A suggested fix

  Summarize your findings at the end with counts of issues by severity.
prompt: "Review the code in this repository"
parameters:
  - key: language
    input_type: string
    requirement: required
    description: "Programming language to review"
  - key: focus
    input_type: select
    requirement: required
    description: "Review focus area"
    options:
      - best practices
      - security
      - performance
      - readability
  - key: complexity_threshold
    input_type: number
    requirement: optional
    default: "20"
    description: "Maximum allowed complexity score"
  - key: test_coverage
    input_type: number
    requirement: optional
    default: "80"
    description: "Minimum test coverage percentage"
  - key: style_guide
    input_type: string
    requirement: user_prompt
    description: "Style guide to follow (e.g., PEP8, Airbnb)"
extensions:
  - type: builtin
    name: developer
    display_name: Developer
    timeout: 300
    bundled: true
settings:
  temperature: 0.3
```

### Simple Example: Trip Planner

```yaml
version: "1.0.0"
title: "Trip Planner"
description: "Plan a detailed travel itinerary for any destination"
instructions: |
  Help the user plan a trip to {{ destination }} for {{ duration }} days.
  Create a detailed itinerary that includes:
  - Places to visit
  - Activities to do
  - Local cuisine to try
  - A rough budget estimate
prompt: "Let's plan your trip!"
parameters:
  - key: destination
    input_type: string
    requirement: required
    description: "Where do you want to go?"
  - key: duration
    input_type: number
    requirement: required
    description: "Number of days for the trip"
settings:
  temperature: 0.8
```
