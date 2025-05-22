import React from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';

interface ListItem {
  text: string;
  value?: string;
  color?: string;
}

interface ListTileProps {
  title: string;
  icon: React.ReactNode;
  items: ListItem[];
  date?: Date;
}

export default function ListTile({ 
  title, 
  icon,
  items,
  date 
}: ListTileProps) {
  const { contentCardStyle } = useTimelineStyles(date);

  return (
    <div 
      className={`
        flex flex-col
        w-[320px] h-[420px] 
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
        <div className="text-gray-600 dark:text-white/40 text-sm mb-4">
          {title}
        </div>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto px-4 pb-4">
        <div className="space-y-3">
          {items.map((item, index) => (
            <div 
              key={index}
              className="flex items-center justify-between"
            >
              <div className="flex items-center space-x-2">
                <div 
                  className={`w-2 h-2 rounded-full ${
                    item.color ? '' : 'bg-gray-400 dark:bg-white/40'
                  }`}
                  style={item.color ? { backgroundColor: item.color } : {}}
                />
                <span className="text-sm text-gray-600 dark:text-white/80">
                  {item.text}
                </span>
              </div>
              {item.value && (
                <span 
                  className="text-sm font-medium"
                  style={item.color ? { color: item.color } : {}}
                >
                  {item.value}
                </span>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}