---
unlisted: true
---
# Terminal Integration

The `goose term` commands let you talk to goose directly from your shell prompt. Instead of switching to a separate REPL session, you stay in your terminal and call goose when you need it.

```bash
@goose "what does this error mean?"
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

### Default Mode

For **bash** and **zsh**, you can make goose the default handler for anything that isn't a valid command:

```bash
# zsh
eval "$(goose term init zsh --default)"

# bash
eval "$(goose term init bash --default)"
```

With this enabled, anything you type that isn't a command will be sent to goose:

```bash
$ what files are in this directory?
🪿 Command 'what' not found. Asking goose...
```

Goose will interpret what you typed and help you accomplish the task.

## Usage

Once set up, your terminal session is linked to a goose session. All commands you run are logged to that session.

To talk to goose about what you've been doing:

```bash
@goose "why did that fail?"
```

You can also use `@g` as a shorter alias:

```bash
@g "explain this error"
```

Both `@goose` and `@g` are aliases for `goose term run`. They open goose with your command history already loaded.

## What Gets Logged

Every command you type gets stored. Goose sees commands you ran since your last message to it.

Commands starting with `goose term`, `@goose`, or `@g` are not logged (to avoid noise).

## Performance

- **Shell startup**: adds ~10ms
- **Per command**: ~10ms, runs in background (non-blocking)

You won't notice any delay. The logging happens asynchronously after your command starts executing.

## How It Works

`goose term init` outputs shell code that:
1. Sets a `GOOSE_SESSION_ID` environment variable linking your terminal to a goose session
2. Creates `@goose` and `@g` aliases for quick access
3. Installs a preexec hook that calls `goose term log` for each command
4. Optionally installs a command-not-found handler (with `--default`)

The hook runs `goose term log <command> &` in the background, which appends to the goose session.
When you run `@goose`, goose reads from the goose session any commands that happened since it 
was last called and incorporates them in the next call.

## Session Management

By default a new goose session is created each time you run init and
that session lasts as long as you keep that terminal open.

You can create a named session by passing --name:

```bash
eval "$(goose term init zsh --name my_project)"
```

which will create a session with the name `my_project` if it doesn't exist yet or continues
that session if it does.