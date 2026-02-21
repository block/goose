import { useEffect, useRef, useState } from 'react';
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

const MAX_HEIGHT = 220;

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

function ChartInner({
  type,
  data,
  xKey,
  yKeys,
  height = 180,
  title,
  colors = CHART_COLORS,
  className,
}: ChartProps) {
  const effectiveHeight = Math.min(height, MAX_HEIGHT);

  if (type === 'pie') {
    return (
      <div className={`space-y-1 ${className ?? ''}`}>
        {title && <h4 className="text-xs font-medium text-text-muted">{title}</h4>}
        <ResponsiveContainer width="100%" height={effectiveHeight}>
          <PieChart>
            <Pie data={data} dataKey={yKeys[0]} nameKey={xKey} cx="50%" cy="50%" outerRadius={60}>
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
                borderRadius: '6px',
                fontSize: '12px',
                color: 'var(--text-default)',
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
    <div className={`space-y-1 ${className ?? ''}`}>
      {title && <h4 className="text-xs font-medium text-text-muted">{title}</h4>}
      <ResponsiveContainer width="100%" height={effectiveHeight}>
        <ChartComponent data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border-default)" />
          <XAxis dataKey={xKey} tick={{ fill: 'var(--text-muted)', fontSize: 10 }} />
          <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 10 }} width={35} />
          <Tooltip
            contentStyle={{
              backgroundColor: 'var(--background-muted)',
              border: '1px solid var(--border-default)',
              borderRadius: '6px',
              fontSize: '12px',
              color: 'var(--text-default)',
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

export function Chart(props: ChartProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { threshold: 0.1 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const effectiveHeight = Math.min(props.height ?? 180, MAX_HEIGHT);

  const ariaLabel = props.title
    ? `${props.title} (${props.type} chart)`
    : `${props.type} chart: ${props.yKeys.join(', ')} by ${props.xKey}`;

  return (
    <div
      ref={ref}
      role="img"
      aria-label={ariaLabel}
      className="overflow-hidden"
      style={{ minHeight: effectiveHeight, maxHeight: effectiveHeight + 24 }}
    >
      {visible ? (
        <ChartInner {...props} />
      ) : (
        <div
          className="animate-pulse rounded bg-background-muted"
          style={{ height: effectiveHeight }}
        />
      )}
    </div>
  );
}
