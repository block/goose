---
sidebar_position: 3
title: Code Mode
sidebar_label: Code Mode
description: Understanding how Code Mode handles tool discovery and tool calling
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Code Mode is a method of interacting with MCP tools programmatically instead of calling them directly. Code Mode is particularly useful when working with many enabled extensions, as it can help manage context window usage more efficiently.

:::info
This functionality requires the built-in [Code Mode extension](/docs/mcp/code-mode-mcp) to be enabled.
:::

Code Mode controls how tools are discovered and called:

- Tools from enabled extensions are discovered on-demand and loaded into context as needed
- Multiple tool calls are batched in one execution
- Intermediate results are chained (output from one tool as input to the next)

## How Code Mode Works

The [Code Mode extension](/docs/mcp/code-mode-mcp) is an MCP server that uses the MCP protocol to expose three foundational meta-tools. When Code Mode is enabled, gosling switches to Code Mode. For every request, the LLM writes JavaScript code that gosling executes using [pctx (Port of Context)](https://portofcontext.com/) ([GitHub](https://github.com/portofcontext/pctx)), a custom Deno-based runtime, to:

- Discover available tools from your enabled extensions (if needed)
- Learn how to work with the tools it needs for the current task
- Call those tools programmatically to complete the task

### Traditional vs. Code Mode Tool Calling

Traditional MCP tool calling and Code Mode are two different approaches to the same goal: giving gosling access to tools.

| Aspect | Traditional | Code Mode |
|--------|------------------|-----------|
| **Tool Discovery** | All tools from enabled extensions, for example:<br/>• `developer.shell`<br/>• `developer.text_editor`<br/>• `github.list_issues`<br/>• `github.get_pull_request`<br/>• `slack.send_message`<br/>• ... *potentially many more* | Code Mode extension's meta-tools:<br/>• `list_functions`<br/>• `get_function_details`<br/>• `execute_typescript`<br/><br/>The LLM uses these tools to discover tools from other enabled extensions as needed |
| **Tool Calling** | • Sequential tool calls<br/>• Each result sent to the LLM before the next call | • May require tool discovery calls<br/>• Multiple tool calls batched in one execution<br/>• Intermediate results are chained and processed locally |
| **Context Window** | Every LLM call includes all tool definitions from enabled extensions | Every LLM call includes the 3 meta-tool definitions, plus any tool definitions previously discovered in the session |
| **Best For** | • 1-3 enabled extensions<br/>• Simple tasks using 1-2 tools | • 5+ extensions<br/>• Well-defined multi-step workflows |

:::info Text-Only Results
Code Mode only supports text content from tool results. Images, binary data, and other content types are ignored.
:::

