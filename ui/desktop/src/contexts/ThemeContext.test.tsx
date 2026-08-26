import type { IpcRendererEvent } from 'electron';
import { act, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider, useTheme } from './ThemeContext';

function ThemeProbe() {
  const { userThemePreference, catppuccinAccent, setUserThemePreference, setCatppuccinAccent } =
    useTheme();
  return (
    <div>
      <span data-testid="preference">{userThemePreference}</span>
      <span data-testid="accent">{catppuccinAccent}</span>
      <button type="button" data-testid="select-mocha" onClick={() => setUserThemePreference('catppuccin-mocha')}>
        mocha
      </button>
      <button type="button" data-testid="select-teal" onClick={() => setCatppuccinAccent('teal')}>
        teal
      </button>
    </div>
  );
}

describe('ThemeContext broadcasts', () => {
  beforeEach(() => {
    vi.mocked(window.electron.getSetting).mockImplementation((key: string) => {
      if (key === 'useSystemTheme') return Promise.resolve(false);
      if (key === 'theme') return Promise.resolve('catppuccin-mocha');
      if (key === 'catppuccinAccent') return Promise.resolve('mauve');
      return Promise.resolve(undefined);
    });
    vi.mocked(window.electron.setSetting).mockResolvedValue(undefined);
    vi.mocked(window.electron.broadcastThemeChange).mockClear();
  });

  afterEach(() => {
    vi.mocked(window.electron.getSetting).mockReset();
    vi.mocked(window.electron.setSetting).mockReset();
  });

  it('does not include a stale accent on theme-only broadcasts', async () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <ThemeProbe />
      </ThemeProvider>
    );

    await waitFor(() => {
      expect(getByTestId('preference').textContent).toBe('catppuccin-mocha');
    });

    await act(async () => {
      getByTestId('select-mocha').click();
    });

    const themeBroadcasts = vi
      .mocked(window.electron.broadcastThemeChange)
      .mock.calls.map(([payload]) => payload);
    expect(themeBroadcasts.some((payload) => 'catppuccinAccent' in payload)).toBe(false);
    expect(themeBroadcasts.at(-1)).toMatchObject({
      theme: 'catppuccin-mocha',
      useSystemTheme: false,
    });
  });

  it('broadcasts only the accent when the swatch changes', async () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <ThemeProbe />
      </ThemeProvider>
    );

    await waitFor(() => {
      expect(getByTestId('accent').textContent).toBe('mauve');
    });

    await act(async () => {
      getByTestId('select-teal').click();
    });

    await waitFor(() => {
      expect(getByTestId('accent').textContent).toBe('teal');
    });

    expect(window.electron.broadcastThemeChange).toHaveBeenCalledWith({
      catppuccinAccent: 'teal',
    });
  });

  it('applies a theme-only IPC update without resetting the local accent', async () => {
    const listeners = new Map<
      string,
      (event: IpcRendererEvent, ...args: unknown[]) => void
    >();
    vi.mocked(window.electron.on).mockImplementation((channel, callback) => {
      listeners.set(channel, callback);
    });

    const { getByTestId } = render(
      <ThemeProvider>
        <ThemeProbe />
      </ThemeProvider>
    );

    await waitFor(() => {
      expect(getByTestId('preference').textContent).toBe('catppuccin-mocha');
    });

    await act(async () => {
      listeners.get('theme-changed')?.({} as IpcRendererEvent, {
        useSystemTheme: false,
        theme: 'catppuccin-latte',
      });
    });

    expect(getByTestId('preference').textContent).toBe('catppuccin-latte');
    expect(getByTestId('accent').textContent).toBe('mauve');
  });
});
