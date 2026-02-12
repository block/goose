/**
 * Integration tests for the goosed binary using the TypeScript API client.
 *
 * These tests spawn a real goosed process and issue requests via the
 * auto-generated API client to verify the server is working correctly.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { startGoosed, type GoosedTestContext } from './setup';
import {
  status,
  readConfig,
  providers,
  startAgent,
  stopAgent,
  listSessions,
  getSession,
  updateAgentProvider,
} from '../../src/api';

function getPathEntries(): string[] {
  const path = process.env.PATH;

  if (!path) {
    return [];
  }

  const delimiter = process.platform === 'win32' ? ';' : ':';

  return path.split(delimiter).filter((entry) => entry.length > 0);
}

const CONSTRAINED_PATH = '/usr/bin:/bin:/usr/sbin:/sbin';

describe('goosed API integration tests', () => {
  let ctx: GoosedTestContext;

  beforeAll(async () => {
    ctx = await startGoosed(CONSTRAINED_PATH);
  });

  afterAll(async () => {
    await ctx.cleanup();
  });

  describe('health', () => {
    it('should respond to status endpoint', async () => {
      const response = await status({ client: ctx.client });
      expect(response.response.ok).toBe(true);
      expect(response.data).toBeDefined();
    });
  });

  describe('configuration', () => {
    it('should read config value (or return null for missing key)', async () => {
      const response = await readConfig({
        client: ctx.client,
        body: {
          key: 'GOOSE_PROVIDER',
          is_secret: false,
        },
      });
      expect(response.response.ok).toBe(true);
    });
  });

  describe('providers', () => {
    it('should list available providers', async () => {
      const response = await providers({ client: ctx.client });
      expect(response.response.ok).toBe(true);
      expect(response.data).toBeDefined();
      expect(Array.isArray(response.data)).toBe(true);
    });
  });

  describe('sessions', () => {
    it('should start an agent and create a session', async () => {
      const startResponse = await startAgent({
        client: ctx.client,
        body: {
          working_dir: '/tmp',
        },
      });
      expect(startResponse.response.ok).toBe(true);
      expect(startResponse.data).toBeDefined();

      const session = startResponse.data!;
      expect(session.id).toBeDefined();
      expect(session.name).toBeDefined();

      // Verify we can retrieve the session by ID
      // Note: path parameter is 'session_id' not 'id'
      const getResponse = await getSession({
        client: ctx.client,
        path: {
          session_id: session.id,
        },
      });
      expect(getResponse.response.ok).toBe(true);
      expect(getResponse.data).toBeDefined();
      expect(getResponse.data!.id).toBe(session.id);
    });

    it('should list sessions', async () => {
      const sessionsResponse = await listSessions({ client: ctx.client });
      expect(sessionsResponse.response.ok).toBe(true);
      expect(sessionsResponse.data).toBeDefined();
      expect(sessionsResponse.data!.sessions).toBeDefined();
      expect(Array.isArray(sessionsResponse.data!.sessions)).toBe(true);
    });
  });

  describe('messaging', () => {
    it('should accept a message request to /reply endpoint', async () => {
      // Start a session first
      const startResponse = await startAgent({
        client: ctx.client,
        body: {
          working_dir: '/tmp',
        },
      });
      expect(startResponse.response.ok).toBe(true);
      const sessionId = startResponse.data!.id;

      const sseResponse = await fetch(`${ctx.baseUrl}/reply`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': ctx.secretKey,
          Accept: 'text/event-stream',
        },
        body: JSON.stringify({
          session_id: sessionId,
          user_message: {
            role: 'user',
            created: Math.floor(Date.now() / 1000), // Unix timestamp in seconds
            content: [
              {
                type: 'text',
                text: 'Hello',
              },
            ],
            metadata: {
              userVisible: true,
              agentVisible: true,
            },
          },
        }),
      });

      // The endpoint should accept the request format
      // 200 = success, the SSE stream will contain the response or error
      expect(sseResponse.status).toBe(200);

      // Read just enough to verify the stream works
      const reader = sseResponse.body?.getReader();
      if (reader) {
        // Cancel after a short read - we just want to verify the endpoint works
        setTimeout(() => reader.cancel(), 1000);
        try {
          const { value } = await reader.read();
          // We should get some data back (either response or error about provider)
          expect(value).toBeDefined();
        } catch {
          // Reader was cancelled, that's fine
        }
      }

      // Cleanup - may fail if agent wasn't fully instantiated
      await stopAgent({
        client: ctx.client,
        body: {
          session_id: sessionId,
        },
      });
    });

    it('should see the full PATH when calling the developer tool', async () => {
      const currentPath = getPathEntries();

      // find a part of current path that is not in CONSTRAINED_PATH
      const pathEntry = currentPath.find((entry) => !CONSTRAINED_PATH.includes(entry));
      if (!pathEntry) {
        expect.fail(`Could not find a path entry not in ${CONSTRAINED_PATH}`);
      }

      // This test requires a configured provider
      // Check if GOOSE_PROVIDER is set in config, or use env var to configure it
      let configResponse = await readConfig({
        client: ctx.client,
        body: {
          key: 'GOOSE_PROVIDER',
          is_secret: false,
        },
      });

      // response.data is the config value directly (or null/undefined if not set)
      let providerName = configResponse.data as string | null | undefined;

      if (!providerName) {
        console.log('Skipping tool execution test - no GOOSE_PROVIDER configured');
        return;
      }

      // Read model from config (or use default)
      const modelResponse = await readConfig({
        client: ctx.client,
        body: {
          key: 'GOOSE_MODEL',
          is_secret: false,
        },
      });
      const modelName = (modelResponse.data as string | null) || undefined;

      // Start a session
      const startResponse = await startAgent({
        client: ctx.client,
        body: {
          working_dir: '/tmp',
        },
      });
      expect(startResponse.response.ok).toBe(true);
      const sessionId = startResponse.data!.id;

      // Configure the provider (and optionally model) for this session
      const providerResponse = await updateAgentProvider({
        client: ctx.client,
        body: {
          session_id: sessionId,
          provider: providerName,
          model: modelName,
        },
      });
      expect(providerResponse.response.ok).toBe(true);

      // Send a message that requires tool use
      const sseResponse = await fetch(`${ctx.baseUrl}/reply`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': ctx.secretKey,
          Accept: 'text/event-stream',
        },
        body: JSON.stringify({
          session_id: sessionId,
          user_message: {
            role: 'user',
            created: Math.floor(Date.now() / 1000),
            content: [
              {
                type: 'text',
                text: 'Use your developer shell tool to read $PATH and return its content directly, with no further information about it',
              },
            ],
            metadata: {
              userVisible: true,
              agentVisible: true,
            },
          },
        }),
      });

      expect(sseResponse.status).toBe(200);

      const reader = sseResponse.body?.getReader();
      const decoder = new TextDecoder();

      let returnedPath: string = undefined;
      if (reader) {
        const timeout = setTimeout(() => reader.cancel(), 60000); // 60s timeout

        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value, { stream: true });

            try {
              // remove data: prefix
              const data = JSON.parse(chunk.replace(/^data:/, ''));
              const output = data?.message?.content?.[0]?.toolResult?.value?.content?.[0]?.text;
              if (output && output.includes('/usr')) {
                // Got a response that includes PATH content
                clearTimeout(timeout);
                reader.cancel();
                returnedPath = output;
                break;
              }
            } catch {
              // The response we care about is always a complete JSON object. Others will be
              // incomplete, so we expect parsing errors.
            }
          }
        } catch {
          // Reader cancelled or error
        }
        clearTimeout(timeout);
      }

      await stopAgent({
        client: ctx.client,
        body: {
          session_id: sessionId,
        },
      });

      expect(returnedPath, 'the agent should return a value for $PATH').toBeDefined();
      expect(returnedPath, '$PATH should contain the expected entry').toContain(pathEntry);
    });
  });
});
