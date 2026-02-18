import { useState, useEffect } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { getToolAnalytics } from '../../api';
import type { ToolAnalytics, ToolUsageStat } from '../../api';

const COLORS = {
  success: '#22c55e',
  error: '#ef4444',
  warning: '#f59e0b',
  muted: '#6b7280',
  accent: '#3b82f6',
};

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function getHealthColor(rate: number): string {
  if (rate >= 95) return COLORS.success;
  if (rate >= 80) return COLORS.warning;
  return COLORS.error;
}

function getHealthLabel(rate: number): string {
  if (rate >= 95) return 'Healthy';
  if (rate >= 80) return 'Degraded';
  return 'Failing';
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6 animate-pulse">
      <div className="grid grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-24 bg-background-default rounded-lg" />
        ))}
      </div>
      <div className="h-64 bg-background-default rounded-lg" />
      <div className="h-96 bg-background-default rounded-lg" />
    </div>
  );
}

function ToolRow({ tool, maxCalls }: { tool: ToolUsageStat; maxCalls: number }) {
  const successRate = tool.success_rate;
  const barWidth = maxCalls > 0 ? (tool.call_count / maxCalls) * 100 : 0;
  const healthColor = getHealthColor(successRate);

  const [extensionName, toolName] = tool.tool_name.includes('__')
    ? tool.tool_name.split('__', 2)
    : ['—', tool.tool_name];

  return (
    <tr className="border-b border-border-default hover:bg-background-default/50 transition-colors">
      <td className="py-3 px-4">
        <div className="flex flex-col">
          <span className="font-medium text-text-default font-semibold text-sm">{toolName}</span>
          <span className="text-xs text-text-muted">{extensionName}</span>
        </div>
      </td>
      <td className="py-3 px-4">
        <div className="flex items-center gap-2">
          <div className="flex-1 h-2 bg-background-default rounded-full overflow-hidden">
            <div
              className="h-full rounded-full transition-all"
              style={{ width: `${barWidth}%`, backgroundColor: COLORS.accent }}
            />
          </div>
          <span className="text-sm text-text-muted w-12 text-right">
            {formatNumber(tool.call_count)}
          </span>
        </div>
      </td>
      <td className="py-3 px-4">
        <div className="flex items-center gap-2">
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: healthColor }}
          />
          <span className="text-sm" style={{ color: healthColor }}>
            {successRate.toFixed(1)}%
          </span>
        </div>
      </td>
      <td className="py-3 px-4">
        <span className="text-sm text-text-muted">
          {tool.error_count > 0 ? (
            <span style={{ color: COLORS.error }}>{tool.error_count}</span>
          ) : (
            '0'
          )}
        </span>
      </td>
      <td className="py-3 px-4">
        <span
          className="text-xs font-medium px-2 py-0.5 rounded-full"
          style={{
            backgroundColor: `${healthColor}20`,
            color: healthColor,
          }}
        >
          {getHealthLabel(successRate)}
        </span>
      </td>
    </tr>
  );
}

export default function ToolsHealthView() {
  const [analytics, setAnalytics] = useState<ToolAnalytics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<'calls' | 'errors' | 'rate'>('calls');
  const [filterText, setFilterText] = useState('');

  useEffect(() => {
    async function load() {
      try {
        const resp = await getToolAnalytics({ query: { days: 30 } });
        if (resp.data) setAnalytics(resp.data);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load');
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (loading) return <div className="p-6"><LoadingSkeleton /></div>;
  if (error) return <div className="p-6 text-red-400">{error}</div>;
  if (!analytics) return <div className="p-6 text-text-muted">No data</div>;

  const successRate = analytics.success_rate;
  const extensionCount = analytics.extension_usage.length;
  const avgToolsPerSession =
    analytics.session_tool_summary.length > 0
      ? analytics.session_tool_summary.reduce((sum, s) => sum + s.tool_calls, 0) /
        analytics.session_tool_summary.length
      : 0;

  // Sort and filter tools
  const sortedTools = [...analytics.tool_usage];
  if (sortBy === 'calls') sortedTools.sort((a, b) => b.call_count - a.call_count);
  else if (sortBy === 'errors') sortedTools.sort((a, b) => b.error_count - a.error_count);
  else sortedTools.sort((a, b) => a.success_rate - b.success_rate);

  const filteredTools = filterText
    ? sortedTools.filter((t) => t.tool_name.toLowerCase().includes(filterText.toLowerCase()))
    : sortedTools;

  const maxCalls = Math.max(...filteredTools.map((t) => t.call_count), 1);
  const failingTools = sortedTools.filter((t) => t.error_count > 0);

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-6xl mx-auto p-6 space-y-6">
        {/* Header */}
        <div>
          <h1 className="text-2xl font-bold text-text-default font-semibold">Tools Health</h1>
          <p className="text-sm text-text-muted mt-1">
            Monitor tool execution health, success rates, and failure patterns across all
            extensions.
          </p>
        </div>

        {/* KPI Cards */}
        <div className="grid grid-cols-4 gap-4">
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <div className="text-xs text-text-muted uppercase tracking-wider mb-1">Total Calls</div>
            <div className="text-2xl font-bold text-text-default font-semibold">
              {formatNumber(analytics.total_tool_calls)}
            </div>
            <div className="text-xs text-text-muted mt-1">Last 30 days</div>
          </div>
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <div className="text-xs text-text-muted uppercase tracking-wider mb-1">
              Success Rate
            </div>
            <div
              className="text-2xl font-bold"
              style={{ color: getHealthColor(successRate) }}
            >
              {successRate.toFixed(1)}%
            </div>
            <div className="text-xs mt-1" style={{ color: getHealthColor(successRate) }}>
              {getHealthLabel(successRate)}
            </div>
          </div>
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <div className="text-xs text-text-muted uppercase tracking-wider mb-1">Extensions</div>
            <div className="text-2xl font-bold text-text-default font-semibold">{extensionCount}</div>
            <div className="text-xs text-text-muted mt-1">Active</div>
          </div>
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <div className="text-xs text-text-muted uppercase tracking-wider mb-1">
              Avg Tools/Session
            </div>
            <div className="text-2xl font-bold text-text-default font-semibold">
              {avgToolsPerSession.toFixed(1)}
            </div>
            <div className="text-xs text-text-muted mt-1">Per session</div>
          </div>
        </div>

        {/* Failing Tools Alert */}
        {failingTools.length > 0 && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4">
            <h3 className="text-sm font-medium text-red-400 mb-2">
              ⚠ {failingTools.length} tool{failingTools.length > 1 ? 's' : ''} with errors
            </h3>
            <div className="flex flex-wrap gap-2">
              {failingTools.slice(0, 5).map((t) => (
                <span
                  key={t.tool_name}
                  className="text-xs px-2 py-1 rounded-md bg-red-500/20 text-red-300"
                >
                  {t.tool_name.split('__').pop()} — {t.error_count} errors (
                  {t.success_rate.toFixed(0)}% success)
                </span>
              ))}
              {failingTools.length > 5 && (
                <span className="text-xs px-2 py-1 text-red-400">
                  +{failingTools.length - 5} more
                </span>
              )}
            </div>
          </div>
        )}

        {/* Daily Activity Chart */}
        {analytics.daily_tool_activity.length > 0 && (
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <h3 className="text-sm font-medium text-text-default font-semibold mb-4">Daily Tool Activity</h3>
            <ResponsiveContainer width="100%" height={200}>
              <AreaChart data={analytics.daily_tool_activity}>
                <defs>
                  <linearGradient id="toolCallGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={COLORS.accent} stopOpacity={0.3} />
                    <stop offset="95%" stopColor={COLORS.accent} stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="toolErrorGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={COLORS.error} stopOpacity={0.3} />
                    <stop offset="95%" stopColor={COLORS.error} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis
                  dataKey="date"
                  tick={{ fill: '#888', fontSize: 11 }}
                  tickFormatter={(d: string) => d.slice(5)}
                />
                <YAxis tick={{ fill: '#888', fontSize: 11 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#1a1a2e',
                    border: '1px solid #333',
                    borderRadius: '8px',
                  }}
                  labelStyle={{ color: '#ccc' }}
                />
                <Area
                  type="monotone"
                  dataKey="tool_calls"
                  stroke={COLORS.accent}
                  fill="url(#toolCallGrad)"
                  name="Calls"
                />
                <Area
                  type="monotone"
                  dataKey="tool_errors"
                  stroke={COLORS.error}
                  fill="url(#toolErrorGrad)"
                  name="Errors"
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        )}

        {/* Tools Table */}
        <div className="bg-background-default rounded-xl border border-border-default">
          <div className="flex items-center justify-between p-4 border-b border-border-default">
            <h3 className="text-sm font-medium text-text-default font-semibold">
              All Tools ({filteredTools.length})
            </h3>
            <div className="flex items-center gap-3">
              <input
                type="text"
                placeholder="Filter tools..."
                value={filterText}
                onChange={(e) => setFilterText(e.target.value)}
                className="text-sm px-3 py-1.5 bg-transparent border border-border-default rounded-md text-text-default font-semibold placeholder-text-muted focus:outline-none focus:border-blue-500"
              />
              <div className="flex gap-1 text-xs">
                {(['calls', 'errors', 'rate'] as const).map((s) => (
                  <button
                    key={s}
                    onClick={() => setSortBy(s)}
                    className={`px-2 py-1 rounded-md transition-colors ${
                      sortBy === s
                        ? 'bg-blue-500/20 text-blue-400'
                        : 'text-text-muted hover:text-text-default font-semibold hover:bg-background-default'
                    }`}
                  >
                    {s === 'calls' ? 'By Calls' : s === 'errors' ? 'By Errors' : 'By Rate'}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="text-xs text-text-muted uppercase tracking-wider">
                  <th className="text-left py-2 px-4 font-medium">Tool</th>
                  <th className="text-left py-2 px-4 font-medium">Calls</th>
                  <th className="text-left py-2 px-4 font-medium">Success Rate</th>
                  <th className="text-left py-2 px-4 font-medium">Errors</th>
                  <th className="text-left py-2 px-4 font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {filteredTools.map((tool) => (
                  <ToolRow key={tool.tool_name} tool={tool} maxCalls={maxCalls} />
                ))}
              </tbody>
            </table>
            {filteredTools.length === 0 && (
              <div className="text-center py-8 text-text-muted text-sm">
                {filterText ? 'No tools match your filter' : 'No tool usage data yet'}
              </div>
            )}
          </div>
        </div>

        {/* Extension Breakdown */}
        {analytics.extension_usage.length > 0 && (
          <div className="bg-background-default rounded-xl p-4 border border-border-default">
            <h3 className="text-sm font-medium text-text-default font-semibold mb-3">Extension Breakdown</h3>
            <div className="space-y-2">
              {analytics.extension_usage.map((ext) => {
                const maxExtCalls = Math.max(
                  ...analytics.extension_usage.map((e) => e.total_calls),
                  1
                );
                const barWidth = (ext.total_calls / maxExtCalls) * 100;
                return (
                  <div key={ext.extension} className="flex items-center gap-3">
                    <span className="text-sm text-text-muted w-32 truncate">{ext.extension}</span>
                    <div className="flex-1 h-2 bg-black/20 rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full"
                        style={{
                          width: `${barWidth}%`,
                          backgroundColor: getHealthColor(ext.success_rate),
                        }}
                      />
                    </div>
                    <span className="text-xs text-text-muted w-16 text-right">
                      {formatNumber(ext.total_calls)}
                    </span>
                    <span
                      className="text-xs w-14 text-right"
                      style={{ color: getHealthColor(ext.success_rate) }}
                    >
                      {ext.success_rate.toFixed(0)}%
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
