import { ReactElement, ReactNode } from 'react';

import { BarChart, Bar, LineChart, Line, CartesianGrid, XAxis } from 'recharts';

import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';

import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart';

interface ChartTileProps {
  title: string;
  value: string;
  trend?: string;
  data: number[];
  icon: ReactNode;
  variant?: 'line' | 'bar';
  date?: Date;
}

export default function ChartTile({
  title,
  value,
  trend,
  data,
  icon,
  variant = 'line',
  date,
}: ChartTileProps): ReactElement {
  const { contentCardStyle } = useTimelineStyles(date);

  // Convert the data array to the format expected by recharts
  const chartData = data.map((value, index) => ({
    value,
    point: `P${index + 1}`,
  }));

  // Chart configuration with proper color variables
  const chartConfig = {
    value: {
      label: title,
      color: variant === 'line' ? 'var(--chart-2)' : 'var(--chart-1)',
    },
  } satisfies ChartConfig;

  return (
    <div
      className={`
        flex flex-col justify-between
        w-[320px] min-h-[380px] 
        ${contentCardStyle}
        rounded-[18px]
        relative
        overflow-hidden
        transition-all duration-200
        hover:scale-[1.02]
        bg-background-default text-text-default
      `}
    >
      {/* Header section with icon */}
      <div className="p-4 space-y-4">
        <div className="w-6 h-6 text-text-default dark:text-white">{icon}</div>

        <div>
          <div className="text-text-muted dark:text-white/60 text-sm mb-1">{title}</div>
          <div className="text-text-default dark:text-white text-2xl font-semibold">
            {value}
            {trend && (
              <span className="ml-1 text-sm text-text-muted dark:text-white/60">{trend}</span>
            )}
          </div>
        </div>
      </div>

      {/* Chart Container */}
      <div className="w-full h-[200px] px-4 pb-6">
        <ChartContainer
          config={chartConfig}
          className="[&_.recharts-cartesian-axis-tick_text]:fill-muted-foreground [&_.recharts-cartesian-grid_line]:stroke-border/50 [&_.recharts-curve.recharts-tooltip-cursor]:stroke-border [&_.recharts-rectangle.recharts-tooltip-cursor]:fill-muted [&_.recharts-tooltip-wrapper]:!pointer-events-none"
        >
          {variant === 'line' ? (
            <LineChart
              width={288}
              height={162}
              data={chartData}
              margin={{ top: 10, right: 10, bottom: 0, left: -20 }}
            >
              <CartesianGrid vertical={false} className="stroke-border/50" />
              <XAxis
                dataKey="point"
                tickLine={false}
                tickMargin={10}
                axisLine={false}
                height={40}
                tick={{ fill: 'var(--text-muted)' }}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent className="border-border/50 bg-background-default text-text-default min-w-[180px] [&_.flex.flex-1]:gap-4 [&_.flex.flex-1>span]:whitespace-nowrap" />
                }
              />
              <Line
                type="monotone"
                dataKey="value"
                stroke="var(--chart-2)"
                strokeWidth={2}
                dot={{ fill: 'var(--chart-2)', r: 4 }}
              />
            </LineChart>
          ) : (
            <BarChart
              width={288}
              height={162}
              data={chartData}
              margin={{ top: 10, right: 10, bottom: 0, left: 10 }}
            >
              <CartesianGrid vertical={false} className="stroke-border/50" />
              <XAxis
                dataKey="point"
                tickLine={false}
                tickMargin={10}
                axisLine={false}
                height={40}
                tick={{ fill: 'var(--text-muted)' }}
                interval={0}
              />
              <ChartTooltip
                cursor={false}
                content={
                  <ChartTooltipContent
                    indicator="dashed"
                    className="border-border/50 bg-background-default text-text-default min-w-[180px] [&_.flex.flex-1]:gap-4 [&_.flex.flex-1>span]:whitespace-nowrap"
                  />
                }
              />
              <Bar dataKey="value" fill="var(--chart-1)" radius={4} maxBarSize={32} />
            </BarChart>
          )}
        </ChartContainer>
      </div>
    </div>
  );
}
