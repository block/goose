import { useEffect, useState } from 'react';
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { getEvalTopics, type TopicAnalytics } from '@/api';

const COLORS = [
  '#3b82f6',
  '#22c55e',
  '#a855f7',
  '#f59e0b',
  '#06b6d4',
  '#ef4444',
  '#ec4899',
  '#8b5cf6',
];

function formatPercent(v: number): string {
  return `${(v * 100).toFixed(1)}%`;
}

function AccuracyBar({ value }: { value: number }) {
  const color = value >= 0.9 ? '#22c55e' : value >= 0.7 ? '#f59e0b' : '#ef4444';
  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 h-2 bg-background-muted rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${value * 100}%`, backgroundColor: color }}
        />
      </div>
      <span className="text-xs text-text-default w-12 text-right">{formatPercent(value)}</span>
    </div>
  );
}

export default function TopicsTab() {
  const [topics, setTopics] = useState<TopicAnalytics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedTopic, setExpandedTopic] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<'name' | 'accuracy' | 'count'>('count');
  const [sortAsc, setSortAsc] = useState(false);

  useEffect(() => {
    const fetchTopics = async () => {
      try {
        setLoading(true);
        const res = await getEvalTopics();
        if (res.data) setTopics(res.data);
        setError(null);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load topics');
      } finally {
        setLoading(false);
      }
    };
    fetchTopics();
  }, []);

  const sortedTopics = [...topics].sort((a, b) => {
    let cmp = 0;
    if (sortBy === 'name') cmp = a.topic.localeCompare(b.topic);
    else if (sortBy === 'accuracy') cmp = a.accuracy - b.accuracy;
    else cmp = a.caseCount - b.caseCount;
    return sortAsc ? cmp : -cmp;
  });

  const handleSort = (col: 'name' | 'accuracy' | 'count') => {
    if (sortBy === col) setSortAsc(!sortAsc);
    else {
      setSortBy(col);
      setSortAsc(false);
    }
  };

  if (loading) {
    return (
      <div className="space-y-4 animate-pulse">
        <div className="h-8 w-48 rounded bg-background-muted" />
        <div className="h-64 rounded-lg bg-background-muted" />
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={`topic-skeleton-${i + 1}`} className="h-14 rounded-lg bg-background-muted" />
        ))}
      </div>
    );
  }

  if (topics.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-64 text-text-muted">
        <p className="text-lg mb-2">No topic data available</p>
        <p className="text-sm">
          Add tags to your test cases and run evaluations to see topic-level analytics
        </p>
      </div>
    );
  }

  const chartData = sortedTopics.slice(0, 12).map((t) => ({
    name: t.topic,
    accuracy: t.accuracy,
    cases: t.caseCount,
  }));

  return (
    <div className="space-y-6">
      {error && (
        <div className="rounded-lg bg-background-danger-muted border border-border-default p-3 text-text-danger text-sm">
          {error}
        </div>
      )}

      <h3 className="text-lg font-semibold text-text-default">Topic Analytics</h3>

      {/* Topic Accuracy Chart */}
      <div className="rounded-lg border border-border-default bg-background-muted p-4">
        <h4 className="text-sm font-medium text-text-default mb-4">Accuracy by Topic</h4>
        <ResponsiveContainer width="100%" height={Math.max(200, chartData.length * 40)}>
          <BarChart data={chartData} layout="vertical">
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
              dataKey="name"
              tick={{ fill: '#e5e7eb', fontSize: 12 }}
              stroke="#4b5563"
              width={120}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: '#1f2937',
                border: '1px solid #374151',
                borderRadius: '8px',
              }}
              formatter={(value, name) => {
                if (String(name) === 'accuracy') {
                  const v = typeof value === 'number' ? value : 0;
                  return [`${(v * 100).toFixed(1)}%`, 'Accuracy'];
                }
                return [String(value), String(name)];
              }}
            />
            <Bar dataKey="accuracy" radius={[0, 4, 4, 0]}>
              {chartData.map((entry) => (
                <Cell
                  key={entry.name}
                  fill={
                    entry.accuracy >= 0.9
                      ? '#22c55e'
                      : entry.accuracy >= 0.7
                        ? '#f59e0b'
                        : '#ef4444'
                  }
                  opacity={0.85}
                />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>

      {/* Topic Table */}
      <div className="rounded-lg border border-border-default overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="bg-background-muted">
              <th
                className="text-left px-4 py-3 text-text-muted font-medium cursor-pointer hover:text-text-default"
                onClick={() => handleSort('name')}
              >
                Topic {sortBy === 'name' && (sortAsc ? '↑' : '↓')}
              </th>
              <th
                className="text-center px-4 py-3 text-text-muted font-medium cursor-pointer hover:text-text-default"
                onClick={() => handleSort('count')}
              >
                Cases {sortBy === 'count' && (sortAsc ? '↑' : '↓')}
              </th>
              <th
                className="text-left px-4 py-3 text-text-muted font-medium cursor-pointer hover:text-text-default w-64"
                onClick={() => handleSort('accuracy')}
              >
                Accuracy {sortBy === 'accuracy' && (sortAsc ? '↑' : '↓')}
              </th>
              <th className="text-left px-4 py-3 text-text-muted font-medium">
                Agent Distribution
              </th>
            </tr>
          </thead>
          <tbody>
            {sortedTopics.map((topic) => (
              <tr
                key={topic.topic}
                className="border-t border-border-muted hover:bg-background-muted cursor-pointer"
                onClick={() => setExpandedTopic(expandedTopic === topic.topic ? null : topic.topic)}
              >
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <span
                      className={`text-xs transition-transform ${expandedTopic === topic.topic ? 'rotate-90' : ''}`}
                    >
                      ▶
                    </span>
                    <span className="text-text-default font-medium">{topic.topic}</span>
                  </div>
                </td>
                <td className="px-4 py-3 text-center text-text-default">{topic.caseCount}</td>
                <td className="px-4 py-3">
                  <AccuracyBar value={topic.accuracy} />
                </td>
                <td className="px-4 py-3">
                  <div className="flex gap-1 flex-wrap">
                    {topic.agentDistribution.slice(0, 4).map((ad, i) => (
                      <span
                        key={ad.agent}
                        className="text-xs px-2 py-0.5 rounded-full border border-border-default"
                        style={{ color: COLORS[i % COLORS.length] }}
                      >
                        {ad.agent}: {ad.count}
                      </span>
                    ))}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
