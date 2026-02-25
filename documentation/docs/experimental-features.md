---
title: Experimental Features
description: A list of experimental features in goose.
---

# Experimental Features

This page tracks experimental features in goose. These features are still in development and may not be fully stable or ready for production use.

| Feature Key | Description |
|-------------|-------------|
| `ollama_toolshim` | Enable tool calling capabilities for language models that don't natively support tool calling (like DeepSeek) using an experimental local interpreter model setup. |
| `mobile_access` | Enable remote access to goose Desktop from the goose AI mobile app via secure tunneling. |
| `vscode_extension` | Interact with goose directly from VS Code via ACP. |
| `acp_clients` | Interact with goose natively in ACP-compatible clients like Zed. |
| `mcp_ui` | Renders interactive UI components from MCP Apps and MCP-UI extensions. |
| `parallel_subrecipes` | Running subrecipes in parallel for faster execution. |

## Managing Experiments

Experiments can be enabled or disabled via the `goose configure` command under `goose settings` -> `Toggle Experiment`.
