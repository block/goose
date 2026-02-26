import { Activity, AlertTriangle, CheckCircle, Clock, RefreshCw, Zap } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { getLiveMonitoring } from '@/api';

// ── Types (from generated API) ──────────────────────────────────────

interface LiveMetrics {
  active_sessions_1h: number;
  active_sessions_24h: number;
  tool_calls_1h: number;
  tool_calls_24h: number;
  tool_errors_1h: number;
  tool_errors_24h: number;
  success_rate_1h: number;
  hot_tools: Array<{ tool_name: string; call_count: number; error_count: number }>;
  recent_errors: Array<{ tool_name: string; error_preview: string; occurred_at: string }>;
  activity_timeline: Array<{ minute: string; tool_calls: number; tool_errors: number }>;
}

// ── Helpers ──────────────────────────────────────────────────────────

function formatTime(isoStr: string): string {
  try {
    const d = new Date(isoStr);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return isoStr;
  }
}

function formatTimeAgo(isoStr: string): string {
  try {
    const d = new Date(isoStr);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHrs = Math.floor(diffMin / 60);
    return `${diffHrs}h ${diffMin % 60}m ago`;
  } catch {
    return isoStr;
  }
}

// ── Pulse indicator ─────────────────────────────────────────────────

function PulseIndicator({ active }: { active: boolean }) {
  return (
    <span className="relative flex h-3 w-3">
      {active && (
        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-background-success-muted opacity-75" />
      )}
      <span
        className={`relative inline-flex rounded-full h-3 w-3 ${
          active ? 'bg-background-success-muted' : 'bg-background-muted'
        }`}
      />
    </span>
  );
}

// ── KPI Card ────────────────────────────────────────────────────────

function LiveKPI({
  label,
  value1h,
  value24h,
  icon: Icon,
  trend,
}: {
  label: string;
  value1h: number | string;
  value24h: number | string;
  icon: React.ElementType;
  trend?: 'up' | 'down' | 'neutral';
}) {
  return (
    <div className="bg-background-default border border-border-default rounded-lg p-4">
      <div className="flex items-center gap-2 mb-2">
        <Icon className="w-4 h-4 text-text-muted" />
        <span className="text-xs text-text-muted uppercase tracking-wider">{label}</span>
      </div>
      <div className="flex items-baseline gap-3">
        <span className="text-2xl font-bold text-text-default">{value1h}</span>
        <span className="text-xs text-text-subtle">last 1h</span>
      </div>
      <div className="flex items-center gap-2 mt-1">
        <span className="text-sm text-text-muted">{value24h}</span>
        <span className="text-xs text-text-subtle">last 24h</span>
        {trend === 'up' && <span className="text-xs text-text-success">↑</span>}
        {trend === 'down' && <span className="text-xs text-text-danger">↓</span>}
      </div>
    </div>
  );
}

// ── Main Component ──────────────────────────────────────────────────

export default function LiveMonitoringTab() {
  const [metrics, setMetrics] = useState<LiveMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchMetrics = useCallback(async () => {
    try {
      const res = await getLiveMonitoring();
      if (res.data) {
        setMetrics(res.data as unknown as LiveMetrics);
        setError(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load live metrics');
    } finally {
      setLoading(false);
      setLastRefresh(new Date());
    }
  }, []);

  // Initial fetch + auto-refresh
  useEffect(() => {
    fetchMetrics();
  }, [fetchMetrics]);

  useEffect(() => {
    if (autoRefresh) {
      intervalRef.current = setInterval(fetchMetrics, 10000); // 10s
    }
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [autoRefresh, fetchMetrics]);

  // ── Loading skeleton ────────────────────────────────────────────

  if (loading && !metrics) {
    return (
      <div className="p-6 space-y-6 animate-pulse">
        <div className="grid grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => (
            <div key={i} className="h-24 bg-background-default rounded-lg" />
          ))}
        </div>
        <div className="h-64 bg-background-default rounded-lg" />
        <div className="h-48 bg-background-default rounded-lg" />
      </div>
    );
  }

  if (error && !metrics) {
    return (
      <div className="p-6 flex flex-col items-center justify-center gap-4 text-text-muted">
        <AlertTriangle className="w-8 h-8 text-text-warning" />
        <p>{error}</p>
        <button
          onClick={fetchMetrics}
          className="px-4 py-2 bg-background-default hover:bg-background-muted rounded-lg text-sm"
        >
          Retry
        </button>
      </div>
    );
  }

  if (!metrics) return null;

  const isActive = metrics.active_sessions_1h > 0;
  const hasErrors = metrics.tool_errors_1h > 0;

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <PulseIndicator active={isActive} />
          <h2 className="text-lg font-semibold text-text-default">Live Monitoring</h2>
          <span className="text-xs text-text-subtle">
            Last updated: {lastRefresh.toLocaleTimeString()}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs ${
              autoRefresh
                ? 'bg-background-success-muted text-text-success border border-border-default'
                : 'bg-background-default text-text-muted border border-border-default'
            }`}
          >
            <Activity className="w-3 h-3" />
            {autoRefresh ? 'Live' : 'Paused'}
          </button>
          <button
            onClick={fetchMetrics}
            className="p-2 hover:bg-background-default rounded-lg text-text-muted hover:text-text-default transition"
          >
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      {/* Error banner */}
      {hasErrors && (
        <div className="flex items-center gap-3 p-3 bg-background-danger-muted border border-border-default rounded-lg">
          <AlertTriangle className="w-4 h-4 text-text-danger shrink-0" />
          <span className="text-sm text-text-danger">
            {metrics.tool_errors_1h} tool error{metrics.tool_errors_1h !== 1 ? 's' : ''} in the last
            hour
          </span>
        </div>
      )}

      {/* KPI Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <LiveKPI
          label="Active Sessions"
          value1h={metrics.active_sessions_1h}
          value24h={metrics.active_sessions_24h}
          icon={Zap}
        />
        <LiveKPI
          label="Tool Calls"
          value1h={metrics.tool_calls_1h}
          value24h={metrics.tool_calls_24h}
          icon={Activity}
        />
        <LiveKPI
          label="Tool Errors"
          value1h={metrics.tool_errors_1h}
          value24h={metrics.tool_errors_24h}
          icon={AlertTriangle}
          trend={metrics.tool_errors_1h > 0 ? 'down' : 'neutral'}
        />
        <LiveKPI
          label="Success Rate"
          value1h={`${(metrics.success_rate_1h * 100).toFixed(1)}%`}
          value24h=""
          icon={CheckCircle}
          trend={metrics.success_rate_1h >= 0.95 ? 'up' : 'down'}
        />
      </div>

      {/* Activity Timeline */}
      {metrics.activity_timeline.length > 0 && (
        <div className="bg-background-default border border-border-default rounded-lg p-4">
          <h3 className="text-sm font-medium text-text-default mb-4">
            Activity Timeline (Last 60 min)
          </h3>
          <ResponsiveContainer width="100%" height={200}>
            <AreaChart data={metrics.activity_timeline}>
              <defs>
                <linearGradient id="colorCalls" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="colorErrors" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ef4444" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#ef4444" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
              <XAxis
                dataKey="minute"
                tick={{ fill: '#71717a', fontSize: 10 }}
                tickFormatter={(v) => formatTime(v)}
                interval="preserveStartEnd"
              />
              <YAxis tick={{ fill: '#71717a', fontSize: 10 }} />
              <Tooltip
                contentStyle={{
                  background: '#18181b',
                  border: '1px solid #3f3f46',
                  borderRadius: '8px',
                }}
                labelFormatter={(v) => formatTime(String(v))}
              />
              <Area
                type="monotone"
                dataKey="tool_calls"
                stroke="#3b82f6"
                fill="url(#colorCalls)"
                name="Tool Calls"
              />
              <Area
                type="monotone"
                dataKey="tool_errors"
                stroke="#ef4444"
                fill="url(#colorErrors)"
                name="Errors"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Hot Tools */}
        <div className="bg-background-default border border-border-default rounded-lg p-4">
          <h3 className="text-sm font-medium text-text-default mb-3 flex items-center gap-2">
            <Zap className="w-4 h-4 text-text-warning" />
            Hot Tools (Last Hour)
          </h3>
          {metrics.hot_tools.length === 0 ? (
            <p className="text-sm text-text-subtle italic">No tool activity in the last hour</p>
          ) : (
            <div className="space-y-2">
              {metrics.hot_tools.map((tool) => {
                const errorRate = tool.call_count > 0 ? tool.error_count / tool.call_count : 0;
                return (
                  <div key={tool.tool_name} className="flex items-center gap-3">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-text-default font-mono truncate">
                          {tool.tool_name}
                        </span>
                        {tool.error_count > 0 && (
                          <span className="text-xs px-1.5 py-0.5 bg-background-danger-muted text-text-danger rounded">
                            {tool.error_count} err
                          </span>
                        )}
                      </div>
                      <div className="mt-1 w-full bg-background-default rounded-full h-1.5">
                        <div
                          className={`h-1.5 rounded-full ${
                            errorRate > 0.1 ? 'bg-background-danger-muted' : 'bg-background-accent'
                          }`}
                          style={{
                            width: `${Math.min(
                              100,
                              (tool.call_count /
                                Math.max(1, metrics.hot_tools[0]?.call_count ?? 1)) *
                                100
                            )}%`,
                          }}
                        />
                      </div>
                    </div>
                    <span className="text-sm text-text-muted font-mono w-12 text-right">
                      {tool.call_count}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Recent Errors */}
        <div className="bg-background-default border border-border-default rounded-lg p-4">
          <h3 className="text-sm font-medium text-text-default mb-3 flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 text-text-danger" />
            Recent Errors
          </h3>
          {metrics.recent_errors.length === 0 ? (
            <div className="flex flex-col items-center py-6 text-text-subtle">
              <CheckCircle className="w-8 h-8 mb-2 text-text-success" />
              <p className="text-sm">No recent errors</p>
            </div>
          ) : (
            <div className="space-y-3">
              {metrics.recent_errors.map((err, i) => (
                <div key={i} className="border-l-2 border-border-default pl-3 py-1">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-text-default font-mono">{err.tool_name}</span>
                    <span className="text-xs text-text-subtle flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      {formatTimeAgo(err.occurred_at)}
                    </span>
                  </div>
                  <p className="text-xs text-text-muted mt-0.5 font-mono truncate">
                    {err.error_preview}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
