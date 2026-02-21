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

const CHART_COLORS = ['#10b981', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6', '#06b6d4'];

export interface ChartProps {
  type: 'bar' | 'line' | 'area' | 'pie';
  data: Array<Record<string, string | number>>;
  xKey: string;
  yKeys: string[];
  height?: number;
  title?: string;
  colors?: string[];
  className?: string;
}

export function Chart({
  type,
  data,
  xKey,
  yKeys,
  height = 300,
  title,
  colors = CHART_COLORS,
  className,
}: ChartProps) {
  if (type === 'pie') {
    return (
      <div className={`space-y-2 ${className ?? ''}`}>
        {title && <h4 className="text-sm font-medium text-text-default">{title}</h4>}
        <ResponsiveContainer width="100%" height={height}>
          <PieChart>
            <Pie data={data} dataKey={yKeys[0]} nameKey={xKey} cx="50%" cy="50%" outerRadius={80}>
              {data.map((entry, i) => (
                <Cell
                  key={entry[xKey]?.toString() ?? `cell-${i}`}
                  fill={colors[i % colors.length]}
                />
              ))}
            </Pie>
            <Tooltip
              contentStyle={{
                backgroundColor: 'var(--background-muted)',
                border: '1px solid var(--border-default)',
                borderRadius: '8px',
              }}
            />
          </PieChart>
        </ResponsiveContainer>
      </div>
    );
  }

  const ChartComponent = type === 'bar' ? BarChart : type === 'line' ? LineChart : AreaChart;
  const DataComponent = type === 'bar' ? Bar : type === 'line' ? Line : Area;

  return (
    <div className={`space-y-2 ${className ?? ''}`}>
      {title && <h4 className="text-sm font-medium text-text-default">{title}</h4>}
      <ResponsiveContainer width="100%" height={height}>
        <ChartComponent data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border-default)" />
          <XAxis dataKey={xKey} tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: 'var(--background-muted)',
              border: '1px solid var(--border-default)',
              borderRadius: '8px',
            }}
          />
          {yKeys.map((key, i) => (
            <DataComponent
              key={key}
              type="monotone"
              dataKey={key}
              fill={colors[i % colors.length]}
              stroke={colors[i % colors.length]}
              fillOpacity={type === 'area' ? 0.3 : 1}
            />
          ))}
        </ChartComponent>
      </ResponsiveContainer>
    </div>
  );
}
