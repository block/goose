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
  upsertConfig,
} from '../../src/api';

describe('goosed API integration tests', () => {
  let ctx: GoosedTestContext;

  beforeAll(async () => {
    ctx = await startGoosed();
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

    it('should execute a tool and return results', async () => {
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

      // Collect SSE events until the stream ends or timeout
      const events: string[] = [];
      const reader = sseResponse.body?.getReader();
      const decoder = new TextDecoder();

      if (reader) {
        const timeout = setTimeout(() => reader.cancel(), 60000); // 60s timeout

        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;

            const chunk = decoder.decode(value, { stream: true });
            events.push(chunk);

            // Check if we got a complete response (look for assistant message with text)
            const fullResponse = events.join('');
            if (fullResponse.includes('"role":"assistant"') && fullResponse.includes('/usr')) {
              // Got a response that includes PATH content
              clearTimeout(timeout);
              reader.cancel();
              break;
            }
          }
        } catch {
          // Reader cancelled or error
        }
        clearTimeout(timeout);
      }

      // Verify we received events
      expect(events.length).toBeGreaterThan(0);

      // The response should contain PATH-like content (directories separated by colons)
      const fullResponse = events.join('');
      console.log(fullResponse);

      // Should have received some SSE data events
      expect(fullResponse).toContain('data:');

      // If provider worked, we should see tool usage or response
      // The response might contain the PATH or an error about the tool
      const hasPathContent = fullResponse.includes('/usr') || fullResponse.includes('/bin');

      // At minimum, we should have gotten some meaningful response
      expect(hasPathContent).toBe(true);

      // Cleanup
      await stopAgent({
        client: ctx.client,
        body: {
          session_id: sessionId,
        },
      });
    });
  });
});
