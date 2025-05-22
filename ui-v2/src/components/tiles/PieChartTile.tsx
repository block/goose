import React, { useState } from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles';
import { ChartConfig, ChartContainer } from "@/components/ui/chart";
import { PieChart, Pie, Cell, Sector, ResponsiveContainer } from 'recharts';

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

// Custom label renderer with connecting lines
const renderCustomizedLabel = ({
  cx,
  cy,
  midAngle,
  innerRadius,
  outerRadius,
  percent,
  payload,
  fill,
}: any) => {
  const RADIAN = Math.PI / 180;
  const sin = Math.sin(-RADIAN * midAngle);
  const cos = Math.cos(-RADIAN * midAngle);

  // Calculate positions with adjusted distances for better fit
  const radius = outerRadius + 10;
  const mx = cx + radius * cos;
  const my = cy + radius * sin;
  const ex = mx + (cos >= 0 ? 1 : -1) * 15;
  const ey = my;

  // Text anchor based on which side of the pie we're on
  const textAnchor = cos >= 0 ? "start" : "end";

  // Calculate percentage
  const percentage = (percent * 100).toFixed(0);
  
  // Determine if label should be on top or bottom half
  const isTopHalf = my < cy;

  return (
    <g>
      {/* Connecting line */}
      <path
        d={`M${cx + outerRadius * cos},${cy + outerRadius * sin}L${mx},${my}L${ex},${ey}`}
        stroke={fill}
        strokeWidth={1}
        fill="none"
      />
      {/* Label text - adjusted y position based on top/bottom placement */}
      <text
        x={ex + (cos >= 0 ? 5 : -5)}
        y={ey + (isTopHalf ? -2 : 10)}
        textAnchor={textAnchor}
        fill="var(--foreground)"
        className="text-[10px]"
      >
        {`${payload.name} (${percentage}%)`}
      </text>
    </g>
  );
};

// Active shape renderer for hover effect
const renderActiveShape = (props: any) => {
  const { cx, cy, innerRadius, outerRadius, startAngle, endAngle, fill } = props;

  return (
    <Sector
      cx={cx}
      cy={cy}
      innerRadius={innerRadius}
      outerRadius={outerRadius + 4}
      startAngle={startAngle}
      endAngle={endAngle}
      fill={fill}
      cornerRadius={4}
    />
  );
};

export default function PieChartTile({ 
  title, 
  icon,
  segments,
  date 
}: PieChartTileProps) {
  const { contentCardStyle } = useTimelineStyles(date);
  const [activeIndex, setActiveIndex] = useState<number>(0);

  // Convert segments to the format expected by recharts
  const chartData = segments.map(segment => ({
    name: segment.label,
    value: segment.value
  }));

  // Create chart configuration
  const chartConfig = segments.reduce((config, segment) => {
    config[segment.label.toLowerCase()] = {
      label: segment.label,
      color: segment.color
    };
    return config;
  }, {} as ChartConfig);

  const onPieEnter = (_: any, index: number) => {
    setActiveIndex(index);
  };

  return (
    <div 
      className={`
        flex flex-col
        w-[320px] min-h-[380px] 
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
      <div className="flex-1 min-h-[260px] pb-4">
        <ChartContainer config={chartConfig}>
          <ResponsiveContainer width="100%" height="100%">
            <PieChart margin={{ top: 20, right: 20, bottom: 20, left: 20 }}>
              <Pie
                activeIndex={activeIndex}
                activeShape={renderActiveShape}
                data={chartData}
                cx="50%"
                cy="50%"
                innerRadius={50}
                outerRadius={70}
                paddingAngle={5}
                dataKey="value"
                onMouseEnter={onPieEnter}
                cornerRadius={4}
                label={renderCustomizedLabel}
                labelLine={false}
              >
                {segments.map((segment, index) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={segment.color}
                    stroke="var(--background)"
                    strokeWidth={2}
                  />
                ))}
              </Pie>
            </PieChart>
          </ResponsiveContainer>
        </ChartContainer>
      </div>
    </div>
  );
}