import React, { useState, useEffect, useRef, useCallback } from 'react';
import { cn } from '../utils';

interface TickerItem {
  id: string;
  text: string;
  type?: 'info' | 'warning' | 'error' | 'success' | 'neutral' | 'notification' | 'invitation';
  timestamp?: Date;
  priority?: 'low' | 'medium' | 'high';
  actionable?: boolean;
}

interface NotificationTickerProps {
  items?: TickerItem[];
  speed?: number; // pixels per second
  className?: string;
  height?: number; // height in pixels
}

const defaultItems: TickerItem[] = [
  { id: '1', text: 'matrix: connected', type: 'success', priority: 'high' },
  { id: '2', text: 'session invitation from alice: project-alpha', type: 'invitation', priority: 'high', actionable: true },
  { id: '3', text: 'new message from bob in dev-team', type: 'notification', priority: 'medium', actionable: true },
  { id: '4', text: 'charlie joined project-beta', type: 'success', priority: 'low' },
  { id: '5', text: 'session waiting: code-review (2 participants)', type: 'invitation', priority: 'high', actionable: true },
  { id: '6', text: '3 new messages from team in general', type: 'notification', priority: 'medium', actionable: true },
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
    // Use single color for all text types
    return 'text-text-default';
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
        'relative overflow-hidden bg-background-default border-b border-border-default',
        'font-mono text-xs leading-none font-bold',
        // Pixelated effect for container
        'image-rendering-pixelated',
        // Add some visual emphasis
        'shadow-sm',
        // Ensure it's visible
        'min-h-[32px]',
        className
      )}
      style={{ 
        height: `${height}px`,
        minHeight: `${height}px`
      }}
    >
      {/* Scanlines effect for retro ticker look */}
      <div className="absolute inset-0 pointer-events-none">
        <div 
          className="absolute inset-0 opacity-20"
          style={{
            backgroundImage: 'repeating-linear-gradient(0deg, transparent, transparent 1px, rgba(0, 0, 0, 0.1) 2px)',
            backgroundSize: '100% 2px'
          }}
        />
      </div>

      {/* Time display - hidden */}
      
      {/* Scrolling content */}
      <div className="absolute top-0 h-full flex items-center pl-0 bg-background-default image-rendering-pixelated">
        <div
          ref={contentRef}
          className="flex items-center whitespace-nowrap"
          style={{
            transform: `translateX(${translateX}px)`,
          }}
        >
          {items.map((item, index) => (
            <React.Fragment key={`${item.id}-${index}`}>
              <span className={cn('px-3 font-mono text-xs font-medium tracking-wide ticker-pixelated', getTypeColor(item.type))}>
                {item.text.toLowerCase()}
              </span>
              <span className="text-text-default px-2 font-mono text-xs ticker-pixelated">●</span>
            </React.Fragment>
          ))}
          {/* Duplicate content for seamless loop */}
          {items.map((item, index) => (
            <React.Fragment key={`${item.id}-duplicate-${index}`}>
              <span className={cn('px-3 font-mono text-xs font-medium tracking-wide ticker-pixelated', getTypeColor(item.type))}>
                {item.text.toLowerCase()}
              </span>
              <span className="text-text-default px-2 font-mono text-xs ticker-pixelated">●</span>
            </React.Fragment>
          ))}
        </div>
      </div>

      {/* Right fade effect */}
      <div className="absolute right-0 top-0 h-full w-8 bg-gradient-to-l from-background-default to-transparent pointer-events-none" />
      
      {/* Terminal cursor blink effect */}
      <div className="absolute right-2 top-1/2 transform -translate-y-1/2">
        <div className="w-2 h-4 bg-text-accent animate-pulse opacity-60" />
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
      text: `system: ${status.toLowerCase()}`,
      type,
    });
  }, [addItem]);

  // Matrix-specific notification functions
  const addSessionInvitation = useCallback((sessionName: string, inviterName: string) => {
    addItem({
      text: `session invitation from ${inviterName.toLowerCase()}: ${sessionName.toLowerCase()}`,
      type: 'invitation',
      priority: 'high',
      actionable: true,
    });
  }, [addItem]);

  const addMessageNotification = useCallback((senderName: string, sessionName: string, messageCount: number = 1) => {
    const messageText = messageCount === 1 
      ? `new message from ${senderName.toLowerCase()} in ${sessionName.toLowerCase()}`
      : `${messageCount} new messages from ${senderName.toLowerCase()} in ${sessionName.toLowerCase()}`;
    
    addItem({
      text: messageText,
      type: 'notification',
      priority: 'medium',
      actionable: true,
    });
  }, [addItem]);

  const addSessionJoined = useCallback((userName: string, sessionName: string) => {
    addItem({
      text: `${userName.toLowerCase()} joined ${sessionName.toLowerCase()}`,
      type: 'success',
      priority: 'low',
    });
  }, [addItem]);

  const addSessionLeft = useCallback((userName: string, sessionName: string) => {
    addItem({
      text: `${userName.toLowerCase()} left ${sessionName.toLowerCase()}`,
      type: 'neutral',
      priority: 'low',
    });
  }, [addItem]);

  const addConnectionStatus = useCallback((status: 'connected' | 'disconnected' | 'reconnecting') => {
    const statusMessages = {
      connected: { text: 'matrix: connected', type: 'success' as const },
      disconnected: { text: 'matrix: disconnected', type: 'error' as const },
      reconnecting: { text: 'matrix: reconnecting...', type: 'warning' as const },
    };
    
    const { text, type } = statusMessages[status];
    addItem({
      text,
      type,
      priority: 'high',
    });
  }, [addItem]);

  const addSessionWaiting = useCallback((sessionName: string, participantCount: number) => {
    addItem({
      text: `session waiting: ${sessionName.toLowerCase()} (${participantCount} participants)`,
      type: 'invitation',
      priority: 'high',
      actionable: true,
    });
  }, [addItem]);

  return {
    items,
    addItem,
    removeItem,
    clearItems,
    updateSystemStatus,
    // Matrix-specific functions
    addSessionInvitation,
    addMessageNotification,
    addSessionJoined,
    addSessionLeft,
    addConnectionStatus,
    addSessionWaiting,
  };
};

export default NotificationTicker;
