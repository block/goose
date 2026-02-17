import { useState, useEffect, useCallback } from 'react';
import {
  X,
  Copy,
  StopCircle,
  FileText,
  Clock,
  Cpu,
  ArrowRight,
  CheckCircle,
  AlertTriangle,
} from 'lucide-react';
import type { InstanceResponse, InstanceResultResponse } from '../../api/instances';
import { getInstanceResult } from '../../api/instances';
import { InstanceStatusBadge } from './InstanceStatusBadge';
import { InstanceEventLog } from './InstanceEventLog';
import { useInstanceEvents } from '../../hooks/useInstanceEvents';

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
      .then((r) => {
        if (!cancelled) setResult(r);
      })
      .catch((err) => {
        if (!cancelled) setResultError(err.message);
      })
      .finally(() => {
        if (!cancelled) setResultLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [instance.id, instance.status, isTerminal]);

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

  return (
    <div className="h-full flex flex-col border-l border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-800">
        <div className="flex items-center gap-3 min-w-0">
          <h3 className="font-semibold text-sm truncate">{instance.persona}</h3>
          <InstanceStatusBadge status={instance.status} size="sm" />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {isRunning && (
            <button
              onClick={handleCancel}
              disabled={cancelling}
              className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md transition-colors disabled:opacity-50"
              title="Cancel instance"
            >
              <StopCircle className="w-3.5 h-3.5" />
              {cancelling ? 'Stopping...' : 'Stop'}
            </button>
          )}
          <button
            onClick={onClose}
            className="p-1.5 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            title="Close detail"
          >
            <X className="w-4 h-4 text-gray-400" />
          </button>
        </div>
      </div>

      {/* Metadata */}
      <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-800 space-y-2">
        <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
          <button
            onClick={handleCopyId}
            className="flex items-center gap-1 font-mono hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
            title="Copy instance ID"
          >
            <Copy className="w-3 h-3" />
            {copied ? 'Copied!' : instance.id.slice(0, 12) + '...'}
          </button>
        </div>

        <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 text-xs">
          {instance.model_name && (
            <div className="flex items-center gap-1.5">
              <Cpu className="w-3 h-3 text-gray-400" />
              <span className="text-gray-500">Model:</span>
              <span className="font-medium truncate">
                {instance.provider_name ? `${instance.provider_name}/` : ''}
                {instance.model_name}
              </span>
            </div>
          )}
          <div className="flex items-center gap-1.5">
            <ArrowRight className="w-3 h-3 text-gray-400" />
            <span className="text-gray-500">Turns:</span>
            <span className="font-medium">{instance.turns}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <Clock className="w-3 h-3 text-gray-400" />
            <span className="text-gray-500">Elapsed:</span>
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
              <FileText className="w-4 h-4 text-gray-400" />
              Result
            </div>

            {resultLoading ? (
              <div className="text-center py-8 text-gray-400 text-sm">
                <Clock className="w-5 h-5 mx-auto mb-2 animate-pulse" />
                Loading result...
              </div>
            ) : resultError ? (
              <div className="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 text-xs">
                <AlertTriangle className="w-3.5 h-3.5 inline mr-1" />
                {resultError}
              </div>
            ) : result ? (
              <div className="space-y-3">
                {/* Summary */}
                <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
                  {result.status === 'completed' ? (
                    <CheckCircle className="w-3.5 h-3.5 text-emerald-500" />
                  ) : (
                    <AlertTriangle className="w-3.5 h-3.5 text-red-500" />
                  )}
                  <span>
                    {result.turns_taken} turn{result.turns_taken !== 1 ? 's' : ''}
                    {result.duration_secs != null && ` · ${formatElapsed(result.duration_secs)}`}
                  </span>
                </div>

                {/* Output */}
                {result.output && (
                  <div className="rounded-lg border border-gray-200 dark:border-gray-800 overflow-hidden">
                    <div className="px-3 py-1.5 bg-gray-50 dark:bg-gray-900/50 text-xs text-gray-500 border-b border-gray-200 dark:border-gray-800">
                      Output
                    </div>
                    <pre className="p-3 text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap break-words overflow-x-auto max-h-96">
                      {result.output}
                    </pre>
                  </div>
                )}

                {/* Error */}
                {result.error && (
                  <div className="rounded-lg border border-red-200 dark:border-red-800 overflow-hidden">
                    <div className="px-3 py-1.5 bg-red-50 dark:bg-red-900/30 text-xs text-red-600 dark:text-red-400 border-b border-red-200 dark:border-red-800">
                      Error
                    </div>
                    <pre className="p-3 text-xs text-red-700 dark:text-red-300 whitespace-pre-wrap break-words overflow-x-auto max-h-48">
                      {result.error}
                    </pre>
                  </div>
                )}
              </div>
            ) : (
              <p className="text-xs text-gray-400 italic">No result available</p>
            )}
          </div>
        )}

        {/* Events history for terminal instances */}
        {isTerminal && events.length > 0 && (
          <div>
            <div className="flex items-center gap-2 text-sm font-medium mb-2">
              <FileText className="w-4 h-4 text-gray-400" />
              Event History
            </div>
            <InstanceEventLog events={events} connected={false} maxHeight="300px" />
          </div>
        )}
      </div>
    </div>
  );
}
