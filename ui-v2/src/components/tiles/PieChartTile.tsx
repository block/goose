import React from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles';
import { ChartConfig, ChartContainer } from "@/components/ui/chart";
import { PieChart, Pie, Cell, Tooltip, Legend } from 'recharts';

interface PieChartSegment {
  value: number;
  color: string;
  label: string;
}

interface PieChartTileProps {
  title: string;
  icon: React.ReactNode;
  segments: PieChartSegment[];
  date?: Date;
}

export default function PieChartTile({ 
  title, 
  icon,
  segments,
  date 
}: PieChartTileProps) {
  const { contentCardStyle } = useTimelineStyles(date);

  // Convert segments to the format expected by recharts
  const chartData = segments.map(segment => ({
    name: segment.label,
    value: segment.value
  }));

  // Create chart configuration with theme colors
  const chartConfig = segments.reduce((config, segment) => {
    config[segment.label] = {
      label: segment.label,
      color: segment.color
    };
    return config;
  }, {} as ChartConfig);

  // Custom tooltip formatter
  const tooltipFormatter = (value: number, name: string) => {
    const total = segments.reduce((sum, segment) => sum + segment.value, 0);
    const percentage = ((value / total) * 100).toFixed(1);
    return [`${percentage}%`, name];
  };

  return (
    <div 
      className={`
        flex flex-col
        w-[320px] h-[380px] 
        ${contentCardStyle}
        rounded-[18px]
        relative
        overflow-hidden
        transition-all duration-200
        hover:scale-[1.02]
      `}
    >
      {/* Header */}
      <div className="p-4">
        <div className="w-6 h-6 mb-4">
          {icon}
        </div>
        <div className="text-gray-600 dark:text-white/40 text-sm">
          {title}
        </div>
      </div>

      {/* Pie Chart */}
      <div className="flex-1 flex flex-col items-center">
        <div className="w-full h-[200px]">
          <ChartContainer config={chartConfig}>
            <PieChart>
              <Pie
                data={chartData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                innerRadius={0}
                outerRadius={70}
                paddingAngle={2}
              >
                {segments.map((segment, index) => (
                  <Cell 
                    key={`cell-${index}`} 
                    fill={segment.color}
                    className="transition-all duration-200 hover:opacity-90"
                  />
                ))}
              </Pie>
              <Tooltip formatter={tooltipFormatter} />
            </PieChart>
          </ChartContainer>
        </div>

        {/* Legend */}
        <div className="mt-2 px-4 w-full space-y-2">
          {segments.map((segment, index) => {
            const percentage = ((segment.value / segments.reduce((sum, s) => sum + s.value, 0)) * 100).toFixed(1);
            return (
              <div key={index} className="flex items-center justify-between">
                <div className="flex items-center">
                  <div 
                    className="w-3 h-3 rounded-full mr-2"
                    style={{ backgroundColor: segment.color }}
                  />
                  <span className="text-sm text-gray-600 dark:text-white/60">
                    {segment.label}
                  </span>
                </div>
                <span className="text-sm font-medium text-gray-900 dark:text-white">
                  {percentage}%
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
