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
- **Local ignore file** - Create a `.gooseignore` file at the root of the directory you'd like it applied to. These restrictions will only apply when working in a specific directory.

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
goose respects ignore rules from three sources: global `.gooseignore`, local `.gooseignore`, and `.gitignore`. It uses a priority system to determine which files should be ignored. 

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
- Overrides `.gitignore` completely

```
~/.config/goose/
└── .gooseignore      ← Global rules applied first

Project/
├── .gooseignore      ← Local rules applied second
├── .gitignore        ← Ignored when .gooseignore exists
└── src/
```

### 3. `.gitignore` Fallback
- Used when no local `.gooseignore` exists
- goose automatically uses your `.gitignore` rules
- If a global `.gooseignore` file exists, those rules will be applied in addition to the `.gitignore` patterns.

```
Project/
├── .gitignore        ← Used by goose (when no local .gooseignore)
└── src/
```

### 4. Default Patterns
By default, if you haven't created any .gooseignore files and no .gitignore file exists, goose will not modify files matching these patterns:
```plaintext
**/.env
**/.env.*
**/secrets.*
```

## Common use cases

Here are some typical scenarios where `.gooseignore` is helpful:

- **Generated Files**: Prevent goose from modifying auto-generated code or build outputs
- **Third-Party Code**: Keep goose from changing external libraries or dependencies
- **Important Configurations**: Protect critical configuration files from accidental modifications
- **Version Control**: Prevent changes to version control files like `.git` directory
- **Existing Projects**: Most projects already have `.gitignore` files that work automatically as ignore patterns for goose
- **Custom Restrictions**: Create `.gooseignore` when you need different patterns than your `.gitignore` (e.g., allowing goose to read files that Git ignores)