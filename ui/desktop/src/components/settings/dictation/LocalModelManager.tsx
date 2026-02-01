import { useState, useEffect } from 'react';
import { Download, Trash2, X, Check } from 'lucide-react';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import {
  listModels,
  downloadModel,
  getDownloadProgress,
  cancelDownload as cancelDownloadApi,
  deleteModel as deleteModelApi,
  type WhisperModel,
  type DownloadProgress,
} from '../../../api';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

const getQualityLabel = (quality: string): string => {
  // Capitalize first letter
  return quality.charAt(0).toUpperCase() + quality.slice(1).replace('_', ' ');
};

export const LocalModelManager = () => {
  const [models, setModels] = useState<WhisperModel[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [hardwareTier, setHardwareTier] = useState<string>('mid_range');
  const { read, upsert } = useConfig();

  useEffect(() => {
    loadModels();
    loadSelectedModel();
    detectHardware();
  }, []);

  const loadSelectedModel = async () => {
    try {
      const value = await read('LOCAL_WHISPER_MODEL', false);
      if (value && typeof value === 'string') {
        setSelectedModelId(value);
      } else {
        // Default to tiny if not set
        setSelectedModelId('tiny');
      }
    } catch (error) {
      console.error('Failed to load selected model:', error);
      setSelectedModelId('tiny');
    }
  };

  const selectModel = async (modelId: string) => {
    // Store just the model ID, the backend will resolve the full path
    try {
      await upsert('LOCAL_WHISPER_MODEL', modelId, false);
      setSelectedModelId(modelId);
    } catch (error) {
      console.error('Failed to save selected model:', error);
    }
  };

  const loadModels = async () => {
    try {
      const response = await listModels();
      if (response.data) {
        setModels(response.data);
      }
    } catch (error) {
      console.error('Failed to load models:', error);
    }
  };

  const detectHardware = async () => {
    // Simple client-side hardware detection
    const memory = (navigator as any).deviceMemory || 4; // GB
    if (memory >= 16) {
      setHardwareTier('high_end');
    } else if (memory >= 8) {
      setHardwareTier('mid_range');
    } else {
      setHardwareTier('low_end');
    }
  };

  const startDownload = async (modelId: string) => {
    try {
      await downloadModel({ path: { model_id: modelId } });
      pollDownloadProgress(modelId);
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const pollDownloadProgress = (modelId: string) => {
    const interval = setInterval(async () => {
      try {
        const response = await getDownloadProgress({ path: { model_id: modelId } });
        if (response.data) {
          const progress = response.data;
          setDownloads((prev) => new Map(prev).set(modelId, progress));

          if (progress.status === 'completed' || progress.status === 'failed') {
            clearInterval(interval);
            loadModels(); // Refresh model list
          }
        } else {
          clearInterval(interval);
        }
      } catch (error) {
        clearInterval(interval);
      }
    }, 500);
  };

  const cancelDownload = async (modelId: string) => {
    try {
      await cancelDownloadApi({ path: { model_id: modelId } });
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
    if (!confirm('Delete this model? You can re-download it later.')) return;

    try {
      await deleteModelApi({ path: { model_id: modelId } });
      loadModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  };

  const getRecommendation = (model: WhisperModel): string | null => {
    const tier = hardwareTier.replace('_', '-');
    if (model.recommended_for.some((r) => r.toLowerCase().includes(tier))) {
      return 'Recommended for your hardware';
    }
    return null;
  };

  return (
    <div className="space-y-3">
      <div className="text-xs text-text-muted mb-2">
        <p>Supports GPU acceleration (CUDA for NVIDIA, Metal for Apple Silicon). GPU features must be enabled at build time for hardware acceleration.</p>
      </div>

      <div className="space-y-2">
        {models.map((model) => {
          const progress = downloads.get(model.id);
          const isDownloading = progress?.status === 'downloading';
          const recommendation = getRecommendation(model);
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
                  <div className="flex items-center gap-2">
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
                      {model.size_display}
                    </span>
                    {isSelected && (
                      <span className="text-xs bg-accent-primary text-white px-2 py-0.5 rounded">
                        Active
                      </span>
                    )}
                    {recommendation && !isSelected && (
                      <span className="text-xs bg-accent-primary/10 text-accent-primary px-2 py-0.5 rounded">
                        Recommended
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-3 mt-1 text-xs text-text-muted">
                    <span>{getQualityLabel(model.quality)}</span>
                    <span>•</span>
                    <span>{model.speed}</span>
                  </div>

                  {model.description && (
                    <p className="text-xs text-text-muted mt-1">
                      {model.description}
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

      {models.length === 0 && (
        <div className="text-center py-6 text-text-muted text-sm">
          No models available
        </div>
      )}
    </div>
  );
};
