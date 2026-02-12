/**
 * Integration tests for the goosed binary using the TypeScript API client.
 *
 * These tests spawn a real goosed process and issue requests via the
 * auto-generated API client to verify the server is working correctly.
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { startGoosed, type GoosedTestContext } from './setup';
import { status, readConfig, providers } from '../../src/api';

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
      // The endpoint should succeed even if the key doesn't exist
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
});
