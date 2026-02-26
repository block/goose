import {
  AlertTriangle,
  ArrowRight,
  CheckCircle,
  Clock,
  Copy,
  Cpu,
  ExternalLink,
  FileText,
  StopCircle,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useInstanceEvents } from '../../../hooks/useInstanceEvents';
import type { InstanceResponse, InstanceResultResponse, InstanceStatus } from '../../../lib/instances';
import { getInstanceResult } from '../../../lib/instances';
import { InstanceEventLog } from './InstanceEventLog';
import { InstanceStatusBadge } from './InstanceStatusBadge';

function formatElapsed(secs?: number): string {
  if (secs == null) return '—';
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${Math.round(secs % 60)}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

interface InstanceDetailProps {
  instance: InstanceResponse;
  onClose: () => void;
  onCancel: (id: string) => Promise<void>;
}

export function InstanceDetail({ instance, onClose, onCancel }: InstanceDetailProps) {
  const navigate = useNavigate();
  const [result, setResult] = useState<InstanceResultResponse | null>(null);
  const [resultLoading, setResultLoading] = useState(false);
  const [resultError, setResultError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [copied, setCopied] = useState(false);

  const isRunning = instance.status === 'running';
  const isTerminal =
    instance.status === 'completed' ||
    instance.status === 'failed' ||
    instance.status === 'cancelled';

  // SSE events for running instances
  const { events, connected } = useInstanceEvents(isRunning ? instance.id : null);

  // Fetch result for terminal instances
  useEffect(() => {
    if (!isTerminal) {
      setResult(null);
      return;
    }

    let cancelled = false;
    setResultLoading(true);
    setResultError(null);

    getInstanceResult(instance.id)
      .then((r: InstanceResultResponse) => {
        if (!cancelled) setResult(r);
      })
      .catch((err: unknown) => {
        if (!cancelled) setResultError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!cancelled) setResultLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [instance.id, isTerminal]);

  const handleCancel = useCallback(async () => {
    setCancelling(true);
    try {
      await onCancel(instance.id);
    } finally {
      setCancelling(false);
    }
  }, [instance.id, onCancel]);

  const handleCopyId = useCallback(() => {
    navigator.clipboard.writeText(instance.id);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [instance.id]);

  const handleJoinSession = useCallback(() => {
    // Navigate to the pair view with the instance's session
    navigate(`/pair?instance=${instance.id}`);
  }, [instance.id, navigate]);

  return (
    <div className="h-full flex flex-col border-l border-border-default bg-background-default">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border-default">
        <div className="flex items-center gap-3 min-w-0">
          <h3 className="font-semibold text-sm truncate">{instance.persona}</h3>
          <InstanceStatusBadge status={instance.status as InstanceStatus} size="sm" />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {isRunning && (
            <>
              <button
                type="button"
                onClick={handleJoinSession}
                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-accent-default hover:bg-accent-default/10 rounded-md transition-colors"
                title="Join this agent's session"
              >
                <ExternalLink className="w-3.5 h-3.5" />
                Join
              </button>
              <button
                type="button"
                onClick={handleCancel}
                disabled={cancelling}
                className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-error-default hover:bg-error-muted rounded-md transition-colors disabled:opacity-50"
                title="Cancel instance"
              >
                <StopCircle className="w-3.5 h-3.5" />
                {cancelling ? 'Stopping...' : 'Stop'}
              </button>
            </>
          )}
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-md hover:bg-background-muted transition-colors"
            title="Close detail"
          >
            <X className="w-4 h-4 text-text-muted" />
          </button>
        </div>
      </div>

      {/* Metadata */}
      <div className="px-4 py-3 border-b border-border-default space-y-2">
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <button
            type="button"
            onClick={handleCopyId}
            className="flex items-center gap-1 font-mono hover:text-text-default transition-colors"
            title="Copy instance ID"
          >
            <Copy className="w-3 h-3" />
            {copied ? 'Copied!' : `${instance.id.slice(0, 12)}...`}
          </button>
        </div>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
          {instance.provider_name && (
            <div className="flex items-center gap-1.5">
              <Cpu className="w-3 h-3 text-text-subtle" />
              <span className="text-text-muted">Model:</span>
              <span className="font-medium">
                {instance.provider_name ? `${instance.provider_name}/` : ''}
                {instance.model_name}
              </span>
            </div>
          )}
          <div className="flex items-center gap-1.5">
            <ArrowRight className="w-3 h-3 text-text-subtle" />
            <span className="text-text-muted">Turns:</span>
            <span className="font-medium">{instance.turns}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <Clock className="w-3 h-3 text-text-subtle" />
            <span className="text-text-muted">Elapsed:</span>
            <span className="font-medium">{formatElapsed(instance.elapsed_secs)}</span>
          </div>
        </div>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Live Events (running) */}
        {isRunning && <InstanceEventLog events={events} connected={connected} maxHeight="400px" />}

        {/* Result (terminal) */}
        {isTerminal && (
          <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <FileText className="w-4 h-4 text-text-subtle" />
              Result
            </div>

            {resultLoading ? (
              <div className="text-center py-8 text-text-muted text-sm">
                <Clock className="w-5 h-5 mx-auto mb-2 animate-pulse" />
                Loading result...
              </div>
            ) : resultError ? (
              <div className="p-3 rounded-lg bg-error-muted border border-error-default text-error-default text-xs">
                <AlertTriangle className="w-3.5 h-3.5 inline mr-1" />
                {resultError}
              </div>
            ) : result ? (
              <div className="space-y-3">
                {/* Summary */}
                <div className="flex items-center gap-2 text-xs text-text-muted">
                  {result.status === 'completed' ? (
                    <CheckCircle className="w-3.5 h-3.5 text-success-default" />
                  ) : (
                    <AlertTriangle className="w-3.5 h-3.5 text-error-default" />
                  )}
                  <span>
                    {result.turns_taken} turn{result.turns_taken !== 1 ? 's' : ''}
                    {result.duration_secs != null && ` · ${formatElapsed(result.duration_secs)}`}
                  </span>
                </div>

                {/* Output */}
                {result.output && (
                  <div className="rounded-lg border border-border-default overflow-hidden">
                    <div className="px-3 py-1.5 bg-background-muted text-xs text-text-muted border-b border-border-default">
                      Output
                    </div>
                    <pre className="p-3 text-xs text-text-default whitespace-pre-wrap break-words overflow-x-auto max-h-96">
                      {result.output}
                    </pre>
                  </div>
                )}

                {/* Error */}
                {result.error && (
                  <div className="rounded-lg border border-error-default overflow-hidden">
                    <div className="px-3 py-1.5 bg-error-muted text-xs text-error-default border-b border-error-default">
                      Error
                    </div>
                    <pre className="p-3 text-xs text-error-default whitespace-pre-wrap break-words overflow-x-auto max-h-48">
                      {result.error}
                    </pre>
                  </div>
                )}
              </div>
            ) : (
              <p className="text-xs text-text-subtle italic">No result available</p>
            )}
          </div>
        )}

        {/* Events history for terminal instances */}
        {isTerminal && events.length > 0 && (
          <div>
            <div className="flex items-center gap-2 text-sm font-medium mb-2">
              <FileText className="w-4 h-4 text-text-subtle" />
              Event History
            </div>
            <InstanceEventLog events={events} connected={false} maxHeight="300px" />
          </div>
        )}
      </div>
    </div>
  );
}
