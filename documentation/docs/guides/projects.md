---
sidebar_position: 2
title: Projects
sidebar_label: Projects
---

Goose Projects automatically track your working directories and associated sessions, making it easy to resume work across multiple codebases with full context preservation.

## What Are Goose Projects?

A **project** in Goose is a record of a working directory where you've used Goose. Every time you run Goose, it automatically tracks the current directory, storing your path, last access time, recent instruction, and session ID for context continuity.

## Key Benefits

- **Eliminate context switching friction** - Jump between projects instantly without manual navigation
- **Preserve work context** - Resume exactly where you left off with full conversation history
- **Seamless session integration** - Maintain continuity across different codebases

:::tip Time Savings
Projects eliminate the typical 2-5 minutes lost when switching between codebases, especially valuable for developers working on multiple projects daily.
:::

## How to Use Projects

**Resume your most recent project:**
```bash
goose project  # or goose p
```

**Browse all your projects:**
```bash
goose projects  # or goose ps
```

When resuming a project, you can continue the previous session or start fresh in that directory.

## Real-World Workflow Example

Let's follow Sarah, a developer working on multiple projects throughout her day:

### Morning: API Development
```bash
cd ~/projects/ecommerce-api
goose session --name "api-auth-work"
```
*Sarah asks Goose to help implement JWT token refresh logic*

### Mid-Morning: Mobile App Bug Fix  
```bash
cd ~/projects/mobile-app
goose session
```
*Sarah gets help debugging an iOS crash in the login screen*

### Afternoon: Admin Dashboard
```bash
cd ~/projects/admin-dashboard  
goose session --name "dashboard-ui"
```
*Sarah works on creating user management interface components*

### Next Day: Quick Resume
```bash
# From any directory, quickly resume the most recent project
goose project
```

Goose shows:
```
┌ Goose Project Manager
│
◆ Choose an option:
│  ○ Resume project with session: .../admin-dashboard
│    Continue with the previous session
│  ○ Resume project with fresh session: .../admin-dashboard  
│    Change to the project directory but start a new session
│  ○ Start new project in current directory: /Users/sarah
│    Stay in the current directory and start a new session
└
```

### Later: Browse All Projects
```bash
goose projects
```

Goose displays:
```
┌ Goose Project Manager
│
◆ Select a project:
│  ○ 1  .../admin-dashboard (2025-01-07 09:15:30) [create user management interface]
│  ○ 2  .../mobile-app (2025-01-06 11:45:20) [login screen crashing on iOS]  
│  ○ 3  .../ecommerce-api (2025-01-06 09:30:15) [JWT token refresh logic]
│  ○ Cancel
└
```

Sarah can see her recent projects with timestamps and context, making it easy to choose where to continue working.



## CLI Reference

For complete command syntax and options, see the [CLI Commands Guide](/docs/guides/goose-cli-commands#project).
