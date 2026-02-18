---
title: Peekaboo Extension
description: Use Peekaboo for macOS screen capture and GUI automation with Goose
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The Peekaboo extension provides macOS GUI automation through annotated screenshots and element targeting. It enables a **see → click → type** workflow for visual UI interaction.

:::info macOS Only
Peekaboo requires **macOS 15+ (Sequoia)** with Screen Recording and Accessibility permissions. It is not available on Linux or Windows.
:::

## How It Works

1. **`peekaboo_see`** — captures an annotated screenshot showing all UI elements with IDs (B1=button, T2=text field, L3=link)
2. **`peekaboo_click`** — clicks on elements by their ID from the `see` output
3. **`peekaboo_type`** — types text into the focused field

Additional tools:
- **`peekaboo_hotkey`** — press keyboard shortcuts (e.g., cmd+c, cmd+shift+t)
- **`peekaboo_app`** — launch, quit, switch, or list applications

## Setup

Enable the Peekaboo extension — Goose will **auto-install Peekaboo via Homebrew** on first use if it's not already installed.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

  1. Open Goose Desktop Settings
  2. Go to **Extensions**
  3. Enable **Peekaboo**

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  ```sh
  goose configure
  # → Toggle Extensions → Enable peekaboo
  ```

  </TabItem>
</Tabs>

Grant permissions when prompted:
- **Screen Recording** — required for screenshots
- **Accessibility** — required for UI element discovery and interaction

Check permission status:
```bash
peekaboo permissions status
```

To install manually instead of auto-install:
```bash
brew install steipete/tap/peekaboo
```

## Tools

| Tool | Description |
|------|-------------|
| `peekaboo_see` | Capture annotated screenshot with UI element IDs |
| `peekaboo_click` | Click elements by ID or coordinates |
| `peekaboo_type` | Type text into focused elements |
| `peekaboo_app` | Launch, quit, switch, or list applications |
| `peekaboo_hotkey` | Press keyboard shortcuts (e.g., cmd+c, cmd+shift+t) |

## Example Usage

### See → Click → Type Workflow

```
Log into the website open in Safari
```

Goose will:
1. Use `peekaboo_see` with `app: "Safari"` to capture an annotated screenshot
2. Identify the username field (e.g., `T1`) and password field (`T2`) from element IDs
3. Use `peekaboo_click` with `on: "T1"` to focus the username field
4. Use `peekaboo_type` with the username
5. Click and type the password similarly
6. Click the login button (`B3` or whatever ID it has)

### App Management

```
Open Notes and create a new note with a shopping list
```

Goose will:
1. Use `peekaboo_app` to launch Notes
2. Use `peekaboo_hotkey` with `"cmd,n"` for new note
3. Use `peekaboo_type` to enter the shopping list

## Alternative: Peekaboo as External MCP Server

For access to the **full Peekaboo tool set** (20+ tools including window management, menu interaction, scrolling, dragging, etc.), you can run Peekaboo as a standalone MCP server instead:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

  1. Open Settings → Extensions → Add Extension
  2. Select **Stdio** type
  3. Configure:
     - **Name**: `peekaboo`
     - **Command**: `peekaboo mcp serve`

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  Add to your config file (`~/.config/goose/config.yaml`):

  ```yaml
  extensions:
    peekaboo:
      enabled: true
      config:
        type: stdio
        name: peekaboo
        description: Full Peekaboo MCP server for macOS GUI automation
        cmd: peekaboo
        args:
          - mcp
          - serve
        timeout: 300
  ```

  </TabItem>
</Tabs>

## Troubleshooting

### Permission Issues
```bash
peekaboo permissions status
peekaboo permissions grant
```

### Auto-Install Failed
If Homebrew is not available, install Peekaboo manually:
```bash
# Install Homebrew first if needed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Then install Peekaboo
brew install steipete/tap/peekaboo
```

## Learn More

- [Peekaboo GitHub](https://github.com/steipete/Peekaboo)
- [Peekaboo Documentation](https://github.com/steipete/Peekaboo/tree/main/docs)
- [Computer Controller Extension](./computer-controller-mcp.md)
