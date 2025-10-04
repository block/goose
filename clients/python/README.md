# Goose Python Client

A simple and powerful Python client for the Goose AI agent.

## Features

- ✅ Simple chat interface
- ✅ Session management with context managers
- ✅ Streaming support (SSE)
- ✅ Tool confirmation handling for non-autonomous mode
- ✅ Type hints throughout
- ✅ Async support for streaming
- ✅ Optional extensions for retry and session management

## Installation

```bash
pip install goose-client
```

## Quick Start

```python
from goose_client import GooseClient

# Initialize client
client = GooseClient(api_key="your-api-key")

# Simple chat
response = client.chat("What is 2+2?")
print(response)  # "4"

# One-shot Q&A
answer = client.ask("What's the weather like?")

# With session context
with client.session() as session:
    response1 = client.chat("Remember the number 42", session.id)
    response2 = client.chat("What number did I ask you to remember?", session.id)
    print(response2)  # "You asked me to remember 42"

# Streaming
for chunk in client.stream_chat("Tell me a story"):
    print(chunk, end="", flush=True)
```

## Tool Confirmation (Non-Autonomous Mode)

When the Goose server is not in autonomous mode, it will request confirmation before executing tools:

```python
# Auto-confirm all tools
for chunk in client.stream_chat_with_confirmations(
    "List files in current directory",
    auto_confirm="allow_once"  # or "always_allow" or "deny"
):
    if isinstance(chunk, str):
        print(chunk, end="")

# Manual confirmation
for chunk in client.stream_chat_with_confirmations(
    "Create a new file",
    auto_confirm=None  # Manual handling
):
    if isinstance(chunk, str):
        print(chunk, end="")
    else:
        # Handle tool confirmation request
        print(f"Tool {chunk['toolName']} needs confirmation")
        
        # Show security prompt if present
        if chunk.get('prompt'):
            print(f"Security: {chunk['prompt']}")
        
        # Confirm the tool
        client.confirm_permission(
            chunk['id'],
            "allow_once",  # or "always_allow" or "deny"
            session.id
        )
```

## Session Management

```python
# Manual session management
session = client.create_session("/home/project")
client.chat("Analyze this project", session_id=session.id)

# List sessions
sessions = client.list_sessions()
for s in sessions[:5]:
    print(f"{s.id}: {s.working_dir}")

# Delete session
client.delete_session(session.id)
```

## Advanced Usage with Extensions

### Retry Extension

Add automatic retry with exponential backoff:

```python
from goose_client import GooseClient
from goose_client.extensions import with_retry

# Add retry capability
client = with_retry(
    GooseClient(api_key="your-key"),
    max_retries=5,
    backoff_factor=2.0
)

# Now all methods have automatic retry
response = client.chat("Hello")  # Will retry on failure
```

### Session Manager Extension

Advanced session management features:

```python
from goose_client.extensions import with_session_manager

# Add session management
client = with_session_manager(GooseClient(api_key="your-key"))

# Save and load sessions
session = client.create_session()
client.session_manager.save_session(session.id, "project_analysis")

# Later, load the session
saved_id = client.session_manager.load_session("project_analysis")
client.chat("Continue the analysis", session_id=saved_id)

# List saved sessions
saved = client.session_manager.list_saved()
print(saved)
```

## Async Support

```python
import asyncio
from goose_client import GooseClient

async def main():
    client = GooseClient(api_key="your-key")
    
    # Async streaming
    async for event in client.astream_reply(session_id, messages):
        print(event)

asyncio.run(main())
```

## Direct Access to Generated Client

For advanced users who need full control:

```python
from goose_client import GooseClient, StartAgentRequest

client = GooseClient(api_key="your-key")

# Access the generated API client directly
raw_response = client._agent_api.start_agent(
    StartAgentRequest(working_dir="/tmp", prompt="Custom prompt")
)

# Use generated models
from goose_client import Message, Content
custom_message = Message(
    role="user",
    content=[{"type": "text", "text": "Hello"}]
)
```

## Configuration

### Environment Variables

- `GOOSE_API_KEY`: Default API key
- `GOOSE_BASE_URL`: Default server URL (default: http://localhost:3000)

### Custom Configuration

```python
client = GooseClient(
    api_key="your-key",
    base_url="https://your-goose-server.com"
)
```

## Error Handling

```python
try:
    response = client.chat("Hello")
except Exception as e:
    print(f"Error: {e}")

# Check server health
if client.is_healthy:
    print("Server is running")
else:
    print("Server is down")
```

## Examples

See the `examples/` directory for more detailed examples:

- `quickstart.py` - Basic usage examples
- `streaming.py` - Streaming responses
- `tool_confirmation.py` - Handling tool confirmations in non-autonomous mode

## API Reference

### GooseClient

Main client class for interacting with Goose.

#### Methods

- `chat(text, session_id=None)` - Send a message and get response
- `stream_chat(text, session_id=None)` - Stream response chunks
- `stream_chat_with_confirmations(text, session_id=None, auto_confirm=None)` - Stream with tool confirmation support
- `ask(question)` - One-shot Q&A
- `create_session(working_dir="/tmp")` - Create a new session
- `session(working_dir="/tmp")` - Context manager for sessions
- `list_sessions()` - List all sessions
- `get_session(session_id)` - Get session details
- `delete_session(session_id)` - Delete a session
- `list_tools(session_id=None)` - List available tools
- `list_extensions()` - List available extensions
- `confirm_permission(confirmation_id, action, session_id=None)` - Confirm or deny tool execution
- `health_check()` - Check server health

#### Properties

- `is_healthy` - Boolean indicating server health
- `active_session_id` - Currently active session ID

## License

Apache 2.0

## Contributing

See [CONTRIBUTING.md](https://github.com/block/goose/blob/main/CONTRIBUTING.md) in the main Goose repository.
