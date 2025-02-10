---
sidebar_position: 2
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

### version

Used to check the current Goose version you have installed

**Usage:**
```bash
goose --version
```

### agents

Used to list all available agents

**Usage:**
```bash
goose agents
```

### mcp

Run an enabled MCP server specified by `<name>` (e.g. 'Google Drive')

**Usage:**
```bash
goose mcp <name>
```

### session [options]

Start or resume sessions with the following options.

**Options:**
- **`-n, --name <NAME>`**

Name for the new chat session (e.g. `'project-x'`)

```bash
goose session --name <name>
```

- **`-r, --resume`** 

Resume the previous session

```bash
goose session --resume
```

- **`--with-extension <COMMAND>`** 

Starts the session with the specified extension. Can also include environment variables (e.g., `'GITHUB_TOKEN={your_token} npx -y @modelcontextprotocol/server-github'`).

```bash
goose session --name <name> --with-extension <command>
```

- **`--with-builtin <NAME>`** 

Starts the session with the specified [built-in extension](/docs/getting-started/using-extensions#built-in-extensions) enabled. (e.g. 'developer')

```bash
goose session --with-builtin <id>
```

### run [options]

Execute commands from an instruction file or stdin

- **`-i, --instructions <FILE>`**: Path to instruction file containing commands  
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly  
- **`-n, --name <NAME>`**: Name for this run session (e.g., 'daily-tasks')  
- **`-r, --resume`**: Resume from a previous run  

**Usage:**
```bash
goose run --instructions plan.md
```

### configure [options]

Configure Goose settings - providers, extensions, etc.



**Usage:**
```bash
goose configure'
```