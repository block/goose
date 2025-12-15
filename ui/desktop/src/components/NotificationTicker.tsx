import React, { useState, useEffect, useRef, useCallback } from 'react';
import { cn } from '../utils';

interface TickerItem {
  id: string;
  text: string;
  type?: 'info' | 'warning' | 'error' | 'success' | 'neutral';
  timestamp?: Date;
}

interface NotificationTickerProps {
  items?: TickerItem[];
  speed?: number; // pixels per second
  className?: string;
  height?: number; // height in pixels
}

const defaultItems: TickerItem[] = [
  { id: '1', text: 'GOOSE AI AGENT ONLINE', type: 'success' },
  { id: '2', text: 'SYSTEM STATUS: OPERATIONAL', type: 'info' },
  { id: '3', text: 'EXTENSIONS LOADED: 12', type: 'info' },
  { id: '4', text: 'LAST SYNC: 2 MIN AGO', type: 'neutral' },
  { id: '5', text: 'MEMORY USAGE: 45%', type: 'info' },
  { id: '6', text: 'ACTIVE SESSIONS: 3', type: 'success' },
];

export const NotificationTicker: React.FC<NotificationTickerProps> = ({
  items = defaultItems,
  speed = 50,
  className,
  height = 32, // Reduced back to 32 for smaller text
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [translateX, setTranslateX] = useState(0);
  const [contentWidth, setContentWidth] = useState(0);
  const [containerWidth, setContainerWidth] = useState(0);

  // Debug log to ensure ticker is rendering
  useEffect(() => {
    console.log('🎯 NotificationTicker rendered with items:', items.length);
  }, [items]);

  // Measure content width
  useEffect(() => {
    if (contentRef.current && containerRef.current) {
      setContentWidth(contentRef.current.scrollWidth);
      setContainerWidth(containerRef.current.clientWidth);
    }
  }, [items]);

  // Animation loop
  useEffect(() => {
    if (contentWidth === 0 || containerWidth === 0) return;

    let animationFrame: number;
    let lastTime = 0;

    const animate = (currentTime: number) => {
      if (lastTime === 0) lastTime = currentTime;
      const deltaTime = (currentTime - lastTime) / 1000; // Convert to seconds
      lastTime = currentTime;

      setTranslateX(prev => {
        const newX = prev - speed * deltaTime;
        // Reset when content has completely scrolled past
        if (Math.abs(newX) >= contentWidth) {
          return containerWidth;
        }
        return newX;
      });

      animationFrame = requestAnimationFrame(animate);
    };

    animationFrame = requestAnimationFrame(animate);

    return () => {
      if (animationFrame) {
        cancelAnimationFrame(animationFrame);
      }
    };
  }, [speed, contentWidth, containerWidth]);

  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      if (containerRef.current) {
        setContainerWidth(containerRef.current.clientWidth);
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const getTypeColor = (type: TickerItem['type']) => {
    switch (type) {
      case 'success':
        return 'text-green-400';
      case 'warning':
        return 'text-yellow-400';
      case 'error':
        return 'text-red-400';
      case 'info':
        return 'text-blue-400';
      case 'neutral':
      default:
        return 'text-gray-300';
    }
  };

  const formatTime = () => {
    return new Date().toLocaleTimeString('en-US', { 
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  return (
    <div 
      ref={containerRef}
      className={cn(
        'relative overflow-hidden bg-black border-b-2 border-green-500/50',
        'font-mono text-xs leading-none font-bold',
        // Pixelated effect
        'image-rendering-pixelated',
        // Add some visual emphasis
        'shadow-lg shadow-green-500/20',
        // Ensure it's visible
        'min-h-[32px]',
        className
      )}
      style={{ 
        height: `${height}px`,
        minHeight: `${height}px`
      }}
    >
      {/* Scanlines effect for retro terminal look */}
      <div className="absolute inset-0 pointer-events-none">
        <div 
          className="absolute inset-0 opacity-10"
          style={{
            backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 1px, rgba(0, 255, 0, 0.1) 2px)',
            backgroundSize: '100% 2px'
          }}
        />
      </div>

      {/* Time display - fixed on the left */}
      <div className="absolute left-0 top-0 h-full flex items-center px-3 bg-black/90 border-r-2 border-green-500/50 z-10">
        <span className="text-green-400 font-bold tracking-wider ticker-pixelated">
          {formatTime()}
        </span>
      </div>

      {/* Scrolling content */}
      <div className="absolute top-0 h-full flex items-center pl-24">
        <div
          ref={contentRef}
          className="flex items-center whitespace-nowrap"
          style={{
            transform: `translateX(${translateX}px)`,
          }}
        >
          {items.map((item, index) => (
            <React.Fragment key={`${item.id}-${index}`}>
              <span className={cn('px-3 font-bold tracking-wide ticker-pixelated', getTypeColor(item.type))}>
                {item.text}
              </span>
              <span className="text-green-500 px-2 ticker-pixelated">●</span>
            </React.Fragment>
          ))}
          {/* Duplicate content for seamless loop */}
          {items.map((item, index) => (
            <React.Fragment key={`${item.id}-duplicate-${index}`}>
              <span className={cn('px-3 font-bold tracking-wide ticker-pixelated', getTypeColor(item.type))}>
                {item.text}
              </span>
              <span className="text-green-500 px-2 ticker-pixelated">●</span>
            </React.Fragment>
          ))}
        </div>
      </div>

      {/* Right fade effect */}
      <div className="absolute right-0 top-0 h-full w-8 bg-gradient-to-l from-black to-transparent pointer-events-none" />
      
      {/* Terminal cursor blink effect */}
      <div className="absolute right-2 top-1/2 transform -translate-y-1/2">
        <div className="w-2 h-4 bg-green-400 animate-pulse opacity-60" />
      </div>
    </div>
  );
};

// Hook to manage ticker items
export const useNotificationTicker = () => {
  const [items, setItems] = useState<TickerItem[]>(defaultItems);

  const addItem = useCallback((item: Omit<TickerItem, 'id' | 'timestamp'>) => {
    const newItem: TickerItem = {
      ...item,
      id: `ticker-${Date.now()}-${Math.random()}`,
      timestamp: new Date(),
    };
    setItems(prev => {
      // Limit the number of items to prevent infinite growth
      const newItems = [...prev, newItem];
      return newItems.length > 20 ? newItems.slice(-20) : newItems;
    });
  }, []);

  const removeItem = useCallback((id: string) => {
    setItems(prev => prev.filter(item => item.id !== id));
  }, []);

  const clearItems = useCallback(() => {
    setItems(defaultItems); // Reset to default items instead of empty
  }, []);

  const updateSystemStatus = useCallback((status: string, type: TickerItem['type'] = 'info') => {
    addItem({
      text: `SYSTEM: ${status.toUpperCase()}`,
      type,
    });
  }, [addItem]);

  return {
    items,
    addItem,
    removeItem,
    clearItems,
    updateSystemStatus,
  };
};

export default NotificationTicker;
