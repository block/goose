import { useEffect, useState } from 'react';
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { EvalOverview, EvalRunSummary, SessionAnalytics } from '../../api';
import { getEvalOverview, getSessionAnalytics, listEvalRuns } from '../../api';

// --- Design tokens ---
const COLORS = {
  accent: '#3b82f6',
  accentLight: '#60a5fa',
  success: '#22c55e',
  warning: '#f59e0b',
  danger: '#ef4444',
  muted: '#6b7280',
  purple: '#a855f7',
  teal: '#14b8a6',
  chart: ['#3b82f6', '#a855f7', '#14b8a6', '#f59e0b', '#ef4444', '#ec4899'],
};

// --- Utility ---
function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

function accuracyColor(pct: number): string {
  if (pct >= 90) return COLORS.success;
  if (pct >= 70) return COLORS.warning;
  return COLORS.danger;
}

function deltaDisplay(delta: number): { text: string; color: string } {
  const sign = delta >= 0 ? '+' : '';
  return {
    text: `${sign}${delta.toFixed(1)}%`,
    color: delta >= 0 ? COLORS.success : COLORS.danger,
  };
}

// --- Components ---

function KpiCard({
  label,
  value,
  subtitle,
  delta,
  color,
  sparkline,
}: {
  label: string;
  value: string;
  subtitle?: string;
  delta?: { text: string; color: string };
  color?: string;
  sparkline?: number[];
}) {
  const sparkMax = sparkline ? Math.max(...sparkline, 1) : 0;
  return (
    <div className="bg-background-default rounded-lg border border-border-default p-4 flex flex-col gap-1 relative overflow-hidden">
      <span className="text-xs font-medium text-text-muted uppercase tracking-wider">{label}</span>
      <div className="flex items-baseline gap-2">
        <span className="text-2xl font-bold" style={{ color: color || 'var(--text-standard)' }}>
          {value}
        </span>
        {delta && (
          <span className="text-xs font-semibold" style={{ color: delta.color }}>
            {delta.text}
          </span>
        )}
      </div>
      {subtitle && <span className="text-[11px] text-text-muted">{subtitle}</span>}
      {/* Inline sparkline */}
      {sparkline && sparkline.length > 1 && (
        <div className="absolute bottom-0 right-0 w-24 h-8 opacity-30">
          <svg
            viewBox={`0 0 ${sparkline.length - 1} ${sparkMax}`}
            className="w-full h-full"
            preserveAspectRatio="none"
          >
            <polyline
              fill="none"
              stroke={color || COLORS.accent}
              strokeWidth="1.5"
              points={sparkline.map((v, i) => `${i},${sparkMax - v}`).join(' ')}
            />
          </svg>
        </div>
      )}
    </div>
  );
}

function RegressionCard({
  severity,
  description,
  delta,
  versionTag,
}: {
  severity: string;
  description: string;
  delta: number;
  versionTag: string;
}) {
  const isError = severity === 'high';
  return (
    <div
      className={`flex items-start gap-3 p-3 rounded-lg border ${
        isError
          ? 'border-border-default bg-background-danger-muted/5'
          : 'border-yellow-500/30 bg-yellow-500/5'
      }`}
    >
      <span className="text-lg mt-0.5">{isError ? '🔴' : '🟡'}</span>
      <div className="flex-1 min-w-0">
        <p className="text-sm text-text-default">{description}</p>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-xs text-text-muted">
            Δ {delta > 0 ? '+' : ''}
            {delta.toFixed(1)}%
          </span>
          {versionTag && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-background-defaultHover text-text-muted">
              {versionTag}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

function RecentRunRow({ run }: { run: EvalRunSummary }) {
  const accColor = accuracyColor(run.overallAccuracy);
  return (
    <div className="flex items-center gap-3 py-2.5 px-3 rounded-md hover:bg-background-defaultHover transition-colors cursor-pointer">
      <div className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: accColor }} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-text-default truncate">{run.datasetName}</span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-background-defaultHover text-text-muted shrink-0">
            v{run.gooseVersion}
          </span>
        </div>
        <span className="text-xs text-text-muted">
          {formatDate(run.startedAt)} · {run.correct}/
          {run.correct +
            (Math.round((run.correct / Math.max(run.overallAccuracy, 1)) * 100) - run.correct)}{' '}
          cases
        </span>
      </div>
      <div className="text-right shrink-0">
        <span className="text-sm font-bold" style={{ color: accColor }}>
          {run.overallAccuracy.toFixed(1)}%
        </span>
      </div>
    </div>
  );
}

function EmptyDashboard() {
  return (
    <div className="flex flex-col items-center justify-center py-20 px-8 text-center max-w-lg mx-auto">
      <div className="text-5xl mb-4">🚀</div>
      <h2 className="text-xl font-semibold text-text-default mb-2">Welcome to Goose Analytics</h2>
      <p className="text-sm text-text-muted mb-8 leading-relaxed">
        Track how well your orchestrator routes messages, monitor agent performance, and catch
        regressions before they impact users.
      </p>

      <div className="w-full space-y-3">
        <div className="flex items-start gap-3 p-4 rounded-lg border border-border-default bg-background-default hover:bg-background-defaultHover transition-colors cursor-pointer text-left">
          <span className="text-2xl">💬</span>
          <div>
            <h3 className="text-sm font-semibold text-text-default">Start using Goose</h3>
            <p className="text-xs text-text-muted mt-0.5">
              Have conversations — we&apos;ll automatically track routing decisions and build your
              usage dashboard.
            </p>
          </div>
        </div>

        <div className="flex items-start gap-3 p-4 rounded-lg border border-border-default bg-background-default hover:bg-background-defaultHover transition-colors cursor-pointer text-left">
          <span className="text-2xl">🧪</span>
          <div>
            <h3 className="text-sm font-semibold text-text-default">Create an eval dataset</h3>
            <p className="text-xs text-text-muted mt-0.5">
              Define test prompts with expected agent routing to start measuring accuracy.
            </p>
          </div>
        </div>

        <div className="flex items-start gap-3 p-4 rounded-lg border border-border-default bg-background-default hover:bg-background-defaultHover transition-colors cursor-pointer text-left">
          <span className="text-2xl">📦</span>
          <div>
            <h3 className="text-sm font-semibold text-text-default">Import a YAML dataset</h3>
            <p className="text-xs text-text-muted mt-0.5">
              Already have test cases? Upload a YAML file and run your first evaluation in seconds.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className="p-6 space-y-6 animate-pulse">
      <div className="grid grid-cols-4 gap-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-24 bg-background-defaultHover rounded-lg" />
        ))}
      </div>
      <div className="h-64 bg-background-defaultHover rounded-lg" />
      <div className="grid grid-cols-2 gap-4">
        <div className="h-48 bg-background-defaultHover rounded-lg" />
        <div className="h-48 bg-background-defaultHover rounded-lg" />
      </div>
    </div>
  );
}

// --- Main Dashboard ---
export default function AnalyticsDashboard() {
  const [usage, setUsage] = useState<SessionAnalytics | null>(null);
  const [evalOverview, setEvalOverview] = useState<EvalOverview | null>(null);
  const [recentRuns, setRecentRuns] = useState<EvalRunSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const [usageRes, evalRes, runsRes] = await Promise.allSettled([
          getSessionAnalytics(),
          getEvalOverview(),
          listEvalRuns(),
        ]);
        if (cancelled) return;

        if (usageRes.status === 'fulfilled') setUsage(usageRes.value.data as SessionAnalytics);
        if (evalRes.status === 'fulfilled') setEvalOverview(evalRes.value.data as EvalOverview);
        if (runsRes.status === 'fulfilled')
          setRecentRuns((runsRes.value.data as EvalRunSummary[]).slice(0, 5));
      } catch (err) {
        if (!cancelled) setError(String(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) return <LoadingSkeleton />;
  if (error) {
    return (
      <div className="p-8 text-center">
        <p className="text-text-danger text-sm">{error}</p>
      </div>
    );
  }

  const hasUsage = usage && usage.totalSessions > 0;
  const hasEval = evalOverview && evalOverview.totalRuns > 0;

  if (!hasUsage && !hasEval) return <EmptyDashboard />;

  // Prepare sparklines from daily activity
  const sessionSparkline = usage?.dailyActivity?.map((d) => d.sessions) ?? [];
  const tokenSparkline = usage?.dailyActivity?.map((d) => d.inputTokens + d.outputTokens) ?? [];

  return (
    <div className="p-6 space-y-6 max-w-[1400px] mx-auto">
      {/* --- Row 1: KPI Cards --- */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {/* Routing Accuracy (hero metric) */}
        {hasEval ? (
          <KpiCard
            label="Routing Accuracy"
            value={`${evalOverview.overallAccuracy.toFixed(1)}%`}
            subtitle={`${evalOverview.totalTestCases} test cases`}
            delta={deltaDisplay(evalOverview.accuracyDelta)}
            color={accuracyColor(evalOverview.overallAccuracy)}
            sparkline={evalOverview.accuracyTrend?.map((t) => t.overallAccuracy)}
          />
        ) : (
          <KpiCard
            label="Routing Accuracy"
            value="—"
            subtitle="No evals yet"
            color={COLORS.muted}
          />
        )}

        {/* Sessions */}
        <KpiCard
          label="Sessions"
          value={hasUsage ? formatNumber(usage.totalSessions) : '0'}
          subtitle={hasUsage ? `${usage.activeDays} active days` : 'Start chatting'}
          color={COLORS.accent}
          sparkline={sessionSparkline}
        />

        {/* Tokens */}
        <KpiCard
          label="Total Tokens"
          value={hasUsage ? formatNumber(usage.totalTokens) : '0'}
          subtitle={
            hasUsage
              ? `${formatNumber(usage.totalInputTokens)} in · ${formatNumber(usage.totalOutputTokens)} out`
              : undefined
          }
          color={COLORS.purple}
          sparkline={tokenSparkline}
        />

        {/* Agent Accuracy */}
        {hasEval ? (
          <KpiCard
            label="Agent Accuracy"
            value={`${evalOverview.agentAccuracy.toFixed(1)}%`}
            subtitle={`Mode: ${evalOverview.modeAccuracy.toFixed(1)}%`}
            delta={deltaDisplay(evalOverview.agentAccuracyDelta)}
            color={accuracyColor(evalOverview.agentAccuracy)}
          />
        ) : (
          <KpiCard
            label="Agent Accuracy"
            value="—"
            subtitle="Run an eval to measure"
            color={COLORS.muted}
          />
        )}
      </div>

      {/* --- Row 2: Regressions + Recent Runs --- */}
      {hasEval && (evalOverview.regressions.length > 0 || recentRuns.length > 0) && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {/* Regression alerts */}
          <div className="bg-background-default rounded-lg border border-border-default p-4">
            <h3 className="text-sm font-semibold text-text-default mb-3">🚨 Regression Alerts</h3>
            {evalOverview.regressions.length === 0 ? (
              <div className="flex items-center gap-2 text-sm text-text-muted py-4">
                <span className="text-lg">✅</span>
                No regressions detected. All metrics stable.
              </div>
            ) : (
              <div className="space-y-2">
                {evalOverview.regressions.map((r, i) => (
                  <RegressionCard
                    key={i}
                    severity={r.severity}
                    description={r.description}
                    delta={r.delta}
                    versionTag={r.versionTag}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Recent runs */}
          <div className="bg-background-default rounded-lg border border-border-default p-4">
            <h3 className="text-sm font-semibold text-text-default mb-3">📈 Recent Eval Runs</h3>
            {recentRuns.length === 0 ? (
              <div className="text-sm text-text-muted py-4 text-center">
                No eval runs yet. Create a dataset and run your first eval.
              </div>
            ) : (
              <div className="space-y-0.5">
                {recentRuns.map((run) => (
                  <RecentRunRow key={run.id} run={run} />
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* --- Row 3: Accuracy Trend + Per-Agent Accuracy --- */}
      {hasEval && evalOverview.accuracyTrend.length > 1 && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Accuracy trend chart (2/3 width) */}
          <div className="lg:col-span-2 bg-background-default rounded-lg border border-border-default p-4">
            <h3 className="text-sm font-semibold text-text-default mb-4">Accuracy Over Time</h3>
            <ResponsiveContainer width="100%" height={240}>
              <AreaChart data={evalOverview.accuracyTrend}>
                <defs>
                  <linearGradient id="gradOverall" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor={COLORS.accent} stopOpacity={0.3} />
                    <stop offset="100%" stopColor={COLORS.accent} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" />
                <XAxis
                  dataKey="date"
                  tickFormatter={formatDate}
                  tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  domain={[0, 100]}
                  tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                  axisLine={false}
                  tickLine={false}
                  tickFormatter={(v) => `${v}%`}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'var(--surface)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: '8px',
                    fontSize: '12px',
                  }}
                  formatter={(value) => [
                    `${typeof value === 'number' ? value.toFixed(1) : value}%`,
                  ]}
                  labelFormatter={(label) => {
                    const point = evalOverview.accuracyTrend.find((t) => t.date === label);
                    return point?.versionTag
                      ? `${formatDate(String(label))} · ${point.versionTag}`
                      : formatDate(String(label));
                  }}
                />
                <Area
                  type="monotone"
                  dataKey="overallAccuracy"
                  stroke={COLORS.accent}
                  strokeWidth={2}
                  fill="url(#gradOverall)"
                  name="Overall"
                />
                <Area
                  type="monotone"
                  dataKey="agentAccuracy"
                  stroke={COLORS.success}
                  strokeWidth={1.5}
                  fill="none"
                  strokeDasharray="4 4"
                  name="Agent"
                />
                <Area
                  type="monotone"
                  dataKey="modeAccuracy"
                  stroke={COLORS.purple}
                  strokeWidth={1.5}
                  fill="none"
                  strokeDasharray="2 2"
                  name="Mode"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>

          {/* Per-agent accuracy (1/3 width) */}
          <div className="bg-background-default rounded-lg border border-border-default p-4">
            <h3 className="text-sm font-semibold text-text-default mb-4">Per-Agent Accuracy</h3>
            <div className="space-y-3">
              {evalOverview.perAgentAccuracy.map((agent) => {
                const pct = agent.accuracy;
                const color = accuracyColor(pct);
                return (
                  <div key={agent.agent}>
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-xs font-medium text-text-default truncate">
                        {agent.agent}
                      </span>
                      <span className="text-xs font-bold" style={{ color }}>
                        {pct.toFixed(0)}%
                      </span>
                    </div>
                    <div className="h-1.5 bg-background-defaultHover rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all"
                        style={{
                          width: `${pct}%`,
                          backgroundColor: color,
                        }}
                      />
                    </div>
                    <span className="text-[10px] text-text-muted">
                      {agent.pass}/{agent.pass + agent.fail} correct
                    </span>
                  </div>
                );
              })}
              {evalOverview.perAgentAccuracy.length === 0 && (
                <p className="text-xs text-text-muted text-center py-4">No agent data yet</p>
              )}
            </div>
          </div>
        </div>
      )}

      {/* --- Row 4: Usage charts (session activity + provider breakdown) --- */}
      {hasUsage && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Daily activity (2/3) */}
          {usage.dailyActivity.length > 0 && (
            <div className="lg:col-span-2 bg-background-default rounded-lg border border-border-default p-4">
              <h3 className="text-sm font-semibold text-text-default mb-4">Daily Activity</h3>
              <ResponsiveContainer width="100%" height={200}>
                <BarChart data={usage.dailyActivity}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" />
                  <XAxis
                    dataKey="date"
                    tickFormatter={formatDate}
                    tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <YAxis
                    tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'var(--surface)',
                      border: '1px solid var(--border-subtle)',
                      borderRadius: '8px',
                      fontSize: '12px',
                    }}
                    labelFormatter={(l) => formatDate(String(l))}
                  />
                  <Bar
                    dataKey="sessions"
                    fill={COLORS.accent}
                    radius={[3, 3, 0, 0]}
                    name="Sessions"
                  />
                  <Bar
                    dataKey="messages"
                    fill={COLORS.accentLight}
                    radius={[3, 3, 0, 0]}
                    opacity={0.6}
                    name="Messages"
                  />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}

          {/* Provider breakdown (1/3) */}
          {usage.providerUsage.length > 0 && (
            <div className="bg-background-default rounded-lg border border-border-default p-4">
              <h3 className="text-sm font-semibold text-text-default mb-4">Provider Usage</h3>
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={usage.providerUsage}
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={70}
                    dataKey="sessions"
                    nameKey="provider"
                    strokeWidth={0}
                  >
                    {usage.providerUsage.map((_, i) => (
                      <Cell key={i} fill={COLORS.chart[i % COLORS.chart.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'var(--surface)',
                      border: '1px solid var(--border-subtle)',
                      borderRadius: '8px',
                      fontSize: '12px',
                    }}
                    formatter={(value) => [`${value} sessions`]}
                  />
                  <Legend
                    formatter={(value) => (
                      <span className="text-xs text-text-muted">{String(value)}</span>
                    )}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
          )}
        </div>
      )}

      {/* --- Row 5: Token trend --- */}
      {hasUsage && usage.dailyActivity.length > 0 && (
        <div className="bg-background-default rounded-lg border border-border-default p-4">
          <h3 className="text-sm font-semibold text-text-default mb-4">Token Usage Trend</h3>
          <ResponsiveContainer width="100%" height={180}>
            <AreaChart data={usage.dailyActivity}>
              <defs>
                <linearGradient id="gradInput" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={COLORS.accent} stopOpacity={0.2} />
                  <stop offset="100%" stopColor={COLORS.accent} stopOpacity={0} />
                </linearGradient>
                <linearGradient id="gradOutput" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={COLORS.purple} stopOpacity={0.2} />
                  <stop offset="100%" stopColor={COLORS.purple} stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border-subtle)" />
              <XAxis
                dataKey="date"
                tickFormatter={formatDate}
                tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={{ fontSize: 11, fill: 'var(--text-subtle)' }}
                axisLine={false}
                tickLine={false}
                tickFormatter={(v) => formatNumber(typeof v === 'number' ? v : 0)}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: 'var(--surface)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: '8px',
                  fontSize: '12px',
                }}
                labelFormatter={(l) => formatDate(String(l))}
                formatter={(value) => [formatNumber(typeof value === 'number' ? value : 0)]}
              />
              <Area
                type="monotone"
                dataKey="inputTokens"
                stroke={COLORS.accent}
                strokeWidth={1.5}
                fill="url(#gradInput)"
                name="Input"
              />
              <Area
                type="monotone"
                dataKey="outputTokens"
                stroke={COLORS.purple}
                strokeWidth={1.5}
                fill="url(#gradOutput)"
                name="Output"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
