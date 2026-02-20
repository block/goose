import { useState, useEffect, useCallback, useRef } from 'react';
import { useConfig } from './ConfigContext';
import {
  listLocalModels,
  downloadHfModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  type DownloadProgress,
  type LocalModelResponse,
} from '../api';
import { toastService } from '../toasts';
import { trackOnboardingSetupFailed } from '../utils/analytics';
import { Goose } from './icons';

interface LocalModelSetupProps {
  onSuccess: () => void;
  onCancel: () => void;
}

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

const formatSize = (bytes: number): string => {
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)}GB` : `${mb.toFixed(0)}MB`;
};

type SetupPhase = 'loading' | 'select' | 'downloading' | 'error';

export function LocalModelSetup({ onSuccess, onCancel }: LocalModelSetupProps) {
  const { upsert } = useConfig();
  const [phase, setPhase] = useState<SetupPhase>('loading');
  const [models, setModels] = useState<LocalModelResponse[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showAllModels, setShowAllModels] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const cleanup = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  useEffect(() => cleanup, [cleanup]);

  useEffect(() => {
    const load = async () => {
      try {
        const response = await listLocalModels();
        if (response.data) {
          setModels(response.data);

          const alreadyDownloaded = response.data.find((m) => m.status.state === 'Downloaded');
          if (alreadyDownloaded) {
            setSelectedModelId(alreadyDownloaded.id);
          } else {
            const recommended = response.data.find((m: LocalModelResponse) => m.recommended);
            if (recommended) setSelectedModelId(recommended.id);
          }
        }
      } catch (error) {
        console.error('Failed to load local models:', error);
        setErrorMessage('Failed to load available models. Please try again.');
        setPhase('error');
        return;
      }
      setPhase('select');
    };
    load();
  }, []);

  const finishSetup = async (modelId: string) => {
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    toastService.success({
      title: 'Local Model Ready',
      msg: `Running entirely on your machine with ${modelId}.`,
    });
    onSuccess();
  };

  const startDownload = async (modelId: string) => {
    setPhase('downloading');
    setDownloadProgress(null);
    setErrorMessage(null);

    const model = models.find((m) => m.id === modelId);
    if (!model) {
      setErrorMessage('Model not found');
      setPhase('error');
      return;
    }

    try {
      await downloadHfModel({ body: { spec: model.id } });
    } catch (error) {
      console.error('Failed to start download:', error);
      setErrorMessage('Failed to start download. Please try again.');
      trackOnboardingSetupFailed('local', 'download_start_failed');
      setPhase('error');
      return;
    }

    pollRef.current = setInterval(async () => {
      try {
        const response = await getLocalModelDownloadProgress({ path: { model_id: modelId } });
        if (response.data) {
          setDownloadProgress(response.data);
          if (response.data.status === 'completed') {
            cleanup();
            await finishSetup(modelId);
          } else if (response.data.status === 'failed') {
            cleanup();
            setErrorMessage(response.data.error || 'Download failed.');
            trackOnboardingSetupFailed('local', response.data.error || 'download_failed');
            setPhase('error');
          } else if (response.data.status === 'cancelled') {
            cleanup();
            setPhase('select');
          }
        }
      } catch {
        cleanup();
        setErrorMessage('Lost connection to download. Please try again.');
        trackOnboardingSetupFailed('local', 'progress_poll_failed');
        setPhase('error');
      }
    }, 500);
  };

  const handleCancel = async () => {
    if (phase === 'downloading' && selectedModelId) {
      cleanup();
      try {
        await cancelLocalModelDownload({ path: { model_id: selectedModelId } });
      } catch {
        // best-effort
      }
      setDownloadProgress(null);
      setPhase('select');
    } else {
      onCancel();
    }
  };

  const handlePrimaryAction = async () => {
    if (!selectedModelId) return;
    const model = models.find((m) => m.id === selectedModelId);
    if (!model) return;
    if (model.status.state === 'Downloaded') {
      await finishSetup(model.id);
    } else {
      await startDownload(model.id);
    }
  };

  const recommended = models.find((m) => m.recommended);
  const otherModels = models.filter((m) => m.id !== recommended?.id);
  const selectedModel = models.find((m) => m.id === selectedModelId);

  if (phase === 'loading') {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-text-muted mb-4"></div>
        <p className="text-text-muted text-sm">Checking available models...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-left space-y-3">
        <div className="origin-bottom-left goose-icon-animation">
          <Goose className="size-6 sm:size-8" />
        </div>
        <h1 className="text-2xl sm:text-4xl font-light">Run Locally</h1>
        <p className="text-text-muted text-base sm:text-lg">
          Download a model to run Goose entirely on your machine — no API keys, no accounts, completely free and private.
        </p>
      </div>

      {/* Error state */}
      {phase === 'error' && (
        <div className="space-y-4">
          <div className="border border-red-500/30 rounded-xl p-4 bg-red-500/5">
            <p className="text-sm text-red-400">{errorMessage}</p>
          </div>
          <button
            onClick={() => {
              setErrorMessage(null);
              setPhase('select');
            }}
            className="w-full px-6 py-3 bg-background-muted text-text-default rounded-lg transition-colors font-medium hover:bg-background-muted/80"
          >
            Try Again
          </button>
          <button
            onClick={onCancel}
            className="w-full px-6 py-3 bg-transparent text-text-muted rounded-lg hover:bg-background-muted transition-colors"
          >
            Back
          </button>
        </div>
      )}

      {/* Model selection */}
      {phase === 'select' && (
        <div className="space-y-5">
          {/* Recommended model card */}
          {recommended && (
            <div
              onClick={() => setSelectedModelId(recommended.id)}
              className={`relative w-full p-4 sm:p-6 border rounded-xl cursor-pointer transition-all duration-200 group ${
                selectedModelId === recommended.id
                  ? 'border-blue-500 bg-blue-500/5'
                  : 'border-border-subtle hover:border-border-default'
              }`}
            >
              <div className="absolute -top-2 -right-2 sm:-top-3 sm:-right-3 z-10">
                <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
                  Best for your machine
                </span>
              </div>
              <div className="flex items-start gap-3">
                <input
                  type="radio"
                  checked={selectedModelId === recommended.id}
                  onChange={() => setSelectedModelId(recommended.id)}
                  className="cursor-pointer flex-shrink-0 mt-1"
                />
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-medium text-text-default text-sm sm:text-base">
                      {recommended.display_name}
                    </span>
                    {recommended.status.state === 'Downloaded' && (
                      <span className="text-xs bg-green-600 text-white px-2 py-0.5 rounded-full">
                        Ready
                      </span>
                    )}
                  </div>
                  <p className="text-text-muted text-xs mt-1">
                    {formatSize(recommended.size_bytes)}
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Expandable other models */}
          {otherModels.length > 0 && (
            <div>
              <button
                onClick={() => setShowAllModels(!showAllModels)}
                className="text-sm text-blue-500 hover:text-blue-400 transition-colors flex items-center gap-1"
              >
                {showAllModels ? 'Hide other sizes' : `Show ${otherModels.length} other sizes`}
                <svg
                  className={`w-3.5 h-3.5 transition-transform ${showAllModels ? 'rotate-180' : ''}`}
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>

              {showAllModels && (
                <div className="mt-3 space-y-2">
                  {otherModels.map((model) => (
                    <div
                      key={model.id}
                      onClick={() => setSelectedModelId(model.id)}
                      className={`w-full p-4 border rounded-xl cursor-pointer transition-all duration-200 ${
                        selectedModelId === model.id
                          ? 'border-blue-500 bg-blue-500/5'
                          : 'border-border-subtle hover:border-border-default'
                      }`}
                    >
                      <div className="flex items-start gap-3">
                        <input
                          type="radio"
                          checked={selectedModelId === model.id}
                          onChange={() => setSelectedModelId(model.id)}
                          className="cursor-pointer flex-shrink-0 mt-0.5"
                        />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="font-medium text-text-default text-sm">{model.display_name}</span>
                            <span className="text-xs text-text-muted">{formatSize(model.size_bytes)}</span>
                            {model.status.state === 'Downloaded' && (
                              <span className="text-xs bg-green-600 text-white px-2 py-0.5 rounded-full">
                                Ready
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Primary action */}
          <button
            onClick={handlePrimaryAction}
            disabled={!selectedModelId}
            className="w-full px-6 py-3 bg-background-muted text-text-default rounded-lg transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed hover:bg-background-muted/80"
          >
            {selectedModel?.status.state === 'Downloaded'
              ? `Use ${selectedModel.display_name}`
              : selectedModel
                ? `Download ${selectedModel.display_name} (${formatSize(selectedModel.size_bytes)})`
                : 'Select a model'}
          </button>

          <button
            onClick={onCancel}
            className="w-full px-6 py-3 bg-transparent text-text-muted rounded-lg hover:bg-background-muted transition-colors"
          >
            Back
          </button>
        </div>
      )}

      {/* Downloading state */}
      {phase === 'downloading' && selectedModel && (
        <div className="space-y-6">
          <div className="border border-border-subtle rounded-xl p-5 sm:p-6 bg-background-default">
            <p className="font-medium text-text-default text-sm sm:text-base mb-4">
              Downloading {selectedModel.display_name}
            </p>

            {downloadProgress ? (
              <div className="space-y-3">
                {/* Progress bar */}
                <div className="w-full bg-background-subtle rounded-full h-2 overflow-hidden">
                  <div
                    className="bg-blue-500 h-2 rounded-full transition-all duration-500 ease-out"
                    style={{ width: `${downloadProgress.progress_percent}%` }}
                  />
                </div>

                {/* Stats row */}
                <div className="flex justify-between text-xs text-text-muted">
                  <span>
                    {formatBytes(downloadProgress.bytes_downloaded)} of{' '}
                    {formatBytes(downloadProgress.total_bytes)}
                  </span>
                  <span>{downloadProgress.progress_percent.toFixed(0)}%</span>
                </div>

                <div className="flex justify-between text-xs text-text-muted">
                  {downloadProgress.speed_bps ? (
                    <span>{formatBytes(downloadProgress.speed_bps)}/s</span>
                  ) : (
                    <span />
                  )}
                  {downloadProgress.eta_seconds != null && downloadProgress.eta_seconds > 0 && (
                    <span>
                      ~{downloadProgress.eta_seconds < 60
                        ? `${Math.round(downloadProgress.eta_seconds)}s`
                        : `${Math.round(downloadProgress.eta_seconds / 60)}m`}{' '}
                      remaining
                    </span>
                  )}
                </div>
              </div>
            ) : (
              <div className="flex items-center gap-3">
                <div className="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-text-muted"></div>
                <span className="text-sm text-text-muted">Starting download...</span>
              </div>
            )}
          </div>

          <button
            onClick={handleCancel}
            className="w-full px-6 py-3 bg-transparent text-text-muted rounded-lg hover:bg-background-muted transition-colors border border-border-subtle"
          >
            Cancel Download
          </button>
        </div>
      )}
    </div>
  );
}
