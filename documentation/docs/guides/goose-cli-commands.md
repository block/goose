# Goose CLI Commands

Goose provides a command-line interface (CLI) with several commands for managing sessions, configurations and extensions. Below is a list of the available commands and their  descriptions:

## Commands

### help

This command is used for displaying the help menu for the Goose CLI

**Usage:**
```bash
goose --help
```

### --version

This command is used for checking the current Goose version you have installed

**Usage:**
```bash
goose --version
```

### agents

This command is used for listing all available agents.

**Usage:**
```bash
goose agents
```

### `mcp <name>`

Run one of the mcp servers bundled with goose, specified by the `<name>` parameter.

**Usage:**
```bash
goose mcp <name>
```

### `session [options]`

Start or resume interactive chat sessions with goose. 

**Options:**
- **`-n, --name <NAME>`** : Name for the chat session (e.g., `'project-x'`)

- **`-p, --provider <PROVIDER>`**: Provider to use (e.g., `'openai'`, `'anthropic'`)

- **`-m, --model <MODEL>`**:Model to use (e.g., `'gpt-4'`, `'claude-3'`)

- **`-a, --agent <AGENT>`**: Agent version to use (e.g., `'default'`, `'v1'`), defaults to `'default'`

- **`-r, --resume`**: Resume a previous session (last used or specified by `--session`)


**Usage:**
```bash
goose session --resume
```

### `run [options]`

Execute commands from an instruction file or stdin

- **`-i, --instructions <FILE>`**: Path to instruction file containing commands  
- **`-t, --text <TEXT>`**: Input text to provide to Goose directly  
- **`-p, --provider <PROVIDER>`**: Provider to use (e.g., 'openai', 'anthropic')  
- **`-m, --model <MODEL>`**: Model to use (e.g., 'gpt-4', 'claude-3')  
- **`-n, --name <NAME>`**: Name for this run session (e.g., 'daily-tasks')  
- **`-a, --agent <AGENT>`**: Agent version to use (e.g., 'default', 'v1')  
- **`-r, --resume`**: Resume from a previous run  

**Usage:**
```bash
goose run --instructions plan.md
```

### `configure [options]`

Configure Goose settings - set providers, models etc. Can be run without any arguments.

- **`-p, --provider <PROVIDER>`**: AI Provider to use (e.g., 'openai', 'databricks', 'ollama')
- **`-m, --model <MODEL>`**: Model to use (e.g., 'gpt-4', 'llama2')


**Usage:**
```bash
goose configure
```