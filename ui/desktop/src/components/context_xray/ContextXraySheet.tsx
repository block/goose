import { useCallback, useEffect, useRef, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { useIntl, defineMessages } from '../../i18n';
import { cn } from '../../utils';
import { getContextReport } from '../../acp/contextReport';
import type { ContextReport } from '../../types/contextReport';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from '../ui/sheet';
import { Skeleton } from '../ui/skeleton';
import { Button } from '../ui/button';
import { CapacityMeter } from './CapacityMeter';
import { BreakdownList } from './BreakdownList';
import { CompactionControls } from './CompactionControls';
import { formatTokenCount } from '../../utils/usageFormatting';
import { formatPercentOf } from './format';

const i18n = defineMessages({
  title: {
    id: 'contextXray.title',
    defaultMessage: 'Context x-ray',
  },
  modelSummary: {
    id: 'contextXray.modelSummary',
    defaultMessage: '{model} · {limit} token context window',
  },
  heroDetail: {
    id: 'contextXray.heroDetail',
    defaultMessage: 'of {limit} tokens · {percent} of context window',
  },
  refresh: {
    id: 'contextXray.refresh',
    defaultMessage: 'Refresh',
  },
  loadError: {
    id: 'contextXray.loadError',
    defaultMessage: 'Could not load the context report.',
  },
  retry: {
    id: 'contextXray.retry',
    defaultMessage: 'Retry',
  },
  footerTokenizer: {
    id: 'contextXray.footerTokenizer',
    defaultMessage: 'Token counts estimated with the o200k tokenizer.',
  },
  footerLive: {
    id: 'contextXray.footerLive',
    defaultMessage: 'Last provider call used {tokens} total tokens, including output.',
  },
});

function XraySkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <Skeleton className="h-9 w-24" />
        <Skeleton className="h-4 w-56" />
      </div>
      <Skeleton className="h-3 w-full rounded-full" />
      <div className="flex flex-col gap-2">
        {Array.from({ length: 6 }, (_, index) => (
          <Skeleton key={index} className="h-6 w-full" />
        ))}
      </div>
    </div>
  );
}

interface ContextXraySheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessionId: string;
  refreshSignal?: number;
  onCompact?: () => void;
  compactDisabled?: boolean;
  agentBusy?: boolean;
}

export function ContextXraySheet({
  open,
  onOpenChange,
  sessionId,
  refreshSignal,
  onCompact,
  compactDisabled,
  agentBusy = false,
}: ContextXraySheetProps) {
  const intl = useIntl();
  const [report, setReport] = useState<ContextReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const requestIdRef = useRef(0);
  const lastRefreshSignalRef = useRef(refreshSignal);
  const compactionPendingRef = useRef(false);
  const wasAgentBusyRef = useRef(agentBusy);

  const fetchReport = useCallback(
    async (showLoading = true, surfaceError = true) => {
      const requestId = ++requestIdRef.current;
      if (showLoading) setLoading(true);
      try {
        const result = await getContextReport(sessionId);
        if (requestId !== requestIdRef.current) return;
        setReport(result);
        setError(false);
      } catch (err) {
        if (requestId !== requestIdRef.current) return;
        console.error('Failed to load context report:', err);
        if (surfaceError) {
          setReport(null);
          setError(true);
        }
      } finally {
        if (requestId === requestIdRef.current) setLoading(false);
      }
    },
    [sessionId]
  );

  useEffect(() => {
    if (open) void fetchReport();
  }, [open, fetchReport]);

  useEffect(() => {
    if (!open) {
      lastRefreshSignalRef.current = refreshSignal;
      return;
    }
    if (refreshSignal == null || refreshSignal === lastRefreshSignalRef.current) return;

    lastRefreshSignalRef.current = refreshSignal;
    const timer = window.setTimeout(() => {
      void fetchReport(false, false);
    }, 500);
    return () => window.clearTimeout(timer);
  }, [open, refreshSignal, fetchReport]);

  // Compaction usage updates are suppressed upstream, so refreshSignal does not move
  // across a compaction - the agent falling back to idle is the completion signal.
  useEffect(() => {
    const wasAgentBusy = wasAgentBusyRef.current;
    wasAgentBusyRef.current = agentBusy;
    if (agentBusy || !wasAgentBusy || !compactionPendingRef.current) return;
    compactionPendingRef.current = false;
    void fetchReport(false, false);
  }, [agentBusy, fetchReport]);

  const handleCompact = useCallback(() => {
    compactionPendingRef.current = true;
    onCompact?.();
  }, [onCompact]);

  const contextLimit = report?.model.contextLimit ?? 0;

  return (
    <Sheet open={open} onOpenChange={onOpenChange} modal={false}>
      <SheetContent
        side="right"
        className="w-full sm:max-w-[480px] z-[70]"
        onInteractOutside={(event) => event.preventDefault()}
      >
        <SheetHeader className="pb-0">
          <SheetTitle className="pr-20">{intl.formatMessage(i18n.title)}</SheetTitle>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            shape="round"
            onClick={() => void fetchReport()}
            disabled={loading}
            aria-label={intl.formatMessage(i18n.refresh)}
            className="no-drag absolute top-3 right-10 text-text-secondary hover:text-text-primary"
          >
            <RefreshCw className={cn('size-3.5', loading && 'animate-spin')} />
          </Button>
          <SheetDescription>
            {report
              ? intl.formatMessage(i18n.modelSummary, {
                  model: report.model.modelName,
                  limit: formatTokenCount(contextLimit),
                })
              : ''}
          </SheetDescription>
        </SheetHeader>
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden">
          <div className="flex w-full max-w-full flex-col gap-6 px-4 pb-6 pt-2">
            {loading ? (
              <XraySkeleton />
            ) : error ? (
              <div className="flex flex-col items-start gap-3 py-4">
                <p className="text-sm text-text-secondary">{intl.formatMessage(i18n.loadError)}</p>
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  onClick={() => void fetchReport()}
                >
                  {intl.formatMessage(i18n.retry)}
                </Button>
              </div>
            ) : report ? (
              <>
                <div>
                  <div className="text-3xl font-semibold text-text-primary">
                    {formatTokenCount(report.wireTotalTokens, true)}
                  </div>
                  <div className="mt-1 text-sm text-text-secondary">
                    {intl.formatMessage(i18n.heroDetail, {
                      limit: formatTokenCount(contextLimit),
                      percent: formatPercentOf(report.wireTotalTokens, contextLimit),
                    })}
                  </div>
                </div>
                <CapacityMeter report={report} />
                <BreakdownList report={report} />
                <CompactionControls
                  provider={report.model.provider ?? null}
                  contextLimit={contextLimit}
                  onCompact={onCompact ? handleCompact : undefined}
                  compactDisabled={compactDisabled || agentBusy}
                />
                <div className="flex flex-col gap-1">
                  <p className="text-xs text-text-tertiary">
                    {intl.formatMessage(i18n.footerTokenizer)}
                  </p>
                  {report.liveTotalTokens != null && (
                    <p className="text-xs text-text-tertiary">
                      {intl.formatMessage(i18n.footerLive, {
                        tokens: formatTokenCount(report.liveTotalTokens),
                      })}
                    </p>
                  )}
                </div>
              </>
            ) : null}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
