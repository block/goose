import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { IpcRendererEvent } from 'electron';
import AccountMenu from '../AccountMenu';

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

const signedIn = {
  state: 'signedIn' as const,
  sub: 'user-1',
  email: 'genario@avcdtech.com',
  name: 'Genario Nogueira',
  roles: ['agent-access'],
  expiresAt: Date.now() + 60_000,
};

describe('AccountMenu - Goal: sidebar identity and sign out', () => {
  beforeEach(() => {
    window.electron = {
      ...window.electron,
      getAuthStatus: vi.fn(),
      authLogin: vi.fn(),
      authLogout: vi.fn().mockResolvedValue(undefined),
      on: (_channel: string, _callback: (event: IpcRendererEvent, ...args: unknown[]) => void) => {},
      off: (_channel: string, _callback: (event: IpcRendererEvent, ...args: unknown[]) => void) => {},
    } as typeof window.electron;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('GivenSignedOut_WhenRendering_ThenRendersNothing', async () => {
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue({ state: 'signedOut' });

    const { container } = render(<AccountMenu />);

    await waitFor(() => {
      expect(screen.queryByTestId('account-menu-trigger')).toBeNull();
    });
    expect(container).toBeEmptyDOMElement();
  });

  it('GivenSignedIn_WhenRendering_ThenShowsNameAndInitials', async () => {
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue(signedIn);

    render(<AccountMenu />);

    await waitFor(() => {
      expect(screen.getByTestId('account-menu-trigger')).toBeTruthy();
    });
    expect(screen.getByText('Genario Nogueira')).toBeTruthy();
    expect(screen.getByText('genario@avcdtech.com')).toBeTruthy();
    // No picture claim, so the avatar falls back to initials.
    expect(screen.getByText('GN')).toBeTruthy();
  });

  it('GivenSignedIn_WhenSignOutClicked_ThenCallsAuthLogout', async () => {
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue(signedIn);

    render(<AccountMenu />);

    const trigger = await waitFor(() => screen.getByTestId('account-menu-trigger'));
    await userEvent.click(trigger);

    const signOut = await waitFor(() => screen.getByTestId('account-menu-sign-out'));
    await userEvent.click(signOut);

    expect(window.electron.authLogout).toHaveBeenCalledTimes(1);
  });

  it('GivenOnlyEmail_WhenRendering_ThenDerivesInitialsFromEmail', async () => {
    vi.mocked(window.electron.getAuthStatus).mockResolvedValue({
      ...signedIn,
      name: undefined,
    });

    render(<AccountMenu />);

    await waitFor(() => {
      expect(screen.getByTestId('account-menu-trigger')).toBeTruthy();
    });
    expect(screen.getByText('GE')).toBeTruthy();
  });
});
