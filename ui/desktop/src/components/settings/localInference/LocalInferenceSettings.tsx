import { useState, useEffect, useCallback, useRef } from 'react';
import { Download, Trash2, X, Check, Settings2 } from 'lucide-react';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import {
  listLocalModels,
  downloadHfModel,
  cancelLocalModelDownload,
  deleteLocalModel,
  type LocalModelResponse,
} from '../../../api';
import { HuggingFaceModelSearch, AuthorAvatar } from './HuggingFaceModelSearch';
import { ModelSettingsPanel } from './ModelSettingsPanel';

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

// Extract author from repo_id like "bartowski/Llama-3.2-1B-Instruct-GGUF"
const extractAuthorFromRepoId = (repoId: string): string | null => {
  const parts = repoId.split('/');
  return parts.length > 0 ? parts[0] : null;
};

const LOCAL_LLM_MODEL_CONFIG_KEY = 'LOCAL_LLM_MODEL';

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

export const LocalInferenceSettings = () => {
  const [models, setModels] = useState<LocalModelResponse[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [settingsOpenFor, setSettingsOpenFor] = useState<string | null>(null);
  const [pollingActive, setPollingActive] = useState(false);
  const { read, upsert } = useConfig();
  const downloadSectionRef = useRef<HTMLDivElement>(null);

  const loadModels = useCallback(async () => {
    try {
      const response = await listLocalModels();
      if (response.data) {
        setModels(response.data);
        // Check if any models are downloading
        const hasDownloading = response.data.some((m) => m.status.state === 'Downloading');
        setPollingActive(hasDownloading);
      }
    } catch (error) {
      console.error('Failed to load models:', error);
    }
  }, []);

  // Poll for updates while downloads are active
  useEffect(() => {
    if (!pollingActive) return;

    const interval = setInterval(() => {
      loadModels();
    }, 500);

    return () => clearInterval(interval);
  }, [pollingActive, loadModels]);

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

  const startDownload = async (model: LocalModelResponse) => {
    try {
      // Use the spec format: repo_id:quantization
      const spec = `${model.repo_id}:${model.quantization}`;
      await downloadHfModel({ body: { spec } });
      setPollingActive(true);
      scrollToDownloads();
      loadModels();
    } catch (error) {
      console.error('Failed to start download:', error);
    }
  };

  const scrollToDownloads = useCallback(() => {
    requestAnimationFrame(() => {
      downloadSectionRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
  }, []);

  const cancelDownload = async (modelId: string) => {
    try {
      await cancelLocalModelDownload({ path: { model_id: modelId } });
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

  const handleHfDownloadStarted = (_modelId: string) => {
    setPollingActive(true);
    scrollToDownloads();
    loadModels();
  };

  // Separate models into categories
  const recommendedModels = models.filter((m) => m.recommended);
  const downloadedModels = models.filter((m) => isDownloaded(m));
  const downloadingModels = models.filter((m) => isDownloading(m));

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-text-default font-medium">Local Inference Models</h3>
        <p className="text-xs text-text-muted max-w-2xl mt-1">
          Download and manage local LLM models for inference without API keys. Search HuggingFace
          for any GGUF model or use the recommended picks below.
        </p>
      </div>

      {/* Active Downloads */}
      {downloadingModels.length > 0 && (
        <div ref={downloadSectionRef}>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloading</h4>
          <div className="space-y-2">
            {downloadingModels.map((model) => {
              const progress = getDownloadProgress(model);
              if (!progress) return null;
              return (
                <div
                  key={model.id}
                  className="border rounded-lg p-3 border-border-subtle bg-background-default"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-text-default truncate">
                      {model.display_name}
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => cancelDownload(model.id)}
                      className="text-destructive hover:text-destructive"
                    >
                      <X className="w-4 h-4" />
                    </Button>
                  </div>
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
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Downloaded Models */}
      {downloadedModels.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-2">Downloaded Models</h4>
          <div className="space-y-2">
            {downloadedModels.map((model) => {
              const isSelected = selectedModelId === model.id;
              const showSettings = settingsOpenFor === model.id;
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
                      <span className="text-sm font-medium text-text-default">
                        {model.display_name}
                      </span>
                      <span className="text-xs text-text-muted">
                        {formatBytes(model.size_bytes)}
                      </span>
                      {model.recommended && (
                        <span className="text-xs bg-blue-500 text-white px-2 py-0.5 rounded">
                          Recommended
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setSettingsOpenFor(showSettings ? null : model.id)}
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
                  {showSettings && <ModelSettingsPanel modelId={model.id} />}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Recommended Models */}
      <div>
        <h4 className="text-sm font-medium text-text-default mb-2">Recommended Models</h4>
        <div className="grid grid-cols-2 gap-4">
          {recommendedModels.map((model) => {
            const downloading = isDownloading(model);
            const downloaded = isDownloaded(model);
            const progress = getDownloadProgress(model);
            const author = extractAuthorFromRepoId(model.repo_id);
            const originalProvider = getOriginalProvider(model.display_name);
            const providerAvatarUrl = originalProvider ? PROVIDER_AVATARS[originalProvider] : null;

            return (
              <div key={model.id} className="relative">
                <div className="border rounded-lg p-4 border-border-subtle bg-background-default hover:border-border-default flex flex-col h-full">
                  {/* Row 1: Avatar left, Download button right */}
                  <div className="flex items-center justify-between mb-3">
                    {providerAvatarUrl ? (
                      <img
                        src={providerAvatarUrl}
                        alt={originalProvider || 'Provider'}
                        className="w-10 h-10 rounded-full object-cover"
                      />
                    ) : author ? (
                      <AuthorAvatar author={author} size={40} />
                    ) : (
                      <div className="w-10 h-10" />
                    )}
                    <div className="flex items-center gap-1">
                      {downloaded ? (
                        <div className="flex items-center text-green-600">
                          <Check className="w-5 h-5" />
                        </div>
                      ) : downloading ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => cancelDownload(model.id)}
                          className="h-8 w-8 p-0"
                        >
                          <X className="w-4 h-4" />
                        </Button>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => startDownload(model)}
                          className="h-8 w-8 p-0"
                          title="Download model"
                        >
                          <Download className="w-4 h-4" />
                        </Button>
                      )}
                    </div>
                  </div>

                  {/* Row 2: Title */}
                  <h4 className="text-sm font-medium text-text-default">{model.display_name}</h4>

                  {/* Row 3: Author */}
                  <p className="text-xs text-text-muted mt-0.5">
                    {originalProvider || author || 'Unknown'}
                  </p>

                  {/* Row 4: Size & Context */}
                  <p className="text-xs text-text-muted mt-0.5">
                    {formatBytes(model.size_bytes)} •{' '}
                    {model.context_limit?.toLocaleString() ?? 'N/A'} ctx
                  </p>

                  {/* Download progress */}
                  {downloading && progress && (
                    <div className="mt-3 space-y-1">
                      <div className="w-full bg-background-subtle rounded-full h-1.5">
                        <div
                          className="bg-accent-primary h-1.5 rounded-full transition-all"
                          style={{ width: `${progress.progress_percent}%` }}
                        />
                      </div>
                      <div className="flex justify-between text-xs text-text-muted">
                        <span>{progress.progress_percent.toFixed(0)}%</span>
                        {progress.speed_bps && <span>{formatBytes(progress.speed_bps)}/s</span>}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* HuggingFace Search */}
      <div className="border-t border-border-subtle pt-4">
        <HuggingFaceModelSearch onDownloadStarted={handleHfDownloadStarted} />
      </div>

      {models.length === 0 && (
        <div className="text-center py-6 text-text-muted text-sm">No models available</div>
      )}
    </div>
  );
};
