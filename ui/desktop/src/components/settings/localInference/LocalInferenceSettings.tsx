import { useState, useEffect, useCallback, useRef } from 'react';
import {
  listLocalModels,
  downloadHfModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  deleteLocalModel,
  LocalModelResponse,
} from '../../../api';
import { useConfig } from '../../ConfigContext';
import { Button } from '../../ui/button';
import { Download, X, Trash2, ChevronDown, ChevronUp, Settings } from 'lucide-react';
import { HuggingFaceModelSearch } from './HuggingFaceModelSearch';
import { ModelSettingsPanel } from './ModelSettingsPanel';

const LOCAL_LLM_MODEL_CONFIG_KEY = 'LOCAL_LLM_MODEL';

interface DownloadProgress {
  bytes_downloaded: number;
  total_bytes: number;
  progress_percent: number;
  speed_bps: number;
  status: string;
}

const formatSize = (bytes: number): string => {
  if (bytes === 0) return '';
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
};

export const LocalInferenceSettings = () => {
  const [featuredModels, setFeaturedModels] = useState<LocalModelResponse[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [currentProvider, setCurrentProvider] = useState<string | null>(null);
  const [showAllFeatured, setShowAllFeatured] = useState(false);
  const [settingsOpenFor, setSettingsOpenFor] = useState<string | null>(null);
  const { read, upsert } = useConfig();
  const downloadSectionRef = useRef<HTMLDivElement>(null);

  const loadModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        // All models from the API are featured or downloaded
        setFeaturedModels(response.data);

        // Start polling for any models that are already downloading
        response.data.forEach((model) => {
          if (model.status.state === 'Downloading') {
            const status = model.status;
            setDownloads((prev) => {
              const next = new Map(prev);
              next.set(model.id, {
                bytes_downloaded: status.bytes_downloaded ?? 0,
                total_bytes: status.total_bytes ?? 0,
                progress_percent: status.progress_percent ?? 0,
                speed_bps: status.speed_bps ?? 0,
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
      const provider = await read('GOOSE_PROVIDER', false);
      setCurrentProvider(typeof provider === 'string' ? provider : null);

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
    setCurrentProvider('local');
  };

  const startFeaturedDownload = async (modelId: string) => {
    const model = featuredModels.find((m) => m.id === modelId);
    if (!model) return;

    try {
      await downloadHfModel({
        body: { spec: model.id },
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
          const { status, bytes_downloaded, total_bytes, speed_bps } = response.data;
          const progress_percent = total_bytes > 0 ? (bytes_downloaded / total_bytes) * 100 : 0;

          if (status === 'completed' || status === 'failed') {
            clearInterval(interval);
            setDownloads((prev) => {
              const next = new Map(prev);
              next.delete(modelId);
              return next;
            });
            // Refresh model list and auto-select if completed
            await loadModels();
            if (status === 'completed') {
              // Auto-select the freshly downloaded model
              await selectModel(modelId);
            }
          } else {
            setDownloads((prev) => {
              const next = new Map(prev);
              next.set(modelId, {
                bytes_downloaded,
                total_bytes,
                progress_percent,
                speed_bps: speed_bps || 0,
                status,
              });
              return next;
            });
          }
        }
      } catch {
        clearInterval(interval);
        setDownloads((prev) => {
          const next = new Map(prev);
          next.delete(modelId);
          return next;
        });
      }
    }, 1000);
  };

  const cancelDownload = async (modelId: string) => {
    try {
      await cancelLocalModelDownload({ path: { model_id: modelId } });
      setDownloads((prev) => {
        const next = new Map(prev);
        next.delete(modelId);
        return next;
      });
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    try {
      await deleteLocalModel({ path: { model_id: modelId } });
      await loadModels();
      // If we deleted the selected model, clear selection
      if (selectedModelId === modelId) {
        setSelectedModelId(null);
      }
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  };

  const handleHfDownloadStarted = (modelId: string) => {
    pollDownloadProgress(modelId);
    loadModels();
    scrollToDownloads();
  };

  const isDownloaded = (model: LocalModelResponse) => model.status.state === 'Downloaded';
  const isDownloading = (modelId: string) =>
    downloads.has(modelId) && downloads.get(modelId)?.status === 'downloading';

  // Check if model is selected (provider is local AND this model is selected)
  const isSelected = (modelId: string) =>
    currentProvider === 'local' && selectedModelId === modelId;

  // Filter models by status
  const downloadedModels = featuredModels.filter(isDownloaded);
  const notDownloadedModels = featuredModels.filter(
    (m) => !isDownloaded(m) && !isDownloading(m.id)
  );
  // In collapsed state, only show recommended models; expanded shows all
  const recommendedModels = notDownloadedModels.filter((m) => m.recommended);
  const displayedFeatured = showAllFeatured ? notDownloadedModels : recommendedModels;
  const showFeaturedToggle = notDownloadedModels.length > recommendedModels.length;

  return (
    <div className="space-y-4">
      {/* Downloaded Models */}
      {downloadedModels.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloaded Models</h4>
          <div className="space-y-2">
            {downloadedModels.map((model) => (
              <div
                key={model.id}
                className={`border rounded-lg p-3 transition-colors cursor-pointer ${
                  isSelected(model.id)
                    ? 'border-blue-500 bg-blue-500/10'
                    : 'border-border-subtle hover:border-border-default'
                }`}
                onClick={() => selectModel(model.id)}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    {/* Radio button */}
                    <div
                      className={`w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0 ${
                        isSelected(model.id) ? 'border-blue-500' : 'border-text-muted'
                      }`}
                    >
                      {isSelected(model.id) && <div className="w-2 h-2 rounded-full bg-blue-500" />}
                    </div>
                    <div className="min-w-0">
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
                  </div>
                  <div className="flex items-center gap-1 flex-shrink-0">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        setSettingsOpenFor(settingsOpenFor === model.id ? null : model.id);
                      }}
                      className="text-text-muted hover:text-text-default"
                    >
                      <Settings className="w-4 h-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteModel(model.id);
                      }}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
                {settingsOpenFor === model.id && (
                  <div className="mt-3 pt-3 border-t border-border-subtle">
                    <ModelSettingsPanel modelId={model.id} />
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Active Downloads */}
      {downloads.size > 0 && (
        <div ref={downloadSectionRef}>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloading</h4>
          <div className="space-y-2">
            {Array.from(downloads.entries()).map(([modelId, progress]) => {
              const model = featuredModels.find((m) => m.id === modelId);
              const displayName = model?.display_name || modelId;

              return (
                <div
                  key={modelId}
                  className="border rounded-lg p-3 border-border-subtle bg-background-default"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-text-default truncate">
                      {displayName}
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => cancelDownload(modelId)}
                      className="text-destructive hover:text-destructive flex-shrink-0"
                    >
                      <X className="w-4 h-4" />
                    </Button>
                  </div>
                  <div className="w-full bg-gray-700 rounded-full h-2 mb-1">
                    <div
                      className="bg-blue-500 h-2 rounded-full transition-all"
                      style={{ width: `${progress.progress_percent}%` }}
                    />
                  </div>
                  <div className="flex justify-between text-xs text-text-muted">
                    <span>
                      {formatSize(progress.bytes_downloaded)} / {formatSize(progress.total_bytes)} (
                      {progress.progress_percent.toFixed(0)}%)
                    </span>
                    {progress.speed_bps > 0 && <span>{formatSize(progress.speed_bps)}/s</span>}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Featured Models (not downloaded) */}
      {displayedFeatured.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-2">Featured Models</h4>
          <div className="space-y-2">
            {displayedFeatured.map((model) => (
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
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => startFeaturedDownload(model.id)}
                    >
                      <Download className="w-4 h-4 mr-1" />
                      Download
                    </Button>
                  </div>
                </div>
              </div>
            ))}
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
                  Show all featured ({notDownloadedModels.length - displayedFeatured.length} more)
                </>
              )}
            </Button>
          )}
        </div>
      )}

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
