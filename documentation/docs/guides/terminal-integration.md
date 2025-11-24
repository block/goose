---
unlisted: true
---
# Terminal Integration

The `goose term` commands let you talk to goose directly from your shell prompt. Instead of switching to a separate REPL session, you stay in your terminal and call goose when you need it.

```bash
gt "what does this error mean?"
```

Goose responds, you read the answer, and you're back at your prompt. The conversation lives alongside your work, not in a separate window you have to manage.

## Command History Awareness

The real power comes from shell integration. Once set up, goose tracks the commands you run, so when you ask a question, it already knows what you've been doing.

No more copy-pasting error messages or explaining "I ran these commands...". Just work normally, then ask goose for help.

## Setup

Add one line to your shell config:

**zsh** (`~/.zshrc`)
```bash
eval "$(goose term init zsh)"
```

**bash** (`~/.bashrc`)
```bash
eval "$(goose term init bash)"
```

**fish** (`~/.config/fish/config.fish`)
```fish
goose term init fish | source
```

**PowerShell** (`$PROFILE`)
```powershell
Invoke-Expression (goose term init powershell)
```

Then restart your terminal or source the config.

### Advanced: Command Not Found Handler

For **bash** and **zsh**, you can enable automatic goose invocation for unknown commands:

```bash
# zsh
eval "$(goose term init zsh --with-command-not-found)"

# bash
eval "$(goose term init bash --with-command-not-found)"
```

With this enabled, any command that doesn't exist will automatically be sent to goose:

```bash
$ analyze_logs
🪿 Command 'analyze_logs' not found. Asking goose...
```

Goose will interpret what you meant and either suggest the correct command or help you accomplish the task.

## Usage

Once set up, your terminal session is linked to a goose session. All commands you run are logged to that session.

To talk to goose about what you've been doing:

```bash
gt "why did that fail?"
```

`gt` is just an alias for `goose term run`. It opens goose with your command history already loaded.

## What Gets Logged

Every command you type gets stored with its timestamp. Goose sees commands you ran since your last message to it.

Commands starting with `goose term` or `gt` are not logged (to avoid noise).

## Performance

- **Shell startup**: adds ~10ms
- **Per command**: ~10ms, runs in background (non-blocking)

You won't notice any delay. The logging happens asynchronously after your command starts executing.

## How It Works

`goose term init` outputs shell code that:
1. Sets a `GOOSE_SESSION_ID` environment variable linking your terminal to a goose session
2. Creates the `gt` alias for quick access
3. Installs a preexec hook that calls `goose term log` for each command
4. Optionally installs a command-not-found handler (with `--with-command-not-found`)

The hook runs `goose term log <command> &` in the background, which appends to a local history file in `~/.config/goose/shell-history/`. When you run `gt`, goose reads commands from this file that were logged since your last message.

## Session Management

Terminal sessions are tied to your working directory. If you `cd` to a different project, goose automatically creates or switches to a session for that directory. This keeps conversations organized by project.

Sessions created by `goose term` don't appear in `goose session list` - they're hidden to avoid cluttering your session history. But they're real sessions with full conversation history that you can access by ID if needed.
