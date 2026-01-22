# goosed-sdk (TypeScript)

TypeScript SDK for goosed API.

## Installation

```bash
npm install @goosed/sdk
```

Or install from source:

```bash
cd sdk/typescript
npm install
npm run build
```

## Prerequisites

The SDK requires a running goosed server. Start the server before using the SDK:

```bash
# From the goose repository root
source bin/activate-hermit
cargo build --release
./target/release/goosed --secret-key test
```

The server will start on `http://127.0.0.1:3000` by default.

## Quick Start

```typescript
import { GoosedClient } from '@goosed/sdk';

// Create client
const client = new GoosedClient({
  baseUrl: 'http://127.0.0.1:3000',
  secretKey: 'your-secret-key',
});

// Check status
console.log(await client.status()); // "ok"

// Create a session
const session = await client.startSession('/path/to/working/dir');
console.log(`Session ID: ${session.id}`);

// Resume session (load extensions)
const { session: resumed, extensionResults } = await client.resumeSession(session.id);

// Get available tools
const tools = await client.getTools(session.id);
for (const tool of tools) {
  console.log(`- ${tool.name}: ${tool.description.slice(0, 50)}...`);
}

// Call a tool directly
const result = await client.callTool(session.id, 'todo__todo_write', { content: 'My TODO' });
console.log(result.content);

// Send a message (streaming)
for await (const event of client.sendMessage(session.id, 'Hello!')) {
  if (event.type === 'Message' && event.message) {
    const content = event.message.content as Array<{ type: string; text?: string }>;
    for (const c of content) {
      if (c.type === 'text') {
        console.log(c.text);
      }
    }
  } else if (event.type === 'Finish') {
    console.log(`Done (tokens: ${event.token_state?.totalTokens})`);
  }
}

// Or get full response
const response = await client.chat(session.id, 'What can you do?');
console.log(response);

// Clean up
await client.stopSession(session.id);
await client.deleteSession(session.id);
```

## API Reference

### Status
- `status()` - Health check, returns "ok"
- `systemInfo()` - Get system information (version, provider, model, OS)

### Agent
- `startSession(workingDir)` - Create new session with working directory
- `resumeSession(sessionId, loadModelAndExtensions?)` - Resume session and load extensions
- `restartSession(sessionId)` - Restart agent in session
- `stopSession(sessionId)` - Stop active session
- `getTools(sessionId, extensionName?)` - Get available tools
- `callTool(sessionId, name, args)` - Call a tool directly

### Chat
- `sendMessage(sessionId, text)` - Send message and stream events (AsyncGenerator)
- `chat(sessionId, text)` - Send message and get full response text

### Sessions
- `listSessions()` - List all sessions
- `getSession(sessionId)` - Get session details
- `updateSessionName(sessionId, name)` - Update session name
- `deleteSession(sessionId)` - Delete session
- `exportSession(sessionId)` - Export session data

## Testing

### Unit Tests

Run the test suite (requires goosed server on port 3002 by default):

```bash
npm test
```

Override the server URL and secret key if needed:

```bash
GOOSED_BASE_URL=http://127.0.0.1:3000 GOOSED_SECRET_KEY=test npm test
```

### Integration Test

Run the complete integration test with DeepSeek:

```bash
# 1. Start goosed server
cargo build --release
./target/release/goosed agent

# 2. In another terminal, run the integration test
cd sdk/typescript
npm run test:integration
```

The integration test validates all SDK capabilities:
- ✓ Status check and system info
- ✓ Session management (create, resume, list, get, update, delete)
- ✓ Extension loading
- ✓ Tool discovery and execution
- ✓ Chat (both streaming and non-streaming)
- ✓ Session export
- ✓ Session restart

## License

Apache-2.0
