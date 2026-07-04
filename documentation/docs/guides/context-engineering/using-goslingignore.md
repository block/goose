---
title: Prevent gosling from Accessing Files
sidebar_label: Using goslingignore
sidebar_position: 9
---


`.goslingignore` is a text file that defines patterns for files and directories that gosling will not access. This means gosling cannot read, modify, delete, or run shell commands on these files when using the Developer extension's tools.

:::info Developer extension only
The .goslingignore feature currently only affects tools in the [Developer](/docs/mcp/developer-mcp) extension. Other extensions are not restricted by these rules.
:::

This guide will show you how to use `.goslingignore` files to prevent gosling from changing specific files and directories.

## Creating your `.goslingignore` file

gosling supports two types of `.goslingignore` files:
- **Global ignore file** - Create a `.goslingignore` file in `~/.config/gosling`. These restrictions will apply to all your sessions with gosling, regardless of directory.
- **Local ignore file** - Create a `.goslingignore` file at the root of the directory you'd like it applied to. These restrictions will only apply when working in a specific directory.

:::tip
You can use both global and local `.goslingignore` files simultaneously. When both exist, gosling will apply patterns from both files, with local patterns able to override global ones using negation.
:::

## Example `.goslingignore` file

In your `.goslingignore` file, you can write patterns to match files you want gosling to ignore. Here are some common patterns:

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
```

## Negation Patterns

Use the `!` prefix to exclude files from ignore rules. This allows you to ignore broad patterns while allowing specific exceptions.

Within each `.goslingignore` file, patterns are processed in order from top to bottom, so later patterns can override earlier ones. Negation patterns also work across files - you can use negation in your local `.goslingignore` to allow access to files blocked by your global `.goslingignore`.

```plaintext
# Ignore all environment files
**/.env*

# But allow the example file
!.env.example

# Ignore all log files
*.log

# But allow error logs
!error.log

# Ignore all JSON files in the config directory
config/*.json

# But allow the template
!config/template.json
```

:::tip Pattern Order Matters
Negation patterns must come after the patterns they're negating. The `!` pattern re-includes files that were previously ignored.
:::

## Ignore File Types and Priority

gosling respects ignore rules from global and local `.goslingignore` files, using a priority system where later patterns can override earlier ones.

### When You Have Ignore Files

When `.goslingignore` files exist, patterns are applied in this order:

1. **Global `.goslingignore`** (applied first)
   - Located at `~/.config/gosling/.goslingignore`
   - Affects all projects on your machine

2. **Local `.goslingignore`** (applied second, can override global)
   - Located in the current working directory (the root of the directory you want these rules applied to)
   - Project-specific rules that can override global patterns

```
~/.config/gosling/
└── .goslingignore      ← Global patterns applied first

Project/
├── .goslingignore      ← Local patterns applied second (can override global)
└── src/
```

Because patterns are processed in order, you can use negation patterns in your local `.goslingignore` to allow access to files that were blocked by global patterns.

**Example: Override global restrictions in a specific project**

```plaintext
# In ~/.config/gosling/.goslingignore (global)
**/.env*              # Block all .env files everywhere

# In your-project/.goslingignore (local)
!.env.example         # Allow .env.example in this project only
```

In this example, `.env` and `.env.local` remain blocked, but `.env.example` is accessible in this specific project.

### Default Patterns (No Ignore Files)

If you haven't created any `.goslingignore` files (neither global nor local), gosling automatically protects these sensitive files:

```plaintext
**/.env
**/.env.*
**/secrets.*
```

:::info
These default patterns are only active when **no** `.goslingignore` files exist. Once you create either a global or local `.goslingignore` file, you'll need to add these patterns yourself if you want to keep them.
:::

## Common use cases

Here are some typical scenarios where `.goslingignore` is helpful:

- **Generated Files**: Prevent gosling from modifying auto-generated code or build outputs
- **Third-Party Code**: Keep gosling from changing external libraries or dependencies
- **Important Configurations**: Protect critical configuration files from accidental modifications
- **Version Control**: Prevent changes to version control files like `.git` directory
- **Custom Restrictions**: Create `.goslingignore` files to define which files gosling should not access 
