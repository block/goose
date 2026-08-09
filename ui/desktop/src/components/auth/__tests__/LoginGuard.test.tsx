/**
 * E2E-2 — desktop LoginGuard gate.
 * covers AC-1, AC-UI
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import type { IpcRendererEvent } from 'electron';
import LoginGuard from '../LoginGuard';

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
  const listeners = new Map<string, Set<(event: IpcRendererEvent, ...args: unknown[]) => void>>();

  beforeEach(() => {
    listeners.clear();
    window.electron = {
      ...window.electron,
      getAuthStatus: vi.fn(),
      authLogin: vi.fn(),
      authLogout: vi.fn(),
      isZitadelAuthEnabled: vi.fn().mockResolvedValue(true),
      on: (channel: string, callback: (event: IpcRendererEvent, ...args: unknown[]) => void) => {
        if (!listeners.has(channel)) listeners.set(channel, new Set());
        listeners.get(channel)!.add(callback);
      },
      off: (channel: string, callback: (event: IpcRendererEvent, ...args: unknown[]) => void) => {
        listeners.get(channel)?.delete(callback);
      },
    } as typeof window.electron;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('GivenSignedOut_WhenRendering_ThenChildrenNotRendered', async () => {
    // covers AC-1
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue({ state: 'signedOut' });

    render(
      <LoginGuard>
        <div data-testid="app-children">secret app</div>
      </LoginGuard>
    );

    await waitFor(() => {
      expect(screen.queryByTestId('app-children')).toBeNull();
      expect(screen.getByRole('button', { name: /sign in/i })).toBeTruthy();
    });
  });

  it('GivenSignedIn_WhenRendering_ThenChildrenRendered', async () => {
    // covers AC-1
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue({
      state: 'signedIn',
      sub: 'user-1',
      roles: ['agent-access'],
      expiresAt: Date.now() + 60_000,
    });

    render(
      <LoginGuard>
        <div data-testid="app-children">secret app</div>
      </LoginGuard>
    );

    await waitFor(() => {
      expect(screen.getByTestId('app-children')).toBeTruthy();
    });
  });

  it('GivenAuthExpired_WhenEventFires_ThenReturnsToLoginScreen', async () => {
    // covers AC-1
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue({
      state: 'signedIn',
      sub: 'user-1',
      roles: ['agent-access'],
      expiresAt: Date.now() + 60_000,
    });

    render(
      <LoginGuard>
        <div data-testid="app-children">secret app</div>
      </LoginGuard>
    );

    await waitFor(() => {
      expect(screen.getByTestId('app-children')).toBeTruthy();
    });

    await act(async () => {
      for (const cb of listeners.get('auth:on-changed') ?? []) {
        cb({} as IpcRendererEvent, { state: 'signedOut' });
      }
    });

    await waitFor(() => {
      expect(screen.queryByTestId('app-children')).toBeNull();
      expect(screen.getByRole('button', { name: /sign in/i })).toBeTruthy();
    });
  });
});
