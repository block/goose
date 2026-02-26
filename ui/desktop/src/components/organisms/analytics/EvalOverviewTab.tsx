import { useEffect, useState } from 'react';
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { EvalDatasetSummary, EvalOverview } from '@/api';
import { getEvalOverview, listEvalDatasets, runEval } from '@/api';

const COLORS = {
  green: '#22c55e',
  red: '#ef4444',
  amber: '#f59e0b',
  blue: '#3b82f6',
  purple: '#a855f7',
};

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

interface KpiCardProps {
  label: string;
  value: string;
  delta?: number | null;
  status?: 'good' | 'warn' | 'bad' | 'neutral';
}

function KpiCard({ label, value, delta, status = 'neutral' }: KpiCardProps) {
  const borderColor =
    status === 'good'
      ? 'border-border-default'
      : status === 'bad'
        ? 'border-border-default'
        : status === 'warn'
          ? 'border-border-default'
          : 'border-border-default';

  return (
    <div className={`rounded-lg border ${borderColor} bg-background-muted p-4 flex flex-col gap-1`}>
      <span className="text-xs text-text-muted uppercase tracking-wide">{label}</span>
      <span className="text-2xl font-bold text-text-default">{value}</span>
      {delta != null && (
        <span
          className={`text-xs font-medium ${delta >= 0 ? 'text-text-success' : 'text-text-danger'}`}
        >
          {delta >= 0 ? '▲' : '▼'} {Math.abs(delta * 100).toFixed(1)}% vs prev
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
          <div key={i} className="h-24 rounded-lg bg-background-muted" />
        ))}
      </div>
      <div className="h-64 rounded-lg bg-background-muted" />
      <div className="grid grid-cols-2 gap-4">
        <div className="h-48 rounded-lg bg-background-muted" />
        <div className="h-48 rounded-lg bg-background-muted" />
      </div>
    </div>
  );
}

function statusFromAccuracy(v: number): 'good' | 'warn' | 'bad' {
  return v >= 0.9 ? 'good' : v >= 0.7 ? 'warn' : 'bad';
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
      setError(e instanceof Error ? e.message : 'Failed to load overview');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleRunAll = async () => {
    if (datasets.length === 0) return;
    setRunning(true);
    try {
      for (const ds of datasets) {
        await runEval({ body: { datasetId: ds.id } });
      }
      await fetchData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Eval run failed');
    } finally {
      setRunning(false);
    }
  };

  if (loading) return <LoadingSkeleton />;

  if (!overview) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-text-muted">
        <p className="text-lg mb-2">No evaluation data yet</p>
        <p className="text-sm">Create a dataset and run your first eval to see analytics</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="rounded-lg bg-background-danger-muted border border-border-default p-3 text-text-danger text-sm">
          {error}
        </div>
      )}

      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-text-default">Evaluation Overview</h3>
        <button
          onClick={handleRunAll}
          disabled={running || datasets.length === 0}
          className="px-4 py-2 rounded-lg bg-background-accent hover:bg-background-accent disabled:bg-background-muted disabled:cursor-not-allowed text-text-on-accent text-sm font-medium transition-colors"
        >
          {running ? 'Running...' : 'Run All Datasets'}
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
            overview.lastRunStatus === 'pass'
              ? 'good'
              : overview.lastRunStatus === 'fail'
                ? 'bad'
                : 'neutral'
          }
        />
      </div>

      {/* Accuracy Trend */}
      {overview.accuracyTrend.length > 0 && (
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-4">Accuracy Trend Over Time</h4>
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
                tick={{ fill: '#9ca3af', fontSize: 11 }}
                stroke="#4b5563"
              />
              <YAxis
                domain={[0, 1]}
                tickFormatter={(v) => `${(Number(v) * 100).toFixed(0)}%`}
                tick={{ fill: '#9ca3af', fontSize: 11 }}
                stroke="#4b5563"
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#1f2937',
                  border: '1px solid #374151',
                  borderRadius: '8px',
                }}
                labelStyle={{ color: '#e5e7eb' }}
                formatter={(value) => {
                  const v = typeof value === 'number' ? value : 0;
                  return [`${(v * 100).toFixed(1)}%`];
                }}
                labelFormatter={(label) => formatDate(String(label))}
              />
              <Area
                type="monotone"
                dataKey="overallAccuracy"
                name="Overall"
                stroke={COLORS.blue}
                fill="url(#gradOverall)"
                strokeWidth={2}
              />
              <Area
                type="monotone"
                dataKey="agentAccuracy"
                name="Agent"
                stroke={COLORS.green}
                fill="url(#gradAgent)"
                strokeWidth={2}
              />
              <Area
                type="monotone"
                dataKey="modeAccuracy"
                name="Mode"
                stroke={COLORS.purple}
                fill="url(#gradMode)"
                strokeWidth={2}
              />
            </AreaChart>
          </ResponsiveContainer>
          <div className="flex gap-6 mt-2 justify-center">
            {[
              { label: 'Overall', color: COLORS.blue },
              { label: 'Agent', color: COLORS.green },
              { label: 'Mode', color: COLORS.purple },
            ].map((item) => (
              <div key={item.label} className="flex items-center gap-2 text-xs text-text-muted">
                <div className="w-3 h-3 rounded-full" style={{ backgroundColor: item.color }} />
                {item.label}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Regressions + Per-Agent Accuracy */}
      <div className="grid grid-cols-2 gap-4">
        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-3">Regression Alerts</h4>
          {overview.regressions.length === 0 ? (
            <div className="flex items-center justify-center h-32 text-text-muted text-sm">
              ✓ No regressions detected
            </div>
          ) : (
            <div className="space-y-2 max-h-48 overflow-y-auto">
              {overview.regressions.map((r, i) => (
                <div
                  key={i}
                  className="flex items-start gap-2 p-2 rounded bg-background-danger-muted border border-border-default"
                >
                  <span className="text-text-danger text-sm mt-0.5">⚠</span>
                  <div className="flex-1 min-w-0">
                    <span className="text-sm text-text-default font-medium">{r.description}</span>
                    <div className="text-xs text-text-muted mt-0.5">
                      <span
                        className={`${r.severity === 'critical' ? 'text-text-danger' : 'text-text-warning'}`}
                      >
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

        <div className="rounded-lg border border-border-default bg-background-muted p-4">
          <h4 className="text-sm font-medium text-text-default mb-3">Per-Agent Accuracy</h4>
          {overview.perAgentAccuracy.length > 0 ? (
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={overview.perAgentAccuracy} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis
                  type="number"
                  domain={[0, 1]}
                  tickFormatter={(v) => `${(Number(v) * 100).toFixed(0)}%`}
                  tick={{ fill: '#9ca3af', fontSize: 11 }}
                  stroke="#4b5563"
                />
                <YAxis
                  type="category"
                  dataKey="agent"
                  tick={{ fill: '#e5e7eb', fontSize: 12 }}
                  stroke="#4b5563"
                  width={100}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1f2937',
                    border: '1px solid #374151',
                    borderRadius: '8px',
                  }}
                  formatter={(value) => {
                    const v = typeof value === 'number' ? value : 0;
                    return [`${(v * 100).toFixed(1)}%`];
                  }}
                />
                <Bar dataKey="accuracy" radius={[0, 4, 4, 0]}>
                  {overview.perAgentAccuracy.map((entry, i) => (
                    <Cell
                      key={i}
                      fill={
                        entry.accuracy >= 0.9
                          ? COLORS.green
                          : entry.accuracy >= 0.7
                            ? COLORS.amber
                            : COLORS.red
                      }
                    />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="flex items-center justify-center h-32 text-text-muted text-sm">
              Run an evaluation to see agent accuracy
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
