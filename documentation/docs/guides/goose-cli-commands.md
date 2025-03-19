---
sidebar_position: 4
---
# CLI Commands

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

    **`-n, --name <name>`**

    **Usage:**

    ```bash
    goose session --name <name>
    ```

- Resume a previous session

    **Options:**

    **`-r, --resume`**

    **Usage:**

    ```bash
    goose session --resume --name <name>
    ```

- Start a session with the specified extension

     **Options:**

     **`--with-extension <command>`**

     **Usage:**

    ```bash
    goose session --with-extension <command>
    ```

    Can also include environment variables (e.g., `'GITHUB_TOKEN={your_token} npx -y @modelcontextprotocol/server-github'`)

- Start a session with the specified [built-in extension](/docs/getting-started/using-extensions#built-in-extensions) enabled (e.g. 'developer')

    **Options:**

    **`--with-builtin <id>`**

     **Usage:**

    ```bash
    goose session --with-builtin <id>
    ```

---
### session list [options]

List all saved sessions.

- **`-v, --verbose`**: (Optional) Includes session file paths in the output.
- **`-f, --format <format>`**: Specify output format (`text` or `json`). Default is `text`.

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

Run an enabled MCP server specified by `<name>` (e.g. 'Google Drive')

**Usage:**
```bash
goose mcp <name>
```

---

### run [options]

Execute commands from an instruction file or stdin. Check out the [full guide](/docs/guides/running-tasks) for more info.

**Options:**

- **`-i, --instructions <FILE>`**: Path to instruction file containing commands
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly
- **`-s, --interactive`**: Continue in interactive mode after processing initial input
- **`-n, --name <NAME>`**: Name for this run session (e.g. 'daily-tasks')
- **`-r, --resume`**: Resume from a previous run
- **`-p, --path <PATH>`**: Path for this run session (e.g. './playground.jsonl')
- **`--with-extension <COMMAND>`**: Add stdio extensions (can be used multiple times in the same command)
- **`--with-builtin <NAME>`**: Add builtin extensions by name (e.g., 'developer' or multiple: 'developer,github')

**Usage:**

```bash
goose run --instructions plan.md
```

---

### agents

Used to show the available implementations of the agent loop itself

**Usage:**

```bash
goose agents
```

---
## Keyboard Shortcuts

Goose CLI supports several shortcuts and built-in commands for easier navigation.

### Slash Commands
- **`/exit` or `/quit`** - Exit the session
- **`/t`** - Toggle between Light/Dark modes
- **`/?` or `/help`** - Display the help menu

### Keyboard Navigation
- **`Ctrl+C`** - Interrupt the current request
- **`Ctrl+J`** - Add a newline
- **Up/Down arrows** - Navigate through command history