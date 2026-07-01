import { describe, expect, it } from 'vitest';
import { evaluateWhenClause } from './when';

describe('evaluateWhenClause', () => {
  it('defaults to visible when clause is missing', () => {
    expect(evaluateWhenClause(undefined, { sessionId: null, route: '/' })).toBe(true);
  });

  it('matches session.active', () => {
    expect(
      evaluateWhenClause('session.active', { sessionId: 'abc', route: '/pair' })
    ).toBe(true);
    expect(evaluateWhenClause('session.active', { sessionId: null, route: '/' })).toBe(false);
  });

  it('matches !session.active', () => {
    expect(evaluateWhenClause('!session.active', { sessionId: null, route: '/' })).toBe(true);
    expect(
      evaluateWhenClause('!session.active', { sessionId: 'abc', route: '/pair' })
    ).toBe(false);
  });
});
