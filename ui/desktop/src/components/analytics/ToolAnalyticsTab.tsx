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
  '#6366f1',
  '#22d3ee',
  '#f59e0b',
  '#10b981',
  '#f43f5e',
  '#8b5cf6',
  '#ec4899',
  '#14b8a6',
];
const SUCCESS_COLOR = '#10b981';
const ERROR_COLOR = '#f43f5e';

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
  return (
    <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
      <div className="text-xs text-text-muted mb-1">{label}</div>
      <div className={`text-2xl font-bold ${color || 'text-text-default'}`}>{value}</div>
      {sub && <div className="text-xs text-text-muted mt-1">{sub}</div>}
    </div>
  );
}

function ToolTable({ tools }: { tools: ToolAnalytics['tool_usage'] }) {
  const [sortBy, setSortBy] = useState<'calls' | 'errors' | 'rate'>('calls');
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
            <th
              className="text-right py-2 px-3 cursor-pointer hover:text-text-default"
              onClick={() => setSortBy('calls')}
            >
              Calls {sortBy === 'calls' && '↓'}
            </th>
            <th
              className="text-right py-2 px-3 cursor-pointer hover:text-text-default"
              onClick={() => setSortBy('errors')}
            >
              Errors {sortBy === 'errors' && '↓'}
            </th>
            <th
              className="text-right py-2 px-3 cursor-pointer hover:text-text-default"
              onClick={() => setSortBy('rate')}
            >
              Success Rate {sortBy === 'rate' && '↑'}
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
                            ? SUCCESS_COLOR
                            : tool.success_rate >= 0.8
                              ? '#f59e0b'
                              : ERROR_COLOR,
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
            {data.map((_, i) => (
              <Cell key={i} fill={COLORS[i % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip
            contentStyle={{ background: '#1f2937', border: '1px solid #374151', borderRadius: 8 }}
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
            <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
          </linearGradient>
          <linearGradient id="toolErrorsGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#f43f5e" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#f43f5e" stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
        <XAxis
          dataKey="date"
          tick={{ fontSize: 11, fill: '#9ca3af' }}
          tickFormatter={(v) => v.slice(5)}
        />
        <YAxis tick={{ fontSize: 11, fill: '#9ca3af' }} />
        <Tooltip
          contentStyle={{ background: '#1f2937', border: '1px solid #374151', borderRadius: 8 }}
          labelFormatter={(label) => `Date: ${label}`}
        />
        <Area
          type="monotone"
          dataKey="tool_calls"
          name="Tool Calls"
          stroke="#6366f1"
          fill="url(#toolCallsGrad)"
        />
        <Area
          type="monotone"
          dataKey="tool_errors"
          name="Errors"
          stroke="#f43f5e"
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
  return (
    <div className="space-y-6 animate-pulse">
      <div className="grid grid-cols-4 gap-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-20 bg-background-muted rounded-xl" />
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
      <div className="flex gap-1 border-b border-border-default pb-0">
        {(['tools', 'extensions', 'sessions', 'agents'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setView(tab)}
            className={`px-4 py-2 text-sm font-medium rounded-t-lg transition-colors ${
              view === tab
                ? 'text-text-default bg-background-muted border-b-2 border-indigo-500'
                : 'text-text-muted hover:text-text-default'
            }`}
          >
            {tab === 'tools'
              ? 'Tool Usage'
              : tab === 'extensions'
                ? 'Extensions'
                : tab === 'sessions'
                  ? 'Sessions'
                  : 'Agent Performance'}
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
      {view === 'tools' && (
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">
            Tool Usage ({toolData.tool_usage.length} tools)
          </h3>
          <ToolTable tools={toolData.tool_usage} />
        </div>
      )}

      {view === 'extensions' && (
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">Extension Breakdown</h3>
          {toolData.extension_usage.length > 0 ? (
            <ExtensionBreakdown extensions={toolData.extension_usage} />
          ) : (
            <div className="text-text-muted text-center py-8">No extension data available</div>
          )}
        </div>
      )}

      {view === 'sessions' && (
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
      )}

      {view === 'agents' && agentData && (
        <div className="bg-background-muted rounded-xl p-4 border border-border-muted">
          <h3 className="text-sm font-medium text-text-default mb-3">Agent Performance</h3>
          <AgentPerformanceView data={agentData} />
        </div>
      )}
    </div>
  );
}
