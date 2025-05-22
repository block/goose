
import React, { useState, useEffect } from 'react';
import { useTimelineStyles } from '../../hooks/useTimelineStyles.ts';
import waveBg from '../../assets/backgrounds/wave-bg.png';

interface ClockCardProps {
  date?: Date;
}

export default function ClockTile({ date }: ClockCardProps) {
  const { contentCardStyle, isPastDate } = useTimelineStyles(date);
  const [currentTime, setCurrentTime] = useState(new Date());
  
  // Don't render for past dates
  if (isPastDate) {
    return null;
  }
  
  // Update time every second for current day
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    
    return () => clearInterval(timer);
  }, []);

  // Format hours (12-hour format)
  const hours = currentTime.getHours() % 12 || 12;
  const minutes = currentTime.getMinutes().toString().padStart(2, '0');
  const period = currentTime.getHours() >= 12 ? 'PM' : 'AM';

  // Format day name
  const dayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  const dayName = dayNames[currentTime.getDay()];

  return (
    <div 
      className={`
        flex flex-col justify-between
        p-4 
        w-[213px] h-[213px] 
        ${contentCardStyle}
        rounded-[18px]
        relative
        overflow-hidden
        group
      `}
    >
      {/* Background Image with Gradient Overlay */}
      <div 
        className="absolute inset-0 bg-cover bg-center bg-no-repeat transition-opacity duration-500"
        style={{ 
          backgroundImage: `url(${waveBg})`,
          opacity: 0.8
        }}
      />
      
      {/* Gradient Overlay */}
      <div 
        className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent"
      />

      {/* Time Display */}
      <div className="flex flex-col items-start mt-auto relative z-10">
        <div className="flex items-baseline">
          <span className="font-['Cash_Sans'] text-[48px] font-light text-white leading-none">
            {hours}:{minutes}
          </span>
          <span className="ml-1 font-['Cash_Sans'] text-xl font-light text-white">
            {period}
          </span>
        </div>
        <span className="text-sm text-white/80 mt-1">
          {dayName}
        </span>
      </div>
    </div>
  );
}