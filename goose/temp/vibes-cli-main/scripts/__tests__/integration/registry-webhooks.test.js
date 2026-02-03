/**
 * Integration tests for registry webhook handling
 *
 * Tests the subscription webhook logic with mocked Clerk events.
 * Uses the clerk-webhooks mock factory for realistic payloads.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  processSubscriptionChange,
  getUserClaims,
  createClaim
} from '../../lib/registry-logic.js';
import {
  createSubscriptionUpdatedEvent,
  createSubscriptionCanceledEvent
} from '../mocks/clerk-webhooks.js';

describe('Registry Webhook Integration', () => {
  let registry;

  beforeEach(() => {
    // Set up a registry with multiple users and claims
    registry = {
      claims: {},
      reserved: ['admin', 'api', 'www'],
      preallocated: {
        'enterprise': 'user_enterprise'
      }
    };

    // User 1 has 3 subdomains claimed at different times
    createClaim(registry, 'user1-first', 'user_1');
    // Simulate time passing
    registry.claims['user1-first'].claimedAt = '2025-01-01T00:00:00Z';

    createClaim(registry, 'user1-second', 'user_1');
    registry.claims['user1-second'].claimedAt = '2025-01-02T00:00:00Z';

    createClaim(registry, 'user1-third', 'user_1');
    registry.claims['user1-third'].claimedAt = '2025-01-03T00:00:00Z';

    // User 2 has 1 subdomain
    createClaim(registry, 'user2-site', 'user_2');
    registry.claims['user2-site'].claimedAt = '2025-01-01T00:00:00Z';
  });

  describe('subscription.updated webhook', () => {
    it('releases newest subdomain when quantity decreases from 3 to 2', () => {
      // User 1 downgrades from 3 to 2 subdomains
      const userClaims = getUserClaims(registry, 'user_1');
      expect(userClaims).toHaveLength(3);

      const result = processSubscriptionChange(registry, 'user_1', 2);

      // Should release the newest claim (LIFO)
      expect(result.released).toHaveLength(1);
      expect(result.released[0]).toBe('user1-third');

      // Verify registry state
      const remainingClaims = getUserClaims(registry, 'user_1');
      expect(remainingClaims).toHaveLength(2);
      expect(remainingClaims).toContain('user1-first');
      expect(remainingClaims).toContain('user1-second');
      expect(remainingClaims).not.toContain('user1-third');
    });

    it('releases multiple subdomains when quantity decreases significantly', () => {
      const result = processSubscriptionChange(registry, 'user_1', 1);

      // Should release two newest claims
      expect(result.released).toHaveLength(2);
      expect(result.released).toEqual(['user1-third', 'user1-second']);

      // Only oldest claim remains
      const remainingClaims = getUserClaims(registry, 'user_1');
      expect(remainingClaims).toEqual(['user1-first']);
    });

    it('does not affect other users when one user downgrades', () => {
      processSubscriptionChange(registry, 'user_1', 1);

      // User 2's claim should be unaffected
      const user2Claims = getUserClaims(registry, 'user_2');
      expect(user2Claims).toEqual(['user2-site']);
    });

    it('handles quantity increase (no releases)', () => {
      const result = processSubscriptionChange(registry, 'user_1', 5);

      expect(result.released).toHaveLength(0);
      expect(getUserClaims(registry, 'user_1')).toHaveLength(3);
    });
  });

  describe('subscription.canceled webhook (deleted)', () => {
    it('releases all subdomains when subscription is canceled', () => {
      // Subscription canceled = quantity 0
      const result = processSubscriptionChange(registry, 'user_1', 0);

      expect(result.released).toHaveLength(3);
      expect(result.released).toEqual(['user1-third', 'user1-second', 'user1-first']);

      // All claims should be removed
      const remainingClaims = getUserClaims(registry, 'user_1');
      expect(remainingClaims).toHaveLength(0);
    });

    it('makes released subdomains available for new claims', () => {
      processSubscriptionChange(registry, 'user_1', 0);

      // Another user should be able to claim the released subdomain
      const result = createClaim(registry, 'user1-first', 'user_3');
      expect(result.success).toBe(true);
      expect(registry.claims['user1-first'].userId).toBe('user_3');
    });
  });

  describe('concurrent user scenarios', () => {
    it('handles multiple users with overlapping subscription changes', () => {
      // User 1 downgrades
      processSubscriptionChange(registry, 'user_1', 2);

      // User 2 also downgrades (had 1, now 0)
      processSubscriptionChange(registry, 'user_2', 0);

      // Verify both changes applied correctly
      expect(getUserClaims(registry, 'user_1')).toHaveLength(2);
      expect(getUserClaims(registry, 'user_2')).toHaveLength(0);
    });

    it('allows new user to claim released subdomain', () => {
      // User 1 cancels, releasing all subdomains
      processSubscriptionChange(registry, 'user_1', 0);

      // New user claims one of the released subdomains
      const result = createClaim(registry, 'user1-third', 'user_new');
      expect(result.success).toBe(true);

      // Verify new owner
      expect(registry.claims['user1-third'].userId).toBe('user_new');
    });
  });

  describe('edge cases', () => {
    it('handles user with no claims gracefully', () => {
      const result = processSubscriptionChange(registry, 'user_nonexistent', 0);
      expect(result.released).toHaveLength(0);
    });

    it('handles negative quantity as 0', () => {
      // Treat negative quantity same as 0
      const result = processSubscriptionChange(registry, 'user_1', -1);

      // All should be released since we can't have negative claims
      expect(result.released).toHaveLength(3);
    });

    it('preserves registry integrity after multiple operations', () => {
      // Series of operations
      processSubscriptionChange(registry, 'user_1', 2); // Release 1
      createClaim(registry, 'newclaim', 'user_1');      // Add 1 back
      processSubscriptionChange(registry, 'user_1', 1); // Release 2

      // Should have only the oldest claim remaining
      const claims = getUserClaims(registry, 'user_1');
      expect(claims).toHaveLength(1);
      expect(claims[0]).toBe('user1-first');
    });
  });
});

describe('Webhook Event Structure', () => {
  it('creates valid subscription.updated event', () => {
    const event = createSubscriptionUpdatedEvent({
      userId: 'user_test',
      planId: 'pro',
      status: 'active'
    });

    expect(event.payload.type).toBe('subscription.updated');
    expect(event.payload.data.user_id).toBe('user_test');
    expect(event.headers['svix-id']).toBeDefined();
    expect(event.headers['svix-timestamp']).toBeDefined();
    expect(event.headers['svix-signature']).toBeDefined();
  });

  it('creates valid subscription.canceled event', () => {
    const event = createSubscriptionCanceledEvent({
      userId: 'user_test'
    });

    expect(event.payload.type).toBe('subscription.canceled');
    expect(event.payload.data.user_id).toBe('user_test');
    expect(event.payload.data.status).toBe('canceled');
  });
});
