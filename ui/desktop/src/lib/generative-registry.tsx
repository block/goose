/**
 * Goose Generative UI Component Registry
 *
 * Maps catalog component names to actual React implementations.
 * These components render the AI-generated JSON specs into real UI.
 *
 * Note: Props are typed as `unknown` by json-render's defineRegistry.
 * We cast to specific shapes in each component for type safety.
 */
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';

const CHART_COLORS = [
  'var(--chart-1)',
  'var(--chart-2)',
  'var(--chart-3)',
  'var(--chart-4)',
  'var(--chart-5)',
  'var(--chart-6)',
];

const statusColors: Record<string, string> = {
  success: 'text-text-success bg-background-success/10 border-text-success/20',
  warning: 'text-text-warning bg-background-warning/10 border-text-warning/20',
  error: 'text-text-danger bg-background-danger/10 border-text-danger/20',
  info: 'text-text-info bg-background-info/10 border-text-info/20',
  neutral: 'text-text-muted bg-text-muted/10 border-text-muted/20',
};

// Helper to safely cast props
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyProps = any;

// @ts-expect-error — zod 3/4 compatibility
export const { registry } = defineRegistry(gooseCatalog, {
  components: {
    // --- Layout ---
    Grid: ({ props: rawProps, children }: { props: AnyProps; children?: React.ReactNode }) => {
      const props = rawProps as { columns: number; gap: string };
      const cols: Record<number, string> = {
        1: 'grid-cols-1',
        2: 'grid-cols-2',
        3: 'grid-cols-3',
        4: 'grid-cols-4',
      };
      const gaps: Record<string, string> = { sm: 'gap-2', md: 'gap-4', lg: 'gap-6' };
      return (
        <div
          className={`grid ${cols[props.columns] || 'grid-cols-2'} ${gaps[props.gap] || 'gap-4'}`}
        >
          {children}
        </div>
      );
    },

    Section: ({ props: rawProps, children }: { props: AnyProps; children?: React.ReactNode }) => {
      const props = rawProps as { title: string; subtitle?: string };
      return (
        <div className="space-y-3">
          <div>
            <h3 className="text-lg font-semibold text-text-default">{props.title}</h3>
            {props.subtitle && <p className="text-sm text-text-muted">{props.subtitle}</p>}
          </div>
          <div>{children}</div>
        </div>
      );
    },

    // --- Data Display ---
    MetricCard: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        label: string;
        value: string;
        delta?: string;
        deltaType?: string;
        description?: string;
      };

      const ariaLabel = `${props.label}: ${props.value}${props.delta ? ` (${props.delta})` : ''}`;
      return (
        <div
          className="bg-background-muted border border-border-default rounded-xl p-4 space-y-1"
          role="group"
          aria-label={ariaLabel}
        >
          <div className="text-sm text-text-muted">{props.label}</div>
          <div className="text-2xl font-bold text-text-default">{props.value}</div>
          {props.delta && (
            <div
              className={`text-sm font-medium ${
                props.deltaType === 'positive'
                  ? 'text-text-success'
                  : props.deltaType === 'negative'
                    ? 'text-text-danger'
                    : 'text-text-muted'
              }`}
            >
              {props.deltaType === 'positive' ? '↑' : props.deltaType === 'negative' ? '↓' : '→'}{' '}
              {props.delta}
            </div>
          )}
          {props.description && <div className="text-xs text-text-muted">{props.description}</div>}
        </div>
      );
    },

    DataTable: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        columns: Array<{ key: string; label: string; align?: string }>;
        rows: Array<Record<string, string | number | boolean>>;
        maxRows?: number;
      };
      const rows = props.rows.slice(0, props.maxRows || 10);

      const ariaLabel = `Data table: ${props.columns.map((c) => c.label).join(', ')}`;
      return (
        <div className="overflow-x-auto rounded-lg border border-border-default">
          <table className="w-full text-sm" aria-label={ariaLabel}>
            <thead>
              <tr className="bg-background-muted">
                {props.columns.map((col) => (
                  <th
                    key={col.key}
                    className={`px-3 py-2 font-medium text-text-default text-${col.align || 'left'}`}
                  >
                    {col.label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-border-default">
              {rows.map((row) => (
                <tr
                  key={props.columns.map((col) => String(row[col.key] ?? '')).join('|')}
                  className="hover:bg-background-active"
                >
                  {props.columns.map((col) => (
                    <td
                      key={col.key}
                      className={`px-3 py-2 text-text-default text-${col.align || 'left'}`}
                    >
                      {String(row[col.key] ?? '')}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    },

    Chart: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        type: string;
        data: Array<Record<string, string | number>>;
        xKey: string;
        yKeys: string[];
        height?: number;
        title?: string;
        colors?: string[];
      };
      const colors = props.colors || CHART_COLORS;
      const height = props.height || 300;

      const ariaLabel = props.title
        ? `${props.title} (${props.type} chart)`
        : `${props.type} chart: ${props.yKeys.join(', ')} by ${props.xKey}`;

      if (props.type === 'pie') {
        return (
          <div className="space-y-2">
            {props.title && (
              <h4 className="text-sm font-medium text-text-default">{props.title}</h4>
            )}
            <div role="img" aria-label={ariaLabel}>
              <ResponsiveContainer width="100%" height={height}>
                <PieChart>
                  <Pie
                    data={props.data}
                    dataKey={props.yKeys[0]}
                    nameKey={props.xKey}
                    cx="50%"
                    cy="50%"
                    outerRadius={80}
                  >
                    {props.data.map((datum, i) => (
                      <Cell key={String(datum[props.xKey] ?? i)} fill={colors[i % colors.length]} />
                    ))}
                  </Pie>
                  <Tooltip
                    contentStyle={{
                      backgroundColor: 'var(--background-muted)',
                      border: '1px solid var(--border-default)',
                      borderRadius: '8px',
                      color: 'var(--text-default)',
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
          </div>
        );
      }

      const ChartComponent =
        props.type === 'bar' ? BarChart : props.type === 'line' ? LineChart : AreaChart;
      const DataComponent = props.type === 'bar' ? Bar : props.type === 'line' ? Line : Area;

      return (
        <div className="space-y-2">
          {props.title && <h4 className="text-sm font-medium text-text-default">{props.title}</h4>}
          <div role="img" aria-label={ariaLabel}>
            <ResponsiveContainer width="100%" height={height}>
              {/* @ts-ignore — Recharts component union type */}
              <ChartComponent data={props.data}>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border-default)" />
                <XAxis dataKey={props.xKey} tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
                <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: 'var(--background-muted)',
                    border: '1px solid var(--border-default)',
                    borderRadius: '8px',
                    color: 'var(--text-default)',
                  }}
                />
                {props.yKeys.map((key, i) => (
                  <DataComponent
                    key={key}
                    type="monotone"
                    dataKey={key}
                    fill={colors[i % colors.length]}
                    stroke={colors[i % colors.length]}
                    fillOpacity={props.type === 'area' ? 0.3 : 1}
                  />
                ))}
              </ChartComponent>
            </ResponsiveContainer>
          </div>
        </div>
      );
    },

    ProgressBar: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { label: string; value: number; color?: string };
      const barColors: Record<string, string> = {
        green: 'bg-green-500',
        yellow: 'bg-yellow-500',
        red: 'bg-red-500',
        blue: 'bg-blue-500',
      };

      const pct = Math.min(100, Math.max(0, props.value));
      return (
        <div className="space-y-1">
          <div className="flex justify-between text-sm">
            <span className="text-text-default">{props.label}</span>
            <span className="text-text-muted">{props.value}%</span>
          </div>
          <div
            className="h-2 bg-background-muted rounded-full overflow-hidden"
            role="progressbar"
            aria-label={props.label || 'Progress'}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={pct}
            aria-valuetext={`${pct}%`}
          >
            <div
              className={`h-full rounded-full transition-all ${barColors[props.color || 'blue'] || 'bg-blue-500'}`}
              style={{ width: `${pct}%` }}
            />
          </div>
        </div>
      );
    },

    // --- Status & Alerts ---
    StatusBadge: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { label: string; status: string };
      return (
        <span
          className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${statusColors[props.status] || statusColors.neutral}`}
        >
          {props.label}
        </span>
      );
    },

    AlertCard: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { title: string; message: string; severity: string };
      const icons: Record<string, string> = { info: 'ℹ️', warning: '⚠️', error: '❌', success: '✅' };
      const bgColors: Record<string, string> = {
        info: 'bg-background-info/10 border-text-info/20',
        warning: 'bg-background-warning/10 border-text-warning/20',
        error: 'bg-background-danger/10 border-text-danger/20',
        success: 'bg-background-success/10 border-text-success/20',
      };
      return (
        <div className={`p-4 rounded-xl border ${bgColors[props.severity] || bgColors.info}`}>
          <div className="flex items-start gap-3">
            <span className="text-lg">{icons[props.severity] || icons.info}</span>
            <div>
              <div className="font-medium text-text-default">{props.title}</div>
              <div className="text-sm text-text-muted mt-1">{props.message}</div>
            </div>
          </div>
        </div>
      );
    },

    // --- Content ---
    Text: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { content: string; variant?: string };
      const styles: Record<string, string> = {
        heading: 'text-xl font-bold text-text-default',
        body: 'text-sm text-text-default',
        caption: 'text-xs text-text-muted',
        code: 'font-mono text-sm text-text-default bg-background-muted px-1 rounded',
      };
      return <p className={styles[props.variant || 'body'] || styles.body}>{props.content}</p>;
    },

    CodeBlock: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { code: string; title?: string };
      return (
        <div className="rounded-lg overflow-hidden border border-border-default">
          {props.title && (
            <div className="bg-background-muted px-3 py-1.5 text-xs text-text-muted border-b border-border-default">
              {props.title}
            </div>
          )}
          <pre className="bg-background-default p-3 text-sm text-text-default overflow-x-auto">
            <code>{props.code}</code>
          </pre>
        </div>
      );
    },

    List: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        items: Array<{ label: string; description?: string; status?: string }>;
        ordered?: boolean;
      };
      const Tag = props.ordered ? 'ol' : 'ul';
      return (
        <Tag className={`space-y-2 ${props.ordered ? 'list-decimal list-inside' : ''}`}>
          {props.items.map((item) => (
            <li
              key={`${item.label}|${item.status ?? ''}|${item.description ?? ''}`}
              className="flex items-start gap-2"
            >
              {item.status && (
                <span
                  className={`mt-0.5 w-2 h-2 rounded-full flex-shrink-0 ${
                    item.status === 'success'
                      ? 'bg-text-success'
                      : item.status === 'error'
                        ? 'bg-text-danger'
                        : item.status === 'warning'
                          ? 'bg-text-warning'
                          : 'bg-text-info'
                  }`}
                />
              )}
              <div>
                <span className="text-sm text-text-default">{item.label}</span>
                {item.description && (
                  <span className="text-xs text-text-muted ml-2">{item.description}</span>
                )}
              </div>
            </li>
          ))}
        </Tag>
      );
    },

    // --- Interactive ---
    ActionButton: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as { label: string; variant?: string };
      const variants: Record<string, string> = {
        primary: 'bg-blue-600 hover:bg-blue-500 text-white',
        secondary: 'bg-background-muted hover:bg-background-active text-text-default',
        destructive: 'bg-red-600 hover:bg-red-500 text-white',
        ghost: 'hover:bg-background-active text-text-default',
      };
      return (
        <button
          type="button"
          className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${variants[props.variant || 'primary'] || variants.primary}`}
        >
          {props.label}
        </button>
      );
    },

    // --- Goose-Specific ---
    SessionCard: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        name: string;
        provider?: string;
        messageCount?: number;
        tokenCount?: number;
        createdAt?: string;
      };
      return (
        <div className="bg-background-muted border border-border-default rounded-xl p-4 hover:border-border-strong transition-colors cursor-pointer">
          <div className="font-medium text-text-default">{props.name}</div>
          <div className="flex items-center gap-3 mt-2 text-xs text-text-muted">
            {props.provider && <span>{props.provider}</span>}
            {props.messageCount != null && <span>{props.messageCount} messages</span>}
            {props.tokenCount != null && <span>{props.tokenCount.toLocaleString()} tokens</span>}
          </div>
          {props.createdAt && <div className="text-xs text-text-muted mt-1">{props.createdAt}</div>}
        </div>
      );
    },

    ToolResult: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        toolName: string;
        status: string;
        duration?: string;
        output?: string;
      };
      return (
        <div
          className={`p-3 rounded-lg border ${props.status === 'success' ? 'border-text-success/20 bg-text-success/5' : 'border-text-danger/20 bg-text-danger/5'}`}
        >
          <div className="flex items-center gap-2">
            <span className={props.status === 'success' ? 'text-text-success' : 'text-text-danger'}>
              {props.status === 'success' ? '✓' : '✗'}
            </span>
            <span className="font-mono text-sm text-text-default">{props.toolName}</span>
            {props.duration && <span className="text-xs text-text-muted">{props.duration}</span>}
          </div>
          {props.output && (
            <pre className="mt-2 text-xs text-text-muted overflow-x-auto">{props.output}</pre>
          )}
        </div>
      );
    },

    EvalResult: ({ props: rawProps }: { props: AnyProps }) => {
      const props = rawProps as {
        datasetName: string;
        accuracy: number;
        agentAccuracy?: number;
        modeAccuracy?: number;
        testCount: number;
        passCount: number;
        failCount: number;
      };
      return (
        <div className="bg-background-muted border border-border-default rounded-xl p-4 space-y-3">
          <div className="flex items-center justify-between">
            <span className="font-medium text-text-default">{props.datasetName}</span>
            <span
              className={`text-lg font-bold ${props.accuracy >= 90 ? 'text-text-success' : props.accuracy >= 70 ? 'text-text-warning' : 'text-text-danger'}`}
            >
              {props.accuracy.toFixed(1)}%
            </span>
          </div>
          <div className="flex gap-4 text-xs text-text-muted">
            <span>✓ {props.passCount} passed</span>
            <span className="text-text-danger">✗ {props.failCount} failed</span>
            <span>/ {props.testCount} total</span>
          </div>
          {(props.agentAccuracy != null || props.modeAccuracy != null) && (
            <div className="flex gap-4 text-xs text-text-muted">
              {props.agentAccuracy != null && <span>Agent: {props.agentAccuracy.toFixed(1)}%</span>}
              {props.modeAccuracy != null && <span>Mode: {props.modeAccuracy.toFixed(1)}%</span>}
            </div>
          )}
        </div>
      );
    },
  },
});
