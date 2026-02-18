import { useState, useEffect, useCallback, useRef } from 'react';
import { useConfig } from './ConfigContext';
import {
  listLocalModels,
  downloadHfModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
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
  const [downloadProgress, setDownloadProgress] = useState<{
    bytes_downloaded: number;
    total_bytes: number;
    progress_percent: number;
    speed_bps?: number | null;
    eta_seconds?: number | null;
  } | null>(null);
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
          const featured = response.data.filter((m): m is LocalModelResponse => 'tier' in m);
          setModels(featured);

          const alreadyDownloaded = featured.find((m) => m.status.state === 'Downloaded');
          if (alreadyDownloaded) {
            setSelectedModelId(alreadyDownloaded.id);
          } else {
            const recommended = featured.find((m) => m.recommended);
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
      await downloadHfModel({
        body: {
          repo_id: model.repo_id,
          filename: model.filename,
        },
      });
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
          const data = response.data;
          setDownloadProgress({
            bytes_downloaded: data.bytes_downloaded,
            total_bytes: data.total_bytes,
            progress_percent:
              data.total_bytes > 0 ? (data.bytes_downloaded / data.total_bytes) * 100 : 0,
            speed_bps: data.speed_bps,
            eta_seconds: data.eta_seconds,
          });
          if (data.status === 'completed') {
            cleanup();
            await finishSetup(modelId);
          } else if (data.status === 'failed') {
            cleanup();
            setErrorMessage('Download failed.');
            trackOnboardingSetupFailed('local', 'download_failed');
            setPhase('error');
          } else if (data.status === 'cancelled') {
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

  const selectedModel = selectedModelId ? models.find((m) => m.id === selectedModelId) : null;
  const recommended = models.find((m) => m.recommended);
  const displayModels = showAllModels ? models : models.filter((m) => m.recommended);

  return (
    <div className="flex flex-col justify-center items-center w-full max-w-lg mx-auto px-4 sm:px-6">
      {/* Header */}
      <div className="flex flex-col items-center gap-3 mb-6 sm:mb-8">
        <div className="relative">
          <Goose className="w-10 h-10 sm:w-12 sm:h-12 text-text-default" />
          <div className="absolute -bottom-1 -right-1 w-4 h-4 sm:w-5 sm:h-5 bg-green-600 rounded-full border-2 border-background-default flex items-center justify-center">
            <span className="text-white text-[8px] sm:text-[10px]">✓</span>
          </div>
        </div>
        <h2 className="text-xl sm:text-2xl font-medium text-text-default text-center">
          Set up Local Model
        </h2>
        <p className="text-sm sm:text-base text-text-muted text-center max-w-sm">
          Run AI entirely on your machine—no data sent to the cloud.
        </p>
      </div>

      {/* Loading state */}
      {phase === 'loading' && (
        <div className="flex items-center gap-3 p-6">
          <div className="animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-text-muted"></div>
          <span className="text-text-muted">Loading available models...</span>
        </div>
      )}

      {/* Error state */}
      {phase === 'error' && (
        <div className="space-y-4 w-full">
          <div className="border border-red-500/30 bg-red-500/10 rounded-xl p-4 text-center">
            <p className="text-red-400 text-sm">{errorMessage}</p>
          </div>
          <button
            onClick={() => {
              setErrorMessage(null);
              setPhase('select');
            }}
            className="w-full px-6 py-3 bg-background-muted text-text-default rounded-lg hover:bg-background-muted/80 transition-colors"
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
        <div className="space-y-4 sm:space-y-5 w-full">
          {/* Recommended model card */}
          {recommended && !showAllModels && (
            <div
              onClick={() => setSelectedModelId(recommended.id)}
              className={`border rounded-xl p-4 sm:p-5 cursor-pointer transition-all ${
                selectedModelId === recommended.id
                  ? 'border-green-600 bg-green-600/5'
                  : 'border-border-subtle hover:border-border-default'
              }`}
            >
              <div className="flex items-start gap-3 sm:gap-4">
                <div
                  className={`w-5 h-5 rounded-full border-2 flex items-center justify-center flex-shrink-0 mt-0.5 ${
                    selectedModelId === recommended.id
                      ? 'border-green-600 bg-green-600'
                      : 'border-border-default'
                  }`}
                >
                  {selectedModelId === recommended.id && (
                    <span className="text-white text-xs">✓</span>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-medium text-text-default text-sm sm:text-base">
                      {recommended.display_name}
                    </span>
                    <span className="text-xs bg-green-600/20 text-green-400 px-2 py-0.5 rounded-full">
                      Recommended
                    </span>
                    {recommended.status.state === 'Downloaded' && (
                      <span className="text-xs bg-green-600 text-white px-2 py-0.5 rounded-full">
                        Ready
                      </span>
                    )}
                  </div>
                  <p className="text-text-muted text-xs sm:text-sm mt-1">
                    {formatSize(recommended.size_bytes)} •{' '}
                    {recommended.context_limit
                      ? `${(recommended.context_limit / 1000).toFixed(0)}K context`
                      : 'Standard context'}
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Show more models toggle */}
          {!showAllModels && models.length > 1 && (
            <button
              onClick={() => setShowAllModels(true)}
              className="w-full text-sm text-text-muted hover:text-text-default transition-colors py-2"
            >
              Show all {models.length} models →
            </button>
          )}

          {/* All models list */}
          {showAllModels && (
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-text-muted">All available models</span>
                <button
                  onClick={() => setShowAllModels(false)}
                  className="text-sm text-text-muted hover:text-text-default transition-colors"
                >
                  ← Back
                </button>
              </div>
              <div className="max-h-64 overflow-y-auto space-y-2 pr-1">
                {displayModels.map((model) => (
                  <div
                    key={model.id}
                    onClick={() => setSelectedModelId(model.id)}
                    className={`border rounded-lg p-3 cursor-pointer transition-all ${
                      selectedModelId === model.id
                        ? 'border-green-600 bg-green-600/5'
                        : 'border-border-subtle hover:border-border-default'
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <div
                        className={`w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
                          selectedModelId === model.id
                            ? 'border-green-600 bg-green-600'
                            : 'border-border-default'
                        }`}
                      >
                        {selectedModelId === model.id && (
                          <span className="text-white text-[10px]">✓</span>
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="font-medium text-text-default text-sm truncate">
                            {model.display_name}
                          </span>
                          <span className="text-xs text-text-muted">
                            {formatSize(model.size_bytes)}
                          </span>
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
                      ~
                      {downloadProgress.eta_seconds < 60
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
