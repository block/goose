import { useState, useCallback, useRef, useEffect } from 'react';
import {
  Search,
  Download,
  ChevronDown,
  ChevronUp,
  Loader2,
  Star,
  Check,
  AlertTriangle,
} from 'lucide-react';
import { Button } from '../../ui/button';
import {
  searchHfModels,
  getRepoFiles,
  downloadHfModel,
  type DownloadModelRequest,
  type HfModelInfo,
  type HfModelVariant,
  type RepoVariantsResponse,
} from '../../../acp/local-inference';
import { defineMessages, useIntl } from '../../../i18n';
import { compareQuality, fitRank } from './modelRecommendations';

const i18n = defineMessages({
  noRecommendations: {
    id: 'huggingFaceModelSearch.noRecommendations',
    defaultMessage: 'No recommendations available. Search Hugging Face to browse models.',
  },
  noVariants: {
    id: 'huggingFaceModelSearch.noVariants',
    defaultMessage: 'No downloadable local variants found in this repository.',
  },
  suggestions: {
    id: 'huggingFaceModelSearch.suggestions',
    defaultMessage: 'Models for your machine',
  },
  profile: {
    id: 'huggingFaceModelSearch.profile',
    defaultMessage:
      'Estimates include loading, 4,096 input tokens, 1,024 output tokens and a 1 GiB reserve. Text only; custom settings and draft models may need more memory.',
  },
  likelyFit: { id: 'huggingFaceModelSearch.likelyFit', defaultMessage: 'Likely fits' },
  unknownFit: { id: 'huggingFaceModelSearch.unknownFit', defaultMessage: 'Memory fit unknown' },
  incompatible: {
    id: 'huggingFaceModelSearch.incompatible',
    defaultMessage: 'Not supported by eredu forecasting',
  },
  downloadAnyway: {
    id: 'huggingFaceModelSearch.downloadAnyway',
    defaultMessage: 'Download anyway',
  },
  popularFit: { id: 'huggingFaceModelSearch.popularFit', defaultMessage: 'Popular · likely fits' },
  checking: {
    id: 'huggingFaceModelSearch.checking',
    defaultMessage: 'Checking memory and benchmarks…',
  },
  more: { id: 'huggingFaceModelSearch.more', defaultMessage: 'Check more models' },
  retry: { id: 'huggingFaceModelSearch.retry', defaultMessage: 'Retry assessment' },
  original: {
    id: 'huggingFaceModelSearch.original',
    defaultMessage: 'Benchmark results for the original model: {model}',
  },
  verified: { id: 'huggingFaceModelSearch.verified', defaultMessage: 'Verified evaluation' },
  reported: { id: 'huggingFaceModelSearch.reported', defaultMessage: 'Reported evaluation' },
  estimates: {
    id: 'huggingFaceModelSearch.estimates',
    defaultMessage: 'Estimated peak: {lower}–{upper}',
  },
  details: { id: 'huggingFaceModelSearch.details', defaultMessage: 'Forecast details' },
  unavailable: {
    id: 'huggingFaceModelSearch.unavailable',
    defaultMessage: 'Assessment unavailable. Expand to retry; the model has not been excluded.',
  },

  searchHuggingFace: {
    id: 'huggingFaceModelSearch.searchHuggingFace',
    defaultMessage: 'Search HuggingFace',
  },
  searchPlaceholder: {
    id: 'huggingFaceModelSearch.searchPlaceholder',
    defaultMessage: 'Search for local models...',
  },
  loadingVariants: {
    id: 'huggingFaceModelSearch.loadingVariants',
    defaultMessage: 'Loading variants...',
  },
  recommended: {
    id: 'huggingFaceModelSearch.recommended',
    defaultMessage: 'Recommended',
  },
  download: {
    id: 'huggingFaceModelSearch.download',
    defaultMessage: 'Download',
  },
  downloaded: {
    id: 'huggingFaceModelSearch.downloaded',
    defaultMessage: 'Downloaded',
  },
  downloading: {
    id: 'huggingFaceModelSearch.downloading',
    defaultMessage: 'Downloading…',
  },
  tooLarge: {
    id: 'huggingFaceModelSearch.tooLarge',
    defaultMessage: 'Eredu forecasts a memory shortfall ({available} available)',
  },
  noGgufModels: {
    id: 'huggingFaceModelSearch.noGgufModels',
    defaultMessage: 'No compatible local models found for this query.',
  },
  searchError: {
    id: 'huggingFaceModelSearch.searchError',
    defaultMessage: 'Search error: {details}',
  },
  searchNoData: {
    id: 'huggingFaceModelSearch.searchNoData',
    defaultMessage: 'Search returned no data.',
  },
  searchFailed: {
    id: 'huggingFaceModelSearch.searchFailed',
    defaultMessage: 'Search failed. Please try again.',
  },
});

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
  variants: HfModelVariant[];
  recommendedVariantId?: string | null;
  quality?: HfModelInfo['quality'];
  availableMemoryBytes: number;
  downloadedQuants: Set<string>;
  downloadedVariants: Set<string>;
}

interface Props {
  showRecommendations?: boolean;
  onDownloadStarted: (modelId: string, request: DownloadModelRequest) => void;
  /** Model IDs (repo:quant) with an active download in progress */
  activeDownloadIds?: Set<string>;
  /** Model IDs (repo:quant) confirmed downloaded on disk */
  downloadedModelIds?: Set<string>;
}

export const HuggingFaceModelSearch = ({
  onDownloadStarted,
  showRecommendations = false,
  activeDownloadIds,
  downloadedModelIds,
}: Props) => {
  const intl = useIntl();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<HfModelInfo[]>([]);
  const [expandedRepo, setExpandedRepo] = useState<string | null>(null);
  const [repoData, setRepoData] = useState<Record<string, RepoData>>({});
  const [searching, setSearching] = useState(false);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [loadingFiles, setLoadingFiles] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [visibleCount, setVisibleCount] = useState(12);
  const [assessmentErrors, setAssessmentErrors] = useState<Set<string>>(new Set());
  const generation = useRef(0);
  const activeRequests = useRef(new Map<string, Promise<void>>());
  const isFrontPage = showRecommendations && !query.trim();

  const assessRepo = useCallback(async (repoId: string, run: number) => {
    const key = `${run}:${repoId}`;
    const existing = activeRequests.current.get(key);
    if (existing) return existing;
    const request = (async () => {
      setLoadingFiles((prev) => new Set(prev).add(repoId));
      try {
        const publish = (response: RepoVariantsResponse, assessedVariant?: string) => {
          if (generation.current !== run) return;
          setRepoData((prev) => {
            const previous = prev[repoId];
            const variants = response.variants.map((variant) =>
              assessedVariant && variant.variantId !== assessedVariant
                ? (previous?.variants.find((old) => old.variantId === variant.variantId) ?? variant)
                : variant
            );
            const quality = response.quality ?? previous?.quality;
            const best = quality?.evidence.length
              ? variants
                  .filter((variant) => variant.supported && variant.assessment?.fit === 'likelyFit')
                  .sort((a, b) => b.qualityRank - a.qualityRank)[0]
              : undefined;
            return {
              ...prev,
              [repoId]: {
                variants,
                recommendedVariantId: best?.variantId,
                quality,
                availableMemoryBytes: response.availableMemoryBytes,
                downloadedQuants: new Set(response.downloadedQuants),
                downloadedVariants: new Set(response.downloadedVariants),
              },
            };
          });
        };
        const listing = await getRepoFiles(repoId, false);
        publish(listing);
        // Inspect common four-bit variants first; this orders work and never rejects a model.
        const variants = [...listing.variants].sort(
          (a, b) => Math.abs(a.qualityRank - 45) - Math.abs(b.qualityRank - 45)
        );
        for (const variant of variants) {
          if (generation.current !== run) return;
          publish(await getRepoFiles(repoId, true, variant.variantId), variant.variantId);
        }
        setAssessmentErrors((prev) => {
          const next = new Set(prev);
          next.delete(repoId);
          return next;
        });
      } catch {
        if (generation.current === run) setAssessmentErrors((prev) => new Set(prev).add(repoId));
      } finally {
        activeRequests.current.delete(key);
        if (generation.current === run)
          setLoadingFiles((prev) => {
            const next = new Set(prev);
            next.delete(repoId);
            return next;
          });
      }
    })();
    activeRequests.current.set(key, request);
    return request;
  }, []);

  const assessPage = useCallback(
    async (models: HfModelInfo[], run: number) => {
      let next = 0;
      await Promise.all(
        Array.from({ length: 2 }, async () => {
          while (next < models.length && generation.current === run) {
            await assessRepo(models[next++].repoId, run);
          }
        })
      );
    },
    [assessRepo]
  );

  useEffect(() => {
    const run = ++generation.current;
    setResults([]);
    setRepoData({});
    setLoadingFiles(new Set());
    setAssessmentErrors(new Set());
    setVisibleCount(12);
    setError(null);
    if (!query.trim() && !showRecommendations) {
      setSearching(false);
      return;
    }
    setSearching(true);
    const timer = setTimeout(
      async () => {
        try {
          const models = await searchHfModels(query, 20);
          if (generation.current !== run) return;
          setResults(models);
          setSearching(false);
          if (models.length === 0 && query.trim()) setError(intl.formatMessage(i18n.noGgufModels));
          void assessPage(models.slice(0, 12), run);
        } catch {
          if (generation.current === run) {
            setError(intl.formatMessage(i18n.searchFailed));
            setSearching(false);
          }
        }
      },
      query.trim() ? 300 : 0
    );
    return () => {
      clearTimeout(timer);
      ++generation.current;
    };
  }, [query, showRecommendations, intl, assessPage]);

  const toggleRepo = (repoId: string) => {
    setExpandedRepo((prev) => (prev === repoId ? null : repoId));
    if (!repoData[repoId]) void assessRepo(repoId, generation.current);
  };

  const showMore = () => {
    const next = visibleCount + 12;
    setVisibleCount(next);
    void assessPage(results.slice(visibleCount, next), generation.current);
  };

  const visibleResults = results.slice(0, visibleCount);
  const rankedResults = [...visibleResults].sort(
    (a, b) =>
      fitRank(repoData[a.repoId]?.variants ?? []) - fitRank(repoData[b.repoId]?.variants ?? []) ||
      compareQuality(repoData[a.repoId]?.quality, repoData[b.repoId]?.quality) ||
      b.downloads - a.downloads
  );
  const recommendedRepos = new Set<string>();
  const recommendedCheckpoints = new Set<string>();
  for (const model of rankedResults) {
    const data = repoData[model.repoId];
    const checkpoint = data?.quality?.evaluatedModelId;
    if (
      recommendedRepos.size < 3 &&
      data?.recommendedVariantId &&
      checkpoint &&
      !recommendedCheckpoints.has(checkpoint)
    ) {
      recommendedRepos.add(model.repoId);
      recommendedCheckpoints.add(checkpoint);
    }
  }
  if (isFrontPage) {
    visibleResults.splice(0, visibleResults.length, ...rankedResults);
  }
  const seenCheckpoints = new Set<string>();
  const displayedResults = visibleResults.filter((model) => {
    if (!isFrontPage) return true;
    const parent = repoData[model.repoId]?.quality?.evaluatedModelId;
    if (!parent || !repoData[model.repoId]?.recommendedVariantId) return true;
    if (seenCheckpoints.has(parent)) return false;
    seenCheckpoints.add(parent);
    return true;
  });

  const startDownload = async (repoId: string, variant: HfModelVariant) => {
    const downloadKey = variant.downloadId;
    const request: DownloadModelRequest = {
      spec: repoId,
      backendId: variant.backendId,
      format: variant.format,
      variantId: variant.variantId,
    };
    setDownloading((prev) => new Set(prev).add(downloadKey));
    try {
      const modelId = await downloadHfModel(request);
      onDownloadStarted(modelId, request);
    } catch (e) {
      console.error('Download failed:', e);
    } finally {
      setDownloading((prev) => {
        const next = new Set(prev);
        next.delete(downloadKey);
        return next;
      });
    }
  };

  return (
    <div className="space-y-4">
      <div>
        <h4 className="text-sm font-medium text-text-default mb-2">
          {intl.formatMessage(i18n.searchHuggingFace)}
        </h4>
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={intl.formatMessage(i18n.searchPlaceholder)}
            className="w-full pl-9 pr-4 py-2 text-sm border border-border-subtle rounded-lg bg-background-default text-text-default placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
          />
          {searching && (
            <Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted animate-spin" />
          )}
        </div>
      </div>

      {isFrontPage && (
        <h4 className="text-sm font-medium">{intl.formatMessage(i18n.suggestions)}</h4>
      )}
      <p className="text-xs text-text-muted">{intl.formatMessage(i18n.profile)}</p>
      {error && !searching && <p className="text-xs text-text-muted">{error}</p>}
      {isFrontPage && !searching && !error && results.length === 0 && (
        <p className="text-xs text-text-muted">{intl.formatMessage(i18n.noRecommendations)}</p>
      )}

      {results.length > 0 && (
        <div className="space-y-1">
          {displayedResults.map((model) => {
            const isExpanded = expandedRepo === model.repoId;
            const data = repoData[model.repoId];
            const variants = data?.variants || [];
            const recommendedVariantId = recommendedRepos.has(model.repoId)
              ? data?.recommendedVariantId
              : null;
            const quality = data?.quality;
            const hasFit = variants.some(
              (variant) => variant.supported && variant.assessment?.fit === 'likelyFit'
            );
            const availableMemory = data?.availableMemoryBytes ?? 0;
            const allShortfall =
              variants.length > 0 &&
              variants.every((variant) => variant.assessment?.fit === 'likelyShortfall');
            const downloadedQuants = data?.downloadedQuants ?? new Set<string>();
            const downloadedVariants = data?.downloadedVariants ?? new Set<string>();

            return (
              <div
                key={model.repoId}
                className={`border border-border-subtle rounded-lg ${allShortfall ? 'opacity-60 hover:opacity-100' : ''}`}
              >
                <button
                  onClick={() => toggleRepo(model.repoId)}
                  className="w-full flex items-center justify-between p-3 text-left hover:bg-background-subtle rounded-lg"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-text-default truncate">
                        {model.repoId}
                      </span>
                    </div>
                    <div className="flex items-center gap-3 mt-0.5">
                      {recommendedVariantId && (
                        <span className="text-xs text-blue-500">
                          {intl.formatMessage(i18n.recommended)}
                        </span>
                      )}
                      {!recommendedVariantId && hasFit && (
                        <span className="text-xs text-text-muted">
                          {intl.formatMessage(
                            model.downloads > 0 ? i18n.popularFit : i18n.likelyFit
                          )}
                        </span>
                      )}
                      {loadingFiles.has(model.repoId) && (
                        <span className="text-xs text-text-muted">
                          {intl.formatMessage(i18n.checking)}
                        </span>
                      )}
                      {allShortfall && (
                        <span className="text-xs text-amber-500">
                          {intl.formatMessage(i18n.tooLarge, {
                            available: formatBytes(availableMemory),
                          })}
                        </span>
                      )}
                      {assessmentErrors.has(model.repoId) && (
                        <span className="text-xs text-text-muted">
                          {intl.formatMessage(i18n.unavailable)}
                        </span>
                      )}
                      <span className="text-xs text-text-muted">
                        ↓ {formatDownloads(model.downloads)}
                      </span>
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
                    {data && !variants.length && !loadingFiles.has(model.repoId) && (
                      <p className="py-2 text-xs text-text-muted">
                        {intl.formatMessage(i18n.noVariants)}
                      </p>
                    )}
                    {loadingFiles.has(model.repoId) && (
                      <div className="flex items-center gap-2 py-2 text-xs text-text-muted">
                        <Loader2 className="w-3 h-3 animate-spin" />
                        {intl.formatMessage(i18n.loadingVariants)}
                      </div>
                    )}
                    {quality && (
                      <div className="py-2 space-y-1 text-xs text-text-muted">
                        {quality.inherited && (
                          <p>
                            {intl.formatMessage(i18n.original, { model: quality.evaluatedModelId })}
                          </p>
                        )}
                        {quality.evidence.map((evidence) => (
                          <div key={evidence.benchmark}>
                            <a
                              href={evidence.sourceUrl}
                              target="_blank"
                              rel="noreferrer"
                              className="underline"
                              title={evidence.notes}
                            >
                              {evidence.benchmark}: {evidence.score} · #{evidence.rank}/
                              {evidence.participants}
                            </a>
                            {' · '}
                            {intl.formatMessage(evidence.verified ? i18n.verified : i18n.reported)}
                          </div>
                        ))}
                      </div>
                    )}
                    {assessmentErrors.has(model.repoId) && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => void assessRepo(model.repoId, generation.current)}
                      >
                        {intl.formatMessage(i18n.retry)}
                      </Button>
                    )}
                    {variants.map((variant) => {
                      const isStarting = downloading.has(variant.downloadId);
                      const isRecommended = variant.variantId === recommendedVariantId;
                      const assessment = variant.assessment;
                      const modelId = variant.modelId;
                      const isActiveDownload = activeDownloadIds?.has(modelId) ?? false;
                      const isDownloaded = downloadedModelIds
                        ? downloadedModelIds.has(modelId)
                        : downloadedVariants.has(modelId) ||
                          downloadedQuants.has(variant.variantId);
                      const tooLarge = assessment?.fit === 'likelyShortfall';
                      const isSupported = variant.supported ?? true;

                      return (
                        <div
                          key={variant.downloadId}
                          className={`flex items-center justify-between py-2 px-2 rounded ${
                            isDownloaded
                              ? 'bg-green-500/5 border border-green-500/20'
                              : isRecommended
                                ? 'bg-blue-500/5 border border-blue-500/20'
                                : tooLarge
                                  ? 'opacity-60 hover:opacity-100'
                                  : 'hover:bg-background-subtle'
                          }`}
                        >
                          <div className="flex flex-col gap-0.5 min-w-0 flex-1 mr-3">
                            <div className="flex items-center gap-2">
                              <span className="text-xs rounded bg-background-muted border border-border-subtle px-1.5 py-0.5 text-text-muted uppercase">
                                {variant.format}
                              </span>
                              <span className="text-xs font-mono font-medium text-text-default">
                                {variant.label}
                              </span>
                              <span className="text-xs text-text-muted">
                                {formatBytes(variant.sizeBytes)}
                              </span>
                              {isRecommended && !isDownloaded && (
                                <span className="inline-flex items-center gap-1 text-xs bg-blue-500 text-white px-1.5 py-0.5 rounded">
                                  <Star className="w-3 h-3" />
                                  {intl.formatMessage(i18n.recommended)}
                                </span>
                              )}
                            </div>
                            {assessment && (
                              <span className="text-xs text-text-muted">
                                {intl.formatMessage(
                                  assessment.fit === 'likelyFit'
                                    ? i18n.likelyFit
                                    : assessment.fit === 'incompatible'
                                      ? i18n.incompatible
                                      : assessment.fit === 'unknown'
                                        ? i18n.unknownFit
                                        : i18n.tooLarge,
                                  { available: formatBytes(availableMemory) }
                                )}
                              </span>
                            )}
                            {assessment && (
                              <details className="text-xs text-text-muted">
                                <summary className="cursor-pointer">
                                  {intl.formatMessage(i18n.details)}
                                </summary>
                                {assessment.lowerBytes != null && (
                                  <p>
                                    {intl.formatMessage(i18n.estimates, {
                                      lower: formatBytes(assessment.lowerBytes),
                                      upper:
                                        assessment.upperBytes == null
                                          ? '?'
                                          : formatBytes(assessment.upperBytes),
                                    })}
                                  </p>
                                )}
                                {assessment.reasons.map((reason, index) => (
                                  <p key={index}>{reason}</p>
                                ))}
                              </details>
                            )}
                            {variant.description && (
                              <span className="text-xs text-text-muted">{variant.description}</span>
                            )}
                            {!isSupported && variant.unsupportedReason && (
                              <span className="inline-flex items-center gap-1 text-xs text-amber-500">
                                <AlertTriangle className="w-3 h-3" />
                                {variant.unsupportedReason}
                              </span>
                            )}
                          </div>
                          {isDownloaded ? (
                            <Button variant="outline" size="sm" disabled className="opacity-60">
                              <Check className="w-3 h-3 mr-1" />
                              {intl.formatMessage(i18n.downloaded)}
                            </Button>
                          ) : isActiveDownload ? (
                            <Button variant="outline" size="sm" disabled className="opacity-60">
                              <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                              {intl.formatMessage(i18n.downloading)}
                            </Button>
                          ) : (
                            <Button
                              variant="outline"
                              size="sm"
                              disabled={isStarting || !isSupported}
                              onClick={() => startDownload(model.repoId, variant)}
                            >
                              {isStarting ? (
                                <Loader2 className="w-3 h-3 animate-spin" />
                              ) : (
                                <>
                                  <Download className="w-3 h-3 mr-1" />
                                  {intl.formatMessage(
                                    tooLarge ? i18n.downloadAnyway : i18n.download
                                  )}
                                </>
                              )}
                            </Button>
                          )}
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
      {visibleCount < results.length && (
        <Button variant="outline" size="sm" onClick={showMore}>
          {intl.formatMessage(i18n.more)}
        </Button>
      )}
    </div>
  );
};
