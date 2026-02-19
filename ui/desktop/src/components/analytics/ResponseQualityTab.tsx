import { useEffect, useState, useCallback } from 'react';
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { getResponseQuality } from '../../api';

// Types from generated API
type ResponseQualityMetrics = {
  total_sessions: number;
  avg_session_duration_secs: number;
  avg_messages_per_session: number;
  avg_user_messages_per_session: number;
  avg_tokens_per_session: number;
  retry_rate: number;
  avg_tool_errors_per_session: number;
  sessions_with_errors: number;
  completion_rate: number;
  daily_quality: DailyQuality[];
  quality_by_provider: ProviderQuality[];
};
type DailyQuality = {
  date: string;
  sessions: number;
  retry_rate: number;
  error_rate: number;
  avg_messages: number;
  avg_duration_secs: number;
};
type ProviderQuality = {
  provider: string;
  sessions: number;
  avg_messages: number;
  avg_tokens: number;
  avg_duration_secs: number;
  retry_rate: number;
  error_rate: number;
};

const COLORS = ['#22c55e', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899'];

/** Read a CSS custom property value from :root for Recharts inline styles */
function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}
function useChartColors() {
  const [colors, setColors] = useState({
    grid: '#27272a',
    axis: '#3f3f46',
    tick: '#71717a',
    tickLabel: '#d4d4d8',
    tooltipBg: '#18181b',
    tooltipBorder: '#3f3f46',
    success: '#22c55e',
    warning: '#f59e0b',
    danger: '#ef4444',
  });
  useEffect(() => {
    setColors({
      grid: cssVar('--border-default'),
      axis: cssVar('--border-default'),
      tick: cssVar('--text-muted'),
      tickLabel: cssVar('--text-default'),
      tooltipBg: cssVar('--background-default'),
      tooltipBorder: cssVar('--border-default'),
      success: cssVar('--text-success'),
      warning: cssVar('--text-warning'),
      danger: cssVar('--text-danger'),
    });
  }, []);
  return colors;
}

function formatDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  return `${(secs / 3600).toFixed(1)}h`;
}

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function formatDate(d: string): string {
  return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

// Score quality: 0-100 based on proxy metrics
function qualityScore(m: ResponseQualityMetrics): number {
  const completionScore = m.completion_rate * 40; // 40% weight
  const retryScore = Math.max(0, (1 - m.retry_rate) * 25); // 25% weight (lower is better)
  const errorScore = Math.max(0, (1 - m.sessions_with_errors / Math.max(m.total_sessions, 1)) * 20); // 20% weight
  const efficiencyScore = Math.min(1, 10 / Math.max(m.avg_messages_per_session, 1)) * 15; // 15% weight (fewer messages = more efficient)
  return Math.round(completionScore + retryScore + errorScore + efficiencyScore);
}

function ScoreGauge({ score }: { score: number }) {
  const colorClass = score >= 80 ? 'text-text-success' : score >= 60 ? 'text-text-warning' : 'text-text-danger';
  const circumference = 2 * Math.PI * 45;
  const filled = (score / 100) * circumference;

  return (
    <div className="flex flex-col items-center">
      <svg width="120" height="120" viewBox="0 0 120 120" className={colorClass}>
        <circle cx="60" cy="60" r="45" fill="none" className="stroke-border-default" strokeWidth="8" />
        <circle
          cx="60"
          cy="60"
          r="45"
          fill="none"
          stroke="currentColor"
          strokeWidth="8"
          strokeDasharray={`${filled} ${circumference - filled}`}
          strokeLinecap="round"
          transform="rotate(-90 60 60)"
        />
        <text x="60" y="55" textAnchor="middle" className="fill-text-default" fontSize="28" fontWeight="bold">
          {score}
        </text>
        <text x="60" y="75" textAnchor="middle" className="fill-text-muted" fontSize="12">
          / 100
        </text>
      </svg>
      <span className="text-xs text-text-muted mt-1">Quality Score</span>
    </div>
  );
}

function MetricCard({
  label,
  value,
  subtext,
  color,
}: {
  label: string;
  value: string;
  subtext?: string;
  color?: string;
}) {
  return (
    <div className="bg-background-muted rounded-lg p-4 border border-border-default">
      <div className="text-xs text-text-muted mb-1">{label}</div>
      <div className={`text-2xl font-bold ${color || 'text-text-default'}`}>{value}</div>
      {subtext && <div className="text-xs text-text-subtle mt-1">{subtext}</div>}
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6 animate-pulse p-6">
      <div className="flex gap-4">
        {[1, 2, 3, 4, 5].map((i) => (
          <div key={i} className="h-24 bg-background-muted rounded-lg flex-1" />
        ))}
      </div>
      <div className="h-64 bg-background-muted rounded-lg" />
      <div className="h-48 bg-background-muted rounded-lg" />
    </div>
  );
}

export default function ResponseQualityTab() {
  const [data, setData] = useState<ResponseQualityMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const c = useChartColors();

  const fetchData = useCallback(async () => {
    try {
      setLoading(true);
      const res = await getResponseQuality({ query: { days: 30 } });
      if (res.data) setData(res.data as ResponseQualityMetrics);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load quality metrics');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  if (loading) return <LoadingSkeleton />;
  if (error) return <div className="p-6 text-text-danger">{error}</div>;
  if (!data) return <div className="p-6 text-text-muted">No data available</div>;

  const score = qualityScore(data);
  const errorRate = data.total_sessions > 0 ? data.sessions_with_errors / data.total_sessions : 0;

  return (
    <div className="p-6 space-y-6 overflow-y-auto max-h-[calc(100vh-120px)]">
      {/* Score + KPIs */}
      <div className="flex gap-6 items-start">
        <ScoreGauge score={score} />
        <div className="flex-1 grid grid-cols-5 gap-3">
          <MetricCard
            label="Completion Rate"
            value={formatPercent(data.completion_rate)}
            subtext="Sessions ending with response"
            color={data.completion_rate >= 0.8 ? 'text-text-success' : 'text-text-warning'}
          />
          <MetricCard
            label="Retry Rate"
            value={formatPercent(data.retry_rate)}
            subtext="User rephrasing frequency"
            color={data.retry_rate <= 0.1 ? 'text-text-success' : 'text-text-warning'}
          />
          <MetricCard
            label="Error Rate"
            value={formatPercent(errorRate)}
            subtext={`${data.sessions_with_errors} sessions with errors`}
            color={errorRate <= 0.1 ? 'text-text-success' : 'text-text-danger'}
          />
          <MetricCard
            label="Avg Duration"
            value={formatDuration(data.avg_session_duration_secs)}
            subtext={`${data.avg_messages_per_session.toFixed(1)} messages avg`}
          />
          <MetricCard
            label="Token Efficiency"
            value={Math.round(data.avg_tokens_per_session).toLocaleString()}
            subtext="Avg tokens per session"
          />
        </div>
      </div>

      {/* Daily Quality Trend */}
      {data.daily_quality.length > 0 && (
        <div className="bg-background-muted rounded-lg p-4 border border-border-default">
          <h3 className="text-sm font-medium text-text-default mb-4">Quality Trends (30 days)</h3>
          <ResponsiveContainer width="100%" height={240}>
            <AreaChart data={data.daily_quality}>
              <defs>
                <linearGradient id="qualCompletion" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={c.success} stopOpacity={0.3} />
                  <stop offset="95%" stopColor={c.success} stopOpacity={0} />
                </linearGradient>
                <linearGradient id="qualRetry" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={c.warning} stopOpacity={0.3} />
                  <stop offset="95%" stopColor={c.warning} stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke={c.grid} />
              <XAxis
                dataKey="date"
                tickFormatter={formatDate}
                tick={{ fill: c.tick, fontSize: 11 }}
                stroke={c.axis}
              />
              <YAxis
                tick={{ fill: c.tick, fontSize: 11 }}
                stroke={c.axis}
                tickFormatter={(v) => `${(v * 100).toFixed(0)}%`}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: c.tooltipBg,
                  border: `1px solid ${c.tooltipBorder}`,
                  borderRadius: 8,
                }}
                labelFormatter={(label) => formatDate(String(label))}
                formatter={(value) => [typeof value === 'number' ? formatPercent(value) : value]}
              />
              <Area
                type="monotone"
                dataKey="retry_rate"
                name="Retry Rate"
                stroke={c.warning}
                fill="url(#qualRetry)"
              />
              <Area
                type="monotone"
                dataKey="error_rate"
                name="Error Rate"
                stroke={c.danger}
                fill="none"
                strokeDasharray="5 5"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Provider Quality Breakdown */}
      {data.quality_by_provider.length > 0 && (
        <div className="bg-background-muted rounded-lg p-4 border border-border-default">
          <h3 className="text-sm font-medium text-text-default mb-4">Quality by Provider</h3>
          <div className="grid grid-cols-2 gap-4">
            {/* Bar chart */}
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={data.quality_by_provider} layout="vertical">
                <CartesianGrid strokeDasharray="3 3" stroke={c.grid} />
                <XAxis
                  type="number"
                  tick={{ fill: c.tick, fontSize: 11 }}
                  stroke={c.axis}
                  domain={[0, 1]}
                  tickFormatter={(v) => `${(v * 100).toFixed(0)}%`}
                />
                <YAxis
                  type="category"
                  dataKey="provider"
                  width={120}
                  tick={{ fill: c.tickLabel, fontSize: 11 }}
                  stroke={c.axis}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: c.tooltipBg,
                    border: `1px solid ${c.tooltipBorder}`,
                    borderRadius: 8,
                  }}
                  formatter={(value) => [typeof value === 'number' ? formatPercent(value) : value]}
                />
                <Bar dataKey="retry_rate" name="Retry Rate" stackId="a" fill={c.warning} />
                <Bar dataKey="error_rate" name="Error Rate" stackId="a" fill={c.danger} />
              </BarChart>
            </ResponsiveContainer>

            {/* Stats table */}
            <div className="overflow-auto">
              <table className="w-full text-xs">
                <thead>
                  <tr className="text-text-muted border-b border-border-default">
                    <th className="text-left py-2 pr-3">Provider</th>
                    <th className="text-right py-2 px-2">Sessions</th>
                    <th className="text-right py-2 px-2">Avg Msgs</th>
                    <th className="text-right py-2 px-2">Avg Tokens</th>
                    <th className="text-right py-2 px-2">Duration</th>
                  </tr>
                </thead>
                <tbody>
                  {data.quality_by_provider.map((p, i) => (
                    <tr
                      key={p.provider}
                      className="border-b border-border-default hover:bg-background-muted/30"
                    >
                      <td className="py-2 pr-3">
                        <div className="flex items-center gap-2">
                          <div
                            className="w-2 h-2 rounded-full"
                            style={{ backgroundColor: COLORS[i % COLORS.length] }}
                          />
                          <span className="text-text-default">{p.provider}</span>
                        </div>
                      </td>
                      <td className="text-right py-2 px-2 text-text-default">{p.sessions}</td>
                      <td className="text-right py-2 px-2 text-text-default">
                        {p.avg_messages.toFixed(1)}
                      </td>
                      <td className="text-right py-2 px-2 text-text-default">
                        {Math.round(p.avg_tokens).toLocaleString()}
                      </td>
                      <td className="text-right py-2 px-2 text-text-default">
                        {formatDuration(p.avg_duration_secs)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}

      {/* Session Quality Indicators */}
      <div className="bg-background-muted rounded-lg p-4 border border-border-default">
        <h3 className="text-sm font-medium text-text-default mb-3">Session Quality Indicators</h3>
        <div className="grid grid-cols-3 gap-4">
          <div className="space-y-2">
            <div className="text-xs text-text-muted">User Messages per Session</div>
            <div className="flex items-center gap-2">
              <div className="flex-1 h-2 bg-background-muted rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full bg-background-accent"
                  style={{
                    width: `${Math.min(100, (data.avg_user_messages_per_session / 20) * 100)}%`,
                  }}
                />
              </div>
              <span className="text-sm text-text-default w-12 text-right">
                {data.avg_user_messages_per_session.toFixed(1)}
              </span>
            </div>
            <div className="text-xs text-text-subtle">Fewer = more efficient AI</div>
          </div>

          <div className="space-y-2">
            <div className="text-xs text-text-muted">Tool Errors per Session</div>
            <div className="flex items-center gap-2">
              <div className="flex-1 h-2 bg-background-muted rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full"
                  style={{
                    width: `${Math.min(100, (data.avg_tool_errors_per_session / 5) * 100)}%`,
                    backgroundColor:
                      data.avg_tool_errors_per_session <= 1
                        ? c.success
                        : data.avg_tool_errors_per_session <= 3
                          ? c.warning
                          : c.danger,
                  }}
                />
              </div>
              <span className="text-sm text-text-default w-12 text-right">
                {data.avg_tool_errors_per_session.toFixed(2)}
              </span>
            </div>
            <div className="text-xs text-text-subtle">Lower = more reliable</div>
          </div>

          <div className="space-y-2">
            <div className="text-xs text-text-muted">Total Sessions Analyzed</div>
            <div className="text-2xl font-bold text-text-default">{data.total_sessions}</div>
            <div className="text-xs text-text-subtle">Last 30 days</div>
          </div>
        </div>
      </div>
    </div>
  );
}
