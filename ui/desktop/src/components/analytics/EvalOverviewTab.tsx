import { useEffect, useState } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  BarChart,
  Bar,
  Cell,
} from "recharts";
import {
  getEvalOverview,
  listEvalDatasets,
  runEval,
} from "../../api";
import type {
  EvalOverview,
  EvalDatasetSummary,
} from "../../api";

const COLORS = {
  green: "#22c55e",
  red: "#ef4444",
  amber: "#f59e0b",
  blue: "#3b82f6",
  purple: "#a855f7",
};

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

interface KpiCardProps {
  label: string;
  value: string;
  delta?: number | null;
  status?: "good" | "warn" | "bad" | "neutral";
}

function KpiCard({ label, value, delta, status = "neutral" }: KpiCardProps) {
  const borderColor =
    status === "good"
      ? "border-green-500/40"
      : status === "bad"
        ? "border-red-500/40"
        : status === "warn"
          ? "border-amber-500/40"
          : "border-gray-600/40";

  return (
    <div className={`rounded-lg border ${borderColor} bg-gray-800/50 p-4 flex flex-col gap-1`}>
      <span className="text-xs text-gray-400 uppercase tracking-wide">{label}</span>
      <span className="text-2xl font-bold text-white">{value}</span>
      {delta != null && (
        <span className={`text-xs font-medium ${delta >= 0 ? "text-green-400" : "text-red-400"}`}>
          {delta >= 0 ? "▲" : "▼"} {Math.abs(delta * 100).toFixed(1)}% vs prev
        </span>
      )}
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6 animate-pulse">
      <div className="grid grid-cols-5 gap-4">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-24 rounded-lg bg-gray-700/50" />
        ))}
      </div>
      <div className="h-64 rounded-lg bg-gray-700/50" />
      <div className="grid grid-cols-2 gap-4">
        <div className="h-48 rounded-lg bg-gray-700/50" />
        <div className="h-48 rounded-lg bg-gray-700/50" />
      </div>
    </div>
  );
}

function statusFromAccuracy(v: number): "good" | "warn" | "bad" {
  return v >= 0.9 ? "good" : v >= 0.7 ? "warn" : "bad";
}

export default function EvalOverviewTab() {
  const [overview, setOverview] = useState<EvalOverview | null>(null);
  const [datasets, setDatasets] = useState<EvalDatasetSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchData = async () => {
    try {
      setLoading(true);
      const [ovRes, dsRes] = await Promise.all([getEvalOverview(), listEvalDatasets()]);
      if (ovRes.data) setOverview(ovRes.data);
      if (dsRes.data) setDatasets(dsRes.data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load overview");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  const handleRunAll = async () => {
    if (datasets.length === 0) return;
    setRunning(true);
    try {
      for (const ds of datasets) {
        await runEval({ body: { datasetId: ds.id } });
      }
      await fetchData();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Eval run failed");
    } finally {
      setRunning(false);
    }
  };

  if (loading) return <LoadingSkeleton />;

  if (!overview) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-gray-400">
        <p className="text-lg mb-2">No evaluation data yet</p>
        <p className="text-sm">Create a dataset and run your first eval to see analytics</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="rounded-lg bg-red-900/30 border border-red-500/40 p-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-white">Evaluation Overview</h3>
        <button
          onClick={handleRunAll}
          disabled={running || datasets.length === 0}
          className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors"
        >
          {running ? "Running..." : "Run All Datasets"}
        </button>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-5 gap-4">
        <KpiCard
          label="Overall Accuracy"
          value={formatPercent(overview.overallAccuracy)}
          delta={overview.accuracyDelta}
          status={statusFromAccuracy(overview.overallAccuracy)}
        />
        <KpiCard
          label="Agent Accuracy"
          value={formatPercent(overview.agentAccuracy)}
          delta={overview.agentAccuracyDelta}
          status={statusFromAccuracy(overview.agentAccuracy)}
        />
        <KpiCard
          label="Mode Accuracy"
          value={formatPercent(overview.modeAccuracy)}
          delta={overview.modeAccuracyDelta}
          status={statusFromAccuracy(overview.modeAccuracy)}
        />
        <KpiCard
          label="Total Test Cases"
          value={String(overview.totalTestCases)}
          status="neutral"
        />
        <KpiCard
          label="Total Runs"
          value={String(overview.totalRuns)}
          status={
            overview.lastRunStatus === "pass"
              ? "good"
              : overview.lastRunStatus === "fail"
                ? "bad"
                : "neutral"
          }
        />
      </div>

      {/* Accuracy Trend */}
      {overview.accuracyTrend.length > 0 && (
        <div className="rounded-lg border border-gray-600/40 bg-gray-800/50 p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-4">Accuracy Trend Over Time</h4>
          <ResponsiveContainer width="100%" height={260}>
            <AreaChart data={overview.accuracyTrend}>
              <defs>
                <linearGradient id="gradOverall" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={COLORS.blue} stopOpacity={0.3} />
                  <stop offset="100%" stopColor={COLORS.blue} stopOpacity={0} />
                </linearGradient>
                <linearGradient id="gradAgent" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={COLORS.green} stopOpacity={0.3} />
                  <stop offset="100%" stopColor={COLORS.green} stopOpacity={0} />
                </linearGradient>
                <linearGradient id="gradMode" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={COLORS.purple} stopOpacity={0.3} />
                  <stop offset="100%" stopColor={COLORS.purple} stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis
                dataKey="date"
                tickFormatter={formatDate}
                tick={{ fill: "#9ca3af", fontSize: 11 }}
                stroke="#4b5563"
              />
              <YAxis
                domain={[0, 1]}
                tickFormatter={(v) => `${(Number(v) * 100).toFixed(0)}%`}
                tick={{ fill: "#9ca3af", fontSize: 11 }}
                stroke="#4b5563"
              />
              <Tooltip
                contentStyle={{ backgroundColor: "#1f2937", border: "1px solid #374151", borderRadius: "8px" }}
                labelStyle={{ color: "#e5e7eb" }}
                formatter={(value) => {
                  const v = typeof value === "number" ? value : 0;
                  return [`${(v * 100).toFixed(1)}%`];
                }}
                labelFormatter={(label) => formatDate(String(label))}
              />
              <Area type="monotone" dataKey="overallAccuracy" name="Overall" stroke={COLORS.blue} fill="url(#gradOverall)" strokeWidth={2} />
              <Area type="monotone" dataKey="agentAccuracy" name="Agent" stroke={COLORS.green} fill="url(#gradAgent)" strokeWidth={2} />
              <Area type="monotone" dataKey="modeAccuracy" name="Mode" stroke={COLORS.purple} fill="url(#gradMode)" strokeWidth={2} />
            </AreaChart>
          </ResponsiveContainer>
          <div className="flex gap-6 mt-2 justify-center">
            {[
              { label: "Overall", color: COLORS.blue },
              { label: "Agent", color: COLORS.green },
              { label: "Mode", color: COLORS.purple },
            ].map((item) => (
              <div key={item.label} className="flex items-center gap-2 text-xs text-gray-400">
                <div className="w-3 h-3 rounded-full" style={{ backgroundColor: item.color }} />
                {item.label}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Regressions + Per-Agent Accuracy */}
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg border border-gray-600/40 bg-gray-800/50 p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Regression Alerts</h4>
          {overview.regressions.length === 0 ? (
            <div className="flex items-center justify-center h-32 text-gray-500 text-sm">
              ✓ No regressions detected
            </div>
          ) : (
            <div className="space-y-2 max-h-48 overflow-y-auto">
              {overview.regressions.map((r, i) => (
                <div key={i} className="flex items-start gap-2 p-2 rounded bg-red-900/20 border border-red-500/20">
                  <span className="text-red-400 text-sm mt-0.5">⚠</span>
                  <div className="flex-1 min-w-0">
                    <span className="text-sm text-white font-medium">{r.description}</span>
                    <div className="text-xs text-gray-400 mt-0.5">
                      <span className={`${r.severity === "critical" ? "text-red-400" : "text-amber-400"}`}>
                        {r.severity}
                      </span>
                      <span className="ml-2">Δ {formatPercent(Math.abs(r.delta))}</span>
                      {r.versionTag && <span className="ml-2">v{r.versionTag}</span>}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="rounded-lg border border-gray-600/40 bg-gray-800/50 p-4">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Per-Agent Accuracy</h4>
          {overview.perAgentAccuracy.length > 0 ? (
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={overview.perAgentAccuracy} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis
                  type="number"
                  domain={[0, 1]}
                  tickFormatter={(v) => `${(Number(v) * 100).toFixed(0)}%`}
                  tick={{ fill: "#9ca3af", fontSize: 11 }}
                  stroke="#4b5563"
                />
                <YAxis type="category" dataKey="agent" tick={{ fill: "#e5e7eb", fontSize: 12 }} stroke="#4b5563" width={100} />
                <Tooltip
                  contentStyle={{ backgroundColor: "#1f2937", border: "1px solid #374151", borderRadius: "8px" }}
                  formatter={(value) => {
                    const v = typeof value === "number" ? value : 0;
                    return [`${(v * 100).toFixed(1)}%`];
                  }}
                />
                <Bar dataKey="accuracy" radius={[0, 4, 4, 0]}>
                  {overview.perAgentAccuracy.map((entry, i) => (
                    <Cell
                      key={i}
                      fill={entry.accuracy >= 0.9 ? COLORS.green : entry.accuracy >= 0.7 ? COLORS.amber : COLORS.red}
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-32 text-gray-500 text-sm">
              Run an evaluation to see agent accuracy
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
