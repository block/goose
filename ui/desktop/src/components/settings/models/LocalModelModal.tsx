import React, { useCallback, useEffect, useState, useRef } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../ui/dialog';
import { Button } from '../../ui/button';
import {
  listLocalModels,
  downloadHfModel,
  cancelLocalModelDownload,
  deleteLocalModel,
  LocalModelResponse,
} from '../../../api';
import { Download, X, Check, Search, Trash2 } from 'lucide-react';
import { HuggingFaceSearchModal } from './HuggingFaceSearchModal';

// Provider avatar URLs for known model providers
const PROVIDER_AVATARS: Record<string, string> = {
  'meta-llama': 'https://huggingface.co/meta-llama/avatar.png',
  mistralai: 'https://huggingface.co/mistralai/avatar.png',
  NousResearch: 'https://huggingface.co/NousResearch/avatar.png',
  bartowski: 'https://huggingface.co/bartowski/avatar.png',
};

// Helper to extract original provider from repo_id
function getOriginalProvider(repoId: string): string | null {
  const parts = repoId.split('/');
  return parts.length > 0 ? parts[0] : null;
}

// Type guard to check if a model is a featured/recommended model
function isFeaturedModel(item: LocalModelResponse): boolean {
  return item.tier !== undefined && item.tier !== null;
}

// Format bytes to human readable
function formatBytes(bytes: number | null | undefined): string {
  if (!bytes) return 'N/A';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}

export const LOCAL_MODEL_CONFIG_KEY = 'local';

interface LocalModelModalProps {
  isOpen: boolean;
  onClose: () => void;
  onModelSelect: (modelId: string) => void;
}

export const LocalModelModal: React.FC<LocalModelModalProps> = ({
  isOpen,
  onClose,
  onModelSelect,
}) => {
  const [featuredModels, setFeaturedModels] = useState<LocalModelResponse[]>([]);
  const [pollingActive, setPollingActive] = useState(false);
  const [showHuggingFaceModal, setShowHuggingFaceModal] = useState(false);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Load models from the registry
  const loadLocalModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        // Filter to only show featured/recommended models in this modal
        const featured = response.data.filter(isFeaturedModel);
        setFeaturedModels(featured);

        // Check if any downloads are in progress
        const hasActiveDownloads = featured.some((m) => m.status.state === 'Downloading');
        if (!hasActiveDownloads && pollingActive) {
          setPollingActive(false);
        }
      }
    } catch (error) {
      console.error('Failed to load local models:', error);
    }
  }, [pollingActive]);

  // Initial load
  useEffect(() => {
    if (isOpen) {
      loadLocalModels();
    }
  }, [isOpen, loadLocalModels]);

  // Polling for download progress
  useEffect(() => {
    if (pollingActive && isOpen) {
      pollingRef.current = setInterval(() => {
        loadLocalModels();
      }, 500);
    }

    return () => {
      if (pollingRef.current) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
    };
  }, [pollingActive, isOpen, loadLocalModels]);

  const selectLocalModel = (modelId: string) => {
    onModelSelect(modelId);
    onClose();
  };

  const startDownload = async (model: LocalModelResponse) => {
    try {
      await downloadHfModel({
        body: {
          spec: `${model.repo_id}:${model.quantization}`,
        },
      });
      setPollingActive(true);
      loadLocalModels();
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const cancelDownload = async (modelId: string) => {
    try {
      await cancelLocalModelDownload({ path: { model_id: modelId } });
      loadLocalModels();
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    try {
      await deleteLocalModel({ path: { model_id: modelId } });
      loadLocalModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  };

  // Separate downloaded and not-downloaded models
  const downloadedModels = featuredModels.filter((m) => m.status.state === 'Downloaded');
  const notDownloadedModels = featuredModels.filter((m) => m.status.state !== 'Downloaded');

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[90vw] md:max-w-[80vw] lg:max-w-[900px] max-h-[85vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>Local Models</DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto py-4 space-y-4 pr-1">
          {featuredModels.length === 0 && (
            <div className="text-center py-6 px-4 bg-amber-500/10 border border-amber-500/20 rounded-lg">
              <p className="text-sm text-text-muted">Loading models...</p>
            </div>
          )}

          {/* Downloaded Models */}
          {downloadedModels.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-text-muted mb-2">Downloaded</h3>
              <div className="grid grid-cols-3 gap-3 pt-2 pr-2">
                {downloadedModels.map((model) => {
                  const originalProvider = getOriginalProvider(model.repo_id);
                  const providerAvatarUrl = originalProvider
                    ? PROVIDER_AVATARS[originalProvider]
                    : null;

                  return (
                    <div
                      key={model.id}
                      onClick={() => selectLocalModel(model.id)}
                      className="cursor-pointer"
                    >
                      <div className="border rounded-lg p-3 flex flex-col h-full transition-all border-border-subtle bg-background-default hover:border-border-default">
                        {/* Row 1: Avatar left, Actions right */}
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
                            <Check className="w-4 h-4 text-green-600" />
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleDeleteModel(model.id);
                              }}
                              className="h-6 w-6 p-0 text-text-muted hover:text-red-500"
                              title="Delete model"
                            >
                              <Trash2 className="w-3 h-3" />
                            </Button>
                          </div>
                        </div>

                        {/* Title */}
                        <h4 className="text-xs font-medium text-text-default leading-tight">
                          {model.display_name}
                        </h4>

                        {/* Author */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {originalProvider || 'Unknown'}
                        </p>

                        {/* Size & Context */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {formatBytes(model.size_bytes)} •{' '}
                          {model.context_limit?.toLocaleString() ?? 'N/A'} ctx
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Not Downloaded / Downloading Models */}
          {notDownloadedModels.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-text-muted mb-2">Recommended</h3>
              <div className="grid grid-cols-3 gap-3 pt-2 pr-2">
                {notDownloadedModels.map((model) => {
                  const originalProvider = getOriginalProvider(model.repo_id);
                  const providerAvatarUrl = originalProvider
                    ? PROVIDER_AVATARS[originalProvider]
                    : null;
                  const isDownloading = model.status.state === 'Downloading';
                  const progress = model.status.state === 'Downloading' ? model.status : null;

                  return (
                    <div key={model.id} className="relative">
                      {/* Recommended badge */}
                      {model.recommended && (
                        <div className="absolute -top-1 -right-1 z-20">
                          <span className="bg-blue-500 text-white text-[10px] px-1.5 py-0.5 rounded-full">
                            Recommended
                          </span>
                        </div>
                      )}

                      <div className="border rounded-lg p-3 flex flex-col h-full transition-all border-border-subtle bg-background-default hover:border-border-default">
                        {/* Row 1: Avatar left, Download/Cancel button right */}
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
                                title="Cancel download"
                              >
                                <X className="w-3 h-3" />
                              </Button>
                            ) : (
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => startDownload(model)}
                                className="h-6 w-6 p-0"
                                title="Download model"
                              >
                                <Download className="w-3 h-3" />
                              </Button>
                            )}
                          </div>
                        </div>

                        {/* Title */}
                        <h4 className="text-xs font-medium text-text-default leading-tight">
                          {model.display_name}
                        </h4>

                        {/* Author */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {originalProvider || 'Unknown'}
                        </p>

                        {/* Size & Context */}
                        <p className="text-xs text-text-muted mt-0.5">
                          {formatBytes(model.size_bytes)} •{' '}
                          {model.context_limit?.toLocaleString() ?? 'N/A'} ctx
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
                              <span>{progress.progress_percent?.toFixed(0) ?? 0}%</span>
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
        onDownloadStarted={(_modelId) => {
          setPollingActive(true);
          setShowHuggingFaceModal(false);
          loadLocalModels();
        }}
      />
    </Dialog>
  );
};
