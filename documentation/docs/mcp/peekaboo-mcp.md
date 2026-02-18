---
title: Peekaboo Extension
description: Use Peekaboo for enhanced macOS screen capture and GUI automation with Goose
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The Peekaboo extension brings high-fidelity screen capture, UI element discovery, and native GUI automation to Goose on macOS. It complements the built-in Computer Controller extension by providing pixel-accurate annotated screenshots, accessibility-based element targeting, and direct interaction tools.

:::info macOS Only
Peekaboo requires **macOS 15+ (Sequoia)** with Screen Recording and Accessibility permissions. It is not available on Linux or Windows.
:::

## Why Peekaboo?

While Goose's built-in `screen_capture` tool provides basic screenshots, Peekaboo enables a powerful **see → click → type** workflow:

1. **`peekaboo_see`** — captures an annotated screenshot showing all UI elements with IDs (B1=button, T2=text field, L3=link)
2. **`peekaboo_click`** — clicks on elements by their ID from the `see` output
3. **`peekaboo_type`** — types text into the focused field

This is far more reliable than AppleScript UI scripting for visual interaction tasks.

## Install Peekaboo

```bash
brew install steipete/tap/peekaboo
```

Grant permissions when prompted:
- **Screen Recording** — required for screenshots
- **Accessibility** — required for UI element discovery and interaction

Check permission status:
```bash
peekaboo permissions status
```

## Built-in Integration

When Peekaboo is installed (`peekaboo` on PATH), the **Computer Controller** extension automatically gains Peekaboo tools:

| Tool | Description |
|------|-------------|
| `peekaboo_see` | Capture annotated screenshot with UI element IDs |
| `peekaboo_click` | Click elements by ID, label, or coordinates |
| `peekaboo_type` | Type text into focused elements |
| `peekaboo_app` | Launch, quit, switch, or list applications |
| `peekaboo_hotkey` | Press keyboard shortcuts (e.g., cmd+c, cmd+shift+t) |

These tools appear alongside the existing Computer Controller tools. No additional configuration needed — just install Peekaboo and enable the Computer Controller extension.

### Enable Computer Controller

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

  1. Open Goose Desktop Settings
  2. Go to **Extensions**
  3. Enable **Computer Controller**

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  ```sh
  goose configure
  # → Toggle Extensions → Enable computercontroller
  ```

  </TabItem>
</Tabs>

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

### Keyboard Shortcuts

```
Copy the selected text and paste it into a new Terminal window
```

Goose will use `peekaboo_hotkey` for cmd+c, then `peekaboo_app` to switch to Terminal, and `peekaboo_hotkey` for cmd+v.

## Alternative: Peekaboo as External MCP Server

For access to the **full Peekaboo tool set** (20+ tools including window management, menu interaction, scrolling, dragging, etc.), you can also run Peekaboo as a standalone MCP server:

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

This gives access to additional tools like `window` (move/resize), `menu` (click menus), `scroll`, `drag`, `dock`, `dialog`, `space`, and more.

## Using with Computer Controller

Peekaboo tools work alongside the existing Computer Controller tools:

- **Peekaboo tools** (`peekaboo_see`, `peekaboo_click`, `peekaboo_type`) — best for visual UI interaction: clicking buttons, filling forms, reading screen content
- **`computer_control`** (AppleScript) — best for application scripting, system settings, and programmatic automation
- **`automation_script`** (shell/Ruby) — best for file processing, data manipulation, CLI tasks

## Troubleshooting

### Peekaboo Not Found
```bash
# Check if installed
which peekaboo

# Install
brew install steipete/tap/peekaboo
```

### Permission Issues
```bash
peekaboo permissions status
peekaboo permissions grant
```

### Tools Return "Peekaboo is not installed"
The Peekaboo tools check for the `peekaboo` binary at runtime. Ensure it's on your PATH and restart Goose.

## Learn More

- [Peekaboo GitHub](https://github.com/steipete/Peekaboo)
- [Peekaboo Documentation](https://github.com/steipete/Peekaboo/tree/main/docs)
- [Computer Controller Extension](./computer-controller-mcp.md)
