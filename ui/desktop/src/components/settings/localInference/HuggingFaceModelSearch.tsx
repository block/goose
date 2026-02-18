import { useState, useCallback, useRef } from 'react';
import { Search, Download, ChevronDown, ChevronUp, Loader2, Star } from 'lucide-react';
import { Button } from '../../ui/button';
import {
  searchHfModels,
  getRepoFiles,
  downloadHfModel,
  type HfSearchResult,
  type HfQuantVariant,
} from '../../../api';

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

interface Props {
  onDownloadStarted: (modelId: string) => void;
}

export const HuggingFaceModelSearch = ({ onDownloadStarted }: Props) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<HfSearchResult[]>([]);
  const [expandedRepo, setExpandedRepo] = useState<string | null>(null);
  const [repoData, setRepoData] = useState<Record<string, RepoData>>({});
  const [searching, setSearching] = useState(false);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [loadingFiles, setLoadingFiles] = useState<Set<string>>(new Set());
  const [directSpec, setDirectSpec] = useState('');
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Build repo_id from search result
  const getRepoId = (result: HfSearchResult): string => {
    return `${result.author}/${result.name}`;
  };

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
        query: { q },
      });
      if (response.data) {
        // Pre-fetch variants for all results and filter out repos with no suitable quantizations
        const modelsWithVariants = await Promise.all(
          response.data.map(async (model) => {
            try {
              const repoId = getRepoId(model);
              const [author, repo] = repoId.split('/');
              const filesResponse = await getRepoFiles({ path: { author, repo } });
              if (filesResponse.data && filesResponse.data.length > 0) {
                return { model, variants: filesResponse.data };
              }
            } catch {
              // Skip repos we can't fetch
            }
            return null;
          })
        );

        const validResults = modelsWithVariants.filter(Boolean) as {
          model: HfSearchResult;
          variants: HfQuantVariant[];
        }[];

        setResults(validResults.map((r) => r.model));
        setRepoData((prev) => {
          const next = { ...prev };
          for (const r of validResults) {
            const repoId = getRepoId(r.model);
            // Find the best variant (highest quality_rank that's not too big)
            let recommendedIdx: number | null = null;
            const sorted = [...r.variants].sort((a, b) => b.quality_rank - a.quality_rank);
            // Recommend the highest quality variant under 8GB, or the smallest if all are big
            const under8GB = sorted.filter((v) => v.size_bytes < 8 * 1024 * 1024 * 1024);
            if (under8GB.length > 0) {
              recommendedIdx = r.variants.findIndex((v) => v.filename === under8GB[0].filename);
            } else {
              // Pick smallest
              const smallest = [...r.variants].sort((a, b) => a.size_bytes - b.size_bytes)[0];
              recommendedIdx = r.variants.findIndex((v) => v.filename === smallest.filename);
            }
            next[repoId] = {
              variants: r.variants,
              recommendedIndex: recommendedIdx,
            };
          }
          return next;
        });

        if (validResults.length === 0) {
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
      setError('Search failed. Check console for details.');
    } finally {
      setSearching(false);
    }
  }, []);

  const handleSearch = useCallback(
    (q: string) => {
      setQuery(q);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => doSearch(q), 400);
    },
    [doSearch]
  );

  const expandRepo = async (repoId: string) => {
    if (expandedRepo === repoId) {
      setExpandedRepo(null);
      return;
    }
    setExpandedRepo(repoId);

    // If we don't have variants yet, fetch them
    if (!repoData[repoId]) {
      setLoadingFiles((prev) => new Set(prev).add(repoId));
      try {
        const [author, repo] = repoId.split('/');
        const response = await getRepoFiles({ path: { author, repo } });
        if (response.data) {
          const variants = response.data;
          // Find recommended
          let recommendedIdx: number | null = null;
          const sorted = [...variants].sort((a, b) => b.quality_rank - a.quality_rank);
          const under8GB = sorted.filter((v) => v.size_bytes < 8 * 1024 * 1024 * 1024);
          if (under8GB.length > 0) {
            recommendedIdx = variants.findIndex((v) => v.filename === under8GB[0].filename);
          } else {
            const smallest = [...variants].sort((a, b) => a.size_bytes - b.size_bytes)[0];
            recommendedIdx = variants.findIndex((v) => v.filename === smallest.filename);
          }
          setRepoData((prev) => ({
            ...prev,
            [repoId]: { variants, recommendedIndex: recommendedIdx },
          }));
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
    const dlKey = `${repoId}/${filename}`;
    setDownloading((prev) => new Set(prev).add(dlKey));
    try {
      const response = await downloadHfModel({
        body: { repo_id: repoId, filename },
      });
      if (response.data) {
        onDownloadStarted(response.data);
      }
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(dlKey);
        return next;
      });
    }
  };

  const startDirectDownload = async () => {
    const spec = directSpec.trim();
    if (!spec) return;

    const dlKey = `direct:${spec}`;
    setDownloading((prev) => new Set(prev).add(dlKey));
    try {
      const response = await downloadHfModel({
        body: { spec },
      });
      if (response.data) {
        onDownloadStarted(response.data);
        setDirectSpec('');
      }
    } catch (e) {
      console.error('Direct download failed:', e);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(dlKey);
        return next;
      });
    }
  };

  return (
    <div className="space-y-4">
      <div>
        <h4 className="text-sm font-medium text-text-default mb-2">Search HuggingFace</h4>
        <div className="relative">
          <input
            type="text"
            value={query}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder="Search for GGUF models..."
            className="w-full px-3 py-2 pl-9 text-sm border border-border-subtle rounded-lg bg-background-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
          />
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-muted" />
          {searching && (
            <Loader2 className="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-text-muted animate-spin" />
          )}
        </div>
      </div>

      {error && <p className="text-sm text-red-500">{error}</p>}

      {results.length > 0 && (
        <div className="space-y-2 max-h-[400px] overflow-y-auto">
          {results.map((model) => {
            const repoId = getRepoId(model);
            const isExpanded = expandedRepo === repoId;
            const data = repoData[repoId];
            const variants = data?.variants || [];
            const recommendedIndex = data?.recommendedIndex ?? null;
            const isLoading = loadingFiles.has(repoId);

            return (
              <div
                key={model.id}
                className="border border-border-subtle rounded-lg overflow-hidden"
              >
                <button
                  className="w-full flex items-center justify-between p-3 text-left hover:bg-background-subtle transition-colors"
                  onClick={() => expandRepo(repoId)}
                >
                  <div className="flex flex-col gap-1 min-w-0">
                    <span className="text-sm font-medium text-text-default truncate">{repoId}</span>
                    <div className="flex items-center gap-3 text-xs text-text-muted">
                      <span>{formatDownloads(model.downloads)} downloads</span>
                      {model.likes > 0 && <span>❤️ {formatDownloads(model.likes)}</span>}
                    </div>
                  </div>
                  {isExpanded ? (
                    <ChevronUp className="w-4 h-4 text-text-muted flex-shrink-0" />
                  ) : (
                    <ChevronDown className="w-4 h-4 text-text-muted flex-shrink-0" />
                  )}
                </button>

                {isExpanded && (
                  <div className="border-t border-border-subtle p-2 space-y-1 bg-background-subtle/50">
                    {isLoading && (
                      <div className="flex items-center gap-2 py-2 px-2 text-sm text-text-muted">
                        <Loader2 className="w-3 h-3 animate-spin" />
                        Loading variants...
                      </div>
                    )}
                    {variants.map((variant, idx) => {
                      const dlKey = `${repoId}/${variant.filename}`;
                      const isStarting = downloading.has(dlKey);
                      const isRecommended = idx === recommendedIndex;

                      return (
                        <div
                          key={variant.filename}
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
                          </div>
                          <Button
                            variant="outline"
                            size="sm"
                            disabled={isStarting}
                            onClick={() => startDownload(repoId, variant.filename)}
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

      <div>
        <h4 className="text-sm font-medium text-text-default mb-2">Direct Download</h4>
        <p className="text-xs text-text-muted mb-2">
          Specify a model directly:{' '}
          <code className="bg-background-subtle px-1 rounded">user/repo:quantization</code>
        </p>
        <div className="flex gap-2">
          <input
            type="text"
            value={directSpec}
            onChange={(e) => setDirectSpec(e.target.value)}
            placeholder="bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M"
            className="flex-1 px-3 py-2 text-sm border border-border-subtle rounded-lg bg-background-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
            onKeyDown={(e) => {
              if (e.key === 'Enter') startDirectDownload();
            }}
          />
          <Button
            variant="outline"
            size="sm"
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
  );
};
