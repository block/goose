import { useState, useEffect, useCallback, useRef } from 'react';
import { Download, Trash2, X, Check, ChevronDown, ChevronUp, Settings2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import {
  listLocalModels,
  downloadHfModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  deleteLocalModel,
  type LocalModelResponse,
  type DownloadProgressResponse,
} from '../../../api';
import { HuggingFaceModelSearch } from './HuggingFaceModelSearch';
import { ModelSettingsPanel } from './ModelSettingsPanel';

const LOCAL_LLM_MODEL_CONFIG_KEY = 'LOCAL_LLM_MODEL';

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

interface DownloadProgress {
  bytes_downloaded: number;
  total_bytes: number;
  progress_percent: number;
  speed_bps?: number | null;
  eta_seconds?: number | null;
  status: string;
  error?: string | null;
}

const toDownloadProgress = (resp: DownloadProgressResponse): DownloadProgress => ({
  bytes_downloaded: resp.bytes_downloaded,
  total_bytes: resp.total_bytes,
  progress_percent: resp.total_bytes > 0 ? (resp.bytes_downloaded / resp.total_bytes) * 100 : 0,
  speed_bps: resp.speed_bps,
  eta_seconds: resp.eta_seconds,
  status: resp.status,
});

const isDownloaded = (model: LocalModelResponse): boolean => model.status.state === 'Downloaded';

export const LocalInferenceSettings = () => {
  const [featuredModels, setFeaturedModels] = useState<LocalModelResponse[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [showAllFeatured, setShowAllFeatured] = useState(false);
  const [settingsOpenFor, setSettingsOpenFor] = useState<string | null>(null);
  const { read, upsert } = useConfig();
  const downloadSectionRef = useRef<HTMLDivElement>(null);

  const loadModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        // All models are now LocalModelResponse with tier for featured
        const featured = response.data.filter((m): m is LocalModelResponse => 'tier' in m);
        setFeaturedModels(featured);

        // Start polling for any models that are already downloading
        featured.forEach((model) => {
          if (model.status.state === 'Downloading') {
            // Initialize with current progress from the model status
            const status = model.status;
            setDownloads((prev) => {
              const next = new Map(prev);
              next.set(model.id, {
                bytes_downloaded: status.bytes_downloaded,
                total_bytes: status.total_bytes,
                progress_percent: status.progress_percent,
                speed_bps: status.speed_bps,
                status: 'downloading',
              });
              return next;
            });
            pollDownloadProgress(model.id);
          }
        });
      }
    } catch (error) {
      console.error('Failed to load models:', error);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  const startFeaturedDownload = async (modelId: string) => {
    const model = featuredModels.find((m) => m.id === modelId);
    if (!model) return;

    try {
      await downloadHfModel({
        body: {
          repo_id: model.repo_id,
          filename: model.filename,
        },
      });
      pollDownloadProgress(modelId);
      scrollToDownloads();
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const scrollToDownloads = useCallback(() => {
    requestAnimationFrame(() => {
      downloadSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  }, []);

  const pollDownloadProgress = (modelId: string) => {
    const interval = setInterval(async () => {
      try {
        const response = await getLocalModelDownloadProgress({ path: { model_id: modelId } });
        if (response.data) {
          const progress = toDownloadProgress(response.data);
          setDownloads((prev) => new Map(prev).set(modelId, progress));

          if (progress.status === 'completed') {
            clearInterval(interval);
            await loadModels();
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

  const handleDeleteModel = async (modelId: string) => {
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

  const handleHfDownloadStarted = (modelId: string) => {
    pollDownloadProgress(modelId);
    scrollToDownloads();
  };

  // Featured models display logic
  const hasDownloadedNonRecommended = featuredModels.some(
    (model) => isDownloaded(model) && !model.recommended
  );
  const displayedFeatured =
    showAllFeatured || hasDownloadedNonRecommended
      ? featuredModels
      : featuredModels.filter((m) => m.recommended);
  const hasNonRecommendedFeatured = featuredModels.some((m) => !m.recommended);
  const showFeaturedToggle = hasNonRecommendedFeatured && !hasDownloadedNonRecommended;

  // Downloaded models
  const downloadedFeatured = featuredModels.filter((m) => isDownloaded(m));
  const hasDownloaded = downloadedFeatured.length > 0;

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-text-default font-medium">Local Inference Models</h3>
        <p className="text-xs text-text-muted max-w-2xl mt-1">
          Download and manage local LLM models for inference without API keys. Search HuggingFace
          for any GGUF model or use the featured picks below.
        </p>
      </div>

      {/* Active Downloads */}
      {downloads.size > 0 && (
        <div ref={downloadSectionRef}>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloading</h4>
          <div className="space-y-2">
            {Array.from(downloads.entries()).map(([modelId, progress]) => {
              if (progress.status === 'completed') return null;
              return (
                <div
                  key={modelId}
                  className="border rounded-lg p-3 border-border-subtle bg-background-default"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-text-default truncate">
                      {modelId}
                    </span>
                    {progress.status === 'downloading' && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => cancelDownload(modelId)}
                        className="text-destructive hover:text-destructive"
                      >
                        <X className="w-4 h-4" />
                      </Button>
                    )}
                  </div>
                  {progress.status === 'downloading' && (
                    <div className="space-y-1">
                      <div className="w-full bg-background-subtle rounded-full h-2">
                        <div
                          className="bg-accent-primary h-2 rounded-full transition-all duration-300"
                          style={{ width: `${progress.progress_percent}%` }}
                        />
                      </div>
                      <div className="flex justify-between text-xs text-text-muted">
                        <span>
                          {formatBytes(progress.bytes_downloaded)} /{' '}
                          {formatBytes(progress.total_bytes)}
                        </span>
                        <span>{progress.progress_percent.toFixed(0)}%</span>
                      </div>
                    </div>
                  )}
                  {progress.status === 'failed' && (
                    <p className="text-xs text-destructive">
                      {progress.error || 'Download failed'}
                    </p>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Downloaded Models */}
      {hasDownloaded && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloaded Models</h4>
          <div className="space-y-2">
            {downloadedFeatured.map((model) => {
              const isSelected = selectedModelId === model.id;
              return (
                <div
                  key={model.id}
                  className={`border rounded-lg p-3 transition-colors ${
                    isSelected
                      ? 'border-accent-primary bg-accent-primary/5'
                      : 'border-border-subtle hover:border-border-default'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <span className="text-sm font-medium text-text-default truncate">
                        {model.display_name}
                      </span>
                      <span className="text-xs text-text-muted flex-shrink-0">
                        {formatSize(model.size_bytes)}
                      </span>
                      {isSelected && (
                        <span className="text-xs bg-accent-primary text-white px-2 py-0.5 rounded-full flex-shrink-0">
                          Active
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1 flex-shrink-0">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() =>
                          setSettingsOpenFor(settingsOpenFor === model.id ? null : model.id)
                        }
                      >
                        <Settings2 className="w-4 h-4" />
                      </Button>
                      {!isSelected && (
                        <Button variant="outline" size="sm" onClick={() => selectModel(model.id)}>
                          <Check className="w-4 h-4 mr-1" />
                          Use
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteModel(model.id)}
                        className="text-destructive hover:text-destructive"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                  {settingsOpenFor === model.id && <ModelSettingsPanel modelId={model.id} />}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Featured Models */}
      <div>
        <h4 className="text-sm font-medium text-text-default mb-2">Featured Models</h4>
        <div className="space-y-2">
          {displayedFeatured.map((model) => {
            const downloaded = isDownloaded(model);
            const isDownloading =
              downloads.has(model.id) && downloads.get(model.id)?.status === 'downloading';
            const progress = downloads.get(model.id);

            // Skip if already shown in downloaded section
            if (downloaded) return null;

            return (
              <div
                key={model.id}
                className="border rounded-lg p-3 border-border-subtle hover:border-border-default transition-colors"
              >
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-text-default truncate">
                        {model.display_name}
                      </span>
                      <span className="text-xs text-text-muted flex-shrink-0">
                        {formatSize(model.size_bytes)}
                      </span>
                      {model.recommended && (
                        <span className="text-xs bg-green-600/20 text-green-400 px-2 py-0.5 rounded-full flex-shrink-0">
                          Recommended
                        </span>
                      )}
                    </div>
                    {model.context_limit && (
                      <p className="text-xs text-text-muted mt-0.5">
                        {(model.context_limit / 1000).toFixed(0)}K context
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2 flex-shrink-0">
                    {isDownloading ? (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => cancelDownload(model.id)}
                        className="text-destructive hover:text-destructive"
                      >
                        <X className="w-4 h-4 mr-1" />
                        Cancel
                      </Button>
                    ) : (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => startFeaturedDownload(model.id)}
                      >
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
                        {formatBytes(progress.bytes_downloaded)} /{' '}
                        {formatBytes(progress.total_bytes)}
                      </span>
                      {progress.speed_bps && <span>{formatBytes(progress.speed_bps)}/s</span>}
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

        {showFeaturedToggle && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowAllFeatured(!showAllFeatured)}
            className="w-full text-text-muted hover:text-text-default mt-2"
          >
            {showAllFeatured ? (
              <>
                <ChevronUp className="w-4 h-4 mr-1" />
                Show recommended only
              </>
            ) : (
              <>
                <ChevronDown className="w-4 h-4 mr-1" />
                Show all featured ({featuredModels.length - displayedFeatured.length} more)
              </>
            )}
          </Button>
        )}
      </div>

      {/* HuggingFace Search */}
      <div className="border-t border-border-subtle pt-4">
        <HuggingFaceModelSearch onDownloadStarted={handleHfDownloadStarted} />
      </div>

      {featuredModels.length === 0 && (
        <div className="text-center py-6 text-text-muted text-sm">No models available</div>
      )}
    </div>
  );
};
