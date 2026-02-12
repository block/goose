import { useState, useEffect, useCallback } from 'react';
import { Cloud, HardDrive, Download, Check, Settings2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { useConfig } from '../../ConfigContext';
import { View } from '../../../utils/navigationUtils';
import { useModelAndProvider } from '../../ModelAndProviderContext';
import {
  listLocalModels,
  downloadLocalModel,
  getLocalModelDownloadProgress,
  cancelLocalModelDownload,
  type DownloadProgress,
  type LocalModelResponse,
  type ModelListItem,
} from '../../../api';
import { LocalModelModal } from './LocalModelModal';
import ResetProviderSection from '../reset_provider/ResetProviderSection';

type FilterType = 'all' | 'cloud' | 'local';

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
const LAST_CLOUD_PROVIDER_KEY = 'LAST_CLOUD_PROVIDER';
const LAST_CLOUD_MODEL_KEY = 'LAST_CLOUD_MODEL';

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

function isFeaturedModel(item: ModelListItem): item is LocalModelResponse & { featured: boolean } {
  return 'tier' in item;
}

interface UnifiedModelSectionProps {
  setView: (view: View) => void;
}

export default function UnifiedModelSection({ setView }: UnifiedModelSectionProps) {
  const [featuredModels, setFeaturedModels] = useState<(LocalModelResponse & { featured?: boolean })[]>([]);
  const [selectedLocalModelId, setSelectedLocalModelId] = useState<string | null>(null);
  const [downloads, setDownloads] = useState<Map<string, DownloadProgress>>(new Map());
  const [activeProvider, setActiveProvider] = useState<'cloud' | 'local' | null>(null);
  const [showLocalModelModal, setShowLocalModelModal] = useState(false);
  const [filter, setFilter] = useState<FilterType>('all');
  
  const { read, upsert } = useConfig();
  const { 
    currentModel, 
    currentProvider,
  } = useModelAndProvider();
  
  const [cloudModel, setCloudModel] = useState<string>('');
  const [cloudProvider, setCloudProvider] = useState<string>('');

  // Load cloud model info - we need to read the stored cloud config, not the current active model
  const loadCloudModelInfo = useCallback(async () => {
    try {
      // First check if current provider is cloud - if so, use current values
      if (currentProvider && currentProvider !== 'local') {
        setCloudProvider(currentProvider);
        if (currentModel) {
          setCloudModel(currentModel);
          // Also save these as the last known cloud settings
          await upsert(LAST_CLOUD_PROVIDER_KEY, currentProvider, false);
          await upsert(LAST_CLOUD_MODEL_KEY, currentModel, false);
        }
      } else {
        // Current provider is local, try to load the last known cloud settings
        const lastCloudProvider = await read(LAST_CLOUD_PROVIDER_KEY, false);
        const lastCloudModel = await read(LAST_CLOUD_MODEL_KEY, false);
        
        if (lastCloudProvider && typeof lastCloudProvider === 'string') {
          setCloudProvider(lastCloudProvider);
        }
        if (lastCloudModel && typeof lastCloudModel === 'string') {
          setCloudModel(lastCloudModel);
        }
      }
    } catch (error) {
      console.error('Failed to load cloud model info:', error);
    }
  }, [read, upsert, currentProvider, currentModel]);

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
    }
  }, [currentProvider]);

  useEffect(() => {
    loadCloudModelInfo();
    loadLocalModels();
    loadSelectedLocalModel();
  }, [loadCloudModelInfo, loadLocalModels, loadSelectedLocalModel]);

  // Refresh when model changes
  useEffect(() => {
    if (currentModel && currentProvider) {
      loadCloudModelInfo();
    }
  }, [currentModel, currentProvider, loadCloudModelInfo]);

  const selectLocalModel = async (modelId: string) => {
    await upsert(LOCAL_LLM_MODEL_CONFIG_KEY, modelId, false);
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    setSelectedLocalModelId(modelId);
    setActiveProvider('local');
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

  // Get the selected local model details
  const selectedLocalModel = featuredModels.find(m => m.id === selectedLocalModelId && m.downloaded);

  return (
    <section className="space-y-6 pr-4 pb-8 pt-3">
      {/* Cloud and Local Model Cards */}
      <div className="grid grid-cols-2 gap-4">
        {/* Cloud Model Card */}
        <div className="relative">
          {activeProvider === 'cloud' && (
            <div className="absolute -top-2 -right-2 z-20">
              <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
                Active
              </span>
            </div>
          )}
          <div 
            className={`border rounded-lg p-4 flex flex-col h-full transition-all ${
              activeProvider === 'cloud'
                ? 'border-blue-500 bg-blue-500/5'
                : 'border-border-subtle bg-background-default hover:border-border-default'
            }`}
          >
            {/* Row 1: Icon left, Settings button right */}
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center">
                <Cloud className="w-5 h-5 text-blue-500" />
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setView('ConfigureProviders')}
                className="h-8 w-8 p-0"
                title="Configure cloud model"
              >
                <Settings2 className="w-4 h-4" />
              </Button>
            </div>

            {/* Title */}
            <h4 className="text-sm font-medium text-text-default">Cloud</h4>

            {/* Subtitle */}
            <p className="text-xs text-text-muted mt-0.5">API-based inference</p>

            {/* Model info */}
            {cloudModel ? (
              <>
                <p className="text-xs text-text-muted mt-0.5">{cloudProvider}</p>
                <p className="text-xs text-text-muted mt-2 flex-1">{cloudModel}</p>
              </>
            ) : (
              <p className="text-xs text-text-muted mt-2 flex-1">No cloud model selected</p>
            )}
          </div>
        </div>

        {/* Local Model Card */}
        <div className="relative">
          {activeProvider === 'local' && (
            <div className="absolute -top-2 -right-2 z-20">
              <span className="inline-block px-2 py-1 text-xs font-medium bg-green-600 text-white rounded-full">
                Active
              </span>
            </div>
          )}
          <div 
            className={`border rounded-lg p-4 flex flex-col h-full transition-all cursor-pointer ${
              activeProvider === 'local'
                ? 'border-green-500 bg-green-500/5'
                : 'border-border-subtle bg-background-default hover:border-border-default'
            }`}
            onClick={() => {
              if (!selectedLocalModel) {
                // No model downloaded - open modal
                setShowLocalModelModal(true);
              } else if (activeProvider !== 'local') {
                // Model exists but not active - activate it
                selectLocalModel(selectedLocalModel.id);
              }
            }}
          >
            {/* Row 1: Icon left, Settings button right */}
            <div className="flex items-center justify-between mb-3">
              <div className="w-10 h-10 rounded-full bg-green-500/10 flex items-center justify-center">
                <HardDrive className="w-5 h-5 text-green-500" />
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  setShowLocalModelModal(true);
                }}
                className="h-8 w-8 p-0"
                title="Configure local model"
              >
                <Settings2 className="w-4 h-4" />
              </Button>
            </div>

            {/* Title */}
            <h4 className="text-sm font-medium text-text-default">Local</h4>

            {/* Subtitle */}
            <p className="text-xs text-text-muted mt-0.5">On-device inference</p>

            {/* Model info */}
            {selectedLocalModel ? (
              <>
                <p className="text-xs text-text-muted mt-0.5">
                  {selectedLocalModel.size_mb}MB • {selectedLocalModel.context_limit.toLocaleString()} ctx
                </p>
                <p className="text-xs text-text-muted mt-2 flex-1">{selectedLocalModel.name}</p>
              </>
            ) : (
              <p className="text-xs text-text-muted mt-2 flex-1">No local model downloaded</p>
            )}
          </div>
        </div>
      </div>

      {/* Local Model Modal */}
      <LocalModelModal
        isOpen={showLocalModelModal}
        onClose={() => setShowLocalModelModal(false)}
        onModelSelected={(modelId) => {
          setSelectedLocalModelId(modelId);
          setActiveProvider('local');
          loadLocalModels();
        }}
      />

      {/* Models Section with Filter Pills */}
      <div>
        {/* Filter Pills */}
        <div className="flex items-center gap-2 mb-4">
          <button
            onClick={() => setFilter('all')}
            className={`px-3 py-1.5 text-xs font-medium rounded-full transition-colors ${
              filter === 'all'
                ? 'bg-text-default text-background-default'
                : 'bg-background-subtle text-text-muted hover:bg-background-default'
            }`}
          >
            All
          </button>
          <button
            onClick={() => setFilter('cloud')}
            className={`px-3 py-1.5 text-xs font-medium rounded-full transition-colors ${
              filter === 'cloud'
                ? 'bg-blue-600 text-white'
                : 'bg-background-subtle text-text-muted hover:bg-background-default'
            }`}
          >
            Cloud
          </button>
          <button
            onClick={() => setFilter('local')}
            className={`px-3 py-1.5 text-xs font-medium rounded-full transition-colors ${
              filter === 'local'
                ? 'bg-green-600 text-white'
                : 'bg-background-subtle text-text-muted hover:bg-background-default'
            }`}
          >
            Local
          </button>
        </div>

        {/* Models Grid */}
        <div className="grid grid-cols-2 gap-4 pt-2">
          {/* Cloud Model - show when filter is 'all' or 'cloud' */}
          {cloudModel && (filter === 'all' || filter === 'cloud') && (
            <div className="relative">
              {activeProvider === 'cloud' && (
                <div className="absolute -top-2 -right-2 z-20">
                  <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
                    Active
                  </span>
                </div>
              )}
              <div 
                className={`border rounded-lg p-4 flex flex-col h-full transition-all cursor-pointer bg-background-default ${
                  activeProvider === 'cloud'
                    ? 'border-blue-500'
                    : 'border-border-subtle hover:border-border-default'
                }`}
                onClick={async () => {
                  // Activate cloud model - restore the stored cloud provider and model
                  if (cloudProvider) {
                    await upsert('GOOSE_PROVIDER', cloudProvider, false);
                    await upsert('GOOSE_MODEL', cloudModel, false);
                    setActiveProvider('cloud');
                  }
                }}
              >
                {/* Row 1: Icon left */}
                <div className="flex items-center justify-between mb-3">
                  <div className="w-10 h-10 rounded-full bg-blue-500/10 flex items-center justify-center">
                    <Cloud className="w-5 h-5 text-blue-500" />
                  </div>
                  {activeProvider === 'cloud' && (
                    <div className="flex items-center text-blue-600">
                      <Check className="w-5 h-5" />
                    </div>
                  )}
                </div>

                {/* Title */}
                <h4 className="text-sm font-medium text-text-default">{cloudModel}</h4>

                {/* Provider */}
                <p className="text-xs text-text-muted mt-0.5">{cloudProvider}</p>

                {/* Type */}
                <p className="text-xs text-text-muted mt-0.5">Cloud • API-based</p>
              </div>
            </div>
          )}

          {/* Local Models - show when filter is 'all' or 'local' */}
          {(filter === 'all' || filter === 'local') && featuredModels.map((model) => {
            const isSelected = selectedLocalModelId === model.id && activeProvider === 'local';
            const originalProvider = getOriginalProvider(model.name);
            const providerAvatarUrl = originalProvider ? PROVIDER_AVATARS[originalProvider] : null;
            const progress = downloads.get(model.id);
            const isDownloading = progress?.status === 'downloading';

            return (
              <div key={model.id} className="relative">
                {/* Badge - Active for selected downloaded, Recommended for undownloaded recommended */}
                {isSelected && (
                  <div className="absolute -top-2 -right-2 z-20">
                    <span className="inline-block px-2 py-1 text-xs font-medium bg-green-600 text-white rounded-full">
                      Active
                    </span>
                  </div>
                )}
                {!model.downloaded && model.recommended && (
                  <div className="absolute -top-2 -right-2 z-20">
                    <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
                      Recommended
                    </span>
                  </div>
                )}

                <div 
                  className={`border rounded-lg p-4 flex flex-col h-full transition-all bg-background-default ${
                    isSelected
                      ? 'border-green-500 cursor-pointer'
                      : model.downloaded
                        ? 'border-border-subtle hover:border-border-default cursor-pointer'
                        : 'border-border-subtle hover:border-border-default'
                  }`}
                  onClick={() => model.downloaded && selectLocalModel(model.id)}
                >
                  {/* Row 1: Avatar left, Action button right */}
                  <div className="flex items-center justify-between mb-3">
                    {providerAvatarUrl ? (
                      <img
                        src={providerAvatarUrl}
                        alt={originalProvider || 'Provider'}
                        className="w-10 h-10 rounded-full object-cover"
                      />
                    ) : (
                      <div className="w-10 h-10 rounded-full bg-green-500/10 flex items-center justify-center">
                        <HardDrive className="w-5 h-5 text-green-500" />
                      </div>
                    )}
                    
                    {/* Action: Check for downloaded, Download/Cancel for not downloaded */}
                    {model.downloaded ? (
                      <div className={`flex items-center ${isSelected ? 'text-green-600' : 'text-text-muted'}`}>
                        <Check className="w-5 h-5" />
                      </div>
                    ) : isDownloading ? (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          cancelDownload(model.id);
                        }}
                        className="h-8 w-8 p-0"
                      >
                        <span className="text-xs">{progress?.progress_percent.toFixed(0)}%</span>
                      </Button>
                    ) : (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          startDownload(model.id);
                        }}
                        className="h-8 w-8 p-0"
                        title="Download model"
                      >
                        <Download className="w-4 h-4" />
                      </Button>
                    )}
                  </div>

                  {/* Title */}
                  <h4 className="text-sm font-medium text-text-default">{model.name}</h4>

                  {/* Author */}
                  <p className="text-xs text-text-muted mt-0.5">
                    {originalProvider || 'Unknown'}
                  </p>

                  {/* Size & Context */}
                  <p className="text-xs text-text-muted mt-0.5">
                    Local • {model.size_mb}MB • {model.context_limit.toLocaleString()} ctx
                  </p>

                  {/* Download progress */}
                  {isDownloading && progress && (
                    <div className="mt-3 space-y-1">
                      <div className="w-full bg-background-subtle rounded-full h-1.5">
                        <div
                          className="bg-green-500 h-1.5 rounded-full transition-all"
                          style={{ width: `${progress.progress_percent}%` }}
                        />
                      </div>
                      <div className="flex justify-between text-xs text-text-muted">
                        <span>{formatBytes(progress.bytes_downloaded)} / {formatBytes(progress.total_bytes)}</span>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* Empty state for cloud filter */}
        {filter === 'cloud' && !cloudModel && (
          <div className="text-center py-8 px-4 bg-background-subtle rounded-lg">
            <Cloud className="w-12 h-12 text-text-muted mx-auto mb-3" />
            <p className="text-sm text-text-muted mb-3">No cloud model configured</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setView('ConfigureProviders')}
            >
              Configure Cloud Provider
            </Button>
          </div>
        )}

        {/* Empty state for local filter */}
        {filter === 'local' && featuredModels.length === 0 && (
          <div className="text-center py-8 px-4 bg-background-subtle rounded-lg">
            <HardDrive className="w-12 h-12 text-text-muted mx-auto mb-3" />
            <p className="text-sm text-text-muted mb-3">No local models available</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowLocalModelModal(true)}
            >
              Browse Local Models
            </Button>
          </div>
        )}
      </div>

      {/* Reset Provider and Model */}
      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle>Reset Provider and Model</CardTitle>
          <CardDescription>
            Clear your selected model and provider settings to start fresh
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ResetProviderSection setView={setView} />
        </CardContent>
      </Card>
    </section>
  );
}
