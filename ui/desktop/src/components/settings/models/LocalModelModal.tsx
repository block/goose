import { useState, useEffect, useCallback } from 'react';
import { HardDrive, Download, Check, X, Search } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import {
  listLocalModels,
  downloadLocalModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  type DownloadProgress,
  type LocalModelResponse,
  type ModelListItem,
} from '../../../api';
import { HuggingFaceSearchModal } from './HuggingFaceSearchModal';

// Original provider avatar URLs from HuggingFace organizations
const PROVIDER_AVATARS: Record<string, string> = {
  'meta-llama': 'https://cdn-avatars.huggingface.co/v1/production/uploads/646cf8084eefb026fb8fd8bc/oCTqufkdTkjyGodsx1vo1.png',
  'mistralai': 'https://cdn-avatars.huggingface.co/v1/production/uploads/634c17653d11eaedd88b314d/9OgyfKstSZtbmsmuG8MbU.png',
};

// Get the original provider for a model based on its name
const getOriginalProvider = (modelName: string): string | null => {
  const lowerName = modelName.toLowerCase();
  if (lowerName.includes('llama') || lowerName.includes('hermes')) {
    return 'meta-llama';
  }
  if (lowerName.includes('mistral')) {
    return 'mistralai';
  }
  return null;
};

const LOCAL_LLM_MODEL_CONFIG_KEY = 'LOCAL_LLM_MODEL';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

function isFeaturedModel(item: ModelListItem): item is LocalModelResponse & { featured: boolean } {
  return 'tier' in item;
}

interface LocalModelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onModelSelected: (modelId: string) => void;
}

export function LocalModelModal({ isOpen, onClose, onModelSelected }: LocalModelModalProps) {
  const [featuredModels, setFeaturedModels] = useState<(LocalModelResponse & { featured?: boolean })[]>([]);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [showHuggingFaceModal, setShowHuggingFaceModal] = useState(false);
  const { upsert } = useConfig();

  // Load local models
  const loadLocalModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        const featured: (LocalModelResponse & { featured?: boolean })[] = [];
        for (const item of response.data) {
          if (isFeaturedModel(item)) {
            featured.push(item);
          }
        }
        setFeaturedModels(featured);
      }
    } catch (error) {
      console.error('Failed to load local models:', error);
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      loadLocalModels();
    }
  }, [isOpen, loadLocalModels]);

  const selectLocalModel = async (modelId: string) => {
    await upsert(LOCAL_LLM_MODEL_CONFIG_KEY, modelId, false);
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    onModelSelected(modelId);
    onClose();
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
            await loadLocalModels();
            // Auto-select the downloaded model
            await selectLocalModel(modelId);
          } else if (progress.status === 'failed') {
            clearInterval(interval);
            await loadLocalModels();
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
      loadLocalModels();
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const downloadedModels = featuredModels.filter(m => m.downloaded);
  const hasDownloadedModels = downloadedModels.length > 0;

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[90vw] md:max-w-[80vw] lg:max-w-[900px] max-h-[85vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <HardDrive size={24} className="text-green-500" />
            Local Models
          </DialogTitle>
          <DialogDescription>
            {hasDownloadedModels 
              ? 'Select a downloaded model or download a new one.'
              : 'No local models downloaded. Download a model to use local inference.'}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto py-4 space-y-4 pr-1">
          {/* Empty state message */}
          {!hasDownloadedModels && (
            <div className="text-center py-6 px-4 bg-amber-500/10 border border-amber-500/20 rounded-lg">
              <HardDrive className="w-12 h-12 text-amber-500 mx-auto mb-3" />
              <p className="text-sm text-text-muted">
                No local model downloaded yet. Choose a featured model below or search HuggingFace.
              </p>
            </div>
          )}

          {/* Available Models (downloaded) */}
          {hasDownloadedModels && (
            <div>
              <h4 className="text-sm font-medium text-text-default mb-3">Available Models</h4>
              <div className="grid grid-cols-3 gap-3 pt-2 pr-2">
                {downloadedModels.map((model) => {
                  const originalProvider = getOriginalProvider(model.name);
                  const providerAvatarUrl = originalProvider ? PROVIDER_AVATARS[originalProvider] : null;

                  return (
                    <div key={model.id} className="relative pt-1">
                      <div 
                        className="border rounded-lg p-3 flex flex-col h-full transition-all border-border-subtle bg-background-default hover:border-border-default cursor-pointer"
                        onClick={() => selectLocalModel(model.id)}
                      >
                        {/* Row 1: Avatar left, Check right */}
                        <div className="flex items-center justify-between mb-2">
                          {providerAvatarUrl ? (
                            <img
                              src={providerAvatarUrl}
                              alt={originalProvider || 'Provider'}
                              className="w-7 h-7 rounded-full object-cover"
                            />
                          ) : (
                            <div className="w-7 h-7 rounded-full bg-background-subtle" />
                          )}
                          <div className="flex items-center text-green-600">
                            <Check className="w-4 h-4" />
                          </div>
                        </div>

                        {/* Title */}
                        <h4 className="text-xs font-medium text-text-default leading-tight">{model.name}</h4>

                        {/* Author */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {originalProvider || 'Unknown'}
                        </p>

                        {/* Size & Context */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {model.size_mb}MB • {model.context_limit.toLocaleString()} ctx
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Featured Local Models (not downloaded) */}
          {featuredModels.filter(m => !m.downloaded).length > 0 && (
            <div>
              <h4 className="text-sm font-medium text-text-default mb-3">Featured Models</h4>
              <div className="grid grid-cols-3 gap-3 pt-2 pr-2">
                {featuredModels.filter(m => !m.downloaded).map((model) => {
                  const progress = downloads.get(model.id);
                  const isDownloading = progress?.status === 'downloading';
                  const originalProvider = getOriginalProvider(model.name);
                  const providerAvatarUrl = originalProvider ? PROVIDER_AVATARS[originalProvider] : null;

                  return (
                    <div key={model.id} className="relative pt-1">
                      {/* Recommended badge */}
                      {model.recommended && (
                        <div className="absolute -top-1 -right-1 z-20">
                          <span className="inline-block px-2 py-0.5 text-xs font-medium bg-blue-600 text-white rounded-full">
                            Recommended
                          </span>
                        </div>
                      )}

                      <div className="border rounded-lg p-3 flex flex-col h-full transition-all border-border-subtle bg-background-default hover:border-border-default">
                        {/* Row 1: Avatar left, Download button right */}
                        <div className="flex items-center justify-between mb-2">
                          {providerAvatarUrl ? (
                            <img
                              src={providerAvatarUrl}
                              alt={originalProvider || 'Provider'}
                              className="w-7 h-7 rounded-full object-cover"
                            />
                          ) : (
                            <div className="w-7 h-7 rounded-full bg-background-subtle" />
                          )}
                          <div className="flex items-center gap-1">
                            {isDownloading ? (
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => cancelDownload(model.id)}
                                className="h-6 w-6 p-0"
                              >
                                <X className="w-3 h-3" />
                              </Button>
                            ) : (
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => startDownload(model.id)}
                                className="h-6 w-6 p-0"
                                title="Download model"
                              >
                                <Download className="w-3 h-3" />
                              </Button>
                            )}
                          </div>
                        </div>

                        {/* Title */}
                        <h4 className="text-xs font-medium text-text-default leading-tight">{model.name}</h4>

                        {/* Author */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {originalProvider || 'Unknown'}
                        </p>

                        {/* Size & Context */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {model.size_mb}MB • {model.context_limit.toLocaleString()} ctx
                        </p>

                        {/* Download progress */}
                        {isDownloading && progress && (
                          <div className="mt-2 space-y-1">
                            <div className="w-full bg-background-subtle rounded-full h-1">
                              <div
                                className="bg-green-500 h-1 rounded-full transition-all"
                                style={{ width: `${progress.progress_percent}%` }}
                              />
                            </div>
                            <div className="flex justify-between text-xs text-text-muted">
                              <span>{progress.progress_percent.toFixed(0)}%</span>
                            </div>
                          </div>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Search HuggingFace Button */}
          <div className="border-t border-border-subtle pt-4">
            <Button
              variant="outline"
              className="w-full"
              onClick={() => setShowHuggingFaceModal(true)}
            >
              <Search className="w-4 h-4 mr-2" />
              Search HuggingFace
            </Button>
          </div>
        </div>
      </DialogContent>

      {/* HuggingFace Search Modal */}
      <HuggingFaceSearchModal
        isOpen={showHuggingFaceModal}
        onClose={() => setShowHuggingFaceModal(false)}
        onDownloadStarted={(modelId) => {
          pollDownloadProgress(modelId);
          setShowHuggingFaceModal(false);
        }}
      />
    </Dialog>
  );
}
