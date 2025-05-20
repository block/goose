import React from 'react';
import { useTimelineStyles } from '../hooks/useTimelineStyles';

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
  
  // Convert data points to SVG coordinates
  const createSmoothPath = () => {
    const points = data.map((value, index) => {
      const x = (index / (data.length - 1)) * 100;
      const y = 100 - ((value - Math.min(...data)) / (Math.max(...data) - Math.min(...data))) * 100;
      return [x, y];
    });

    let path = `M ${points[0][0]},${points[0][1]}`;
    for (let i = 0; i < points.length - 1; i++) {
      const current = points[i];
      const next = points[i + 1];
      const controlPoint1X = current[0] + (next[0] - current[0]) / 3;
      const controlPoint2X = current[0] + 2 * (next[0] - current[0]) / 3;
      path += ` C ${controlPoint1X},${current[1]} ${controlPoint2X},${next[1]} ${next[0]},${next[1]}`;
    }
    return path;
  };

  // Create bar chart elements
  const createBars = () => {
    const maxValue = Math.max(...data.map(Math.abs));
    const barWidth = 8;
    const spacing = (100 - (data.length * barWidth)) / (data.length - 1);
    
    return data.map((value, index) => {
      const x = index * (barWidth + spacing);
      const height = Math.abs(value) / maxValue * 50;
      const y = value > 0 ? 50 - height : 50;
      
      return {
        x,
        y,
        height,
        isPositive: value > 0
      };
    });
  };

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
      <div className="w-full h-[160px]">
        <svg 
          width="100%" 
          height="100%" 
          viewBox="0 0 100 100" 
          preserveAspectRatio="none"
          className="relative z-10"
        >
          {variant === 'line' ? (
            <>
              <defs>
                <linearGradient id="chart-gradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#00CAF7" />
                  <stop offset="100%" stopColor="#0B54DE" />
                </linearGradient>
              </defs>

              <path
                d={createSmoothPath()}
                style={{ stroke: 'url(#chart-gradient)' }}
                className="stroke-[1.5] fill-none"
                strokeLinecap="round"
                strokeLinejoin="round"
              />

              {/* Data points */}
              {data.map((value, index) => {
                const x = (index / (data.length - 1)) * 100;
                const y = 100 - ((value - Math.min(...data)) / (Math.max(...data) - Math.min(...data))) * 100;
                return (
                  <circle
                    key={index}
                    cx={x}
                    cy={y}
                    r="2"
                    className="fill-blue-500"
                  />
                );
              })}
            </>
          ) : (
            <>
              {createBars().map((bar, index) => (
                <rect
                  key={index}
                  x={bar.x}
                  y={bar.y}
                  width="8"
                  height={bar.height}
                  className={`${bar.isPositive ? 'fill-green-500' : 'fill-red-500'} opacity-80`}
                  rx="4"
                />
              ))}
            </>
          )}
        </svg>
      </div>
    </div>
  );
}