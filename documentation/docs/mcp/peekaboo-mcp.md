---
title: Peekaboo (macOS GUI Automation)
description: How Computer Controller uses Peekaboo for macOS UI automation
---

# Peekaboo — macOS GUI Automation

On macOS, the **Computer Controller** extension's `computer_control` tool uses [Peekaboo](https://github.com/steipete/Peekaboo) for GUI automation. Peekaboo is auto-installed via Homebrew on first use — no separate extension or configuration needed.

## Requirements

- **macOS 15+** (Sequoia)
- **Screen Recording** and **Accessibility** permissions
- **Homebrew** (for auto-install)

## How It Works

When you enable the Computer Controller extension (the same one available on all platforms), the `computer_control` tool on macOS becomes a Peekaboo CLI passthrough. The agent passes subcommand strings like `"see --app Safari --annotate"` through the `command` parameter.

On Windows and Linux, `computer_control` continues to work as before (PowerShell / shell scripts).

## Checking Permissions

```bash
peekaboo permissions status
```

If permissions are missing, grant them in **System Settings → Privacy & Security**.

## Manual Install

If auto-install fails:

```bash
# Install Homebrew if needed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Peekaboo
brew install steipete/tap/peekaboo
```

## Alternative: Peekaboo as External MCP Server

For access to Peekaboo's full MCP tool set, you can also run it as a standalone MCP server:

```yaml
extensions:
  peekaboo:
    name: peekaboo
    cmd: peekaboo
    args: [mcp, serve]
```

## Links

- [Peekaboo GitHub](https://github.com/steipete/Peekaboo)
- [Peekaboo Documentation](https://github.com/steipete/Peekaboo/tree/main/docs)
