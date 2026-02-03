import { useState, useEffect } from 'react';
import { Download, Trash2, X, Check, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import {
  listLocalModels,
  downloadLocalModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  deleteLocalModel,
  type LocalModelResponse,
  type DownloadProgress,
} from '../../../api';

const LOCAL_LLM_MODEL_CONFIG_KEY = 'LOCAL_LLM_MODEL';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

export const LocalInferenceSettings = () => {
  const [models, setModels] = useState<LocalModelResponse[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [showAllModels, setShowAllModels] = useState(false);
  const { read, upsert } = useConfig();

  useEffect(() => {
    loadModels();
    loadSelectedModel();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadSelectedModel = async () => {
    try {
      const value = await read(LOCAL_LLM_MODEL_CONFIG_KEY, false);
      if (value && typeof value === 'string') {
        setSelectedModelId(value);
      } else {
        setSelectedModelId(null);
      }
    } catch (error) {
      console.error('Failed to load selected model:', error);
      setSelectedModelId(null);
    }
  };

  const selectModel = async (modelId: string) => {
    await upsert(LOCAL_LLM_MODEL_CONFIG_KEY, modelId, false);
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    setSelectedModelId(modelId);
  };

  const loadModels = async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        setModels(response.data);
      }
    } catch (error) {
      console.error('Failed to load models:', error);
    }
  };

  const startDownload = async (modelId: string) => {
    try {
      await downloadLocalModel({ path: { model_id: modelId } });
      pollDownloadProgress(modelId);
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const pollDownloadProgress = (modelId: string) => {
    const interval = setInterval(async () => {
      try {
        const response = await getLocalModelDownloadProgress({ path: { model_id: modelId } });
        if (response.data) {
          const progress = response.data;
          setDownloads((prev) => new Map(prev).set(modelId, progress));

          if (progress.status === 'completed') {
            clearInterval(interval);
            await loadModels(); // Refresh model list
            // Auto-select the model that was just downloaded
            await selectModel(modelId);
          } else if (progress.status === 'failed') {
            clearInterval(interval);
            await loadModels();
          }
        } else {
          clearInterval(interval);
        }
      } catch {
        clearInterval(interval);
      }
    }, 500);
  };

  const cancelDownload = async (modelId: string) => {
    try {
      await cancelLocalModelDownload({ path: { model_id: modelId } });
      setDownloads((prev) => {
        const next = new Map(prev);
        next.delete(modelId);
        return next;
      });
      loadModels();
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const deleteModel = async (modelId: string) => {
    if (!window.confirm('Delete this model? You can re-download it later.')) return;

    try {
      await deleteLocalModel({ path: { model_id: modelId } });
      if (selectedModelId === modelId) {
        await upsert(LOCAL_LLM_MODEL_CONFIG_KEY, '', false);
        setSelectedModelId(null);
      }
      loadModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  };

  const hasDownloadedNonRecommended = models.some(
    (model) => model.downloaded && !model.recommended
  );
  const displayedModels = showAllModels || hasDownloadedNonRecommended
    ? models
    : models.filter((m) => m.recommended);
  const hasNonRecommendedModels = models.some((m) => !m.recommended);
  const showToggleButton = hasNonRecommendedModels && !hasDownloadedNonRecommended;

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-text-default font-medium">Local Inference Models</h3>
        <p className="text-xs text-text-muted max-w-2xl mt-1">
          Download and manage local LLM models for inference without API keys. Supports GPU acceleration (Metal for Apple Silicon).
        </p>
      </div>

      <div className="space-y-2">
        {displayedModels.map((model) => {
          const progress = downloads.get(model.id);
          const isDownloading = progress?.status === 'downloading';
          const isSelected = selectedModelId === model.id;
          const canSelect = model.downloaded && !isDownloading;

          return (
            <div
              key={model.id}
              className={`border rounded-lg p-3 transition-colors ${
                isSelected
                  ? 'border-accent-primary bg-accent-primary/5'
                  : 'border-border-subtle bg-background-default hover:border-border-default'
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    {canSelect && (
                      <input
                        type="radio"
                        checked={isSelected}
                        onChange={() => selectModel(model.id)}
                        className="cursor-pointer"
                      />
                    )}
                    <h4 className="text-sm font-medium text-text-default">
                      {model.name}
                    </h4>
                    <span className="text-xs text-text-muted">
                      {model.size_mb}MB
                    </span>
                    <span className="text-xs text-text-muted">
                      {model.context_limit.toLocaleString()} tokens
                    </span>
                    {model.recommended && (
                      <span className="text-xs bg-blue-500 text-white px-2 py-0.5 rounded">
                        Recommended
                      </span>
                    )}
                    {isSelected && (
                      <span className="text-xs bg-accent-primary text-white px-2 py-0.5 rounded">
                        Active
                      </span>
                    )}
                  </div>

                  <p className="text-xs text-text-muted mt-1">
                    {model.description}
                  </p>
                  {model.recommended && (
                    <p className="text-xs text-blue-600 mt-1 font-medium">
                      Recommended for your hardware
                    </p>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  {model.downloaded ? (
                    <>
                      <div className="flex items-center gap-1 text-xs text-green-600">
                        <Check className="w-4 h-4" />
                        <span>Downloaded</span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => deleteModel(model.id)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </>
                  ) : isDownloading ? (
                    <>
                      <div className="text-xs text-text-muted min-w-[60px]">
                        {progress.progress_percent.toFixed(0)}%
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => cancelDownload(model.id)}
                      >
                        <X className="w-4 h-4" />
                      </Button>
                    </>
                  ) : (
                    <Button variant="outline" size="sm" onClick={() => startDownload(model.id)}>
                      <Download className="w-4 h-4 mr-1" />
                      Download
                    </Button>
                  )}
                </div>
              </div>

              {isDownloading && progress && (
                <div className="mt-2 space-y-1">
                  <div className="w-full bg-background-subtle rounded-full h-1.5">
                    <div
                      className="bg-accent-primary h-1.5 rounded-full transition-all"
                      style={{ width: `${progress.progress_percent}%` }}
                    />
                  </div>
                  <div className="flex justify-between text-xs text-text-muted">
                    <span>
                      {formatBytes(progress.bytes_downloaded)} / {formatBytes(progress.total_bytes)}
                    </span>
                    {progress.speed_bps && (
                      <span>{formatBytes(progress.speed_bps)}/s</span>
                    )}
                  </div>
                </div>
              )}

              {progress?.status === 'failed' && progress.error && (
                <div className="mt-2 text-xs text-destructive">{progress.error}</div>
              )}
            </div>
          );
        })}
      </div>

      {showToggleButton && (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowAllModels(!showAllModels)}
          className="w-full text-text-muted hover:text-text-default"
        >
          {showAllModels ? (
            <>
              <ChevronUp className="w-4 h-4 mr-1" />
              Show recommended only
            </>
          ) : (
            <>
              <ChevronDown className="w-4 h-4 mr-1" />
              Show all models ({models.length - displayedModels.length} more)
            </>
          )}
        </Button>
      )}

      {models.length === 0 && (
        <div className="text-center py-6 text-text-muted text-sm">
          No models available
        </div>
      )}
    </div>
  );
};
