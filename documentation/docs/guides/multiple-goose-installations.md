---
sidebar_position: 26
title: Running Multiple goose Installations
sidebar_label: Multiple Installations
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Running two versions of goose side by side — for example, the open-source release alongside a custom distribution built by your organisation — is a supported and encouraged pattern. This guide explains how to set them up without conflicts, and why running both is worth it.

## Why run both?

| Version | Best for |
|---|---|
| Open-source goose | Personal projects, community extensions, OSS contributions |
| Custom distribution (e.g. your org's build) | Preconfigured providers, internal tools, bundled extensions |

They complement each other. You don't have to choose. Running both means you keep full access to the OSS ecosystem while getting the productivity benefits of your organisation's preconfigured setup.

## How conflicts happen

Both versions install a binary called `goose`. On macOS and Linux, your shell resolves the name using `$PATH` — whichever directory appears first wins. The **desktop apps** (`.app` bundles on macOS, `.deb`/`.AppImage` on Linux) never conflict; they run independently. The clash is **terminal-only**.

Check which binary your shell currently resolves:

```sh
which goose
```

If you have both installed, only one will win. The fix is to give each a stable, predictable path.

## Setup

<Tabs groupId="os">
  <TabItem value="mac" label="macOS" default>

### Option 1 — Shell aliases (recommended)

Add these to your `~/.zshrc` (or `~/.bashrc`):

```sh
# Open-source goose (Homebrew)
alias goose-oss="/opt/homebrew/bin/goose"

# Custom distribution (adjust path to match your install)
alias goose="/Applications/Goose.app/Contents/MacOS/goose"
```

Then reload your shell:

```sh
source ~/.zshrc
```

You can name the aliases anything that makes sense to you. The key is that each points to a fully-qualified path, bypassing `$PATH` resolution entirely.

### Option 2 — PATH ordering

If you prefer to keep `goose` as the name for one version and use the other by full path only, put the version you want as default **first** in your `$PATH`:

```sh
# In ~/.zshrc — put custom distro first
export PATH="/Applications/Goose.app/Contents/MacOS:$PATH"
```

Then invoke the other by full path when needed.

  </TabItem>
  <TabItem value="linux" label="Linux">

### Shell aliases

Add these to your `~/.bashrc` or `~/.zshrc`:

```sh
# Open-source goose (installed via script or package manager)
alias goose-oss="$HOME/.local/bin/goose"

# Custom distribution
alias goose="/opt/your-org-goose/bin/goose"
```

Reload:

```sh
source ~/.bashrc
```

  </TabItem>
  <TabItem value="windows" label="Windows">

### PowerShell aliases

Add to your PowerShell profile (`$PROFILE`):

```powershell
# Open-source goose
Set-Alias goose-oss "C:\Users\you\AppData\Local\goose\goose.exe"

# Custom distribution
Set-Alias goose "C:\Program Files\YourOrgGoose\goose.exe"
```

  </TabItem>
</Tabs>

## Keeping config separate

Each goose installation reads from `~/.config/goose/` by default. If both versions share this directory, they will share provider keys and extension config — which is usually fine, but can cause unexpected behaviour if the two builds use different config schemas.

To give a version its own isolated config, set the `GOOSE_CONFIG_DIR` environment variable before launching:

```sh
GOOSE_CONFIG_DIR=~/.config/goose-oss goose-oss session
```

Or add it permanently to your alias:

```sh
alias goose-oss="GOOSE_CONFIG_DIR=~/.config/goose-oss /opt/homebrew/bin/goose"
```

## Verify both are working

```sh
goose --version
goose-oss --version
```

Both should print their respective version numbers. If either prints the same path as the other, recheck your alias definitions with `type goose` and `type goose-oss`.

## Desktop apps

Desktop app installs (`.app`, `.deb`, `.exe`) are independent of the terminal setup above. macOS and Linux will happily run two `.app` / `.deb` installs simultaneously — they are separate processes with separate windows. No extra configuration needed.

If both apps appear in your Dock or launcher with the same icon and name, rename one at the OS level (right-click → Rename on macOS) to tell them apart visually. This does not affect functionality or auto-updates.

## Custom distributions

If you are building a custom distribution of goose for your organisation, consider setting a distinct `productName` in your build config so the binary ships with a unique name. This removes the need for alias configuration entirely for your users.

See [Custom Distributions](/docs/guides/custom-distributions) for details on building and packaging your own goose distro.
