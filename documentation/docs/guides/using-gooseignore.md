---
title: Prevent goose from Accessing Files
sidebar_label: Using gooseignore
sidebar_position: 80
---


`.gooseignore` is a text file that defines patterns for files and directories that goose will not access. This means goose cannot read, modify, delete, or run shell commands on these files when using the Developer extension's tools.

:::info Developer extension only
The .gooseignore feature currently only affects tools in the [Developer](/docs/mcp/developer-mcp) extension. Other extensions are not restricted by these rules.
:::

This guide will show you how to use `.gooseignore` files to prevent goose from changing specific files and directories.

## Creating your `.gooseignore` file

goose supports two types of `.gooseignore` files:
- **Global ignore file** - Create a `.gooseignore` file in `~/.config/goose`. These restrictions will apply to all your sessions with goose, regardless of directory.
- **Local ignore file** - Located at the root of your project directory. Goose automatically creates this file with default patterns if it doesn't exist (see [Auto-Created Local `.gooseignore`](#3-auto-created-local-gooseignore) below). You can edit it to add project-specific restrictions.

:::tip
You can use both global and local `.gooseignore` files simultaneously. When both exist, goose will combine the restrictions from both files to determine which paths are restricted.
:::

## Example `.gooseignore` file

In your `.gooseignore` file, you can write patterns to match files you want goose to ignore. Here are some common patterns:

```plaintext
# Ignore specific files by name
settings.json         # Ignore only the file named "settings.json"

# Ignore files by extension
*.pdf                # Ignore all PDF files
*.config             # Ignore all files ending in .config

# Ignore directories and their contents
backup/              # Ignore everything in the "backup" directory
downloads/           # Ignore everything in the "downloads" directory

# Ignore all files with this name in any directory
**/credentials.json  # Ignore all files named "credentials.json" in any directory

# Complex patterns
*.log                # Ignore all .log files
!error.log           # Except for error.log file
```

## Ignore File Types and Priority
goose respects ignore rules from global `.gooseignore` and local `.gooseignore` files. It uses a priority system to determine which files should be ignored. 

### 1. Global `.gooseignore`
- Highest priority and always applied first
- Located at `~/.config/goose/.gooseignore`
- Affects all projects on your machine

```
~/.config/goose/
└── .gooseignore      ← Applied to all projects
```

### 2. Local `.gooseignore`
- Project-specific rules
- Located in your project root directory

```
~/.config/goose/
└── .gooseignore      ← Global rules applied first

Project/
├── .gooseignore      ← Local rules applied second
└── src/
```

### 3. Auto-Created Local `.gooseignore`

When goose starts in a directory that doesn't have a local `.gooseignore` file, it automatically creates one with sensible defaults. This makes the ignore rules visible and easy to customize.

The auto-created file contains:

```plaintext
# This file is created automatically if no .gooseignore exists.
# Customize or uncomment the patterns below instead of deleting the file.
# Removing it will simply cause goose to recreate it on the next start.
#
# Suggested patterns you can uncomment if desired:
# **/.ssh/**        # block SSH keys and configs
# **/*.key         # block loose private keys
# **/*.pem         # block certificates/private keys
# **/.git/**        # block git metadata entirely
# **/target/**     # block Rust build artifacts
# **/node_modules/** # block JS/TS dependencies
# **/*.db          # block local database files
# **/*.sqlite      # block SQLite databases
#

**/.env
**/.env.*
**/secrets.*
```

The three active patterns at the bottom protect sensitive files by default. The commented patterns above are suggestions you can enable by removing the `#` prefix.

:::tip
If you don't want the auto-created `.gooseignore` file tracked in version control, add `.gooseignore` to your project's `.gitignore`.
:::

## Common use cases

Here are some typical scenarios where `.gooseignore` is helpful:

- **Generated Files**: Prevent goose from modifying auto-generated code or build outputs
- **Third-Party Code**: Keep goose from changing external libraries or dependencies
- **Important Configurations**: Protect critical configuration files from accidental modifications
- **Version Control**: Prevent changes to version control files like `.git` directory
- **Custom Restrictions**: Create `.gooseignore` files to define which files goose should not access 