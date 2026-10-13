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
import { compareQuality, fitRank, recommendedVariant } from './modelRecommendations';

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
  likelyFit: { id: 'huggingFaceModelSearch.likelyFit', defaultMessage: 'Likely fits' },
  incompatible: {
    id: 'huggingFaceModelSearch.incompatible',
    defaultMessage: 'Not compatible with eredu',
  },
  compatibilityUnknown: {
    id: 'huggingFaceModelSearch.compatibilityUnknown',
    defaultMessage: 'Eredu compatibility unverified',
  },
  unsuitable: {
    id: 'huggingFaceModelSearch.unsuitable',
    defaultMessage: 'Not recommended for conversational use in goose',
  },
  suitabilityUnknown: {
    id: 'huggingFaceModelSearch.suitabilityUnknown',
    defaultMessage: 'Conversational suitability unverified',
  },
  downloadAnyway: {
    id: 'huggingFaceModelSearch.downloadAnyway',
    defaultMessage: 'Download anyway',
  },
  popularFit: { id: 'huggingFaceModelSearch.popularFit', defaultMessage: 'Popular · likely fits' },
  checking: {
    id: 'huggingFaceModelSearch.checking',
    defaultMessage: 'Checking suitability, compatibility, memory and benchmarks…',
  },
  more: { id: 'huggingFaceModelSearch.more', defaultMessage: 'Check more models' },
  retry: { id: 'huggingFaceModelSearch.retry', defaultMessage: 'Retry assessment' },
  original: {
    id: 'huggingFaceModelSearch.original',
    defaultMessage: 'Benchmark results for the original model: {model}',
  },
  downloadSize: {
    id: 'huggingFaceModelSearch.downloadSize',
    defaultMessage: 'Download size: {size}',
  },
  estimates: {
    id: 'huggingFaceModelSearch.estimates',
    defaultMessage: 'Estimated peak memory: {lower}–{upper}',
  },
  estimateLowerBound: {
    id: 'huggingFaceModelSearch.estimateLowerBound',
    defaultMessage: 'Estimated peak memory: at least {lower}',
  },
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
    defaultMessage: 'No models found for this query.',
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

const formatName = (format: string): string =>
  format === 'safetensors' ? 'SafeTensors' : format.toUpperCase();

const variantName = (variant: HfModelVariant): string =>
  variant.format === 'safetensors' ? variant.label.replace(/^SafeTensors\s*/i, '') : variant.label;

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
  const peakMemoryEstimate = (assessment: HfModelVariant['assessment']) => {
    if (assessment?.lowerBytes == null) return null;
    return intl.formatMessage(
      assessment.upperBytes == null ? i18n.estimateLowerBound : i18n.estimates,
      {
        lower: formatBytes(assessment.lowerBytes),
        upper: assessment.upperBytes == null ? undefined : formatBytes(assessment.upperBytes),
      }
    );
  };
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<HfModelInfo[]>([]);
  const [expandedRepo, setExpandedRepo] = useState<string | null>(null);
  const [repoData, setRepoData] = useState<Record<string, RepoData>>({});
  const [searching, setSearching] = useState(showRecommendations);
  const [downloading, setDownloading] = useState<Set<string>>(new Set());
  const [loadingFiles, setLoadingFiles] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [visibleCount, setVisibleCount] = useState(12);
  const [assessmentErrors, setAssessmentErrors] = useState<Set<string>>(new Set());
  const generation = useRef(0);
  const repoDataRef = useRef<Record<string, RepoData>>({});
  const activeRequests = useRef(new Map<string, Promise<void>>());
  const isFrontPage = showRecommendations && !query.trim();

  const assessRepo = useCallback(async (repoId: string, run: number) => {
    const key = `${run}:${repoId}`;
    const existing = activeRequests.current.get(key);
    if (existing) return existing;
    const request = (async () => {
      setLoadingFiles((prev) => new Set(prev).add(repoId));
      try {
        let merged = repoDataRef.current[repoId]?.variants ?? [];
        let mergedQuality = repoDataRef.current[repoId]?.quality;
        const publish = (response: RepoVariantsResponse, assessedVariant?: string) => {
          if (generation.current !== run) return;
          merged = response.quality
            ? response.variants
            : response.variants.map((variant) =>
                !assessedVariant || variant.variantId !== assessedVariant
                  ? (merged.find((old) => old.variantId === variant.variantId) ?? variant)
                  : variant
              );
          mergedQuality = response.quality ?? mergedQuality;
          const variants = merged;
          const quality = mergedQuality;
          const best = recommendedVariant(variants, quality);
          const data = {
            variants,
            recommendedVariantId: best?.variantId,
            quality,
            availableMemoryBytes: response.availableMemoryBytes,
            downloadedQuants: new Set(response.downloadedQuants),
            downloadedVariants: new Set(response.downloadedVariants),
          };
          repoDataRef.current = { ...repoDataRef.current, [repoId]: data };
          setRepoData((prev) => ({ ...prev, [repoId]: data }));
        };
        const listing = await getRepoFiles(repoId, false);
        publish(listing);
        // Inspect common four-bit variants first; this orders work and never rejects a model.
        const variants = [...listing.variants].sort(
          (a, b) => Math.abs(a.qualityRank - 45) - Math.abs(b.qualityRank - 45)
        );
        for (const variant of listing.quality ? [] : variants) {
          if (generation.current !== run) return;
          publish(await getRepoFiles(repoId, true, variant.variantId), variant.variantId);
        }
        if (generation.current !== run) return;
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
      return undefined;
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
    repoDataRef.current = {};
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
          const initial: Record<string, RepoData> = {};
          for (const model of models) {
            if (!model.variants?.length) continue;
            initial[model.repoId] = {
              variants: model.variants,
              recommendedVariantId: recommendedVariant(model.variants, model.quality)?.variantId,
              quality: model.quality,
              availableMemoryBytes: model.variants[0].assessment?.availableMemoryBytes ?? 0,
              downloadedQuants: new Set(),
              downloadedVariants: new Set(),
            };
          }
          repoDataRef.current = initial;
          setRepoData(initial);
          setResults(models);
          setSearching(false);
          if (models.length === 0 && query.trim()) setError(intl.formatMessage(i18n.noGgufModels));
          if (query.trim()) {
            void assessPage(
              models.slice(0, 12).filter((model) => !model.variants?.length),
              run
            );
          }
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
      generation.current = run + 1;
    };
  }, [query, showRecommendations, intl, assessPage]);

  useEffect(() => {
    if (expandedRepo && repoDataRef.current[expandedRepo]?.quality) {
      void assessRepo(expandedRepo, generation.current);
    }
  }, [expandedRepo, downloadedModelIds, activeDownloadIds, assessRepo]);

  const toggleRepo = (repoId: string) => {
    setExpandedRepo((prev) => (prev === repoId ? null : repoId));
    if (expandedRepo !== repoId) void assessRepo(repoId, generation.current);
  };

  const showMore = () => {
    const next = visibleCount + 12;
    setVisibleCount(next);
    void assessPage(results.slice(visibleCount, next), generation.current);
  };

  const visibleResults = isFrontPage ? [...results] : results.slice(0, visibleCount);
  const rankedResults = [...visibleResults].sort(
    (a, b) =>
      fitRank(repoData[a.repoId]?.variants ?? []) - fitRank(repoData[b.repoId]?.variants ?? []) ||
      compareQuality(repoData[a.repoId]?.quality, repoData[b.repoId]?.quality) ||
      b.downloads - a.downloads
  );
  const downloadedRepos = new Set(
    [...(downloadedModelIds ?? [])].map((modelId) => modelId.split(':')[0])
  );
  const downloadedCheckpoints = new Set(downloadedRepos);
  for (const model of results) {
    const data = repoData[model.repoId];
    const isDownloaded = downloadedModelIds
      ? downloadedRepos.has(model.repoId)
      : (data?.downloadedVariants.size ?? 0) > 0 || (data?.downloadedQuants.size ?? 0) > 0;
    if (isDownloaded) {
      downloadedRepos.add(model.repoId);
      if (data?.quality) downloadedCheckpoints.add(data.quality.evaluatedModelId);
    }
  }
  const recommendedRepos = new Set<string>();
  const recommendedCheckpoints = new Set<string>();
  for (const model of rankedResults) {
    const data = repoData[model.repoId];
    const checkpoint = data?.quality?.evaluatedModelId;
    if (
      (!isFrontPage || recommendedRepos.size < 10) &&
      data?.recommendedVariantId &&
      checkpoint &&
      (!isFrontPage ||
        (!downloadedRepos.has(model.repoId) &&
          !downloadedCheckpoints.has(checkpoint) &&
          !recommendedCheckpoints.has(checkpoint)))
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
    if (!recommendedRepos.has(model.repoId)) return false;
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
      revision: repoData[repoId]?.quality ? variant.assessment?.revision : undefined,
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
        <>
          <hr className="border-border-subtle" />
          <h4 className="text-sm font-medium">{intl.formatMessage(i18n.suggestions)}</h4>
          <p className="text-xs text-text-muted">
            <a
              href="https://artificialanalysis.ai/"
              target="_blank"
              rel="noreferrer"
              className="underline"
            >
              {intl.formatMessage({
                id: 'localInference.aaAttribution',
                defaultMessage:
                  'Recommendations ordered using Artificial Analysis. Selection and memory estimates by goose; not endorsed by Artificial Analysis.',
              })}
            </a>
          </p>
        </>
      )}
      {error && !searching && <p className="text-xs text-text-muted">{error}</p>}
      {isFrontPage && !searching && !error && displayedResults.length === 0 && (
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
            const recommendedModelVariant = variants.find(
              (variant) => variant.variantId === recommendedVariantId
            );
            const summaryVariant =
              recommendedModelVariant ?? (variants.length === 1 ? variants[0] : undefined);
            const summaryEstimate = peakMemoryEstimate(summaryVariant?.assessment);
            const hasFit = variants.some(
              (variant) => variant.supported && variant.assessment?.fit === 'likelyFit'
            );
            const unsuitable = variants.some(
              (variant) => variant.assessment?.suitability === 'unsuitable'
            );
            const incompatible =
              variants.length > 0 &&
              variants.every((variant) => variant.assessment?.compatibility === 'incompatible');
            const availableMemory = data?.availableMemoryBytes ?? 0;
            const allShortfall =
              variants.length > 0 &&
              variants.every((variant) => variant.assessment?.fit === 'likelyShortfall');
            const downloadedQuants = data?.downloadedQuants ?? new Set<string>();
            const downloadedVariants = data?.downloadedVariants ?? new Set<string>();

            return (
              <div
                key={model.repoId}
                className={`border border-border-subtle rounded-lg ${allShortfall || unsuitable || incompatible ? 'opacity-60 hover:opacity-100' : ''}`}
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
                      <span className="ml-auto mr-2 shrink-0 text-xs text-text-muted">
                        ↓ {formatDownloads(model.downloads)}
                      </span>
                    </div>
                    <div className="flex flex-wrap items-center gap-3 mt-0.5 empty:hidden">
                      {!isFrontPage && recommendedVariantId && (
                        <span className="text-xs text-blue-500">
                          {intl.formatMessage(i18n.recommended)}
                        </span>
                      )}
                      {unsuitable && (
                        <span className="text-xs text-amber-500">
                          {intl.formatMessage(i18n.unsuitable)}
                        </span>
                      )}
                      {incompatible && (
                        <span className="text-xs text-amber-500">
                          {intl.formatMessage(i18n.incompatible)}
                        </span>
                      )}
                      {!recommendedVariantId && !unsuitable && !incompatible && hasFit && (
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
                    </div>
                    {summaryVariant && (
                      <div className="mt-1 space-y-0.5 text-xs text-text-muted">
                        <p>
                          {intl.formatMessage(i18n.downloadSize, {
                            size: formatBytes(summaryVariant.sizeBytes),
                          })}
                        </p>
                        {summaryEstimate && <p>{summaryEstimate}</p>}
                      </div>
                    )}
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
                        <a
                          href={quality.sourceUrl}
                          target="_blank"
                          rel="noreferrer"
                          className="underline"
                          title={quality.evaluationName}
                        >
                          {intl.formatMessage({
                            id: 'localInference.aaEvaluation',
                            defaultMessage: 'Evaluation details on Artificial Analysis',
                          })}
                        </a>
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
                      const isDownloaded =
                        quality || !downloadedModelIds
                          ? downloadedVariants.has(modelId) ||
                            downloadedQuants.has(variant.variantId)
                          : downloadedModelIds.has(modelId);
                      const tooLarge = assessment?.fit === 'likelyShortfall';
                      const isSupported = variant.supported ?? true;
                      const disrecommended =
                        tooLarge ||
                        assessment?.compatibility === 'incompatible' ||
                        assessment?.suitability === 'unsuitable';

                      return (
                        <div
                          key={variant.downloadId}
                          className={`flex items-center justify-between py-2 px-2 rounded ${
                            isDownloaded
                              ? 'bg-green-500/5 border border-green-500/20'
                              : isRecommended
                                ? 'bg-blue-500/5 border border-blue-500/20'
                                : disrecommended
                                  ? 'opacity-60 hover:opacity-100'
                                  : 'hover:bg-background-subtle'
                          }`}
                        >
                          <div className="flex flex-col gap-0.5 min-w-0 flex-1 mr-3">
                            <div className="flex items-center gap-2">
                              <span className="text-xs rounded bg-background-muted border border-border-subtle px-1.5 py-0.5 text-text-muted">
                                {formatName(variant.format)}
                              </span>
                              {variantName(variant) && (
                                <span className="text-xs font-mono font-medium text-text-default">
                                  {variantName(variant)}
                                </span>
                              )}
                              {isRecommended && !isDownloaded && (
                                <span className="inline-flex items-center gap-1 text-xs bg-blue-500 text-white px-1.5 py-0.5 rounded">
                                  <Star className="w-3 h-3" />
                                  {intl.formatMessage(i18n.recommended)}
                                </span>
                              )}
                            </div>
                            {variants.length > 1 && (
                              <>
                                <span className="text-xs text-text-muted">
                                  {intl.formatMessage(i18n.downloadSize, {
                                    size: formatBytes(variant.sizeBytes),
                                  })}
                                </span>
                                {assessment?.lowerBytes != null && (
                                  <span className="text-xs text-text-muted">
                                    {peakMemoryEstimate(assessment)}
                                  </span>
                                )}
                              </>
                            )}
                            {assessment && assessment.fit !== 'unknown' && (
                              <span className="text-xs text-text-muted">
                                {intl.formatMessage(
                                  assessment.fit === 'likelyFit' ? i18n.likelyFit : i18n.tooLarge,
                                  { available: formatBytes(availableMemory) }
                                )}
                              </span>
                            )}
                            {assessment && assessment.compatibility !== 'compatible' && (
                              <span className="text-xs text-amber-500">
                                {intl.formatMessage(
                                  assessment.compatibility === 'incompatible'
                                    ? i18n.incompatible
                                    : i18n.compatibilityUnknown
                                )}
                              </span>
                            )}
                            {assessment && assessment.suitability !== 'suitable' && (
                              <span className="text-xs text-text-muted">
                                {intl.formatMessage(
                                  assessment.suitability === 'unsuitable'
                                    ? i18n.unsuitable
                                    : i18n.suitabilityUnknown
                                )}
                              </span>
                            )}
                            {variant.format !== 'safetensors' && variant.description && (
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
                                    disrecommended ? i18n.downloadAnyway : i18n.download
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
      {!isFrontPage && visibleCount < results.length && (
        <Button variant="outline" size="sm" onClick={showMore}>
          {intl.formatMessage(i18n.more)}
        </Button>
      )}
    </div>
  );
};
