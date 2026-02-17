import { useState, useEffect, useCallback } from 'react';
import { Cloud, HardDrive, Download, Check, Settings2, X, Trash2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { useConfig } from '../../ConfigContext';
import { View } from '../../../utils/navigationUtils';
import { useModelAndProvider } from '../../ModelAndProviderContext';
import {
  listLocalModels,
  downloadHfModel,
  cancelLocalModelDownload,
  deleteLocalModel,
  type LocalModelResponse,
} from '../../../api';
import { HuggingFaceSearchModal } from './HuggingFaceSearchModal';
import ResetProviderSection from '../reset_provider/ResetProviderSection';

type FilterType = 'all' | 'cloud' | 'local';

// Original provider avatar URLs from HuggingFace organizations
const PROVIDER_AVATARS: Record<string, string> = {
  'meta-llama':
    'https://cdn-avatars.huggingface.co/v1/production/uploads/646cf8084eefb026fb8fd8bc/oCTqufkdTkjyGodsx1vo1.png',
  mistralai:
    'https://cdn-avatars.huggingface.co/v1/production/uploads/634c17653d11eaedd88b314d/9OgyfKstSZtbmsmuG8MbU.png',
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
const LAST_CLOUD_PROVIDER_KEY = 'LAST_CLOUD_PROVIDER';
const LAST_CLOUD_MODEL_KEY = 'LAST_CLOUD_MODEL';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

// Helper to check if model is downloaded
const isDownloaded = (model: LocalModelResponse): boolean => {
  return model.status.state === 'Downloaded';
};

// Helper to check if model is downloading
const isDownloading = (model: LocalModelResponse): boolean => {
  return model.status.state === 'Downloading';
};

// Helper to get download progress
const getDownloadProgress = (model: LocalModelResponse) => {
  if (model.status.state === 'Downloading') {
    return model.status;
  }
  return null;
};

interface UnifiedModelSectionProps {
  setView: (view: View) => void;
}

export default function UnifiedModelSection({ setView }: UnifiedModelSectionProps) {
  const [localModels, setLocalModels] = useState<LocalModelResponse[]>([]);
  const [selectedLocalModelId, setSelectedLocalModelId] = useState<string | null>(null);
  const [activeProvider, setActiveProvider] = useState<'cloud' | 'local' | null>(null);
  const [showHuggingFaceModal, setShowHuggingFaceModal] = useState(false);
  const [filter, setFilter] = useState<FilterType>('all');
  const [pollingActive, setPollingActive] = useState(false);

  const { read, upsert } = useConfig();
  const { currentModel, currentProvider } = useModelAndProvider();

  const [cloudModel, setCloudModel] = useState<string>('');
  const [cloudProvider, setCloudProvider] = useState<string>('');

  // Load cloud model info
  const loadCloudModelInfo = useCallback(async () => {
    try {
      if (currentProvider && currentProvider !== 'local') {
        setCloudModel(currentModel || '');
        setCloudProvider(currentProvider || '');
        return;
      }

      const savedProvider = await read(LAST_CLOUD_PROVIDER_KEY, false);
      const savedModel = await read(LAST_CLOUD_MODEL_KEY, false);

      if (savedProvider && typeof savedProvider === 'string') {
        setCloudProvider(savedProvider);
      }
      if (savedModel && typeof savedModel === 'string') {
        setCloudModel(savedModel);
      }
    } catch (error) {
      console.error('Failed to load cloud model info:', error);
    }
  }, [currentProvider, currentModel, read]);

  // Load local models
  const loadLocalModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        setLocalModels(response.data);
        const hasDownloading = response.data.some((m) => m.status.state === 'Downloading');
        setPollingActive(hasDownloading);
      }
    } catch (error) {
      console.error('Failed to load local models:', error);
    }
  }, []);

  // Poll for updates while downloads are active
  useEffect(() => {
    if (!pollingActive) return;

    const interval = setInterval(() => {
      loadLocalModels();
    }, 500);

    return () => clearInterval(interval);
  }, [pollingActive, loadLocalModels]);

  // Load selected local model
  const loadSelectedLocalModel = useCallback(async () => {
    try {
      const value = await read(LOCAL_LLM_MODEL_CONFIG_KEY, false);
      if (value && typeof value === 'string') {
        setSelectedLocalModelId(value);
      }
    } catch (error) {
      console.error('Failed to load selected local model:', error);
    }
  }, [read]);

  // Determine active provider
  useEffect(() => {
    if (currentProvider === 'local') {
      setActiveProvider('local');
    } else if (currentProvider) {
      setActiveProvider('cloud');
    } else {
      setActiveProvider(null);
    }
  }, [currentProvider]);

  useEffect(() => {
    loadCloudModelInfo();
    loadLocalModels();
    loadSelectedLocalModel();
  }, [loadCloudModelInfo, loadLocalModels, loadSelectedLocalModel]);

  // Select local model
  const selectLocalModel = async (modelId: string) => {
    await upsert(LOCAL_LLM_MODEL_CONFIG_KEY, modelId, false);
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    setSelectedLocalModelId(modelId);
    setActiveProvider('local');
  };

  // Start download
  const startDownload = async (model: LocalModelResponse) => {
    try {
      const spec = `${model.repo_id}:${model.quantization}`;
      await downloadHfModel({ body: { spec } });
      setPollingActive(true);
      loadLocalModels();
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  // Cancel download
  const cancelDownload = async (modelId: string) => {
    try {
      await cancelLocalModelDownload({ path: { model_id: modelId } });
      loadLocalModels();
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  // Delete model
  const handleDeleteModel = async (modelId: string) => {
    try {
      await deleteLocalModel({ path: { model_id: modelId } });
      // Clear selection if we deleted the selected model
      if (selectedLocalModelId === modelId) {
        setSelectedLocalModelId(null);
      }
      loadLocalModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
    }
  };

  // Get selected local model info
  const selectedLocalModel = localModels.find(
    (m) => m.id === selectedLocalModelId && isDownloaded(m)
  );

  // Separate models
  const recommendedModels = localModels.filter((m) => m.recommended);
  const downloadedModels = localModels.filter((m) => isDownloaded(m));

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h3 className="text-lg font-semibold">Model Configuration</h3>
        <p className="text-sm text-muted-foreground">
          Choose between cloud-based or local models for inference
        </p>
      </div>

      {/* Model Cards */}
      <div className="grid grid-cols-2 gap-4">
        {/* Cloud Card */}
        {(filter === 'all' || filter === 'cloud') && (
          <Card
            className={`cursor-pointer transition-all ${
              activeProvider === 'cloud' ? 'ring-2 ring-primary' : ''
            }`}
          >
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Cloud className="w-5 h-5" />
                  <CardTitle className="text-base">Cloud</CardTitle>
                </div>
                {activeProvider === 'cloud' && (
                  <span className="text-xs bg-primary text-primary-foreground px-2 py-0.5 rounded">
                    Active
                  </span>
                )}
              </div>
              <CardDescription className="text-xs">Use API-based models</CardDescription>
            </CardHeader>
            <CardContent>
              {cloudProvider ? (
                <div className="space-y-2">
                  <p className="text-sm font-medium">{cloudProvider}</p>
                  <p className="text-xs text-muted-foreground truncate">{cloudModel}</p>
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full mt-2"
                    onClick={() => setView('ConfigureProviders')}
                  >
                    <Settings2 className="w-4 h-4 mr-2" />
                    Configure
                  </Button>
                </div>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setView('ConfigureProviders')}
                >
                  Set up Cloud Provider
                </Button>
              )}
            </CardContent>
          </Card>
        )}

        {/* Local Card */}
        {(filter === 'all' || filter === 'local') && (
          <Card
            className={`cursor-pointer transition-all ${
              activeProvider === 'local' ? 'ring-2 ring-primary' : ''
            }`}
          >
            <CardHeader className="pb-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <HardDrive className="w-5 h-5" />
                  <CardTitle className="text-base">Local</CardTitle>
                </div>
                {activeProvider === 'local' && (
                  <span className="text-xs bg-primary text-primary-foreground px-2 py-0.5 rounded">
                    Active
                  </span>
                )}
              </div>
              <CardDescription className="text-xs">Run models on your machine</CardDescription>
            </CardHeader>
            <CardContent>
              {selectedLocalModel ? (
                <div className="space-y-2">
                  <p className="text-sm font-medium">{selectedLocalModel.display_name}</p>
                  <p className="text-xs text-muted-foreground">
                    {formatBytes(selectedLocalModel.size_bytes)} •{' '}
                    {selectedLocalModel.context_limit?.toLocaleString() ?? 'N/A'} ctx
                  </p>
                  <Button
                    variant="outline"
                    size="sm"
                    className="w-full mt-2"
                    onClick={() => setShowHuggingFaceModal(true)}
                  >
                    <Settings2 className="w-4 h-4 mr-2" />
                    Browse Models
                  </Button>
                </div>
              ) : downloadedModels.length > 0 ? (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setShowHuggingFaceModal(true)}
                >
                  Select Local Model
                </Button>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setShowHuggingFaceModal(true)}
                >
                  Download Model
                </Button>
              )}
            </CardContent>
          </Card>
        )}
      </div>

      {/* Filter Pills */}
      <div className="flex gap-2">
        {(['all', 'cloud', 'local'] as FilterType[]).map((f) => (
          <Button
            key={f}
            variant={filter === f ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter(f)}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </Button>
        ))}
      </div>

      {/* Local Models Section */}
      {(filter === 'all' || filter === 'local') && (
        <div className="space-y-4">
          {/* Downloading Models */}
          {localModels.filter((m) => isDownloading(m)).length > 0 && (
            <div>
              <h4 className="text-sm font-medium mb-2">Downloading</h4>
              <div className="space-y-2">
                {localModels
                  .filter((m) => isDownloading(m))
                  .map((model) => {
                    const progress = getDownloadProgress(model);
                    if (!progress) return null;
                    return (
                      <div
                        key={model.id}
                        className="flex items-center justify-between p-3 border rounded-lg"
                      >
                        <div className="flex-1">
                          <p className="text-sm font-medium">{model.display_name}</p>
                          <div className="mt-1 w-full">
                            <div className="w-full bg-muted rounded-full h-1.5">
                              <div
                                className="bg-primary h-1.5 rounded-full transition-all"
                                style={{ width: `${progress.progress_percent}%` }}
                              />
                            </div>
                            <p className="text-xs text-muted-foreground mt-0.5">
                              {progress.progress_percent.toFixed(0)}% •{' '}
                              {formatBytes(progress.bytes_downloaded)} /{' '}
                              {formatBytes(progress.total_bytes)}
                            </p>
                          </div>
                        </div>
                        <Button variant="ghost" size="sm" onClick={() => cancelDownload(model.id)}>
                          <X className="w-4 h-4" />
                        </Button>
                      </div>
                    );
                  })}
              </div>
            </div>
          )}

          {/* Downloaded Models */}
          {downloadedModels.length > 0 && (
            <div>
              <h4 className="text-sm font-medium mb-2">Downloaded Models</h4>
              <div className="grid grid-cols-3 gap-3">
                {downloadedModels.map((model) => {
                  const isSelected = selectedLocalModelId === model.id;
                  const originalProvider = getOriginalProvider(model.display_name);
                  const providerAvatarUrl = originalProvider
                    ? PROVIDER_AVATARS[originalProvider]
                    : null;

                  return (
                    <div
                      key={model.id}
                      className={`p-3 border rounded-lg cursor-pointer hover:bg-accent/50 ${
                        isSelected ? 'ring-2 ring-primary' : ''
                      }`}
                      onClick={() => selectLocalModel(model.id)}
                    >
                      <div className="flex items-center justify-between mb-2">
                        {providerAvatarUrl ? (
                          <img
                            src={providerAvatarUrl}
                            alt={originalProvider || 'Provider'}
                            className="w-8 h-8 rounded-full"
                          />
                        ) : (
                          <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
                            <HardDrive className="w-4 h-4" />
                          </div>
                        )}
                        <div className="flex items-center gap-1">
                          {isSelected && <Check className="w-4 h-4 text-green-600" />}
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-6 w-6 p-0 text-muted-foreground hover:text-destructive"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDeleteModel(model.id);
                            }}
                          >
                            <Trash2 className="w-4 h-4" />
                          </Button>
                        </div>
                      </div>
                      <p className="text-sm font-medium">{model.display_name}</p>
                      <p className="text-xs text-muted-foreground">
                        {formatBytes(model.size_bytes)} •{' '}
                        {model.context_limit?.toLocaleString() ?? 'N/A'} ctx
                      </p>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Recommended Models */}
          <div>
            <h4 className="text-sm font-medium mb-2">Recommended Models</h4>
            <div className="grid grid-cols-2 gap-3">
              {recommendedModels.map((model) => {
                const downloaded = isDownloaded(model);
                const downloading = isDownloading(model);
                const progress = getDownloadProgress(model);
                const originalProvider = getOriginalProvider(model.display_name);
                const providerAvatarUrl = originalProvider
                  ? PROVIDER_AVATARS[originalProvider]
                  : null;

                return (
                  <div
                    key={model.id}
                    className={`p-3 border rounded-lg ${
                      downloaded ? 'cursor-pointer hover:bg-accent/50' : ''
                    }`}
                    onClick={() => downloaded && selectLocalModel(model.id)}
                  >
                    <div className="flex items-center justify-between mb-2">
                      {providerAvatarUrl ? (
                        <img
                          src={providerAvatarUrl}
                          alt={originalProvider || 'Provider'}
                          className="w-8 h-8 rounded-full"
                        />
                      ) : (
                        <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
                          <HardDrive className="w-4 h-4" />
                        </div>
                      )}
                      {downloaded ? (
                        <Check className="w-4 h-4 text-green-600" />
                      ) : downloading ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 w-8 p-0"
                          onClick={(e) => {
                            e.stopPropagation();
                            cancelDownload(model.id);
                          }}
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 w-8 p-0"
                          onClick={(e) => {
                            e.stopPropagation();
                            startDownload(model);
                          }}
                        >
                          <Download className="w-4 h-4" />
                        </Button>
                      )}
                    </div>
                    <p className="text-sm font-medium">{model.display_name}</p>
                    <p className="text-xs text-muted-foreground">
                      {formatBytes(model.size_bytes)} •{' '}
                      {model.context_limit?.toLocaleString() ?? 'N/A'} ctx
                    </p>
                    {downloading && progress && (
                      <div className="mt-2">
                        <div className="w-full bg-muted rounded-full h-1.5">
                          <div
                            className="bg-primary h-1.5 rounded-full transition-all"
                            style={{ width: `${progress.progress_percent}%` }}
                          />
                        </div>
                        <p className="text-xs text-muted-foreground mt-0.5">
                          {progress.progress_percent.toFixed(0)}%
                        </p>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Browse HuggingFace Button */}
          <Button
            variant="outline"
            className="w-full"
            onClick={() => setShowHuggingFaceModal(true)}
          >
            Browse HuggingFace Models
          </Button>
        </div>
      )}

      {/* Cloud section empty state */}
      {filter === 'cloud' && !cloudProvider && (
        <div className="text-center py-6 text-muted-foreground text-sm">
          No cloud provider configured.{' '}
          <button className="text-primary underline" onClick={() => setView('ConfigureProviders')}>
            Set up a provider
          </button>
        </div>
      )}

      {/* Local section empty state */}
      {filter === 'local' && localModels.length === 0 && (
        <div className="text-center py-6 text-muted-foreground text-sm">
          No local models available.{' '}
          <button className="text-primary underline" onClick={() => setShowHuggingFaceModal(true)}>
            Browse models
          </button>
        </div>
      )}

      {/* Reset Provider Section */}
      <ResetProviderSection setView={setView} />

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
    </div>
  );
}
