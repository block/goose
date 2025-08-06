---
title: Quick Goose Tips
sidebar_position: 6
sidebar_label: Quick Tips
description: Best practices for working with Goose
---

### Goose works on your behalf
Goose is an AI agent, which means you can prompt Goose to perform tasks for you like opening applications, running shell commands, automating workflows, writing code, browsing the web, and more.

### Prompt Goose using natural language
You don't need fancy language or special syntax to prompt Goose. Talk with Goose like you would talk to a friend. You can even use slang or say please and thank you; Goose will understand.

### Extend Goose's capabilities to any application
Goose's capabilities are extensible. As an [MCP](https://modelcontextprotocol.io/) client, Goose can connect to your apps and services through [extensions](/extensions), allowing it to work across your entire workflow.

### Choose how much control Goose has
You can customize how much [supervision](/docs/guides/goose-permissions) Goose needs. Choose between full autonomy, requiring approval before actions, or simply chatting without any actions.

### Choose the right LLM
Your experience with Goose is shaped by your [choice of LLM](/blog/2025/03/31/goose-benchmark), as it handles all the planning while Goose manages the execution. When choosing an LLM, consider its tool support, specific capabilities, and associated costs.

### Keep sessions short
LLMs have context windows, which are limits on how much conversation history they can retain. Once exceeded, they may forget earlier parts of the conversation. Monitor your token usage and [start new sessions](/docs/guides/managing-goose-sessions) as needed.

### Turn off unnecessary extensions or tool
Turning on too many extensions can degrade performance. Enable only essential [extensions and tools](/docs/guides/managing-tools/tool-permissions) to improve tool selection accuracy, save context window space, and stay within provider tool limits.

### Teach Goose your preferences
Help Goose remember how you like to work by using [`.goosehints` or other context files](/docs/guides/using-goosehints/) for permanent project preferences and the [Memory extension](/docs/mcp/memory-mcp) for things you want Goose to dynamically recall later. Both can help save valuable context window space while keeping your preferences available.

### Protect sensitive files
Goose is often eager to make changes. You can stop it from changing specific files by creating a [.gooseignore](/docs/guides/using-gooseignore) file. In this file, you can list all the file paths you want it to avoid.

### Version Control
Commit your code changes early and often. This allows you to rollback any unexpected changes.

### Control which extensions Goose can use
Administrators can use an [allowlist](/docs/guides/allowlist) to restrict Goose to approved extensions only. This helps prevent risky installs from unknown MCP servers.

### Set up starter templates
You can turn a successful session into a reusable "[recipe](/docs/guides/recipes/session-recipes)" to share with others or use again later—no need to start from scratch.

### Embrace an experimental mindset
You don’t need to get it right the first time. Iterating on prompts and tools is part of the workflow.

### Keep Goose updated
Regularly [update](/docs/guides/updating-goose) Goose to benefit from the latest features, bug fixes, and performance improvements.
