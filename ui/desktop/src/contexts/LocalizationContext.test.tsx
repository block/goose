import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import LanguageSelector from '../components/settings/app/LanguageSelector';
import { LocalizationProvider, useLocalization } from './LocalizationContext';

function LocalizationProbe() {
  const { language, t } = useLocalization();

  return (
    <div>
      <span data-testid="ui-language">{language}</span>
      <span>{t('settings.title')}</span>
    </div>
  );
}

function TestHarness() {
  return (
    <>
      <LanguageSelector />
      <LocalizationProbe />
    </>
  );
}

describe('LocalizationContext', () => {
  let languageChangedHandler: Parameters<typeof window.electron.on>[1] | undefined;

  beforeEach(async () => {
    languageChangedHandler = undefined;
    await window.electron.setSetting('uiLanguage', 'en');
    vi.mocked(window.electron.setSetting).mockClear();
    vi.mocked(window.electron.broadcastLanguageChange).mockClear();
    vi.mocked(window.electron.on).mockImplementation((channel, callback) => {
      if (channel === 'language-changed') {
        languageChangedHandler = callback;
      }
    });
    vi.mocked(window.electron.off).mockImplementation(() => undefined);
  });

  it('loads saved language from settings', async () => {
    await window.electron.setSetting('uiLanguage', 'zh-CN');

    render(
      <LocalizationProvider>
        <LocalizationProbe />
      </LocalizationProvider>
    );

    expect(await screen.findByText('设置')).toBeInTheDocument();
    expect(screen.getByTestId('ui-language')).toHaveTextContent('zh-CN');
    expect(document.documentElement.lang).toBe('zh-CN');
  });

  it('persists and broadcasts language changes', async () => {
    const user = userEvent.setup();

    render(
      <LocalizationProvider>
        <TestHarness />
      </LocalizationProvider>
    );

    expect(await screen.findByText('Settings')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '简体中文' }));

    await waitFor(() => {
      expect(window.electron.setSetting).toHaveBeenCalledWith('uiLanguage', 'zh-CN');
      expect(window.electron.broadcastLanguageChange).toHaveBeenCalledWith({
        language: 'zh-CN',
      });
    });

    expect(screen.getByText('设置')).toBeInTheDocument();
    expect(screen.getByTestId('ui-language')).toHaveTextContent('zh-CN');
    expect(document.documentElement.lang).toBe('zh-CN');
  });

  it('updates language when another window broadcasts a change', async () => {
    render(
      <LocalizationProvider>
        <LocalizationProbe />
      </LocalizationProvider>
    );

    expect(await screen.findByText('Settings')).toBeInTheDocument();

    act(() => {
      const event = {} as Parameters<NonNullable<typeof languageChangedHandler>>[0];
      languageChangedHandler?.(event, { language: 'zh-CN' });
    });

    expect(await screen.findByText('设置')).toBeInTheDocument();
    expect(screen.getByTestId('ui-language')).toHaveTextContent('zh-CN');
  });
});
