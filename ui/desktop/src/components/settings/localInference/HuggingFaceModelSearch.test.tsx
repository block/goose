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
        compatibility: 'compatible',
        suitability: 'suitable',
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

  it('hides shortfalls on the landing page but keeps them downloadable in search', async () => {
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
    expect(screen.queryByText('org/model')).not.toBeInTheDocument();
    expect(screen.getByText(/No recommendations available/)).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText('Search for local models...'), {
      target: { value: 'model' },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
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

  it('shows pre-assessed catalogue recommendations without fetching repository metadata', async () => {
    const quality = {
      evaluatedModelId: 'org/model',
      inherited: false,
      aaModelId: 'aa-model',
      sourceUrl: 'https://artificialanalysis.ai/models/model',
      evaluationName: 'Model (reasoning)',
      catalogueOrder: 1,
    };
    vi.mocked(searchHfModels).mockResolvedValue([
      {
        ...model('org/model'),
        quality,
        variants: [
          {
            ...response.variants[0],
            assessment: {
              ...response.variants[0].assessment!,
              fit: 'likelyFit',
              revision: 'a'.repeat(40),
            },
          },
        ],
      },
    ]);
    render(
      <HuggingFaceModelSearch
        showRecommendations
        downloadedModelIds={new Set()}
        onDownloadStarted={vi.fn()}
      />,
      {
        wrapper: IntlTestWrapper,
      }
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(screen.getByText('org/model')).toBeInTheDocument();
    expect(
      screen.getByRole('link', { name: /Recommendations ordered using Artificial Analysis/ })
    ).toHaveAttribute('href', 'https://artificialanalysis.ai/');
    expect(getRepoFiles).not.toHaveBeenCalled();
    vi.mocked(downloadHfModel).mockResolvedValue('org/model:Q4');
    vi.mocked(getRepoFiles).mockResolvedValue({
      ...response,
      quality,
      variants: [
        {
          ...response.variants[0],
          assessment: {
            ...response.variants[0].assessment!,
            fit: 'likelyFit',
            revision: 'a'.repeat(40),
          },
        },
      ],
    });
    await act(async () => {
      fireEvent.click(screen.getByText('org/model'));
    });
    expect(
      screen.getByRole('link', { name: 'Evaluation details on Artificial Analysis' })
    ).toHaveAttribute('href', quality.sourceUrl);
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Download' }));
    });
    expect(downloadHfModel).toHaveBeenCalledWith(
      expect.objectContaining({ revision: 'a'.repeat(40) })
    );
  });

  it('deduplicates recommendations and excludes downloaded checkpoints without filtering search results', async () => {
    const models: HfModelInfo[] = Array.from({ length: 3 }, (_, index) => ({
      ...model(`org/model-${index}`),
      quality: {
        evaluatedModelId: `org/model-${index}`,
        inherited: false,
        aaModelId: 'aa-model',
        sourceUrl: 'https://artificialanalysis.ai/models/model',
        evaluationName: 'Model (reasoning)',
        catalogueOrder: index + 1,
      },
      variants: [
        {
          ...response.variants[0],
          assessment: {
            ...response.variants[0].assessment!,
            fit: index === 0 ? 'unknown' : 'likelyFit',
            lowerBytes: 1,
            upperBytes: undefined,
          },
        },
      ],
    }));
    models.push({ ...models[0], repoId: 'org/model-0-copy', downloads: 1 });
    vi.mocked(searchHfModels).mockResolvedValue(models);
    const { rerender } = render(
      <HuggingFaceModelSearch showRecommendations onDownloadStarted={vi.fn()} />,
      {
        wrapper: IntlTestWrapper,
      }
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(screen.getByText('org/model-0')).toBeInTheDocument();
    expect(screen.queryByText('org/model-0-copy')).not.toBeInTheDocument();
    rerender(
      <HuggingFaceModelSearch
        showRecommendations
        downloadedModelIds={new Set(['org/model-0:Q8'])}
        onDownloadStarted={vi.fn()}
      />
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(screen.queryByText('org/model-0')).not.toBeInTheDocument();
    expect(screen.queryByText('org/model-0-copy')).not.toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText('Search for local models...'), {
      target: { value: 'model' },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    for (const model of models) {
      expect(screen.getByText(model.repoId)).toBeInTheDocument();
    }
    expect(getRepoFiles).not.toHaveBeenCalled();
  });

  it('keeps unsupported and specialised search results downloadable', async () => {
    vi.mocked(searchHfModels).mockResolvedValue([model('org/specialist')]);
    vi.mocked(getRepoFiles).mockResolvedValue({
      ...response,
      variants: [
        {
          ...response.variants[0],
          assessment: {
            ...response.variants[0].assessment!,
            fit: 'unknown',
            compatibility: 'incompatible',
            suitability: 'unsuitable',
          },
        },
      ],
    });
    render(<HuggingFaceModelSearch onDownloadStarted={vi.fn()} />, { wrapper: IntlTestWrapper });
    fireEvent.change(screen.getByPlaceholderText('Search for local models...'), {
      target: { value: 'specialist' },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    expect(screen.getByText('Not compatible with eredu')).toBeInTheDocument();
    expect(screen.getByText('Not recommended for conversational use in goose')).toBeInTheDocument();
    await act(async () => {
      fireEvent.click(screen.getByText('org/specialist'));
    });
    expect(screen.getByRole('button', { name: 'Download anyway' })).not.toBeDisabled();
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
