# Enhanced Code Editing with AI Models

The developer extension now supports using AI models for enhanced code editing through the `str_replace` command. When configured, it will use an AI model to intelligently apply code changes instead of simple string replacement.

## Configuration

Set these environment variables to enable AI-powered code editing:

```bash
export GOOSE_EDITOR_API_KEY="your-api-key-here"
export GOOSE_EDITOR_HOST="https://api.openai.com/v1"
export GOOSE_EDITOR_MODEL="gpt-4o"
```

**All three environment variables must be set and non-empty for the feature to activate.**

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
export GOOSE_EDITOR_MODEL="claude-sonnet-4-20250514"
```

**Morph:**
```bash
export GOOSE_EDITOR_API_KEY="sk-..."
export GOOSE_EDITOR_HOST="https://api.morphllm.com/v1"
export GOOSE_EDITOR_MODEL="morph-v3-large"
```

**Relace**
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

## How it works

When you use the `str_replace` command in the text editor:

1. **Configuration check**: The system first checks if all three environment variables are properly set and non-empty.

2. **With AI enabled**: If configured, the system sends the original code and your requested change to the configured AI model, which intelligently applies the change while maintaining code structure, formatting, and context.

3. **Fallback**: If the AI API is not configured or the API call fails, it falls back to simple string replacement as before.

4. **User feedback**: The first time you use `str_replace` without AI configuration, you'll see a helpful message explaining how to enable the feature.

## Benefits

- **Context-aware editing**: The AI understands code structure and can make more intelligent changes
- **Better formatting**: Maintains consistent code style and formatting
- **Error prevention**: Can catch and fix potential issues during the edit
- **Flexible**: Works with any OpenAI-compatible API
- **Clean implementation**: Uses proper control flow instead of exception handling for configuration checks

## Implementation Details

The implementation follows idiomatic Rust patterns:
- Environment variables are checked upfront before attempting API calls
- No exceptions are used for normal control flow
- Clear separation between configured and unconfigured states
- Graceful fallback behavior in all cases

The feature is completely optional and backwards compatible - if not configured, the system works exactly as before with simple string replacement.