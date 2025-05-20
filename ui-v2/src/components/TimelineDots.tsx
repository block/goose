import React, { useMemo } from 'react';

interface TimelineDotsProps {
  height: number | string;
  isUpper?: boolean;
  isCurrentDay?: boolean;
}

interface Dot {
  top: string;
  size: number;
  opacity: number;
}

export default function TimelineDots({ height, isUpper = false, isCurrentDay = false }: TimelineDotsProps) {
  // Generate random dots with clusters
  const dots = useMemo(() => {
    const generateDots = () => {
      const dots: Dot[] = [];
      const numDots = Math.floor(Math.random() * 8) + 8; // 8-15 dots
      
      // Create 2-3 cluster points
      const clusterPoints = Array.from({ length: Math.floor(Math.random() * 2) + 2 }, 
        () => Math.random() * 100);
      
      for (let i = 0; i < numDots; i++) {
        // Decide if this dot should be part of a cluster
        const isCluster = Math.random() < 0.7; // 70% chance of being in a cluster
        
        let top;
        if (isCluster) {
          // Pick a random cluster point and add some variation
          const clusterPoint = clusterPoints[Math.floor(Math.random() * clusterPoints.length)];
          top = clusterPoint + (Math.random() - 0.5) * 15; // ±7.5% variation
        } else {
          top = Math.random() * 100;
        }
        
        // Ensure dot is within bounds
        top = Math.max(5, Math.min(95, top));
        
        dots.push({
          top: `${top}%`,
          size: Math.random() * 2 + 2, // 2-4px
          opacity: Math.random() * 0.5 + 0.2, // 0.2-0.7 opacity
        });
      }
      return dots;
    };
    
    return generateDots();
  }, []); // Empty dependency array means this only runs once

  return (
    <div 
      className="flex h-full left-1/2 -translate-x-[0.375px] flex flex-col items-center"
      style={{ 
        height: height,
        bottom: isUpper ? 'calc(50% + 96px)' : '0',
        top: isUpper ? undefined : 'calc(50% + 96px)'
      }}
    >
      {/* Main line */}
      <div className="w-[0.75px] h-full bg-black/10 dark:bg-white/10 relative">
        {/* Top dot for current day */}
        {isUpper && isCurrentDay && (
          <div
            className="absolute rounded-full bg-black dark:bg-white"
            style={{
              width: '4px',
              height: '4px',
              left: '-1.625px', // Center 4px dot on 0.75px line
              top: '0',
              transform: 'translateY(-50%)'
            }}
          />
        )}
        
        {/* Random dots */}
        {dots.map((dot, index) => (
          <div
            key={index}
            className="absolute rounded-full bg-black/40 dark:bg-white/40"
            style={{
              width: `${dot.size}px`,
              height: `${dot.size}px`,
              left: `${-(dot.size - 0.75) / 2}px`, // Center dot on the line
              top: dot.top,
              opacity: dot.opacity,
            }}
          />
        ))}
      </div>
    </div>
  );
};
