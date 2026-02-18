/**
 * Integration tests for the goosed binary using the TypeScript API client.
 *
 * These tests spawn a real goosed process and issue requests via the
 * auto-generated API client to verify the server is working correctly.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { setupGoosed, type GoosedTestContext } from './setup';
import {
  status,
  readConfig,
  providers,
  startAgent,
  stopAgent,
  listSessions,
  getSession,
  updateAgentProvider,
  reply,
} from '../../src/api';
import { execSync } from 'child_process';
import os from 'node:os';

const CONSTRAINED_PATH = '/usr/bin:/bin:/usr/sbin:/sbin';

function getUserPath(): string[] {
  try {
    const userShell = process.env.SHELL || '/bin/bash';
    const path = execSync(`${userShell} -l -i -c 'echo $PATH'`, {
      encoding: 'utf-8',
      timeout: 5000,
      env: {
        PATH: CONSTRAINED_PATH,
      },
    }).trim();

    const delimiter = process.platform === 'win32' ? ';' : ':';
    return path.split(delimiter).filter((entry: string) => entry.length > 0);
  } catch (error) {
    console.error('Error executing shell:', error);
    throw error;
  }
}

describe('goosed API integration tests', () => {
  let ctx: GoosedTestContext;

  beforeAll(async () => {
    const configYaml = `
extensions:
  developer:
    enabled: true
    type: builtin
    name: developer
    description: General development tools useful for software engineering.
    display_name: Developer
    timeout: 300
    bundled: true
    available_tools: []
`;

    ctx = await setupGoosed({ pathOverride: '/usr/bin:/bin', configYaml });
  });

  afterAll(async () => {
    await ctx.cleanup();
  });

  describe('health', () => {
    it('should respond to status endpoint', async () => {
      const response = await status({ client: ctx.client });
      expect(response.response).toBeOkResponse();
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
      expect(response.response).toBeOkResponse();
    });
  });

  describe('providers', () => {
    it('should list available providers', async () => {
      const response = await providers({ client: ctx.client });
      expect(response.response).toBeOkResponse();
      expect(response.data).toBeDefined();
      expect(Array.isArray(response.data)).toBe(true);
    });
  });

  describe('sessions', () => {
    it('should start an agent and create a session', async () => {
      const startResponse = await startAgent({
        client: ctx.client,
        body: {
          working_dir: os.tmpdir(),
        },
      });
      expect(startResponse.response).toBeOkResponse();
      expect(startResponse.data).toBeDefined();

      const session = startResponse.data!;
      expect(session.id).toBeDefined();
      expect(session.name).toBeDefined();

      const getResponse = await getSession({
        client: ctx.client,
        path: {
          session_id: session.id,
        },
      });
      expect(getResponse.response).toBeOkResponse();
      expect(getResponse.data).toBeDefined();
      expect(getResponse.data!.id).toBe(session.id);
    });

    it('should list sessions', async () => {
      const sessionsResponse = await listSessions({ client: ctx.client });
      expect(sessionsResponse.response).toBeOkResponse();
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
          working_dir: os.tmpdir(),
        },
      });
      expect(startResponse.response).toBeOkResponse();
      const sessionId = startResponse.data!.id;

      const abortController = new AbortController();
      const { stream } = await reply({
        client: ctx.client,
        body: {
          session_id: sessionId,
          user_message: {
            role: 'user',
            created: Math.floor(Date.now() / 1000),
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
        },
        throwOnError: true,
        signal: abortController.signal,
      });

      const timeout = setTimeout(() => abortController.abort(), 1000);
      try {
        for await (const event of stream) {
          expect(event).toBeDefined();
          break;
        }
      } catch {
        // Aborted or error, that's fine
      }
      clearTimeout(timeout);

      await stopAgent({
        client: ctx.client,
        body: {
          session_id: sessionId,
        },
      });
    });
  });

  describe('the developer tool', () => {
    it('should see the full PATH when calling the developer tool', async (testContext) => {
      const currentPath = getUserPath();

      const pathEntry = currentPath.find((entry) => !CONSTRAINED_PATH.includes(entry));
      if (!pathEntry) {
        expect.fail(`Could not find a path entry not in ${CONSTRAINED_PATH}`);
      }

      let configResponse = await readConfig({
        client: ctx.client,
        body: {
          key: 'GOOSE_PROVIDER',
          is_secret: false,
        },
      });

      let providerName = configResponse.data as string | null | undefined;

      if (!providerName) {
        testContext.skip('Skipping tool execution test - no GOOSE_PROVIDER configured');
        return;
      }

      const modelResponse = await readConfig({
        client: ctx.client,
        body: {
          key: 'GOOSE_MODEL',
          is_secret: false,
        },
      });
      const modelName = (modelResponse.data as string | null) || undefined;

      const startResponse = await startAgent({
        client: ctx.client,
        body: {
          working_dir: os.tmpdir(),
        },
      });
      expect(startResponse.response).toBeOkResponse();
      const sessionId = startResponse.data!.id;

      const providerResponse = await updateAgentProvider({
        client: ctx.client,
        body: {
          session_id: sessionId,
          provider: providerName,
          model: modelName,
        },
      });
      expect(providerResponse.response).toBeOkResponse();

      const abortController = new AbortController();
      const { stream } = await reply({
        client: ctx.client,
        body: {
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
        },
        throwOnError: true,
        signal: abortController.signal,
      });

      let returnedPath: string | undefined = undefined;
      const timeout = setTimeout(() => abortController.abort(), 60000); // 60s timeout

      try {
        for await (const event of stream) {
          console.log('stream: ', JSON.stringify(event));

          if (event.type === 'Message') {
            const content = event.message?.content?.[0];
            if (content?.type === 'toolResponse') {
              const toolResult = content as {
                toolResult?: { value?: { content?: Array<{ text?: string }> } };
              };
              const output = toolResult?.toolResult?.value?.content?.[0]?.text;
              if (output && output.includes('/usr')) {
                clearTimeout(timeout);
                abortController.abort();
                returnedPath = output;
                break;
              }
            }
          }
        }
      } catch (error) {
        // Aborted or error
        if (!(error instanceof Error && error.name === 'AbortError')) {
          console.log('Stream error: ', error);
        }
      }
      clearTimeout(timeout);

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
