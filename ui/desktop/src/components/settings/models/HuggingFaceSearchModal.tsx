import { useState, useCallback, useRef } from 'react';
import { Search, Download, ChevronDown, ChevronUp, Loader2, Star, X, MessageSquare, Code, MessagesSquare, FileText, Brain, Zap } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';
import { Button } from '../../ui/button';
import {
  searchHfModels,
  getRepoFiles,
  downloadHfModel,
  type HfModelInfo,
  type HfQuantVariant,
} from '../../../api';
import { AuthorAvatar } from '../localInference/HuggingFaceModelSearch';

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return 'unknown';
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)}MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)}GB`;
};

const formatDownloads = (n: number): string => {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
};

interface RepoData {
  variants: HfQuantVariant[];
  recommendedIndex: number | null;
}

interface HuggingFaceSearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onDownloadStarted: (modelId: string) => void;
}

export function HuggingFaceSearchModal({ isOpen, onClose, onDownloadStarted }: HuggingFaceSearchModalProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<HfModelInfo[]>([]);
  const [expandedRepo, setExpandedRepo] = useState<string | null>(null);
  const [repoData, setRepoData] = useState<Record<string, RepoData>>({});
  const [searching, setSearching] = useState(false);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [loadingFiles, setLoadingFiles] = useState<Set<string>>(new Set());
  const [directSpec, setDirectSpec] = useState('');
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const doSearch = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      setError(null);
      return;
    }
    setSearching(true);
    setError(null);
    try {
      const response = await searchHfModels({
        query: { q, limit: 20 },
      });
      if (response.data) {
        setResults(response.data);
        if (response.data.length === 0) {
          setError('No GGUF models found for this query.');
        }
      } else {
        console.error('Search response:', response);
        const errMsg = response.error
          ? `Search error: ${JSON.stringify(response.error)}`
          : 'Search returned no data.';
        setError(errMsg);
      }
    } catch (e) {
      console.error('Search failed:', e);
      setError('Search failed. Please try again.');
    } finally {
      setSearching(false);
    }
  }, []);

  const handleQueryChange = (value: string) => {
    setQuery(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(value), 300);
  };

  const toggleRepo = async (repoId: string) => {
    if (expandedRepo === repoId) {
      setExpandedRepo(null);
      return;
    }
    setExpandedRepo(repoId);

    if (!repoData[repoId]?.variants.length) {
      setLoadingFiles((prev) => new Set(prev).add(repoId));
      try {
        const [author, repo] = repoId.split('/');
        const response = await getRepoFiles({
          path: { author, repo },
        });
        if (response.data) {
          const variants = response.data.variants;
          setRepoData((prev) => ({
            ...prev,
            [repoId]: {
              variants,
              recommendedIndex: response.data!.recommended_index ?? null,
            },
          }));
          if (variants.length === 0) {
            setExpandedRepo(null);
            setResults((prev) => prev.filter((m) => m.repo_id !== repoId));
          }
        }
      } catch (e) {
        console.error('Failed to fetch repo files:', e);
      } finally {
        setLoadingFiles((prev) => {
          const next = new Set(prev);
          next.delete(repoId);
          return next;
        });
      }
    }
  };

  const startDownload = async (repoId: string, filename: string) => {
    const key = `${repoId}/${filename}`;
    setDownloading((prev) => new Set(prev).add(key));
    try {
      const response = await downloadHfModel({
        body: { repo_id: repoId, filename },
      });
      if (response.data) {
        onDownloadStarted(response.data.model_id);
      } else {
        console.error('Download error:', response.error);
      }
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  const startDirectDownload = async () => {
    if (!directSpec.trim()) return;
    const key = `direct:${directSpec}`;
    setDownloading((prev) => new Set(prev).add(key));
    try {
      const response = await downloadHfModel({
        body: { spec: directSpec.trim() },
      });
      if (response.data) {
        onDownloadStarted(response.data.model_id);
        setDirectSpec('');
      }
    } catch (e) {
      console.error('Direct download failed:', e);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }
  };

  // Provider avatar URLs
  const PROVIDER_AVATARS: Record<string, string> = {
    'meta': 'https://cdn-avatars.huggingface.co/v1/production/uploads/646cf8084eefb026fb8fd8bc/oCTqufkdTkjyGodsx1vo1.png',
    'mistral': 'https://cdn-avatars.huggingface.co/v1/production/uploads/634c17653d11eaedd88b314d/9OgyfKstSZtbmsmuG8MbU.png',
    'microsoft': 'https://cdn-avatars.huggingface.co/v1/production/uploads/1583646260758-5e64858c87403103f9f1055d.png',
    'qwen': 'https://cdn-avatars.huggingface.co/v1/production/uploads/620760a26e3b7210c2ff1943/-s1gyJfvbE1RgO5iBeNOi.png',
    'google': 'https://cdn-avatars.huggingface.co/v1/production/uploads/5dd96eb166059660ed1ee413/WtA3YYitedOr9n02eHfJe.png',
    'deepseek': 'https://cdn-avatars.huggingface.co/v1/production/uploads/6538815d1bdb3c40db94fbfa/xMBly9PUMphrFVMxLX4kq.png',
  };

  // Popular search suggestions
  const popularSearches = [
    { label: 'Llama 3.2', query: 'llama-3.2', provider: 'meta' },
    { label: 'Mistral', query: 'mistral', provider: 'mistral' },
    { label: 'Phi', query: 'phi', provider: 'microsoft' },
    { label: 'Qwen', query: 'qwen', provider: 'qwen' },
    { label: 'Gemma', query: 'gemma', provider: 'google' },
    { label: 'DeepSeek', query: 'deepseek', provider: 'deepseek' },
  ];

  const handleSuggestionClick = (searchQuery: string) => {
    setQuery(searchQuery);
    doSearch(searchQuery);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="!fixed !inset-0 !top-0 !left-0 !translate-x-0 !translate-y-0 !w-screen !h-screen !max-w-none !max-h-none !m-0 !p-0 !rounded-none !border-none flex flex-col z-[100]">
        {/* Header - extra top padding to avoid macOS stoplight buttons */}
        <div className="flex items-center justify-between px-6 pt-10 pb-4 border-b border-border-subtle bg-background-default">
          <DialogHeader className="p-0 space-y-0">
            <DialogTitle className="flex items-center gap-2">
              <Search size={24} className="text-blue-500" />
              Search HuggingFace
            </DialogTitle>
          </DialogHeader>
        </div>

        <div className="flex-1 overflow-hidden flex">
          {/* Left Sidebar - Popular Models, Categories, Direct Download */}
          <div className="w-80 flex-shrink-0 border-r border-border-subtle overflow-y-auto p-6 space-y-6">
            {/* Search Input */}
            <div>
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
                <input
                  type="text"
                  value={query}
                  onChange={(e) => handleQueryChange(e.target.value)}
                  placeholder="Search for GGUF models..."
                  className="w-full pl-9 pr-4 py-2 text-sm border border-border-subtle rounded-lg bg-background-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
                  autoFocus
                />
                {searching && (
                  <Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted animate-spin" />
                )}
              </div>
            </div>

            {/* Popular Models */}
            <div>
              <h3 className="text-sm font-medium text-text-default mb-3">Popular Models</h3>
              <div className="flex flex-col gap-1">
                {popularSearches.map((item) => (
                  <button
                    key={item.query}
                    onClick={() => handleSuggestionClick(item.query)}
                    className="flex items-center gap-2 px-2 py-2 text-sm font-medium rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                  >
                    <img
                      src={PROVIDER_AVATARS[item.provider]}
                      alt={item.provider}
                      className="w-6 h-6 rounded-full object-cover"
                    />
                    {item.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Tasks */}
            <div>
              <h3 className="text-sm font-medium text-text-default mb-3">Tasks</h3>
              <div className="flex flex-col gap-1">
                <button
                  onClick={() => handleSuggestionClick('text-generation')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <MessageSquare className="w-4 h-4" />
                  Text Generation
                </button>
                <button
                  onClick={() => handleSuggestionClick('code')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <Code className="w-4 h-4" />
                  Code
                </button>
                <button
                  onClick={() => handleSuggestionClick('chat')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <MessagesSquare className="w-4 h-4" />
                  Chat
                </button>
                <button
                  onClick={() => handleSuggestionClick('instruct')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <FileText className="w-4 h-4" />
                  Instruct
                </button>
                <button
                  onClick={() => handleSuggestionClick('reasoning')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <Brain className="w-4 h-4" />
                  Reasoning
                </button>
                <button
                  onClick={() => handleSuggestionClick('small')}
                  className="flex items-center gap-2 px-2 py-2 text-sm rounded-lg text-text-default hover:bg-blue-500/10 hover:text-blue-600 transition-colors text-left"
                >
                  <Zap className="w-4 h-4" />
                  Small & Fast
                </button>
              </div>
            </div>

            {/* Direct Download Section */}
            <div className="border-t border-border-subtle pt-4">
              <h4 className="text-sm font-medium text-text-default mb-2">Direct Download</h4>
              <p className="text-xs text-text-muted mb-2">
                Specify a model directly:
              </p>
              <div className="space-y-2">
                <input
                  type="text"
                  value={directSpec}
                  onChange={(e) => setDirectSpec(e.target.value)}
                  placeholder="user/repo:quantization"
                  className="w-full px-3 py-2 text-sm border border-border-subtle rounded-lg bg-background-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') startDirectDownload();
                  }}
                />
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  disabled={!directSpec.trim() || downloading.has(`direct:${directSpec}`)}
                  onClick={startDirectDownload}
                >
                  {downloading.has(`direct:${directSpec}`) ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <>
                      <Download className="w-4 h-4 mr-1" />
                      Download
                    </>
                  )}
                </Button>
              </div>
            </div>
          </div>

          {/* Right Side - Search Results */}
          <div className="flex-1 overflow-y-auto p-6">
            {/* Error Message */}
            {error && !searching && (
              <p className="text-sm text-text-muted mb-4">{error}</p>
            )}

            {/* Empty State - Show Featured Models */}
            {!query && results.length === 0 && !searching && (
              <div className="space-y-4">
                <div>
                  <h3 className="text-sm font-medium text-text-default mb-1">Featured Models</h3>
                  <p className="text-xs text-text-muted mb-4">Popular models ready to download</p>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  {popularSearches.map((item) => (
                    <button
                      key={item.query}
                      onClick={() => handleSuggestionClick(item.query)}
                      className="flex items-start gap-3 p-4 text-left rounded-lg border border-border-subtle bg-background-default hover:border-blue-500/50 hover:bg-blue-500/5 transition-colors"
                    >
                      <img
                        src={PROVIDER_AVATARS[item.provider]}
                        alt={item.provider}
                        className="w-10 h-10 rounded-full object-cover flex-shrink-0"
                      />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-text-default">{item.label}</p>
                        <p className="text-xs text-text-muted mt-0.5">
                          {item.provider === 'meta' && 'Meta AI'}
                          {item.provider === 'mistral' && 'Mistral AI'}
                          {item.provider === 'microsoft' && 'Microsoft'}
                          {item.provider === 'qwen' && 'Alibaba Cloud'}
                          {item.provider === 'google' && 'Google'}
                          {item.provider === 'deepseek' && 'DeepSeek AI'}
                        </p>
                        <p className="text-xs text-blue-500 mt-1">Browse models →</p>
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Searching State */}
            {searching && results.length === 0 && (
              <div className="flex flex-col items-center justify-center h-full text-center">
                <Loader2 className="w-8 h-8 text-text-muted mb-4 animate-spin" />
                <p className="text-sm text-text-muted">Searching HuggingFace...</p>
              </div>
            )}

            {/* Search Results */}
            {results.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs text-text-muted mb-3">{results.length} models found</p>
                {results.map((model) => {
                  const isExpanded = expandedRepo === model.repo_id;
                  const data = repoData[model.repo_id];
                  const variants = data?.variants || [];
                  const recommendedIndex = data?.recommendedIndex ?? null;

                  return (
                    <div key={model.repo_id} className="border border-border-subtle rounded-lg bg-background-default">
                      <button
                        onClick={() => toggleRepo(model.repo_id)}
                        className="w-full flex items-center justify-between p-3 text-left hover:bg-background-subtle rounded-lg transition-colors"
                      >
                        <div className="flex items-center gap-3 flex-1 min-w-0">
                          <AuthorAvatar author={model.author} size={40} />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <span className="text-sm font-medium text-text-default truncate">
                                {model.model_name}
                              </span>
                            </div>
                            <div className="flex items-center gap-2 mt-0.5">
                              <span className="text-xs text-text-muted">{model.author}</span>
                              <span className="text-xs text-text-muted">•</span>
                              <span className="text-xs text-text-muted">
                                ↓ {formatDownloads(model.downloads)}
                              </span>
                            </div>
                          </div>
                        </div>
                        {isExpanded ? (
                          <ChevronUp className="w-4 h-4 text-text-muted flex-shrink-0" />
                        ) : (
                          <ChevronDown className="w-4 h-4 text-text-muted flex-shrink-0" />
                        )}
                      </button>

                      {isExpanded && (
                        <div className="border-t border-border-subtle px-3 pb-3 space-y-1">
                          {loadingFiles.has(model.repo_id) && (
                            <div className="flex items-center gap-2 py-2 text-xs text-text-muted">
                              <Loader2 className="w-3 h-3 animate-spin" />
                              Loading variants...
                            </div>
                          )}
                          {variants.map((variant, idx) => {
                            const dlKey = `${model.repo_id}/${variant.filename}`;
                            const isStarting = downloading.has(dlKey);
                            const isRecommended = idx === recommendedIndex;

                            return (
                              <div
                                key={variant.quantization}
                                className={`flex items-center justify-between py-2 px-2 rounded ${
                                  isRecommended
                                    ? 'bg-blue-500/5 border border-blue-500/20'
                                    : 'hover:bg-background-subtle'
                                }`}
                              >
                                <div className="flex flex-col gap-0.5 min-w-0 flex-1 mr-3">
                                  <div className="flex items-center gap-2">
                                    <span className="text-xs font-mono font-medium text-text-default">
                                      {variant.quantization}
                                    </span>
                                    <span className="text-xs text-text-muted">
                                      {formatBytes(variant.size_bytes)}
                                    </span>
                                    {isRecommended && (
                                      <span className="inline-flex items-center gap-1 text-xs bg-blue-500 text-white px-1.5 py-0.5 rounded">
                                        <Star className="w-3 h-3" />
                                        Recommended
                                      </span>
                                    )}
                                  </div>
                                  {variant.description && (
                                    <span className="text-xs text-text-muted">
                                      {variant.description}
                                    </span>
                                  )}
                                </div>
                                <Button
                                  variant="outline"
                                  size="sm"
                                  disabled={isStarting}
                                  onClick={() => startDownload(model.repo_id, variant.filename)}
                                >
                                  {isStarting ? (
                                    <Loader2 className="w-3 h-3 animate-spin" />
                                  ) : (
                                    <>
                                      <Download className="w-3 h-3 mr-1" />
                                      Download
                                    </>
                                  )}
                                </Button>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
