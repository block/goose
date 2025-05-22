import React from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { BarChart, Bar, LineChart, Line, CartesianGrid, XAxis, ResponsiveContainer } from 'recharts';

interface ChartTileProps {
  title: string;
  value: string;
  trend?: string;
  data: number[];
  icon: React.ReactNode;
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
  date 
}: ChartTileProps) {
  const { contentCardStyle } = useTimelineStyles(date);
  
  // Convert the data array to the format expected by recharts
  const chartData = data.map((value, index) => ({
    value,
    point: `P${index + 1}`
  }));

  // Chart configuration
  const chartConfig = {
    value: {
      label: title,
      color: variant === 'line' ? 'var(--chart-1)' : 'var(--chart-2)'
    }
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
      `}
    >
      {/* Header section with icon */}
      <div className="p-4 space-y-4">
        <div className="w-6 h-6">
          {icon}
        </div>

        <div>
          <div className="text-gray-600 dark:text-white/40 text-sm mb-1">{title}</div>
          <div className="text-gray-900 dark:text-white text-2xl font-semibold">
            {value}
            {trend && <span className="ml-1 text-sm">{trend}</span>}
          </div>
        </div>
      </div>

      {/* Chart Container */}
      <div className="w-full h-[200px] px-4 pb-6">
        <ChartContainer config={chartConfig}>
          <ResponsiveContainer width="100%" height="100%">
            {variant === 'line' ? (
              <LineChart data={chartData} margin={{ top: 10, right: 10, bottom: 0, left: -20 }}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="point"
                  tickLine={false}
                  tickMargin={10}
                  axisLine={false}
                  height={40}
                />
                <ChartTooltip
                  content={<ChartTooltipContent />}
                />
                <Line
                  type="monotone"
                  dataKey="value"
                  stroke="var(--color-value)"
                  strokeWidth={2}
                  dot={{ fill: 'var(--color-value)', r: 4 }}
                />
              </LineChart>
            ) : (
              <BarChart data={chartData} margin={{ top: 10, right: 10, bottom: 0, left: -20 }}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="point"
                  tickLine={false}
                  tickMargin={10}
                  axisLine={false}
                  height={40}
                />
                <ChartTooltip
                  cursor={false}
                  content={<ChartTooltipContent />}
                />
                <Bar
                  dataKey="value"
                  fill="var(--color-value)"
                  radius={4}
                />
              </BarChart>
            )}
          </ResponsiveContainer>
        </ChartContainer>
      </div>
    </div>
  );
}