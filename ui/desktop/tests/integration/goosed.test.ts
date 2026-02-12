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
      console.log(startResponse);
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

      // Cleanup - stop the agent
      const stopResponse = await stopAgent({
        client: ctx.client,
        body: {
          session_id: session.id,
        },
      });
      console.log(stopResponse);
      expect(stopResponse.response.ok).toBe(true);
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

      // Send a message via the /reply endpoint
      // The ChatRequest requires session_id and user_message (Message type)
      // Note: Message fields use camelCase due to #[serde(rename_all = "camelCase")]
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
