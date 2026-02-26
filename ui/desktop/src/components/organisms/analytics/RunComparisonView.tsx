import { useEffect, useState } from 'react';
import type { RunComparison } from '@/api';
import { compareEvalRuns } from '@/api';

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDelta(v: number): string {
  const pct = (v * 100).toFixed(1);
  return v >= 0 ? `+${pct}%` : `${pct}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function DeltaIndicator({ delta, label }: { delta: number; label: string }) {
  const isPositive = delta >= 0;
  const isNeutral = Math.abs(delta) < 0.001;
  const bgColor = isNeutral
    ? 'bg-background-default border-border-muted'
    : isPositive
      ? 'bg-background-success-muted border-border-default'
      : 'bg-background-danger-muted border-border-default';
  const textColor = isNeutral
    ? 'text-text-default'
    : isPositive
      ? 'text-text-success'
      : 'text-text-danger';
  const arrow = isNeutral ? '→' : isPositive ? '↑' : '↓';

  return (
    <div className={`rounded-lg border p-4 ${bgColor}`}>
      <div className="text-xs text-text-muted mb-1">{label}</div>
      <div className={`text-2xl font-bold ${textColor}`}>
        {arrow} {formatDelta(delta)}
      </div>
    </div>
  );
}

interface RunComparisonViewProps {
  baselineId: string;
  candidateId: string;
  onClose: () => void;
}

export default function RunComparisonView({
  baselineId,
  candidateId,
  onClose,
}: RunComparisonViewProps) {
  const [comparison, setComparison] = useState<RunComparison | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await compareEvalRuns({
          query: {
            baseline_id: baselineId,
            candidate_id: candidateId,
          },
        });
        if (res.data) {
          setComparison(res.data);
        } else {
          setError('Failed to load comparison');
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load comparison');
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [baselineId, candidateId]);

  if (loading) {
    return (
      <div className="space-y-4 animate-pulse">
        <div className="h-8 bg-background-default rounded w-1/3" />
        <div className="grid grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-24 bg-background-default rounded-lg" />
          ))}
        </div>
        <div className="h-64 bg-background-default rounded-lg" />
      </div>
    );
  }

  if (error || !comparison) {
    return (
      <div className="text-center py-12">
        <div className="text-text-danger mb-2">⚠️ {error || 'No data'}</div>
        <button onClick={onClose} className="text-sm text-text-muted hover:text-text-default">
          ← Back to runs
        </button>
      </div>
    );
  }

  const { baseline, candidate } = comparison;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <button
            onClick={onClose}
            className="text-sm text-text-muted hover:text-text-default mb-2 flex items-center gap-1"
          >
            ← Back to runs
          </button>
          <h2 className="text-lg font-semibold text-text-default">Run Comparison</h2>
        </div>
      </div>

      {/* Side-by-side run info */}
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <div className="text-xs text-text-subtle uppercase tracking-wider mb-2">Baseline</div>
          <div className="text-sm text-text-default font-medium">{baseline.datasetName}</div>
          <div className="text-xs text-text-muted mt-1">{formatDate(baseline.startedAt)}</div>
          <div className="flex items-center gap-2 mt-2">
            {baseline.versionTag && (
              <span className="text-xs bg-background-muted px-2 py-0.5 rounded">
                {baseline.versionTag}
              </span>
            )}
            <span className="text-xs text-text-subtle">v{baseline.gooseVersion}</span>
          </div>
          <div className="text-lg font-bold text-text-default mt-2">
            {formatPercent(baseline.overallAccuracy)}
          </div>
          <div className="text-xs text-text-muted">
            {baseline.correct}/{baseline.totalCases} correct
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <div className="text-xs text-text-subtle uppercase tracking-wider mb-2">Candidate</div>
          <div className="text-sm text-text-default font-medium">{candidate.datasetName}</div>
          <div className="text-xs text-text-muted mt-1">{formatDate(candidate.startedAt)}</div>
          <div className="flex items-center gap-2 mt-2">
            {candidate.versionTag && (
              <span className="text-xs bg-background-muted px-2 py-0.5 rounded">
                {candidate.versionTag}
              </span>
            )}
            <span className="text-xs text-text-subtle">v{candidate.gooseVersion}</span>
          </div>
          <div className="text-lg font-bold text-text-default mt-2">
            {formatPercent(candidate.overallAccuracy)}
          </div>
          <div className="text-xs text-text-muted">
            {candidate.correct}/{candidate.totalCases} correct
          </div>
        </div>
      </div>

      {/* Delta cards */}
      <div className="grid grid-cols-3 gap-4">
        <DeltaIndicator delta={comparison.overallDelta} label="Overall Accuracy" />
        <DeltaIndicator delta={comparison.agentDelta} label="Agent Accuracy" />
        <DeltaIndicator delta={comparison.modeDelta} label="Mode Accuracy" />
      </div>

      {/* Correlation insight */}
      {comparison.correlation && (
        <div
          className={`rounded-lg border p-4 ${comparison.correlation.versionChanged ? 'border-border-default bg-background-warning-muted' : 'border-border-default bg-background-muted'}`}
        >
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm">{comparison.correlation.versionChanged ? '⚠️' : 'ℹ️'}</span>
            <span className="text-sm font-medium text-text-default">Correlation Insight</span>
          </div>
          <div className="text-sm text-text-default">{comparison.correlation.summary}</div>
          {comparison.correlation.versionChanged && (
            <div className="flex gap-4 mt-2 text-xs text-text-muted">
              {comparison.correlation.gooseVersionDelta && (
                <span>Goose: {comparison.correlation.gooseVersionDelta}</span>
              )}
              {comparison.correlation.versionTagDelta && (
                <span>Tag: {comparison.correlation.versionTagDelta}</span>
              )}
            </div>
          )}
        </div>
      )}

      {/* Per-agent delta */}
      {comparison.perAgentDelta.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-text-default mb-3">Per-Agent Accuracy Delta</h3>
          <div className="space-y-2">
            {comparison.perAgentDelta.map((ad) => {
              const isRegression = ad.delta < -0.01;
              const isImprovement = ad.delta > 0.01;
              return (
                <div
                  key={ad.agent}
                  className="flex items-center gap-3 rounded-lg border border-border-default bg-background-muted p-3"
                >
                  <div className="w-32 text-sm text-text-default truncate">{ad.agent}</div>
                  <div className="flex-1 flex items-center gap-2">
                    <div className="w-20 text-right text-xs text-text-muted">
                      {formatPercent(ad.baselineAccuracy)}
                    </div>
                    <div className="flex-1 relative h-2 bg-background-muted rounded-full overflow-hidden">
                      <div
                        className="absolute left-0 top-0 h-full bg-background-muted rounded-full"
                        style={{
                          width: `${ad.baselineAccuracy * 100}%`,
                        }}
                      />
                      <div
                        className={`absolute left-0 top-0 h-full rounded-full ${isRegression ? 'bg-background-danger-muted' : isImprovement ? 'bg-background-success-muted' : 'bg-background-muted'}`}
                        style={{
                          width: `${ad.candidateAccuracy * 100}%`,
                        }}
                      />
                    </div>
                    <div className="w-20 text-xs text-text-muted">
                      {formatPercent(ad.candidateAccuracy)}
                    </div>
                  </div>
                  <div
                    className={`w-16 text-right text-sm font-mono ${isRegression ? 'text-text-danger' : isImprovement ? 'text-text-success' : 'text-text-muted'}`}
                  >
                    {formatDelta(ad.delta)}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* New failures */}
      {comparison.newFailures.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-text-danger mb-3">
            🔴 New Failures ({comparison.newFailures.length})
          </h3>
          <div className="rounded-lg border border-border-default overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-background-danger-muted text-left">
                  <th className="px-3 py-2 text-xs text-text-muted">Input</th>
                  <th className="px-3 py-2 text-xs text-text-muted">Expected</th>
                  <th className="px-3 py-2 text-xs text-text-muted">Actual</th>
                </tr>
              </thead>
              <tbody>
                {comparison.newFailures.map((f, i) => (
                  <tr
                    key={i}
                    className="border-t border-border-muted hover:bg-background-danger-muted"
                  >
                    <td className="px-3 py-2 text-text-default max-w-xs truncate">{f.input}</td>
                    <td className="px-3 py-2 text-text-success text-xs">
                      {f.expectedAgent} / {f.expectedMode}
                    </td>
                    <td className="px-3 py-2 text-text-danger text-xs">
                      {f.actualAgent} / {f.actualMode}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Fixed cases */}
      {comparison.fixedCases.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-text-success mb-3">
            🟢 Fixed Cases ({comparison.fixedCases.length})
          </h3>
          <div className="rounded-lg border border-border-default overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-background-success-muted text-left">
                  <th className="px-3 py-2 text-xs text-text-muted">Input</th>
                  <th className="px-3 py-2 text-xs text-text-muted">Agent</th>
                  <th className="px-3 py-2 text-xs text-text-muted">Mode</th>
                </tr>
              </thead>
              <tbody>
                {comparison.fixedCases.map((f, i) => (
                  <tr
                    key={i}
                    className="border-t border-border-muted hover:bg-background-success-muted"
                  >
                    <td className="px-3 py-2 text-text-default max-w-xs truncate">{f.input}</td>
                    <td className="px-3 py-2 text-text-success text-xs">{f.agent}</td>
                    <td className="px-3 py-2 text-text-success text-xs">{f.mode}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* No changes */}
      {comparison.newFailures.length === 0 && comparison.fixedCases.length === 0 && (
        <div className="text-center py-8 text-text-subtle">
          <div className="text-2xl mb-2">✅</div>
          <div>No regressions or fixes detected between these runs</div>
        </div>
      )}
    </div>
  );
}
