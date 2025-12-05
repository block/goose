---
title: Enhanced Code Editing with AI Models
sidebar_label: Enhanced Code Editing
description: Use AI models to intelligently apply code changes
sidebar_position: 110
---

The [Developer extension](/docs/mcp/developer-mcp) supports using AI models for enhanced code editing through the `str_replace` command. When configured, it intelligently applies code changes using an AI model instead of simple string replacement.

The use of models specializing in code editing can reduce the load on the main LLM providers while increasing accuracy, quality, and speed and lowering cost. This enhanced approach provides:

- **Context-aware editing**: The AI understands code structure and can make more intelligent changes
- **Better formatting**: Maintains consistent code style and formatting
- **Error prevention**: Can catch and fix potential issues during the edit
- **Flexible model support**: Works with any OpenAI-compatible API
- **Clean implementation**: Uses proper control flow instead of exception handling for configuration checks

## Configuration

Set these [environment variables](/docs/guides/environment-variables#enhanced-code-editing) to enable AI-powered code editing:

```bash
export GOOSE_EDITOR_API_KEY="your-api-key-here"
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"
```

**All three environment variables must be set and non-empty for the feature to activate.** 

This optional feature is completely backwards compatible - if not configured, the extension works exactly as before with simple string replacement.

### Supported Providers

Any OpenAI-compatible API endpoint should work. Examples:

**OpenAI:**
```bash
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"
```

**Anthropic (via OpenAI-compatible proxy):**
```bash
export GOOSE_EDITOR_API_KEY="sk-ant-..."
export GOOSE_EDITOR_HOST="https://api.anthropic.com/v1"
export GOOSE_EDITOR_MODEL="claude-3-5-sonnet-20241022"
```

**Morph:**
```bash
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.morphllm.com/v1"
export GOOSE_EDITOR_MODEL="morph-v0"
```

**Relace:**
```bash
export GOOSE_EDITOR_API_KEY="rlc-..."
export GOOSE_EDITOR_HOST="https://instantapply.endpoint.relace.run/v1/apply"
export GOOSE_EDITOR_MODEL="auto"
```

**Local/Custom endpoints:**
```bash
export GOOSE_EDITOR_API_KEY="your-key"
export GOOSE_EDITOR_HOST="http://localhost:8000/v1"
export GOOSE_EDITOR_MODEL="your-model"
```

## How It Works

When the `str_replace` tool is used to edit code:

1. **Configuration Check**: goose checks if all three environment variables are properly set and non-empty.

2. **With AI Enabled**: If configured, goose sends the original code and your requested change to the configured AI model for processing.

3. **Fallback**: If the AI API is not configured or the API call fails, it falls back to simple string replacement.

4. **User Feedback**: The first time you use `str_replace` without AI configuration, you'll see a helpful message explaining how to enable the feature.
