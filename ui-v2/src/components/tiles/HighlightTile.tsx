import { ReactElement, ReactNode } from 'react';

import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';

interface HighlightTileProps {
  title: string;
  value: string;
  icon: ReactNode;
  subtitle?: string;
  date?: Date;
  accentColor?: string;
}

export default function HighlightTile({
  title,
  value,
  icon,
  subtitle,
  date,
  accentColor = '#00CAF7',
}: HighlightTileProps): ReactElement {
  const { contentCardStyle } = useTimelineStyles(date);

  return (
    <div
      className={`
        flex flex-col justify-between
        w-[320px] h-[280px] 
        ${contentCardStyle}
        rounded-[18px]
        relative
        overflow-hidden
        transition-all duration-200
        hover:scale-[1.02]
      `}
    >
      {/* Background accent */}
      <div
        className="absolute inset-0 opacity-5"
        style={{
          background: `radial-gradient(circle at top right, ${accentColor}, transparent 70%)`,
        }}
      />

      {/* Content */}
      <div className="p-4 h-full flex flex-col justify-between relative z-10">
        <div className="w-6 h-6 text-text-default dark:text-white">{icon}</div>

        <div>
          <div className="text-gray-600 dark:text-white/60 text-sm mb-1">{title}</div>
          <div
            className="text-gray-900 dark:text-white text-2xl font-semibold"
            style={{ color: accentColor }}
          >
            {value}
          </div>
          {subtitle && (
            <div className="text-gray-500 dark:text-white/60 text-sm mt-1">{subtitle}</div>
          )}
        </div>
      </div>
    </div>
  );
}
