import { useState, useEffect, useCallback, useRef } from 'react';
import { Download, Trash2, X, Check, ChevronDown, ChevronUp, Settings2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { useModelAndProvider } from '../../ModelAndProviderContext';
import {
  listLocalModels,
  downloadLocalModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  deleteLocalModel,
  setConfigProvider,
  type DownloadProgress,
  type LocalModelResponse,
  type RegistryModelResponse,
  type ModelListItem,
} from '../../../api';
import { HuggingFaceModelSearch } from './HuggingFaceModelSearch';
import { ModelSettingsPanel } from './ModelSettingsPanel';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

function isFeaturedModel(item: ModelListItem): item is LocalModelResponse & { featured: boolean } {
  return 'tier' in item;
}

function isRegistryModel(item: ModelListItem): item is RegistryModelResponse {
  return 'display_name' in item && !('tier' in item);
}

export const LocalInferenceSettings = () => {
  const [featuredModels, setFeaturedModels] = useState<(LocalModelResponse & { featured?: boolean })[]>([]);
  const [registryModels, setRegistryModels] = useState<RegistryModelResponse[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [showAllFeatured, setShowAllFeatured] = useState(false);
  const [settingsOpenFor, setSettingsOpenFor] = useState<string | null>(null);
  const { currentModel, currentProvider, setProviderAndModel } = useModelAndProvider();
  const downloadSectionRef = useRef<HTMLDivElement>(null);
  const selectedModelId = currentProvider === 'local' ? currentModel : null;

  const loadModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        const featured: (LocalModelResponse & { featured?: boolean })[] = [];
        const registry: RegistryModelResponse[] = [];

        for (const item of response.data) {
          if (isFeaturedModel(item)) {
            featured.push(item);
          } else if (isRegistryModel(item)) {
            registry.push(item);
          }
        }

        setFeaturedModels(featured);
        setRegistryModels(registry);
      }
    } catch (error) {
      console.error('Failed to load models:', error);
    }
  }, []);

  useEffect(() => {
    loadModels();
  }, [loadModels]);

  const selectModel = async (modelId: string) => {
    setProviderAndModel('local', modelId);
    try {
      await setConfigProvider({
        body: { provider: 'local', model: modelId },
        throwOnError: true,
      });
    } catch (error) {
      console.error('Failed to select model:', error);
    }
  };

  const startFeaturedDownload = async (modelId: string) => {
    try {
      await downloadLocalModel({ path: { model_id: modelId } });
      pollDownloadProgress(modelId);
      scrollToDownloads();
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const scrollToDownloads = useCallback(() => {
    // Wait a tick for the download section to render before scrolling.
    requestAnimationFrame(() => {
      downloadSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  }, []);

  const pollDownloadProgress = (modelId: string) => {
    const interval = setInterval(async () => {
      try {
        const response = await getLocalModelDownloadProgress({ path: { model_id: modelId } });
        if (response.data) {
          const progress = response.data;
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
    (model) => model.downloaded && !model.recommended
  );
  const displayedFeatured = showAllFeatured || hasDownloadedNonRecommended
    ? featuredModels
    : featuredModels.filter((m) => m.recommended);
  const hasNonRecommendedFeatured = featuredModels.some((m) => !m.recommended);
  const showFeaturedToggle = hasNonRecommendedFeatured && !hasDownloadedNonRecommended;

  // Downloaded models from both featured and registry
  const downloadedFeatured = featuredModels.filter((m) => m.downloaded);
  const downloadedRegistry = registryModels.filter((m) => m.downloaded);
  const hasDownloaded = downloadedFeatured.length > 0 || downloadedRegistry.length > 0;

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-text-default font-medium">Local Inference Models</h3>
        <p className="text-xs text-text-muted max-w-2xl mt-1">
          Download and manage local LLM models for inference without API keys. Search HuggingFace for any GGUF model or use the featured picks below.
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
                    <span className="text-sm font-medium text-text-default truncate">{modelId}</span>
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
                        <span>{formatBytes(progress.bytes_downloaded)} / {formatBytes(progress.total_bytes)}</span>
                        <span>{progress.progress_percent.toFixed(0)}%</span>
                      </div>
                    </div>
                  )}
                  {progress.status === 'failed' && (
                    <p className="text-xs text-destructive">{progress.error || 'Download failed'}</p>
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
                      : 'border-border-subtle bg-background-default hover:border-border-default'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <input
                        type="radio"
                        checked={isSelected}
                        onChange={() => selectModel(model.id)}
                        className="cursor-pointer"
                      />
                      <span className="text-sm font-medium text-text-default">{model.name}</span>
                      <span className="text-xs text-text-muted">{model.size_mb}MB</span>
                      {model.recommended && (
                        <span className="text-xs bg-blue-500 text-white px-2 py-0.5 rounded">Recommended</span>
                      )}
                    </div>
                    <div className="flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setSettingsOpenFor(model.id)}
                        title="Model settings"
                      >
                        <Settings2 className="w-4 h-4" />
                      </Button>
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
                </div>
              );
            })}

            {downloadedRegistry.map((model) => {
              const isSelected = selectedModelId === model.id;
              return (
                <div
                  key={model.id}
                  className={`border rounded-lg p-3 transition-colors ${
                    isSelected
                      ? 'border-accent-primary bg-accent-primary/5'
                      : 'border-border-subtle bg-background-default hover:border-border-default'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <input
                        type="radio"
                        checked={isSelected}
                        onChange={() => selectModel(model.id)}
                        className="cursor-pointer"
                      />
                      <span className="text-sm font-medium text-text-default">{model.display_name}</span>
                      <span className="text-xs text-text-muted">{formatBytes(model.size_bytes)}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setSettingsOpenFor(model.id)}
                        title="Model settings"
                      >
                        <Settings2 className="w-4 h-4" />
                      </Button>
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
            const progress = downloads.get(model.id);
            const isDownloading = progress?.status === 'downloading';

            return (
              <div
                key={model.id}
                className="border rounded-lg p-3 border-border-subtle bg-background-default hover:border-border-default"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h4 className="text-sm font-medium text-text-default">{model.name}</h4>
                      <span className="text-xs text-text-muted">{model.size_mb}MB</span>
                      <span className="text-xs text-text-muted">
                        {model.context_limit.toLocaleString()} tokens
                      </span>
                      {model.recommended && (
                        <span className="text-xs bg-blue-500 text-white px-2 py-0.5 rounded">
                          Recommended
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-text-muted mt-1">{model.description}</p>
                  </div>

                  <div className="flex items-center gap-2">
                    {model.downloaded ? (
                      <div className="flex items-center gap-1 text-xs text-green-600">
                        <Check className="w-4 h-4" />
                        <span>Downloaded</span>
                      </div>
                    ) : isDownloading ? (
                      <>
                        <div className="text-xs text-text-muted min-w-[60px]">
                          {progress.progress_percent.toFixed(0)}%
                        </div>
                        <Button variant="ghost" size="sm" onClick={() => cancelDownload(model.id)}>
                          <X className="w-4 h-4" />
                        </Button>
                      </>
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
                        {formatBytes(progress.bytes_downloaded)} / {formatBytes(progress.total_bytes)}
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

      {/* Non-downloaded registry models being downloaded */}
      {registryModels
        .filter((m) => !m.downloaded && downloads.has(m.id))
        .map((model) => {
          const progress = downloads.get(model.id);
          if (!progress || progress.status !== 'downloading') return null;
          return (
            <div key={model.id} className="border rounded-lg p-3 border-border-subtle bg-background-default">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-text-default">{model.display_name}</span>
                  <span className="text-xs text-text-muted">{progress.progress_percent.toFixed(0)}%</span>
                </div>
                <Button variant="ghost" size="sm" onClick={() => cancelDownload(model.id)}>
                  <X className="w-4 h-4" />
                </Button>
              </div>
              <div className="mt-2">
                <div className="w-full bg-background-subtle rounded-full h-1.5">
                  <div
                    className="bg-accent-primary h-1.5 rounded-full transition-all"
                    style={{ width: `${progress.progress_percent}%` }}
                  />
                </div>
              </div>
            </div>
          );
        })}

      {/* HuggingFace Search */}
      <div className="border-t border-border-subtle pt-4">
        <HuggingFaceModelSearch onDownloadStarted={handleHfDownloadStarted} />
      </div>

      {featuredModels.length === 0 && registryModels.length === 0 && (
        <div className="text-center py-6 text-text-muted text-sm">No models available</div>
      )}

      <Dialog open={!!settingsOpenFor} onOpenChange={(open) => { if (!open) setSettingsOpenFor(null); }}>
        <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Model Settings</DialogTitle>
            <p className="text-sm text-text-muted">
              {(() => {
                const featured = featuredModels.find((m) => m.id === settingsOpenFor);
                if (featured) return featured.name;
                const registry = registryModels.find((m) => m.id === settingsOpenFor);
                if (registry) return registry.display_name;
                return settingsOpenFor;
              })()}
            </p>
          </DialogHeader>
          {settingsOpenFor && <ModelSettingsPanel modelId={settingsOpenFor} />}
        </DialogContent>
      </Dialog>
    </div>
  );
};
