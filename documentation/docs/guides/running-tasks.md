---
sidebar_position: 75
title: Running Tasks
sidebar_label: Run Tasks
---

When working with the goose CLI, you can pass files and instructions to the `goose run` command to execute tasks and workflows. This could be a simple one-liner command or a complex set of instructions stored in a file.

## Basic Usage

The `goose run` command starts a new session, begins executing using any arguments provided and exits the session automatically once the task is complete. 

There are multiple ways to run tasks with goose; check out the [list of options](/docs/guides/goose-cli-commands.md#run-options).

### Text in the command
```bash
goose run -t "your instructions here"
```

Using the `-t` flag, one is able to pass a text instruction directly to the command. This is great for quick, one-off commands where you do not need an interactive session with goose. The instructions will be executed, and the session will end. An example usage could be using in a CI/CD pipeline or running alongside other scripts.

### Using an instruction file
If you have a complex set of instructions or a workflow that you want to automate, you can store them in a file and pass it to the `goose run` command:

```bash
goose run -i instructions.md
```

Here's an example of an instruction file that runs a security audit on project dependencies:

```md
# Dependency Security Audit

1. Analyze project dependencies:
   - Check package.json and requirements.txt files
   - List all dependencies with versions
   - Identify outdated packages

2. Security check:
   - Run npm audit (for JavaScript packages)
   - Check for known vulnerabilities in Python packages
   - Identify dependencies with critical security issues

3. Create an upgrade plan:
   - List packages requiring immediate updates
   - Note breaking changes in latest versions
   - Estimate impact of required updates

Save findings in 'security_audit.md' with severity levels highlighted.
```

### With stdin
You can also pass instructions to goose using standard input via `-i -`. This is useful when you want to pipe commands from another tool or script into goose.

#### Simple echo pipe

```bash
echo "What is 2+2?" | goose run -i -
```

#### Multi-line instructions
```bash
cat << EOF | goose run -i -
Please help me with these tasks:
1. Calculate 15% of 85
2. Convert 32°C to Fahrenheit
EOF
```

## Key Features

### Interactive Mode

If you don't want goose to exit at the end of the task, you can pass the `-s` or `--interactive` flag to start an interactive session after processing your initial commands:

```bash
goose run -i instructions.txt -s
```

This is useful when you want to continue working with goose after your initial commands are processed.

### Session Management

You can name and manage your sessions:

```bash
# Start a new named session
goose run -n my-project -t "initial instructions"

# Resume a previous session
goose run -n my-project -r
```

You can also run commands without creating or storing a session file by using the `--no-session` flag. This is useful for automated scripts, or one-off tasks where you don't need to maintain the conversation history or state. This flag routes the session output to a temporary null path (`/dev/null` on Unix or `NUL` on Windows), and discards it when complete.

```bash
# Run a command without creating a session file
goose run --no-session -t "your command here"
```
### Set Provider and Model
You can run goose sessions with a specific provider and model, which overrides the provider and model settings in your [environment variables](/docs/guides/environment-variables.md).

```bash
goose run --provider anthropic --model claude-4-sonnet -t "initial prompt"
```

### Working with Extensions

If you want to ensure specific extensions are available when running your task, you can indicate this with arguments. This can be done using the `--with-extension`, `--with-remote-extension`, `--with-streamable-http-extension`, or `--with-builtin` flags:

- Using built-in extensions e.g developer and computercontroller extensions

```bash
goose run --with-builtin "developer,computercontroller" -t "your instructions"
```

- Using custom extensions

```bash
goose run --with-extension "ENV1=value1 custom-extension-args" -t "your instructions"
```

- Using remote SSE extensions

```bash
goose run --with-remote-extension "url" -t "your instructions"
```

- Using streamable HTTP extensions

```bash
goose run --with-streamable-http-extension "https://example.com/streamable" -t "your instructions"
```

### Debug Mode

When troubleshooting or developing complex workflows, you can enable debug mode to get more detailed information about tool execution. The `--debug` flag provides:

- Complete tool responses
- Detailed parameter values
- Full file paths

Debug mode can be useful when:
- Developing new automation scripts
- Troubleshooting extension behavior
- Verifying tool parameters and responses

```bash
# Run a task with debug output enabled
goose run --debug -t "your instructions"

# Debug a recipe execution
goose run --debug --recipe recipe.yaml
```

### JSON Output Format

For automation, scripting, and CI/CD integration, you can get structured output from `goose run` using the `--output-format json` flag:

```bash
# Run with JSON output for automation
goose run --output-format json -t "your instructions"

# Run a recipe with JSON output
goose run --output-format json --recipe recipe.yaml

# Combine with other options
goose run --output-format json --no-session -t "automated task"
```

The JSON output includes:
- Session metadata and execution results
- Tool outputs and any errors
- Structured data suitable for parsing by scripts and CI/CD pipelines

## Common Use Cases

### Running Script Files

Create an instruction file (e.g., `build-script.txt`):
```text
Check the current branch
Run the test suite
Build the documentation
```

Then run it:
```bash
goose run -i build-script.txt
```

### Quick Commands

For one-off commands, use the text option:
```bash
goose run -t "Create a CHANGELOG.md entry comparing current git branch with main"
```

### Development Workflows

Start a session with specific extensions:
```bash
goose run --with-builtin "developer,git" -n dev-session -s
```

### Combining Options

You can combine multiple options to create powerful workflows:

```bash
# Complex example combining multiple options
goose run \
  --with-builtin "developer,git" \
  --with-extension "API_KEY=xyz123 custom-tool" \
  -n project-setup \
  -t "Initialize project" 
```

This command:
1. Loads the developer and git built-in extensions
2. Adds a custom extension with an API key
3. Names the session "project-setup"
4. Starts with "Initialize project" instruction
5. Exits automatically after processing the command.