/**
 * Unit tests for Clerk webhook signature verification
 *
 * Clerk uses Svix for webhook delivery. Webhooks include these headers:
 * - svix-id: Unique message ID
 * - svix-timestamp: Unix timestamp when sent
 * - svix-signature: HMAC-SHA256 signature
 *
 * The signature is computed as:
 * HMAC-SHA256(webhook_secret, `${svix_id}.${svix_timestamp}.${body}`)
 *
 * See: https://clerk.com/docs/webhooks/sync-data#verify-the-webhook-signature
 */

import { describe, it, expect } from 'vitest';
import { createHmac } from 'crypto';

// ============== Signature Verification (to be extracted to worker) ==============

/**
 * Verify Clerk webhook signature
 *
 * @param {string} payload - Raw request body
 * @param {object} headers - Request headers (svix-id, svix-timestamp, svix-signature)
 * @param {string} secret - Clerk webhook signing secret (starts with whsec_)
 * @param {number} [toleranceSeconds=300] - Max age of timestamp in seconds
 * @returns {{ valid: boolean, error?: string }}
 */
function verifyClerkSignature(payload, headers, secret, toleranceSeconds = 300) {
  const svixId = headers['svix-id'];
  const svixTimestamp = headers['svix-timestamp'];
  const svixSignature = headers['svix-signature'];

  // Check required headers
  if (!svixId || !svixTimestamp || !svixSignature) {
    return { valid: false, error: 'Missing required Svix headers' };
  }

  // Parse timestamp
  const timestamp = parseInt(svixTimestamp, 10);
  if (isNaN(timestamp)) {
    return { valid: false, error: 'Invalid timestamp' };
  }

  // Check timestamp tolerance (prevent replay attacks)
  const now = Math.floor(Date.now() / 1000);
  if (Math.abs(now - timestamp) > toleranceSeconds) {
    return { valid: false, error: 'Timestamp outside tolerance window' };
  }

  // Parse secret (remove whsec_ prefix if present)
  const secretBytes = secret.startsWith('whsec_')
    ? Buffer.from(secret.slice(6), 'base64')
    : Buffer.from(secret, 'base64');

  // Compute expected signature
  const signedPayload = `${svixId}.${svixTimestamp}.${payload}`;
  const expectedSignature = createHmac('sha256', secretBytes)
    .update(signedPayload)
    .digest('base64');

  // Parse signature header (format: v1,signature or just signature)
  const signatures = svixSignature.split(' ').map(sig => {
    const parts = sig.split(',');
    return parts.length === 2 ? parts[1] : parts[0];
  });

  // Check if any signature matches
  const isValid = signatures.some(sig => sig === expectedSignature);

  if (!isValid) {
    return { valid: false, error: 'Signature mismatch' };
  }

  return { valid: true };
}

/**
 * Generate a valid signature for testing
 */
function generateTestSignature(payload, svixId, svixTimestamp, secret) {
  const secretBytes = secret.startsWith('whsec_')
    ? Buffer.from(secret.slice(6), 'base64')
    : Buffer.from(secret, 'base64');

  const signedPayload = `${svixId}.${svixTimestamp}.${payload}`;
  return createHmac('sha256', secretBytes)
    .update(signedPayload)
    .digest('base64');
}

// ============== Tests ==============

describe('verifyClerkSignature', () => {
  // Test secret (base64 encoded)
  const testSecret = 'whsec_' + Buffer.from('test-secret-key-for-testing').toString('base64');
  const testSecretRaw = Buffer.from('test-secret-key-for-testing').toString('base64');

  describe('header validation', () => {
    it('rejects missing svix-id', () => {
      const result = verifyClerkSignature(
        '{"test": true}',
        { 'svix-timestamp': '1234567890', 'svix-signature': 'v1,abc' },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Missing');
    });

    it('rejects missing svix-timestamp', () => {
      const result = verifyClerkSignature(
        '{"test": true}',
        { 'svix-id': 'msg_123', 'svix-signature': 'v1,abc' },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Missing');
    });

    it('rejects missing svix-signature', () => {
      const result = verifyClerkSignature(
        '{"test": true}',
        { 'svix-id': 'msg_123', 'svix-timestamp': '1234567890' },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Missing');
    });

    it('rejects invalid timestamp format', () => {
      const result = verifyClerkSignature(
        '{"test": true}',
        { 'svix-id': 'msg_123', 'svix-timestamp': 'not-a-number', 'svix-signature': 'v1,abc' },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid timestamp');
    });
  });

  describe('timestamp tolerance', () => {
    it('rejects timestamp too old', () => {
      const oldTimestamp = Math.floor(Date.now() / 1000) - 600; // 10 minutes ago
      const payload = '{"test": true}';
      const svixId = 'msg_old';
      const signature = generateTestSignature(payload, svixId, String(oldTimestamp), testSecret);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(oldTimestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret,
        300 // 5 minute tolerance
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('tolerance');
    });

    it('rejects timestamp too far in future', () => {
      const futureTimestamp = Math.floor(Date.now() / 1000) + 600; // 10 minutes from now
      const payload = '{"test": true}';
      const svixId = 'msg_future';
      const signature = generateTestSignature(payload, svixId, String(futureTimestamp), testSecret);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(futureTimestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret,
        300
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('tolerance');
    });

    it('accepts timestamp within tolerance', () => {
      const recentTimestamp = Math.floor(Date.now() / 1000) - 60; // 1 minute ago
      const payload = '{"test": true}';
      const svixId = 'msg_recent';
      const signature = generateTestSignature(payload, svixId, String(recentTimestamp), testSecret);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(recentTimestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret,
        300
      );

      expect(result.valid).toBe(true);
    });
  });

  describe('signature verification', () => {
    it('accepts valid signature with v1 prefix', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"type":"user.created","data":{"id":"user_123"}}';
      const svixId = 'msg_valid';
      const signature = generateTestSignature(payload, svixId, String(timestamp), testSecret);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret
      );

      expect(result.valid).toBe(true);
    });

    it('accepts valid signature without v1 prefix', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"type":"user.deleted"}';
      const svixId = 'msg_noprefix';
      const signature = generateTestSignature(payload, svixId, String(timestamp), testSecret);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': signature
        },
        testSecret
      );

      expect(result.valid).toBe(true);
    });

    it('accepts secret without whsec_ prefix', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"test": true}';
      const svixId = 'msg_rawsecret';
      const signature = generateTestSignature(payload, svixId, String(timestamp), testSecretRaw);

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecretRaw
      );

      expect(result.valid).toBe(true);
    });

    it('rejects tampered payload', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const originalPayload = '{"amount":100}';
      const svixId = 'msg_tampered';
      const signature = generateTestSignature(originalPayload, svixId, String(timestamp), testSecret);

      // Attacker modifies payload
      const tamperedPayload = '{"amount":1000000}';

      const result = verifyClerkSignature(
        tamperedPayload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Signature mismatch');
    });

    it('rejects wrong secret', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"test": true}';
      const svixId = 'msg_wrongsecret';
      const signature = generateTestSignature(payload, svixId, String(timestamp), testSecret);

      const wrongSecret = 'whsec_' + Buffer.from('wrong-secret').toString('base64');

      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,${signature}`
        },
        wrongSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Signature mismatch');
    });

    it('rejects replayed message with different id', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"test": true}';
      const originalId = 'msg_original';
      const signature = generateTestSignature(payload, originalId, String(timestamp), testSecret);

      // Attacker tries to replay with different ID
      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': 'msg_replayed',
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,${signature}`
        },
        testSecret
      );

      expect(result.valid).toBe(false);
      expect(result.error).toContain('Signature mismatch');
    });
  });

  describe('multiple signatures', () => {
    it('accepts if any signature in space-separated list matches', () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const payload = '{"test": true}';
      const svixId = 'msg_multi';
      const validSignature = generateTestSignature(payload, svixId, String(timestamp), testSecret);

      // Multiple signatures (old rotation + new)
      const result = verifyClerkSignature(
        payload,
        {
          'svix-id': svixId,
          'svix-timestamp': String(timestamp),
          'svix-signature': `v1,invalid123 v1,${validSignature}`
        },
        testSecret
      );

      expect(result.valid).toBe(true);
    });
  });
});

// Export for use in integration tests
export { verifyClerkSignature, generateTestSignature };
