---
sidebar_position: 5
title: Managing Projects
sidebar_label: Managing Projects
---

gosling Projects automatically track your working directories and associated sessions, making it easy to resume work across multiple codebases with full context preservation.

A **project** in gosling is a record of a working directory where you've used gosling. Every time you run gosling, it automatically tracks the current directory as a project, storing:

- **Path**: The absolute path to the project directory
- **Last accessed**: When you last worked on this project  
- **Last instruction**: The most recent command you gave to gosling
- **Session ID**: The associated session for context continuity

Projects are stored in `~/.local/share/gosling/projects.json`.

:::info CLI Only Feature
Projects are currently available only through the gosling CLI. Desktop support is planned for future releases.
:::

## Basic Usage

**Resume your most recent project:**

```bash
gosling project  
```

**Browse all your projects:**

```bash
gosling projects  
```
:::tip
When resuming a project, you can continue the previous session or start fresh in that directory.
:::

For complete command syntax and options, see the [CLI Commands Guide](/docs/guides/gosling-cli-commands#project).

## Workflow Example

Let's follow Sarah, a developer working on multiple projects throughout her day:

### Morning: API Development
```bash
cd ~/projects/ecommerce-api
gosling session --name "api-auth-work"
```
*Sarah asks gosling to help implement JWT token refresh logic*

### Mid-Morning: Mobile App Bug Fix  
```bash
cd ~/projects/mobile-app
gosling session
```
*Sarah gets help debugging an iOS crash in the login screen*

### Afternoon: Admin Dashboard
```bash
cd ~/projects/admin-dashboard  
gosling session --name "dashboard-ui"
```
*Sarah works on creating user management interface components*

### Next Day: Quick Resume
```bash
# From any directory, quickly resume the most recent project
gosling project
```

gosling shows:
```
┌ gosling Project Manager
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
gosling projects
```

gosling displays:
```
┌ gosling Project Manager
│
◆ Select a project:
│  ○ 1  .../admin-dashboard (2025-01-07 09:15:30) [create user management interface]
│  ○ 2  .../mobile-app (2025-01-06 11:45:20) [login screen crashing on iOS]  
│  ○ 3  .../ecommerce-api (2025-01-06 09:30:15) [JWT token refresh logic]
│  ○ Cancel
└
```

Sarah can see her recent projects with timestamps and context, making it easy to choose where to continue working.

## Benefits

:::tip Time Savings
Projects eliminate the typical 2-5 minutes lost when switching between codebases, especially valuable for developers working on multiple projects daily.
:::

- **Eliminate context switching friction** - Jump between projects instantly without manual navigation
- **Preserve work context** - Resume exactly where you left off with full conversation history
- **Seamless session integration** - Maintain continuity across different codebases

