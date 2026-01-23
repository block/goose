# goosed-sdk (Python)

Python SDK for goosed API.

## Installation

```bash
pip install goosed-sdk
```

Or install from source:

```bash
cd sdk/python
pip install -e .
```

## Prerequisites

The SDK requires a running goosed server. Start the server before using the SDK:

```bash
# From the goose repository root
source bin/activate-hermit
cargo build --release
./target/release/goosed --secret-key test agent
```

The server will start on `http://127.0.0.1:3000` by default.

## Quick Start

```python
from goosed_sdk import GoosedClient

# Create client
client = GoosedClient(
    base_url="http://127.0.0.1:3000",
    secret_key="your-secret-key",
)

# Check status
print(client.status())  # "ok"

# Create a session
session = client.start_session("/path/to/working/dir")
print(f"Session ID: {session.id}")

# Resume session (load extensions)
resumed, extension_results = client.resume_session(session.id)

# Get available tools
tools = client.get_tools(session.id)
for tool in tools[:5]:
    print(f"- {tool.name}: {tool.description[:50]}...")

# Call a tool directly
result = client.call_tool(session.id, "todo__todo_write", {"content": "My TODO"})
print(result.content)

# Send a message (streaming)
for event in client.send_message(session.id, "Hello!"):
    if event.type == "Message" and event.message:
        for c in event.message.get("content", []):
            if c.get("type") == "text":
                print(c.get("text", ""), end="")
    elif event.type == "Finish":
        print(f"\nDone (tokens: {event.token_state.total_tokens if event.token_state else 'N/A'})")

# Or get full response
response = client.chat(session.id, "What can you do?")
print(response)

# Clean up
client.stop_session(session.id)
client.delete_session(session.id)
```

## Async Usage

```python
import asyncio
from goosed_sdk import AsyncGoosedClient

async def main():
    client = AsyncGoosedClient(
        base_url="http://127.0.0.1:3000",
        secret_key="your-secret-key",
    )

    # Check status
    print(await client.status())  # "ok"

    # Create and use session
    session = await client.start_session("/tmp/test")
    await client.resume_session(session.id)

    # Chat
    response = await client.chat(session.id, "Hello!")
    print(response)

    # Clean up
    await client.stop_session(session.id)
    await client.delete_session(session.id)

asyncio.run(main())
```

## API Reference

### Status
- `status()` - Health check, returns "ok"
- `system_info()` - Get system information (version, provider, model, OS)

### Agent
- `start_session(working_dir)` - Create new session with working directory
- `resume_session(session_id, load_model_and_extensions=True)` - Resume session and load extensions
- `restart_session(session_id)` - Restart agent in session
- `stop_session(session_id)` - Stop active session
- `get_tools(session_id, extension_name=None)` - Get available tools
- `call_tool(session_id, name, arguments)` - Call a tool directly

### Chat
- `send_message(session_id, text)` - Send message and stream events (Generator)
- `chat(session_id, text)` - Send message and get full response text

### Sessions
- `list_sessions()` - List all sessions
- `get_session(session_id)` - Get session details
- `update_session_name(session_id, name)` - Update session name
- `delete_session(session_id)` - Delete session
- `export_session(session_id)` - Export session data

## Testing

### Integration Tests

Run the test suite (requires goosed server on port 3002 by default):

```bash
# 1. Start goosed server
GOOSE_PORT=3002 GOOSE_SERVER__SECRET_KEY=test-secret cargo run -p goose-server --bin goosed -- agent

# 2. In another terminal, run the tests
cd sdk/python
pip install -e ".[dev]"
GOOSED_BASE_URL=http://127.0.0.1:3002 GOOSED_SECRET_KEY=test-secret pytest tests/ -v
```

## License

Apache-2.0
