---
sidebar_position: 7
title: CLI Commands
sidebar_label: CLI Commands
---

Goose provides a command-line interface (CLI) with several commands for managing sessions, configurations and extensions. Below is a list of the available commands and their  descriptions:

## Commands

### help

Used to display the help menu

**Usage:**
```bash
goose --help
```

---

### configure [options]

Configure Goose settings - providers, extensions, etc.

**Usage:**
```bash
goose configure
```

---

### session [options]

- Start a session and give it a name

    **Options:**

    **`-n, --name <n>`**

    **Usage:**

    ```bash
    goose session --name <n>
    ```

- Resume a previous session

    **Options:**

    **`-r, --resume`**

    **Usage:**

    ```bash
    goose session --resume --name <n>
    ```

- Start a session with the specified extension

     **Options:**

     **`--with-extension <command>`**

     **Usage:**

    ```bash
    goose session --with-extension <command>
    ```

    **Examples:**

    ```bash
    goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
    ```

    With environment variable:

    ```bash
    goose session --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=<YOUR_TOKEN> npx -y @modelcontextprotocol/server-github"
    ```

- Start a session with the specified remote extension over SSE

     **Options:**

     **`--with-remote-extension <url>`**

     **Usage:**

    ```bash
    goose session --with-remote-extension <url>
    ```

    **Examples:**

    ```bash
    goose session --with-remote-extension "http://localhost:8080/sse"
    ```

- Start a session with the specified [built-in extension](/docs/getting-started/using-extensions#built-in-extensions) enabled (e.g. 'developer')

    **Options:**

    **`--with-builtin <id>`**

     **Usage:**

    ```bash
    goose session --with-builtin <id>
    ```

    **Example:**

    ```bash
    goose session --with-builtin computercontroller
    ```

---
### session list [options]

List all saved sessions.

- **`-v, --verbose`**: (Optional) Includes session file paths in the output.
- **`-f, --format <format>`**: Specify output format (`text` or `json`). Default is `text`.
- **`--ascending`**: Sort sessions by date in ascending order (oldest first). Default is descending order (newest first).

**Usage:**

```bash
# List all sessions in text format (default)
goose session list
```
```bash
# List sessions with file paths
goose session list --verbose
```

```bash
# List sessions in JSON format
goose session list --format json
```
```bash
# Sort sessions by date in ascending order.
goose session list --ascending
```
---

### session remove [options]

Remove one or more saved sessions.

**Options:**
- **`-i, --id <id>`**: Remove a specific session by its ID
- **`-r, --regex <pattern>`**: Remove sessions matching a regex pattern. For example:

**Usage:**

```bash
# Remove a specific session by ID
goose session remove -i 20250305_113223

# Remove all sessions starting with "project-"
goose session remove -r "project-.*"

# Remove all sessions containing "migration"
goose session remove -r ".*migration.*"
```

:::caution
Session removal is permanent and cannot be undone. Goose will show which sessions will be removed and ask for confirmation before deleting.
::: 

---

### info [options]

Shows Goose information, including the version, configuration file location, session storage, and logs.

- **`-v, --verbose`**: (Optional) Show detailed configuration settings, including environment variables and enabled extensions.

**Usage:**
```bash
goose info
```

---

### version

Used to check the current Goose version you have installed

**Usage:**
```bash
goose --version
```

---

### update [options]

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

### mcp

Run an enabled MCP server specified by `<n>` (e.g. `'Google Drive'`)

**Usage:**
```bash
goose mcp <n>
```

---

### run [options]

Execute commands from an instruction file or stdin. Check out the [full guide](/docs/guides/running-tasks) for more info.

**Options:**

- **`-i, --instructions <FILE>`**: Path to instruction file containing commands. Use - for stdin.
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly
- **`-s, --interactive`**: Continue in interactive mode after processing initial input
- **`-n, --name <n>`**: Name for this run session (e.g. `daily-tasks`)
- **`-r, --resume`**: Resume from a previous run
- **`--recipe <RECIPE_FILE_NAME> <OPTIONS>`**: Load a custom recipe in current session
- **`-p, --path <PATH>`**: Path for this run session (e.g. `./playground.jsonl`)
- **`--with-extension <COMMAND>`**: Add stdio extensions (can be used multiple times in the same command)
- **`--with-builtin <n>`**: Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')

**Usage:**

```bash
goose run --instructions plan.md

#Load a recipe with a prompt that Goose executes and then exits  
goose run --recipe recipe.yaml

#Load a recipe from this chat and then stays in an interactive session
goose run --recipe recipe.yaml -s

#Load a recipe containing a prompt which Goose executes and then drops into an interactive session
goose run --recipe recipe.yaml --interactive

#Generates an error: no text provided for prompt in headless mode
goose run --recipe recipe_no_prompt.yaml

```

---

### bench

Used to evaluate system-configuration across a range of practical tasks. See the [detailed guide](/docs/guides/benchmarking) for more information.

**Usage:**

```bash
goose bench ...etc.
```

### recipe
Used to validate a recipe file and get a link to share the recipe (aka "shared agent") with another Goose user.

```bash
goose recipe <COMMAND>
```

**Options:**

- **`--help, -h`**: Print this message or the help for the subcommand

**Command Usage:**

```bash
# Validate a recipe file
goose recipe validate $FILE.yaml

# Generate a deeplink for a recipe file
goose recipe deeplink $FILE.yaml

# Print this message or the help for the given command
goose recipe help
```

---
## Prompt Completion

The CLI provides a set of slash commands that can be accessed during a session. These commands support tab completion for easier use.

#### Available Commands
- `/?` or `/help` - Display this help message
- `/builtin <names>` - Add builtin extensions by name (comma-separated)
- `/exit` or `/quit` - Exit the current session
- `/extension <command>` - Add a stdio extension (format: ENV1=val1 command args...)
- `/mode <n>` - Set the goose mode to use ('auto', 'approve', 'chat')
- `/plan <message>` - Create a structured plan based on the given message
- `/prompt <n> [--info] [key=value...]` - Get prompt info or execute a prompt
- `/prompts [--extension <n>]` - List all available prompts, optionally filtered by extension
- `/recipe <recipe file name>` - Generate and save a session recipe to `recipe.yaml` or the filename specified by the command parameter.
- `/summarize` - Summarize the current session to reduce context length while preserving key information
- `/t` - Toggle between Light/Dark/Ansi themes

All commands support tab completion. Press `<Tab>` after a slash (/) to cycle through available commands or to complete partial commands. 

#### Examples
```bash
# Create a plan for triaging test failures
/plan let's create a plan for triaging test failures

# List all prompts from the developer extension
/prompts --extension developer

# Switch to chat mode
/mode chat
```


---
## Keyboard Shortcuts

Goose CLI supports several shortcuts and built-in commands for easier navigation.

- **`Ctrl+C`** - Interrupt the current request
- **`Ctrl+J`** - Add a newline
- **`Cmd+Up/Down arrows`** - Navigate through command history