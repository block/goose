# Goosed Integration Tests

This test suite validates the `goosed` binary by issuing requests through the TypeScript API client.

## Overview

These tests spawn a real `goosed` process and exercise the HTTP API endpoints using the auto-generated TypeScript client from `src/api/`. This ensures the server binary works correctly end-to-end.

## Prerequisites

1. Build the goosed binary:
   ```bash
   cd /path/to/goose
   cargo build
   ```

2. Install npm dependencies:
   ```bash
   cd ui/desktop
   npm install
   ```

## Running Tests

```bash
# Run all integration tests
npm run test:integration

# Run in watch mode (re-runs on file changes)
npm run test:integration:watch

# Run with debug output
npm run test:integration:debug
```

## Writing Tests

### Basic Structure

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { startGoosed, type GoosedTestContext } from './setup';
import { someApiFunction } from '../../src/api';

describe('my feature tests', () => {
  let ctx: GoosedTestContext;

  beforeAll(async () => {
    ctx = await startGoosed();
  });

  afterAll(async () => {
    await ctx.cleanup();
  });

  it('should do something', async () => {
    const response = await someApiFunction({ client: ctx.client });
    expect(response.response.ok).toBe(true);
  });
});
```

### Test Context

The `startGoosed()` function returns a context object with:

- `client` - Configured API client with authentication headers
- `port` - The port the server is running on
- `baseUrl` - Full base URL (e.g., `http://127.0.0.1:12345`)
- `cleanup()` - Function to stop the server process

### Sharing a Server Instance

For tests that can share a server instance (faster execution):

```typescript
import { getSharedContext, cleanupSharedContext } from './setup';

describe('shared server tests', () => {
  let ctx: GoosedTestContext;

  beforeAll(async () => {
    ctx = await getSharedContext();
  });

  afterAll(async () => {
    await cleanupSharedContext();
  });

  // ... tests
});
```

### Available API Functions

Import API functions from `../../src/api`. Common ones include:

- `status` - Health check endpoint
- `providers` - List available LLM providers
- `readConfig` - Read a configuration value
- `readAllConfig` - Read all configuration
- `listSessions` - List chat sessions
- `startAgent` - Start the AI agent

See `src/api/sdk.gen.ts` for all available functions.

## Architecture

```
tests/integration/
├── setup.ts          # Test utilities (server spawn, client creation)
├── goosed.test.ts    # Main API tests
└── README.md         # This file
```

The setup handles:
- Finding the goosed binary (debug or release build)
- Spawning with a random available port
- Waiting for server readiness
- Configuring the API client with authentication
- Graceful shutdown and cleanup

## Troubleshooting

### "Binary not found" error
Build the project first: `cargo build` from the repository root.

### Tests timing out
- Increase `testTimeout` in `vitest.integration.config.ts`
- Check if goosed is starting correctly (run with `npm run test:integration:debug`)

### Authentication errors (401)
The setup automatically configures the `X-Secret-Key` header. If you're making manual requests, include this header with value `test`.
