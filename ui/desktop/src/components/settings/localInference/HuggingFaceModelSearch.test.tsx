import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import {
  downloadHfModel,
  getRepoFiles,
  searchHfModels,
  type HfModelInfo,
  type RepoVariantsResponse,
} from '../../../acp/local-inference';
import { HuggingFaceModelSearch } from './HuggingFaceModelSearch';

vi.mock('../../../acp/local-inference', () => ({
  searchHfModels: vi.fn(),
  getRepoFiles: vi.fn(),
  downloadHfModel: vi.fn(),
}));

const model = (repoId: string): HfModelInfo => ({
  repoId,
  author: 'org',
  modelName: repoId,
  downloads: 100,
});
const response: RepoVariantsResponse = {
  variants: [
    {
      variantId: 'Q4',
      label: 'Q4',
      backendId: 'llamacpp',
      format: 'gguf',
      modelId: 'org/model:Q4',
      downloadId: 'org/model:Q4',
      sizeBytes: 1,
      description: '',
      qualityRank: 45,
      sharded: false,
      supported: true,
      assessment: {
        fit: 'likelyShortfall',
        recommended: false,
        inputTokens: 4096,
        outputTokens: 1024,
        availableMemoryBytes: 8e9,
        reserveBytes: 1e9,
        reasons: [],
      },
    },
  ],
  recommendedIndex: null,
  availableMemoryBytes: 8e9,
  downloadedQuants: [],
  downloadedVariants: [],
};

describe('local model discovery', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetAllMocks();
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('starts the settings landing page without a query and keeps shortfalls downloadable', async () => {
    vi.mocked(searchHfModels).mockResolvedValue([model('org/model')]);
    vi.mocked(getRepoFiles).mockResolvedValue(response);
    vi.mocked(downloadHfModel).mockResolvedValue('org/model:Q4');
    const onDownloadStarted = vi.fn();
    render(<HuggingFaceModelSearch showRecommendations onDownloadStarted={onDownloadStarted} />, {
      wrapper: IntlTestWrapper,
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(searchHfModels).toHaveBeenCalledWith('', 20);
    fireEvent.click(screen.getByText('org/model'));
    const download = screen.getByRole('button', { name: 'Download anyway' });
    expect(download).not.toBeDisabled();
    await act(async () => {
      fireEvent.click(download);
    });
    expect(onDownloadStarted).toHaveBeenCalledWith(
      'org/model:Q4',
      expect.objectContaining({ variantId: 'Q4' })
    );
  });

  it('does not replace a new query with late search results', async () => {
    let finishOld!: (models: HfModelInfo[]) => void;
    vi.mocked(searchHfModels).mockImplementation((query) =>
      query === 'old'
        ? new Promise((resolve) => {
            finishOld = resolve;
          })
        : Promise.resolve([model('org/new')])
    );
    vi.mocked(getRepoFiles).mockRejectedValue(new Error('offline'));
    render(<HuggingFaceModelSearch onDownloadStarted={vi.fn()} />, { wrapper: IntlTestWrapper });
    const input = screen.getByPlaceholderText('Search for local models...');
    fireEvent.change(input, { target: { value: 'old' } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    fireEvent.change(input, { target: { value: 'new' } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    await act(async () => {
      finishOld([model('org/old')]);
    });
    expect(screen.getByText('org/new')).toBeInTheDocument();
    expect(screen.queryByText('org/old')).not.toBeInTheDocument();
    expect(screen.getByText(/Assessment unavailable/)).toBeInTheDocument();
  });
});
