import type { ReactElement } from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render as rtlRender, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LocalizationProvider, useLocalization } from '../contexts/LocalizationContext';
import { LocalModelSetup } from './LocalModelSetup';
import * as api from '../api';

const render = (ui: ReactElement) => rtlRender(<LocalizationProvider>{ui}</LocalizationProvider>);

vi.mock('../api');
vi.mock('../toasts', () => ({
  toastService: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockUpsert = vi.fn();

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({
    upsert: mockUpsert,
  }),
}));

function LanguageSwitchHarness() {
  const { setLanguage } = useLocalization();

  return (
    <>
      <button data-testid="switch-language" onClick={() => void setLanguage('zh-CN')}>
        Switch language
      </button>
      <LocalModelSetup onSuccess={() => undefined} onCancel={() => undefined} />
    </>
  );
}

describe('LocalModelSetup', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(api.listLocalModels).mockResolvedValue({
      data: [
        {
          filename: 'model.gguf',
          id: 'Qwen/Qwen3-4B-GGUF:Q4_K_M',
          quantization: 'Q4_K_M',
          recommended: true,
          repo_id: 'Qwen/Qwen3-4B-GGUF',
          settings: {},
          size_bytes: 200 * 1024 * 1024,
          status: {
            error: null,
            state: 'NotDownloaded',
          },
        },
      ],
    } as unknown as Awaited<ReturnType<typeof api.listLocalModels>>);

    vi.mocked(api.downloadHfModel).mockResolvedValue({
      data: undefined,
    } as unknown as Awaited<ReturnType<typeof api.downloadHfModel>>);

    vi.mocked(api.getLocalModelDownloadProgress).mockResolvedValue({
      data: {
        bytes_downloaded: 0,
        eta_seconds: null,
        progress_percent: 0,
        speed_bps: null,
        status: 'downloading',
        total_bytes: 200 * 1024 * 1024,
      },
    } as unknown as Awaited<ReturnType<typeof api.getLocalModelDownloadProgress>>);
  });

  it('does not restart setup when the ui language changes during download', async () => {
    const user = userEvent.setup();

    render(<LanguageSwitchHarness />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Download Qwen\/Qwen3-4B-GGUF:Q4_K_M/ })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /Download Qwen\/Qwen3-4B-GGUF:Q4_K_M/ }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Cancel download/i })).toBeInTheDocument();
    });

    expect(api.listLocalModels).toHaveBeenCalledTimes(1);

    await user.click(screen.getByTestId('switch-language'));

    await waitFor(() => {
      expect(document.documentElement.lang).toBe('zh-CN');
    });

    expect(api.listLocalModels).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: /取消下载/ })).toBeInTheDocument();
  });
});
