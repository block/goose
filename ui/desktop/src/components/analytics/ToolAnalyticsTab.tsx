import { useEffect, useState } from 'react';
import {
  Area,
  AreaChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { AgentPerformanceMetrics, ToolAnalytics } from '../../api';
import { getAgentPerformance, getToolAnalytics } from '../../api';

const COLORS = [
  'var(--chart-1)',
  'var(--chart-2)',
  'var(--chart-3)',
  'var(--chart-4)',
  'var(--chart-5)',
  'var(--chart-6)',
];

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function shortenToolName(name: string): string {
  // "developer__shell" → "shell", "code_execution__execute_code" → "execute_code"
  const parts = name.split('__');
  return parts.length > 1 ? parts[parts.length - 1] : name;
}

function MetricCard({
  label,
  value,
  sub,
  color,
}: {
  label: string;
  value: string;
  sub?: string;
  color?: string;
}) {
  const ariaLabel = `${label}: ${value}${sub ? ` (${sub})` : ''}`;

  return (
    <section
      className="bg-background-muted rounded-xl p-4 border border-border-muted"
      aria-label={ariaLabel}
    >
      <div className="text-xs text-text-muted mb-1">{label}</div>
      <div className={`text-2xl font-bold ${color || 'text-text-default'}`}>{value}</div>
      {sub && <div className="text-xs text-text-muted mt-1">{sub}</div>}
    </section>
  );
}

function ToolTable({ tools }: { tools: ToolAnalytics['tool_usage'] }) {
  const [sortBy, setSortBy] = useState<'calls' | 'errors' | 'rate'>('calls');

  const ariaSortFor = (column: typeof sortBy): 'none' | 'ascending' | 'descending' => {
    if (sortBy !== column) return 'none';
    // calls/errors are sorted desc; success rate is sorted asc.
    return column === 'rate' ? 'ascending' : 'descending';
  };

  const srSortLabel = (column: typeof sortBy) => {
    if (sortBy !== column) return '';
    return column === 'rate' ? 'Sorted ascending.' : 'Sorted descending.';
  };
  const sorted = [...tools].sort((a, b) => {
    if (sortBy === 'calls') return b.call_count - a.call_count;
    if (sortBy === 'errors') return b.error_count - a.error_count;
    return a.success_rate - b.success_rate;
  });

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="text-text-muted border-b border-border-default">
            <th className="text-left py-2 px-3">Tool</th>
            <th className="text-left py-2 px-3">Extension</th>
            <th className="text-right py-2 px-3" aria-sort={ariaSortFor('calls')}>
              <button
                type="button"
                className="inline-flex items-center justify-end gap-1 hover:text-text-default"
                onClick={() => setSortBy('calls')}
              >
                Calls <span aria-hidden="true">{sortBy === 'calls' && '↓'}</span>
                <span className="sr-only">{srSortLabel('calls')}</span>
              </button>
            </th>
            <th className="text-right py-2 px-3" aria-sort={ariaSortFor('errors')}>
              <button
                type="button"
                className="inline-flex items-center justify-end gap-1 hover:text-text-default"
                onClick={() => setSortBy('errors')}
              >
                Errors <span aria-hidden="true">{sortBy === 'errors' && '↓'}</span>
                <span className="sr-only">{srSortLabel('errors')}</span>
              </button>
            </th>
            <th className="text-right py-2 px-3" aria-sort={ariaSortFor('rate')}>
              <button
                type="button"
                className="inline-flex items-center justify-end gap-1 hover:text-text-default"
                onClick={() => setSortBy('rate')}
              >
                Success Rate <span aria-hidden="true">{sortBy === 'rate' && '↑'}</span>
                <span className="sr-only">{srSortLabel('rate')}</span>
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((tool) => (
            <tr
              key={tool.tool_name}
              className="border-b border-border-muted hover:bg-background-muted"
            >
              <td className="py-2 px-3 font-mono text-xs text-text-default">
                {shortenToolName(tool.tool_name)}
              </td>
              <td className="py-2 px-3 text-text-muted text-xs">{tool.extension || '—'}</td>
              <td className="py-2 px-3 text-right text-text-default">
                {tool.call_count.toLocaleString()}
              </td>
              <td className="py-2 px-3 text-right">
                <span className={tool.error_count > 0 ? 'text-text-danger' : 'text-text-muted'}>
                  {tool.error_count}
                </span>
              </td>
              <td className="py-2 px-3 text-right">
                <div className="flex items-center justify-end gap-2">
                  <div className="w-16 h-1.5 bg-background-muted rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full"
                      style={{
                        width: `${tool.success_rate * 100}%`,
                        backgroundColor:
                          tool.success_rate >= 0.95
                            ? 'var(--text-success)'
                            : tool.success_rate >= 0.8
                              ? 'var(--text-warning)'
                              : 'var(--text-danger)',
                      }}
                    />
                  </div>
                  <span
                    className={
                      tool.success_rate >= 0.95
                        ? 'text-text-success'
                        : tool.success_rate >= 0.8
                          ? 'text-text-warning'
                          : 'text-text-danger'
                    }
                  >
                    {formatPercent(tool.success_rate)}
                  </span>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ExtensionBreakdown({ extensions }: { extensions: ToolAnalytics['extension_usage'] }) {
  const data = extensions.map((e) => ({ name: e.extension, value: e.total_calls }));
  return (
    <div className="flex items-center gap-6">
      <ResponsiveContainer width={200} height={200}>
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            innerRadius={50}
            outerRadius={80}
            dataKey="value"
            nameKey="name"
            stroke="none"
          >
            {data.map((entry, i) => (
              <Cell key={entry.name} fill={COLORS[i % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip
            contentStyle={{
              backgroundColor: 'var(--background-muted)',
              border: '1px solid var(--border-default)',
              borderRadius: 8,
              color: 'var(--text-default)',
            }}
            formatter={(value) => [
              typeof value === 'number' ? value.toLocaleString() : value,
              'Calls',
            ]}
          />
        </PieChart>
      </ResponsiveContainer>
      <div className="flex-1 space-y-2">
        {extensions.map((ext, i) => (
          <div key={ext.extension} className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded-sm"
              style={{ backgroundColor: COLORS[i % COLORS.length] }}
            />
            <span className="text-sm text-text-default flex-1">{ext.extension}</span>
            <span className="text-xs text-text-muted">{ext.total_calls} calls</span>
            <span
              className={`text-xs ${ext.success_rate >= 0.95 ? 'text-text-success' : ext.success_rate >= 0.8 ? 'text-text-warning' : 'text-text-danger'}`}
            >
              {formatPercent(ext.success_rate)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

function DailyActivityChart({ data }: { data: ToolAnalytics['daily_tool_activity'] }) {
  return (
    <ResponsiveContainer width="100%" height={220}>
      <AreaChart data={data}>
        <defs>
          <linearGradient id="toolCallsGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="var(--chart-2)" stopOpacity={0.3} />
            <stop offset="95%" stopColor="var(--chart-2)" stopOpacity={0} />
          </linearGradient>
          <linearGradient id="toolErrorsGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="var(--chart-5)" stopOpacity={0.3} />
            <stop offset="95%" stopColor="var(--chart-5)" stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="var(--border-default)" />
        <XAxis
          dataKey="date"
          tick={{ fontSize: 11, fill: 'var(--text-muted)' }}
          tickFormatter={(v) => v.slice(5)}
        />
        <YAxis tick={{ fontSize: 11, fill: 'var(--text-muted)' }} />
        <Tooltip
          contentStyle={{
            backgroundColor: 'var(--background-muted)',
            border: '1px solid var(--border-default)',
            borderRadius: 8,
            color: 'var(--text-default)',
          }}
          labelFormatter={(label) => `Date: ${label}`}
        />
        <Area
          type="monotone"
          dataKey="tool_calls"
          name="Tool Calls"
          stroke="var(--chart-2)"
          fill="url(#toolCallsGrad)"
        />
        <Area
          type="monotone"
          dataKey="tool_errors"
          name="Errors"
          stroke="var(--chart-5)"
          fill="url(#toolErrorsGrad)"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

function SessionToolTable({ sessions }: { sessions: ToolAnalytics['session_tool_summary'] }) {
  return (
    <div className="overflow-x-auto max-h-[300px] overflow-y-auto">
      <table className="w-full text-sm">
        <thead className="sticky top-0 bg-background-default">
          <tr className="text-text-muted border-b border-border-default">
            <th className="text-left py-2 px-3">Session</th>
            <th className="text-right py-2 px-3">Tool Calls</th>
            <th className="text-right py-2 px-3">Errors</th>
            <th className="text-right py-2 px-3">Unique Tools</th>
            <th className="text-left py-2 px-3">Top Tool</th>
            <th className="text-right py-2 px-3">Date</th>
          </tr>
        </thead>
        <tbody>
          {sessions.slice(0, 20).map((s) => (
            <tr
              key={s.session_id}
              className="border-b border-border-muted hover:bg-background-muted"
            >
              <td className="py-2 px-3 text-text-default text-xs truncate max-w-[200px]">
                {s.session_name || s.session_id.slice(0, 8)}
              </td>
              <td className="py-2 px-3 text-right text-text-default">{s.tool_calls}</td>
              <td className="py-2 px-3 text-right">
                <span className={s.tool_errors > 0 ? 'text-text-danger' : 'text-text-muted'}>
                  {s.tool_errors}
                </span>
              </td>
              <td className="py-2 px-3 text-right text-text-default">{s.unique_tools}</td>
              <td className="py-2 px-3 text-text-muted font-mono text-xs">
                {shortenToolName(s.most_used_tool)}
              </td>
              <td className="py-2 px-3 text-right text-text-muted text-xs">
                {s.created_at.slice(0, 10)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function AgentPerformanceView({ data }: { data: AgentPerformanceMetrics }) {
  return (
    <div className="space-y-6">
      {/* Provider breakdown */}
      <div>
        <h4 className="text-sm font-medium text-text-default mb-3">Sessions by Provider</h4>
        <div className="grid grid-cols-2 gap-3">
          {data.sessions_by_provider.map((p) => (
            <div
              key={p.provider}
              className="bg-background-muted rounded-lg p-3 border border-border-muted"
            >
              <div className="text-xs text-text-muted">{p.provider}</div>
              <div className="text-lg font-bold text-text-default">{p.session_count} sessions</div>
              <div className="text-xs text-text-muted mt-1">
                Avg {Math.round(p.avg_tokens).toLocaleString()} tokens · {p.avg_messages.toFixed(1)}{' '}
                msgs/session
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Session duration stats */}
      <div>
        <h4 className="text-sm font-medium text-text-default mb-3">Session Duration</h4>
        <div className="grid grid-cols-3 gap-3">
          <div className="bg-background-muted rounded-lg p-3 border border-border-muted text-center">
            <div className="text-xs text-text-muted">Average</div>
            <div className="text-lg font-bold text-text-default">
              {formatDuration(data.session_duration_stats.avg_seconds)}
            </div>
          </div>
          <div className="bg-background-muted rounded-lg p-3 border border-border-muted text-center">
            <div className="text-xs text-text-muted">Median</div>
            <div className="text-lg font-bold text-text-default">
              {formatDuration(data.session_duration_stats.median_seconds)}
            </div>
          </div>
          <div className="bg-background-muted rounded-lg p-3 border border-border-muted text-center">
            <div className="text-xs text-text-muted">P90</div>
            <div className="text-lg font-bold text-text-default">
              {formatDuration(data.session_duration_stats.p90_seconds)}
            </div>
          </div>
        </div>
      </div>

      {/* Active extensions */}
      {data.active_extensions.length > 0 && (
        <div>
          <h4 className="text-sm font-medium text-text-default mb-3">Active Extensions</h4>
          <div className="flex flex-wrap gap-2">
            {data.active_extensions.map((ext) => (
              <span
                key={ext.extension}
                className="px-2.5 py-1 text-xs rounded-full bg-indigo-900/30 text-indigo-300 border border-indigo-700/30"
              >
                {ext.extension} · {ext.session_count} sessions
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  return `${(seconds / 3600).toFixed(1)}h`;
}

function LoadingSkeleton() {
  const kpiSkeletonKeys = ['kpi-1', 'kpi-2', 'kpi-3', 'kpi-4'];
  return (
    <div className="space-y-6 animate-pulse">
      <div className="grid grid-cols-4 gap-4">
        {kpiSkeletonKeys.map((key) => (
          <div key={key} className="h-20 bg-background-muted rounded-xl" />
        ))}
      </div>
      <div className="h-[220px] bg-background-muted rounded-xl" />
      <div className="h-[300px] bg-background-muted rounded-xl" />
    </div>
  );
}

export default function ToolAnalyticsTab() {
  const [toolData, setToolData] = useState<ToolAnalytics | null>(null);
  const [agentData, setAgentData] = useState<AgentPerformanceMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<'tools' | 'extensions' | 'sessions' | 'agents'>('tools');

  const tabs = [
    { key: 'tools' as const, label: 'Tool Usage' },
    { key: 'extensions' as const, label: 'Extensions' },
    { key: 'sessions' as const, label: 'Sessions' },
    { key: 'agents' as const, label: 'Agent Performance' },
  ];

  const tabId = (key: (typeof tabs)[number]['key']) => `tool-analytics-tab-${key}`;
  const panelId = (key: (typeof tabs)[number]['key']) => `tool-analytics-panel-${key}`;

  const handleTabKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, currentKey: typeof view) => {
    const currentIndex = tabs.findIndex((t) => t.key === currentKey);
    if (currentIndex === -1) return;

    const focusTab = (nextIndex: number) => {
      const next = tabs[nextIndex];
      setView(next.key);
      document.getElementById(tabId(next.key))?.focus();
    };

    if (e.key === 'ArrowRight') {
      e.preventDefault();
      focusTab((currentIndex + 1) % tabs.length);
      return;
    }
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      focusTab((currentIndex - 1 + tabs.length) % tabs.length);
      return;
    }
    if (e.key === 'Home') {
      e.preventDefault();
      focusTab(0);
      return;
    }
    if (e.key === 'End') {
      e.preventDefault();
      focusTab(tabs.length - 1);
    }
  };

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const [toolRes, agentRes] = await Promise.all([
          getToolAnalytics({ query: { days: 30 } }),
          getAgentPerformance({ query: { days: 30 } }),
        ]);
        if (toolRes.data) setToolData(toolRes.data);
        if (agentRes.data) setAgentData(agentRes.data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load analytics');
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, []);

  if (loading) return <LoadingSkeleton />;
  if (error) return <div className="text-text-danger text-center py-8">{error}</div>;
  if (!toolData)
    return <div className="text-text-muted text-center py-8">No tool data available</div>;

  return (
    <div className="space-y-6">
      {/* KPI Cards */}
      <div className="grid grid-cols-4 gap-4">
        <MetricCard
          label="Total Tool Calls"
          value={toolData.total_tool_calls.toLocaleString()}
          sub={`${toolData.tool_usage.length} unique tools`}
        />
        <MetricCard
          label="Success Rate"
          value={formatPercent(toolData.success_rate)}
          color={
            toolData.success_rate >= 0.95
              ? 'text-text-success'
              : toolData.success_rate >= 0.8
                ? 'text-text-warning'
                : 'text-text-danger'
          }
          sub={`${toolData.total_tool_errors} errors`}
        />
        <MetricCard
          label="Extensions"
          value={String(toolData.extension_usage.length)}
          sub={`${toolData.extension_usage.filter((e) => e.success_rate >= 0.95).length} healthy`}
        />
        <MetricCard
          label="Avg Tools/Session"
          value={agentData ? agentData.avg_tools_per_session.toFixed(1) : '—'}
          sub={agentData ? `${agentData.avg_messages_per_session.toFixed(1)} msgs/session` : ''}
        />
      </div>

      {/* Sub-navigation */}
      <div
        role="tablist"
        aria-label="Tool analytics views"
        className="flex gap-1 border-b border-border-default pb-0"
      >
        {tabs.map((tab) => (
          <button
            key={tab.key}
            id={tabId(tab.key)}
            type="button"
            role="tab"
            aria-selected={view === tab.key}
            aria-controls={panelId(tab.key)}
            tabIndex={view === tab.key ? 0 : -1}
            onKeyDown={(e) => handleTabKeyDown(e, tab.key)}
            onClick={() => setView(tab.key)}
            className={`px-4 py-2 text-sm font-medium rounded-t-lg transition-colors ${
              view === tab.key
                ? 'text-text-default bg-background-muted border-b-2 border-indigo-500'
                : 'text-text-muted hover:text-text-default'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Daily activity chart (always visible) */}
      {toolData.daily_tool_activity.length > 0 && (
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">Daily Tool Activity</h3>
          <DailyActivityChart data={toolData.daily_tool_activity} />
        </div>
      )}

      {/* View-specific content */}
      <div
        id={panelId('tools')}
        role="tabpanel"
        aria-labelledby={tabId('tools')}
        hidden={view !== 'tools'}
      >
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">
            Tool Usage ({toolData.tool_usage.length} tools)
          </h3>
          <ToolTable tools={toolData.tool_usage} />
        </div>
      </div>

      <div
        id={panelId('extensions')}
        role="tabpanel"
        aria-labelledby={tabId('extensions')}
        hidden={view !== 'extensions'}
      >
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">Extension Breakdown</h3>
          {toolData.extension_usage.length > 0 ? (
            <ExtensionBreakdown extensions={toolData.extension_usage} />
          ) : (
            <div className="text-text-muted text-center py-8">No extension data available</div>
          )}
        </div>
      </div>

      <div
        id={panelId('sessions')}
        role="tabpanel"
        aria-labelledby={tabId('sessions')}
        hidden={view !== 'sessions'}
      >
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">
            Recent Sessions ({toolData.session_tool_summary.length})
          </h3>
          {toolData.session_tool_summary.length > 0 ? (
            <SessionToolTable sessions={toolData.session_tool_summary} />
          ) : (
            <div className="text-text-muted text-center py-8">No session data available</div>
          )}
        </div>
      </div>

      <div
        id={panelId('agents')}
        role="tabpanel"
        aria-labelledby={tabId('agents')}
        hidden={view !== 'agents'}
      >
        {agentData ? (
          <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
            <h3 className="text-sm font-medium text-text-default mb-3">Agent Performance</h3>
            <AgentPerformanceView data={agentData} />
          </div>
        ) : (
          <div className="text-text-muted text-center py-8">No agent performance data</div>
        )}
      </div>
    </div>
  );
}
