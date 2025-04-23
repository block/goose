---
title: Shared Goose Configuration File
sidebar_label: Shared Goose Configuration File
sidebar_position: 14
---

The Goose Desktop and CLI agents share configuration settings by reading and writing to 
a config.yaml file stored at `~/.config/goose/config.yaml` (macOS/Linux) or `%APPDATA%\Block\goose\config\config.yaml` (Windows).


The following configuration options are shared between the CLI and desktop versions:

## Core Configuration Options

1. **Provider Configuration**
   - `GOOSE_PROVIDER`: The AI provider to use (e.g., OpenAI, Anthropic)
   - `GOOSE_MODEL`: The specific model to use from the provider

2. **Extension Management**
   - `extensions`: A map of extension configurations that includes:
     - Extension name
     - Enabled status (boolean)
     - Extension type (builtin, stdio, sse)
     - Extension-specific configuration (timeout, display name, etc.)

3. **Goose Mode**
   - `GOOSE_MODE`: Controls how Goose operates (auto, approve, smart_approve, chat)

4. **Tool Permissions**
   - Tool-specific permission settings (always_allow, ask_before, never_allow)

5. **Experiment Features**
   - Experimental feature toggles

## Secret Storage

Both versions use the same mechanism for storing secrets:
- System keyring for secure storage (macOS Keychain, Windows Credential Manager)
- Environment variables as overrides
- Fallback to file-based storage when keyring is disabled


## Desktop-Specific Settings

While the desktop app shares the core configuration with CLI, it has some additional settings that are stored separately:

1. **UI-specific settings** stored in `settings.json` in the app's user data directory:
   - Environment toggles (GOOSE_SERVER__MEMORY, GOOSE_SERVER__COMPUTER_CONTROLLER)

2. **Runtime configuration** that's not persisted in config.yaml:
   - Port settings for the Goose server
   - Working directory
   - Secret key for the current session

## Key Differences

1. **Configuration Interface**:
   - CLI: Uses command-line `goose configure` for setup
   - Desktop: Uses both the shared config.yaml and additional UI settings

2. **Extension Activation**:
   - Both use the same extension configuration format, but the desktop app has additional UI toggles for certain extensions

3. **Server Configuration**:
   - The desktop app manages server settings (port, host) dynamically at runtime
   - The CLI uses more static configuration

## Configuration Loading Precedence

Both versions follow the same precedence for loading configuration values:
1. Environment variables (highest priority)
2. Configuration file
3. Default values (lowest priority)

This shared configuration architecture allows users to switch between CLI and desktop versions while maintaining consistent settings and behavior.