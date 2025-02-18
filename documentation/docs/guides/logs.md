---
title: Logging System
sidebar_position: 5
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Goose Logging System

Goose uses a unified storage system for conversations and interactions. All conversations and interactions (both CLI and Desktop) are stored **locally** in:

  * `~/.local/share/goose/sessions/` - Session records
  * `~/.local/state/goose/logs/` - System logs (CLI, server, extensions)

:::info Privacy
Goose is a local application and all log files are stored locally. These logs are never sent to external servers or third parties, ensuring that all data remains private and under your control.
:::



## Session Records

Goose maintains session records in `~/.local/share/goose/sessions/` that track the conversation history and interactions for each session. These files use the `.jsonl` format (JSON Lines), where each line is a valid JSON object representing a message or interaction.

Session files are named with the pattern `[session-id].jsonl` where the session ID matches the identifier used in the corresponding log files. For example, `ccK9OTmS.jsonl` corresponds to log files like `20250211_133920-ccK9OTmS.log`.

Each session file contains a chronological record of:
- User messages and commands
- Assistant (Goose) responses
- Tool requests and their results
- Timestamps for all interactions
- Role information (user/assistant)
- Message content and formatting
- Tool call details including:
  - Tool IDs
  - Arguments passed
  - Results returned
  - Success/failure status

Each line in a session file is a JSON object with the following key fields:
- `role`: Identifies the source ("user" or "assistant")
- `created`: Timestamp of the interaction
- `content`: Array of interaction elements, which may include:
  - Text messages
  - Tool requests
  - Tool responses
  - Error messages

## System Logs

### Main System Log

The main system log is located at `~/.local/state/goose/logs/goose.log`. This log contains general application-level logging including:

* Session file locations
* Token usage statistics as well as token counts (input, output, total)
* LLM information (model names, versions)


### Desktop Application Log

The log file for the Goose Desktop application is `~/Library/Application Support/Goose/logs/main.log` and focuses on the desktop application's operation rather than conversation content or tool usage.

The Desktop application follows macOS conventions for its own operational logs and state data, but uses the standard Goose session records for actual conversations and interactions. This means your conversation history is consistent regardless of which interface you use to interact with Goose.

### CLI Logs 

The CLI logs are located under `~/.local/state/goose/logs/cli` and contain information about CLI sessions and MCP extensions used within CLI sessions. 

CLI session logs contain:
* Tool invocations and responses
* Command execution details
* Session identifiers
* Timestamps

Extension logs contain:
* Tool initialization
* Tool capabilities and schemas
* Extension-specific operations
* Command execution results
* Error messages and debugging information
* Extension configuration states
* Extension-specific protocol information

### Server Logs

The Server logs are located under `~/.local/state/goose/logs/server` and contain information about the Goose daemon (`goosed`), which is a local server process that runs on your computer. This server component manages communication between the CLI, extensions, and LLMs. 

Server logs include:
* Server initialization details
* JSON-RPC communication logs
* Server capabilities
* Protocol version information
* Client-server interactions
* Extension loading and initialization
* Tool definitions and schemas
* Extension instructions and capabilities
* Debug-level transport information
* System capabilities and configurations
* Operating system information
* Working directory information
* Transport layer communication details
* Message parsing and handling information
* Request/response cycles
* Error states and handling
* Extension initialization sequences
