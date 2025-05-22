import React from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';
import { ChartConfig, ChartContainer } from "@/components/ui/chart.tsx";
import { BarChart, Bar, LineChart, Line, ResponsiveContainer, Tooltip } from 'recharts';

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
    index: `Point ${index + 1}`
  }));

  // Chart configuration
  const chartConfig = {
    value: {
      label: title,
      theme: {
        light: variant === 'line' ? '#0B54DE' : '#4CAF50',
        dark: variant === 'line' ? '#00CAF7' : '#4CAF50'
      }
    }
  } satisfies ChartConfig;

  return (
    <div 
      className={`
        flex flex-col justify-between
        w-[320px] h-[380px] 
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
      <div className="w-full h-[160px] px-4">
        <ChartContainer config={chartConfig}>
          {variant === 'line' ? (
            <LineChart data={chartData}>
              <Line
                type="monotone"
                dataKey="value"
                stroke="var(--color-value)"
                strokeWidth={2}
                dot={{ fill: 'var(--color-value)', r: 4 }}
              />
              <Tooltip />
            </LineChart>
          ) : (
            <BarChart data={chartData}>
              <Bar
                dataKey="value"
                fill="var(--color-value)"
                radius={[4, 4, 0, 0]}
              />
              <Tooltip />
            </BarChart>
          )}
        </ChartContainer>
      </div>
    </div>
  );
}
