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

  // Adjust these values to position labels closer to the pie
  const labelOffset = 12;
  const labelDistance = 18;

  // Calculate positions with shorter distances
  const mx = cx + (outerRadius + labelOffset) * cos;
  const my = cy + (outerRadius + labelOffset) * sin;
  const ex = mx + (cos >= 0 ? 1 : -1) * labelDistance;
  const ey = my;

  // Text anchor based on which side of the pie we're on
  const textAnchor = cos >= 0 ? "start" : "end";

  // Calculate percentage
  const value = (percent * 100).toFixed(0);

  // Determine if label should be on top or bottom half for potential y-offset
  const isTopHalf = my < cy;
  const yOffset = isTopHalf ? -2 : 2;

  // Force specific adjustments for "In Progress" label if needed
  const isInProgress = payload.name === "In Progress";
  const adjustedEx = isInProgress ? ex - 5 : ex;

  return (
    <g>
      {/* Label line - using absolute coordinates for reliability */}
      <path
        d={`M${cx + outerRadius * cos},${cy + outerRadius * sin}L${mx},${my}L${adjustedEx},${ey}`}
        stroke={fill}
        strokeWidth={1}
        fill="none"
        style={{ opacity: 1 }}
      />
      {/* Label text with adjusted position */}
      <text
        x={adjustedEx + (cos >= 0 ? 5 : -5)}
        y={ey + yOffset}
        textAnchor={textAnchor}
        fill="var(--text-default)"
        className="text-[10px]"
        style={{ 
          pointerEvents: 'none',
        }}
      >
        {payload.name} ({value}%)
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

  // Convert segments to the format expected by recharts and assign chart colors
  const chartData = segments.map((segment, index) => ({
    name: segment.label,
    value: segment.value,
    chartColor: `var(--chart-${index + 1})`  // Use chart-1, chart-2, chart-3, etc.
  }));

  // Create chart configuration using the chart color variables
  const chartConfig = {
    [segments[0].label.toLowerCase()]: {
      label: segments[0].label,
      color: 'var(--chart-1)'
    },
    [segments[1].label.toLowerCase()]: {
      label: segments[1].label,
      color: 'var(--chart-2)'
    },
    [segments[2].label.toLowerCase()]: {
      label: segments[2].label,
      color: 'var(--chart-3)'
    }
  } satisfies ChartConfig;

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
        bg-background-default text-text-default
      `}
    >
      {/* Header */}
      <div className="p-4">
        <div className="w-6 h-6 mb-4 text-text-default">
          {icon}
        </div>
        <div className="text-text-muted text-sm">
          {title}
        </div>
      </div>

      {/* Pie Chart */}
      <div className="flex-1 flex items-center justify-center p-4">
        <div style={{ width: '100%', height: '260px', position: 'relative' }}>
          <ChartContainer config={chartConfig}>
            <ResponsiveContainer>
              <PieChart margin={{ top: 30, right: 40, bottom: 10, left: 40 }}>
                <Pie
                  activeIndex={activeIndex}
                  activeShape={renderActiveShape}
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  innerRadius={45}
                  outerRadius={65}
                  paddingAngle={5}
                  dataKey="value"
                  onMouseEnter={onPieEnter}
                  cornerRadius={4}
                  label={renderCustomizedLabel}
                  labelLine={false}
                  startAngle={90}
                  endAngle={-270}
                  isAnimationActive={false}
                >
                  {chartData.map((entry, index) => (
                    <Cell
                      key={`cell-${index}`}
                      fill={entry.chartColor}
                      stroke="var(--background-default)"
                      strokeWidth={2}
                    />
                  ))}
                </Pie>
              </PieChart>
            </ResponsiveContainer>
          </ChartContainer>
        </div>
      </div>
    </div>
  );
}
