import React from 'react';
import { useTimelineStyles } from '../hooks/useTimelineStyles';

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

  const total = segments.reduce((sum, segment) => sum + segment.value, 0);
  let currentAngle = 0;

  const createPieSegments = () => {
    return segments.map((segment, index) => {
      const startAngle = currentAngle;
      const percentage = segment.value / total;
      const angle = percentage * 360;
      currentAngle += angle;

      const startX = Math.cos((startAngle - 90) * Math.PI / 180) * 32 + 50;
      const startY = Math.sin((startAngle - 90) * Math.PI / 180) * 32 + 50;
      const endX = Math.cos((currentAngle - 90) * Math.PI / 180) * 32 + 50;
      const endY = Math.sin((currentAngle - 90) * Math.PI / 180) * 32 + 50;

      const largeArcFlag = angle > 180 ? 1 : 0;

      const d = [
        `M 50 50`,
        `L ${startX} ${startY}`,
        `A 32 32 0 ${largeArcFlag} 1 ${endX} ${endY}`,
        'Z'
      ].join(' ');

      return {
        path: d,
        color: segment.color,
        label: segment.label,
        percentage: (percentage * 100).toFixed(1)
      };
    });
  };

  const pieSegments = createPieSegments();

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
        <div className="relative w-[150px] h-[150px]">
          <svg 
            viewBox="0 0 100 100"
            className="w-full h-full transform -rotate-90"
          >
            {pieSegments.map((segment, index) => (
              <path
                key={index}
                d={segment.path}
                fill={segment.color}
                className="transition-all duration-200 hover:opacity-90"
              />
            ))}
          </svg>
        </div>

        {/* Legend */}
        <div className="mt-4 px-4 w-full space-y-2">
          {pieSegments.map((segment, index) => (
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
                {segment.percentage}%
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}