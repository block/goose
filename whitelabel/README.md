# Whitelabel Configs

Whitelabel configs customize Goose's branding, system prompt, extensions, skills, and tools for a specific use case.

## Quick Start

```bash
# Run the example
just run-ui-whitelabel whitelabel/example/whitelabel.yaml

# Run with an external config
just run-ui-whitelabel /path/to/your/whitelabel.yaml
```

## Config Structure

```
my-whitelabel/
  whitelabel.yaml     # Main config file
  skills/             # Optional: skill directories with SKILL.md + scripts
    my-skill/
      SKILL.md
      scripts/
  bin/                # Optional: CLI tool binaries
```

## How It Works

1. **Build time**: Vite plugin reads the YAML, bundles referenced skills/tools into the app
2. **Launch**: Electron applies provider/model/extension config to goosed
3. **Session creation**: System prompt override and extension overrides are sent to goosed
4. **Skills**: Copied to `{workingDir}/.goose/skills/` at startup, discovered by the `summon` extension
5. **Gateway (Telegram)**: Inherits working dir, system prompt, and extensions from the whitelabel config

## Key Sections

| Section | Purpose |
|---------|---------|
| `branding` | App name, greeting, logo, starter prompts, home screen |
| `defaults` | Provider, model, system prompt, extensions, skills, tools, working dir |
| `features` | Hide nav items, model selector, navigation labels, setting tabs |
| `window` | Window size, resizable, always on top |

See `example/whitelabel.yaml` for a commented template.
