---
sidebar_position: 35
title: CLI Commands
sidebar_label: CLI Commands
toc_max_heading_level: 4
---

Goose provides a command-line interface (CLI) with several commands for managing sessions, configurations and extensions. This guide covers all available CLI commands and interactive session features.

### Core Commands

#### help
Display the help menu.

**Usage:**
```bash
goose --help
```

---

#### configure
Configure Goose settings - providers, extensions, etc.

**Usage:**
```bash
goose configure
```

---

#### info [options]
Shows Goose information, including the version, configuration file location, session storage, and logs.

**Options:**
- **`-v, --verbose`**: Show detailed configuration settings, including environment variables and enabled extensions

**Usage:**
```bash
goose info
```

---

#### version
Check the current Goose version you have installed.

**Usage:**
```bash
goose --version
```

---

#### update [options]
Update the Goose CLI to a newer version.

**Options:**
- **`--canary, -c`**: Update to the canary (development) version instead of the stable version
- **`--reconfigure, -r`**: Forces Goose to reset configuration settings during the update process

**Usage:**
```bash
# Update to latest stable version
goose update

# Update to latest canary version
goose update --canary

# Update and reconfigure settings
goose update --reconfigure
```

---

### Session Management

#### session [options]
Start or resume interactive chat sessions.

**Basic Options:**
- **`-n, --name <name>`**: Give the session a name
- **`-r, --resume`**: Resume a previous session  
- **`--debug`**: Enable debug mode to output complete tool responses, detailed parameter values, and full file paths
- **`--max-turns <NUMBER>`**: Set the maximum number of turns allowed without user input (default: 1000)

**Extension Options:**
- **`--with-extension <command>`**: Add stdio extensions
- **`--with-remote-extension <url>`**: Add remote extensions over SSE
- **`--with-streamable-http-extension <url>`**: Add remote extensions over Streaming HTTP
- **`--with-builtin <id>`**: Enable built-in extensions (e.g., 'developer', 'computercontroller')

**Usage:**
```bash
# Start a basic session
goose session --name my-project

# Resume a previous session
goose session --resume --name my-project
goose session --resume --id 2025250620_013617

# Start with extensions
goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
goose session --with-builtin developer
goose session --with-remote-extension "http://localhost:8080/sse"

# Advanced: Mix multiple extension types
goose session \
  --with-extension "echo hello" \
  --with-remote-extension "http://sse.example.com/sse" \
  --with-streamable-http-extension "http://http.example.com" \
  --with-builtin "developer"

# Control session behavior
goose session --name my-session --debug --max-turns 25
```

---

#### session list [options]
List all saved sessions.

**Options:**
- **`-v, --verbose`**: Include session file paths in the output
- **`-f, --format <format>`**: Specify output format (`text` or `json`). Default is `text`
- **`--ascending`**: Sort sessions by date in ascending order (oldest first)

**Usage:**
```bash
# List all sessions in text format (default)
goose session list

# List sessions with file paths
goose session list --verbose

# List sessions in JSON format
goose session list --format json

# Sort sessions by date in ascending order
goose session list --ascending
```

---

#### session remove [options]
Remove one or more saved sessions.

**Options:**
- **`-i, --id <id>`**: Remove a specific session by its ID
- **`-n, --name <name>`**: Remove a specific session by its name
- **`-r, --regex <pattern>`**: Remove sessions matching a regex pattern

**Usage:**
```bash
# Remove a specific session by ID
goose session remove -i 20250305_113223

# Remove a specific session by its name
goose session remove -n my-session

# Remove all sessions starting with "project-"
goose session remove -r "project-.*"

# Remove all sessions containing "migration"
goose session remove -r ".*migration.*"
```

:::caution
Session removal is permanent and cannot be undone. Goose will show which sessions will be removed and ask for confirmation before deleting.
:::

---

#### session export [options]
Export a session to Markdown format for sharing, documentation, or archival purposes.

**Options:**
- **`-i, --id <id>`**: Export a specific session by ID
- **`-n, --name <name>`**: Export a specific session by name
- **`-p, --path <path>`**: Export a specific session by file path
- **`-o, --output <file>`**: Save exported content to a file (default: stdout)

**Usage:**
```bash
# Export specific session to file
goose session export --name my-session --output session.md

# Export specific session to stdout
goose session export --name my-session

# Interactive export (prompts for session selection)
goose session export

# Export session by path
goose session export --path ./my-session.jsonl --output exported.md
```

---

### Task Execution

#### run [options]
Execute commands from an instruction file or stdin. Check out the [full guide](/docs/guides/running-tasks) for more info.

**Input Options:**
- **`-i, --instructions <FILE>`**: Path to instruction file containing commands. Use `-` for stdin
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly
- **`--recipe <RECIPE_FILE_NAME> <OPTIONS>`**: Load a custom recipe in current session

**Session Options:**
- **`-s, --interactive`**: Continue in interactive mode after processing initial input
- **`-n, --name <name>`**: Name for this run session (e.g. `daily-tasks`)
- **`-r, --resume`**: Resume from a previous run
- **`-p, --path <PATH>`**: Path for this run session (e.g. `./playground.jsonl`)
- **`--no-session`**: Run goose commands without creating or storing a session file

**Extension Options:**
- **`--with-extension <COMMAND>`**: Add stdio extensions (can be used multiple times)
- **`--with-remote-extension <URL>`**: Add remote extensions over SSE (can be used multiple times)
- **`--with-streamable-http-extension <URL>`**: Add remote extensions over Streaming HTTP (can be used multiple times)
- **`--with-builtin <name>`**: Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')

**Control Options:**
- **`--debug`**: Output complete tool responses, detailed parameter values, and full file paths
- **`--max-turns <NUMBER>`**: Maximum number of turns allowed without user input (default: 1000)
- **`--explain`**: Show a recipe's title, description, and parameters
- **`--provider`**: Specify the provider to use for this session (overrides environment variable)
- **`--model`**: Specify the model to use for this session (overrides environment variable)

**Usage:**
```bash
# Run from instruction file
goose run --instructions plan.md

# Load a recipe with a prompt that Goose executes and then exits  
goose run --recipe recipe.yaml

# Load a recipe and stay in an interactive session
goose run --recipe recipe.yaml --interactive

# Load a recipe in debug mode
goose run --recipe recipe.yaml --debug

# Show recipe details
goose run --recipe recipe.yaml --explain

# Run instructions from a file without session storage
goose run --no-session -i instructions.txt

# Run with a specified provider and model
goose run --provider anthropic --model claude-4-sonnet -t "initial prompt"

# Run with limited turns before prompting user
goose run --recipe recipe.yaml --max-turns 10
```

---

#### bench
Used to evaluate system-configuration across a range of practical tasks. See the [detailed guide](/docs/tutorials/benchmarking) for more information.

**Usage:**
```bash
goose bench ...etc.
```

---

#### recipe
Used to validate recipe files and manage recipe sharing.

**Commands:**
- `validate <FILE>`: Validate a recipe file
- `deeplink <FILE>`: Generate a shareable link for a recipe file

**Usage:**
```bash
goose recipe <COMMAND>

# Validate a recipe file
goose recipe validate my-recipe.yaml

# Generate a shareable link
goose recipe deeplink my-recipe.yaml

# Get help about recipe commands
goose recipe help
```

---

#### schedule
Automate recipes by running them on a [schedule](/docs/guides/recipes/session-recipes.md#schedule-recipe).

**Commands:**
- `add <OPTIONS>`: Create a new scheduled job. Copies the current version of the recipe to `~/.local/share/goose/scheduled_recipes`
- `list`: View all scheduled jobs
- `remove`: Delete a scheduled job
- `sessions`: List sessions created by a scheduled recipe
- `run-now`: Run a scheduled recipe immediately

**Temporal Commands (requires Temporal CLI):**
- `services-status`: Check if any Temporal services are running
- `services-stop`: Stop any running Temporal services

**Options:**
- `--id <NAME>`: A unique ID for the scheduled job (e.g. `daily-report`)
- `--cron "* * * * * *"`: Specifies when a job should run using a [cron expression](https://en.wikipedia.org/wiki/Cron#Cron_expression)
- `--recipe-source <PATH>`: Path to the recipe YAML file
- `--limit <NUMBER>`: Max number of sessions to display when using the `sessions` command

**Usage:**
```bash
goose schedule <COMMAND>

# Add a new scheduled recipe which runs every day at 9 AM
goose schedule add --id daily-report --cron "0 0 9 * * *" --recipe-source ./recipes/daily-report.yaml

# List all scheduled jobs
goose schedule list

# List the 10 most recent Goose sessions created by a scheduled job
goose schedule sessions --id daily-report --limit 10

# Run a recipe immediately
goose schedule run-now --id daily-report

# Remove a scheduled job
goose schedule remove --id daily-report
```

---

#### mcp
Run an enabled MCP server specified by `<name>` (e.g. `'Google Drive'`).

**Usage:**
```bash
goose mcp <name>
```

---

#### acp
Run Goose as an Agent Client Protocol (ACP) agent server over stdio. This enables Goose to work with ACP-compatible clients like Zed.

ACP is an emerging protocol specification that standardizes communication between AI agents and client applications, making it easier for clients to integrate with various AI agents.

**Usage:**
```bash
goose acp
```

:::info
This command is automatically invoked by ACP-compatible clients and is not typically run directly by users. The client manages the lifecycle of the `goose acp` process. See [Using Goose in ACP Clients](/docs/guides/acp-clients) for details.
:::

---

### Project Management

#### project
Start working on your last project or create a new one. For detailed usage examples and workflows, see [Managing Projects Guide](/docs/guides/managing-projects).

**Alias**: `p`

**Usage:**
```bash
goose project
```

---

#### projects
Choose one of your projects to start working on.

**Alias**: `ps`

**Usage:**
```bash
goose projects
```

---

### Interface

#### web
Start a new session in Goose Web, a lightweight web-based interface launched via the CLI that mirrors the desktop app's chat experience.

Goose Web is particularly useful when:
- You want to access Goose with a graphical interface without installing the desktop app
- You need to use Goose from different devices, including mobile
- You're working in an environment where installing desktop apps isn't practical

:::warning
Don't expose the web interface to the internet without proper security measures.
:::

**Options:**
- **`-p, --port <PORT>`**: Port number to run the web server on. Default is `3000`
- **`--host <HOST>`**: Host to bind the web server to. Default is `127.0.0.1`
- **`--open`**: Automatically open the browser when the server starts

**Usage:**
```bash
# Start web interface at `http://127.0.0.1:3000` and open the browser
goose web --open

# Start web interface at `http://127.0.0.1:8080` 
goose web --port 8080

# Start web interface accessible from local network at `http://192.168.1.7:8080`
goose web --host 192.168.1.7 --port 8080
```

:::info
Use `Ctrl+C` to stop the server.
:::

**Limitations:**

While the web interface provides most core features, be aware of these limitations:
- Some file system operations may require additional confirmation
- Extension management must be done through the CLI
- Certain tool interactions might need extra setup
- Configuration changes require a server restart



---

## Interactive Session Features

### Slash Commands

Once you're in an interactive session (via `goose session` or `goose run --interactive`), you can use these slash commands. All commands support tab completion. Press `/ + <Tab>` to cycle through available commands.

**Available Commands:**
- **`/?` or `/help`** - Display the help menu
- **`/builtin <names>`** - Add builtin extensions by name (comma-separated)
- **`/clear`** - Clear the current chat history
- **`/endplan`** - Exit plan mode and return to 'normal' goose mode
- **`/exit` or `/quit`** - Exit the session
- **`/extension <command>`** - Add a stdio extension (format: ENV1=val1 command args...)
- **`/mode <name>`** - Set the goose mode to use ('auto', 'approve', 'chat', 'smart_approve')
- **`/plan <message_text>`** - Enter 'plan' mode with optional message. Create a plan based on the current messages and ask user if they want to act on it
- **`/prompt <n> [--info] [key=value...]`** - Get prompt info or execute a prompt
- **`/prompts [--extension <name>]`** - List all available prompts, optionally filtered by extension
- **`/recipe [filepath]`** - Generate a recipe from the current conversation and save it to the specified filepath (must end with .yaml). If no filepath is provided, it will be saved to ./recipe.yaml
- **`/summarize`** - Summarize the current conversation to reduce context length while preserving key information
- **`/t`** - Toggle between `light`, `dark`, and `ansi` themes. [More info](#themes).
- **`/t <name>`** - Set theme directly (light, dark, ansi)

**Examples:**
```bash
# Create a plan for triaging test failures
/plan let's create a plan for triaging test failures

# List all prompts from the developer extension
/prompts --extension developer

# Switch to chat mode
/mode chat

# Add a builtin extension during the session
/builtin developer

# Clear the current conversation history
/clear
```

---

### Themes

The `/t` command controls the syntax highlighting theme for markdown content in Goose CLI responses. This affects the styles used for headers, code blocks, bold/italic text, and other markdown elements in the response output.

**Commands:**
- `/t` - Cycles through themes: `light` → `dark` → `ansi` → `light`
- `/t light` - Sets `light` theme (subtle light colors)
- `/t dark` - Sets `dark` theme (subtle darker colors)
- `/t ansi` - Sets `ansi` theme (most visually distinct option with brighter colors)

**Configuration:**
- The default theme is `dark`
- The theme setting is saved to the [configuration file](/docs/guides/config-files) as `GOOSE_CLI_THEME` and persists between sessions
- The saved configuration can be overridden for the session using the `GOOSE_CLI_THEME` [environment variable](/docs/guides/environment-variables#session-management)

:::info
Syntax highlighting styles only affect the font, not the overall terminal interface. The `light` and `dark` themes have subtle differences in font color and weight.

The Goose CLI theme is independent from the Goose Desktop theme.
:::

**Examples:**
```bash
# Set ANSI theme for the session via environment variable
export GOOSE_CLI_THEME=ansi
goose session --name use-custom-theme

# Toggle theme during a session
/t

# Set the light theme during a session
/t light
```

---

## Navigation and Controls

### Keyboard Shortcuts

**Session Control:**
- **`Ctrl+C`** - Interrupt the current request
- **`Ctrl+J`** - Add a newline

**Navigation:**
- **`Cmd+Up/Down arrows`** - Navigate through command history
- **`Ctrl+R`** - Interactive command history search (reverse search). [More info](#command-history-search).

---

### Command History Search

The `Ctrl+R` shortcut provides interactive search through your stored CLI [command history](/docs/guides/logs#command-history). This feature makes it easy to find and reuse recent commands without retyping them. When you type a search term, Goose searches backwards through your history for matches.

**How it works:**
1. Press `Ctrl+R` in your Goose CLI session
2. Type a search term
3. Navigate through the results using:
   - `Ctrl+R` to cycle backwards through earlier matches
   - `Ctrl+S` to cycle forward through newer matches
4. Press `Return` (or `Enter`) to run the found command, or `Esc` to cancel

For example, instead of retyping this long command:

```
analyze the performance issues in the sales database queries and suggest optimizations
```

Use the `"sales database"` or `"optimization"` search term to find and rerun it.

**Search tips:**
- **Distinctive terms work best**: Choose unique words or phrases to help filter the results
- **Partial matches and multiple words are supported**: You can search for phrases like `"gith"` and `"run the unit test"`