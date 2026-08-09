/**
 * E2E-2 — desktop LoginGuard gate.
 * covers AC-1, AC-UI
 *
 * Phase 0: written failing (LoginGuard not implemented yet).
 */
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';

vi.mock('../../../i18n', async () => {
  const actual = await vi.importActual<typeof import('../../../i18n')>('../../../i18n');
  return {
    ...actual,
    useIntl: () => ({
      formatMessage: (msg: { defaultMessage?: string; id?: string }) =>
        msg.defaultMessage ?? msg.id ?? '',
    }),
  };
});

describe('E2E: LoginGuard - Goal: block app until Zitadel login', () => {
  it('GivenSignedOut_WhenRendering_ThenChildrenNotRendered', async () => {
    // covers AC-1
    const mod = await import('../LoginGuard');
    const LoginGuard = mod.default ?? mod.LoginGuard;
    expect(LoginGuard).toBeTypeOf('function');

    render(
      <LoginGuard>
        <div data-testid="app-children">secret app</div>
      </LoginGuard>
    );

    expect(screen.queryByTestId('app-children')).toBeNull();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeTruthy();
  });

  it('GivenSignedIn_WhenRendering_ThenChildrenRendered', async () => {
    // covers AC-1
    const mod = await import('../LoginGuard');
    const LoginGuard = mod.default ?? mod.LoginGuard;

    // When implemented, pass a mock auth state or IPC stub for signedIn
    render(
      <LoginGuard>
        <div data-testid="app-children">secret app</div>
      </LoginGuard>
    );

    // This assertion intentionally fails until LoginGuard can be signed-in in tests
    expect(screen.getByTestId('app-children')).toBeTruthy();
  });

  it('GivenAuthExpired_WhenEventFires_ThenReturnsToLoginScreen', async () => {
    // covers AC-1
    const mod = await import('../LoginGuard');
    expect(mod.default ?? mod.LoginGuard).toBeTypeOf('function');
    // Phase 7 wires auth:on-changed → signedOut and re-shows Sign in
    expect(false).toBe(true);
  });
});
