import { useEffect, useState } from "react";
import { compareEvalRuns } from "../../api";
import type { RunComparison } from "../../api";

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDelta(v: number): string {
  const pct = (v * 100).toFixed(1);
  return v >= 0 ? `+${pct}%` : `${pct}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function DeltaIndicator({ delta, label }: { delta: number; label: string }) {
  const isPositive = delta >= 0;
  const isNeutral = Math.abs(delta) < 0.001;
  const bgColor = isNeutral
    ? "bg-zinc-800 border-zinc-600"
    : isPositive
      ? "bg-green-900/30 border-green-500/30"
      : "bg-red-900/30 border-red-500/30";
  const textColor = isNeutral
    ? "text-zinc-300"
    : isPositive
      ? "text-green-300"
      : "text-red-300";
  const arrow = isNeutral ? "→" : isPositive ? "↑" : "↓";

  return (
    <div className={`rounded-lg border p-4 ${bgColor}`}>
      <div className="text-xs text-zinc-400 mb-1">{label}</div>
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
          setError("Failed to load comparison");
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load comparison");
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [baselineId, candidateId]);

  if (loading) {
    return (
      <div className="space-y-4 animate-pulse">
        <div className="h-8 bg-zinc-800 rounded w-1/3" />
        <div className="grid grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-24 bg-zinc-800 rounded-lg" />
          ))}
        </div>
        <div className="h-64 bg-zinc-800 rounded-lg" />
      </div>
    );
  }

  if (error || !comparison) {
    return (
      <div className="text-center py-12">
        <div className="text-red-400 mb-2">⚠️ {error || "No data"}</div>
        <button
          onClick={onClose}
          className="text-sm text-zinc-400 hover:text-white"
        >
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
            className="text-sm text-zinc-400 hover:text-white mb-2 flex items-center gap-1"
          >
            ← Back to runs
          </button>
          <h2 className="text-lg font-semibold text-white">Run Comparison</h2>
        </div>
      </div>

      {/* Side-by-side run info */}
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg border border-zinc-700 bg-zinc-800/50 p-4">
          <div className="text-xs text-zinc-500 uppercase tracking-wider mb-2">
            Baseline
          </div>
          <div className="text-sm text-white font-medium">
            {baseline.datasetName}
          </div>
          <div className="text-xs text-zinc-400 mt-1">
            {formatDate(baseline.startedAt)}
          </div>
          <div className="flex items-center gap-2 mt-2">
            {baseline.versionTag && (
              <span className="text-xs bg-zinc-700 px-2 py-0.5 rounded">
                {baseline.versionTag}
              </span>
            )}
            <span className="text-xs text-zinc-500">
              v{baseline.gooseVersion}
            </span>
          </div>
          <div className="text-lg font-bold text-white mt-2">
            {formatPercent(baseline.overallAccuracy)}
          </div>
          <div className="text-xs text-zinc-400">
            {baseline.correct}/{baseline.totalCases} correct
          </div>
        </div>

        <div className="rounded-lg border border-zinc-700 bg-zinc-800/50 p-4">
          <div className="text-xs text-zinc-500 uppercase tracking-wider mb-2">
            Candidate
          </div>
          <div className="text-sm text-white font-medium">
            {candidate.datasetName}
          </div>
          <div className="text-xs text-zinc-400 mt-1">
            {formatDate(candidate.startedAt)}
          </div>
          <div className="flex items-center gap-2 mt-2">
            {candidate.versionTag && (
              <span className="text-xs bg-zinc-700 px-2 py-0.5 rounded">
                {candidate.versionTag}
              </span>
            )}
            <span className="text-xs text-zinc-500">
              v{candidate.gooseVersion}
            </span>
          </div>
          <div className="text-lg font-bold text-white mt-2">
            {formatPercent(candidate.overallAccuracy)}
          </div>
          <div className="text-xs text-zinc-400">
            {candidate.correct}/{candidate.totalCases} correct
          </div>
        </div>
      </div>

      {/* Delta cards */}
      <div className="grid grid-cols-3 gap-4">
        <DeltaIndicator
          delta={comparison.overallDelta}
          label="Overall Accuracy"
        />
        <DeltaIndicator delta={comparison.agentDelta} label="Agent Accuracy" />
        <DeltaIndicator delta={comparison.modeDelta} label="Mode Accuracy" />
      </div>

      {/* Correlation insight */}
      {comparison.correlation && (
        <div
          className={`rounded-lg border p-4 ${comparison.correlation.versionChanged ? "border-amber-500/30 bg-amber-900/20" : "border-zinc-700 bg-zinc-800/50"}`}
        >
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm">
              {comparison.correlation.versionChanged ? "⚠️" : "ℹ️"}
            </span>
            <span className="text-sm font-medium text-white">
              Correlation Insight
            </span>
          </div>
          <div className="text-sm text-zinc-300">
            {comparison.correlation.summary}
          </div>
          {comparison.correlation.versionChanged && (
            <div className="flex gap-4 mt-2 text-xs text-zinc-400">
              {comparison.correlation.gooseVersionDelta && (
                <span>
                  Goose: {comparison.correlation.gooseVersionDelta}
                </span>
              )}
              {comparison.correlation.versionTagDelta && (
                <span>
                  Tag: {comparison.correlation.versionTagDelta}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {/* Per-agent delta */}
      {comparison.perAgentDelta.length > 0 && (
        <div>
          <h3 className="text-sm font-medium text-white mb-3">
            Per-Agent Accuracy Delta
          </h3>
          <div className="space-y-2">
            {comparison.perAgentDelta.map((ad) => {
              const isRegression = ad.delta < -0.01;
              const isImprovement = ad.delta > 0.01;
              return (
                <div
                  key={ad.agent}
                  className="flex items-center gap-3 rounded-lg border border-zinc-700 bg-zinc-800/50 p-3"
                >
                  <div className="w-32 text-sm text-white truncate">
                    {ad.agent}
                  </div>
                  <div className="flex-1 flex items-center gap-2">
                    <div className="w-20 text-right text-xs text-zinc-400">
                      {formatPercent(ad.baselineAccuracy)}
                    </div>
                    <div className="flex-1 relative h-2 bg-zinc-700 rounded-full overflow-hidden">
                      <div
                        className="absolute left-0 top-0 h-full bg-zinc-500 rounded-full"
                        style={{
                          width: `${ad.baselineAccuracy * 100}%`,
                        }}
                      />
                      <div
                        className={`absolute left-0 top-0 h-full rounded-full ${isRegression ? "bg-red-500" : isImprovement ? "bg-green-500" : "bg-zinc-400"}`}
                        style={{
                          width: `${ad.candidateAccuracy * 100}%`,
                        }}
                      />
                    </div>
                    <div className="w-20 text-xs text-zinc-400">
                      {formatPercent(ad.candidateAccuracy)}
                    </div>
                  </div>
                  <div
                    className={`w-16 text-right text-sm font-mono ${isRegression ? "text-red-400" : isImprovement ? "text-green-400" : "text-zinc-400"}`}
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
          <h3 className="text-sm font-medium text-red-400 mb-3">
            🔴 New Failures ({comparison.newFailures.length})
          </h3>
          <div className="rounded-lg border border-red-500/20 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-red-900/20 text-left">
                  <th className="px-3 py-2 text-xs text-zinc-400">Input</th>
                  <th className="px-3 py-2 text-xs text-zinc-400">Expected</th>
                  <th className="px-3 py-2 text-xs text-zinc-400">Actual</th>
                </tr>
              </thead>
              <tbody>
                {comparison.newFailures.map((f, i) => (
                  <tr
                    key={i}
                    className="border-t border-red-500/10 hover:bg-red-900/10"
                  >
                    <td className="px-3 py-2 text-zinc-300 max-w-xs truncate">
                      {f.input}
                    </td>
                    <td className="px-3 py-2 text-green-400 text-xs">
                      {f.expectedAgent} / {f.expectedMode}
                    </td>
                    <td className="px-3 py-2 text-red-400 text-xs">
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
          <h3 className="text-sm font-medium text-green-400 mb-3">
            🟢 Fixed Cases ({comparison.fixedCases.length})
          </h3>
          <div className="rounded-lg border border-green-500/20 overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-green-900/20 text-left">
                  <th className="px-3 py-2 text-xs text-zinc-400">Input</th>
                  <th className="px-3 py-2 text-xs text-zinc-400">Agent</th>
                  <th className="px-3 py-2 text-xs text-zinc-400">Mode</th>
                </tr>
              </thead>
              <tbody>
                {comparison.fixedCases.map((f, i) => (
                  <tr
                    key={i}
                    className="border-t border-green-500/10 hover:bg-green-900/10"
                  >
                    <td className="px-3 py-2 text-zinc-300 max-w-xs truncate">
                      {f.input}
                    </td>
                    <td className="px-3 py-2 text-green-400 text-xs">
                      {f.agent}
                    </td>
                    <td className="px-3 py-2 text-green-400 text-xs">
                      {f.mode}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* No changes */}
      {comparison.newFailures.length === 0 &&
        comparison.fixedCases.length === 0 && (
          <div className="text-center py-8 text-zinc-500">
            <div className="text-2xl mb-2">✅</div>
            <div>
              No regressions or fixes detected between these runs
            </div>
          </div>
        )}
    </div>
  );
}
