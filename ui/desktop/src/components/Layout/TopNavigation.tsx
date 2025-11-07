import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Home, History, FileText, Clock, Puzzle, Settings as SettingsIcon, GripVertical } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChatSmart } from '../icons';
import { listSessions, getSessionInsights } from '../../api';
import { useConfig } from '../ConfigContext';
import { listSavedRecipes } from '../../recipe/recipe_management';

interface NavItem {
  id: string;
  path?: string;
  label: string;
  icon?: React.ComponentType<{ className?: string }>;
  getTag?: () => string;
  tagAlign?: 'left' | 'right';
  isWidget?: boolean;
  renderContent?: () => React.ReactNode;
}

interface TopNavigationProps {
  isExpanded: boolean;
  setIsExpanded: (expanded: boolean) => void;
}

// Analog Clock Widget Component
const AnalogClock: React.FC = () => {
  const [secondAngle, setSecondAngle] = useState(() => {
    const now = new Date();
    return now.getSeconds() * 6;
  });
  const [angles, setAngles] = useState(() => {
    const now = new Date();
    const hours = now.getHours() % 12;
    const minutes = now.getMinutes();
    return {
      hour: (hours * 30) + (minutes * 0.5),
      minute: minutes * 6,
    };
  });

  useEffect(() => {
    const interval = setInterval(() => {
      const now = new Date();
      const hours = now.getHours() % 12;
      const minutes = now.getMinutes();
      
      // Increment second angle by 6 degrees each second
      setSecondAngle(prev => prev + 6);
      
      setAngles({
        hour: (hours * 30) + (minutes * 0.5),
        minute: minutes * 6,
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const hourAngle = angles.hour;
  const minuteAngle = angles.minute;

  return (
    <div className="w-full h-full flex items-center justify-center">
      <div className="relative w-32 h-32">
        {/* Clock face */}
        <div className="absolute inset-0 rounded-full border-2 border-text-muted/20" />
        
        {/* Hour markers */}
        {[...Array(12)].map((_, i) => (
          <div
            key={i}
            className="absolute w-0.5 h-2 bg-text-muted/40 top-2 left-1/2 -translate-x-1/2"
            style={{
              transformOrigin: '50% 60px',
              transform: `translateX(-50%) rotate(${i * 30}deg)`,
            }}
          />
        ))}
        
        {/* Hour hand */}
        <div
          className="absolute w-1 h-10 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${hourAngle}deg)`,
          }}
        />
        
        {/* Minute hand */}
        <div
          className="absolute w-0.5 h-14 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${minuteAngle}deg)`,
          }}
        />
        
        {/* Second hand */}
        <div
          className="absolute w-px h-16 bg-red-500 rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${secondAngle}deg)`,
          }}
        />
        
        {/* Center dot */}
        <div className="absolute w-3 h-3 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" />
      </div>
    </div>
  );
};

export const TopNavigation: React.FC<TopNavigationProps> = ({ isExpanded, setIsExpanded }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { extensionsList, getExtensions } = useConfig();
  const [currentTime, setCurrentTime] = useState('');
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recipesCount, setRecipesCount] = useState(0);
  const [scheduledTodayCount, setScheduledTodayCount] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);

  const [sessionHeatmapData, setSessionHeatmapData] = useState<Record<string, number>>({});
  
  // Track previous values for pulse animation
  const [prevValues, setPrevValues] = useState<Record<string, string>>({});
  const [pulsingItems, setPulsingItems] = useState<Set<string>>(new Set());
  
  // Drag and drop state
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [tileOrder, setTileOrder] = useState<string[]>([]);

  // Update time every second for smooth clock animation
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true }));
    };
    updateTime();
    const interval = setInterval(updateTime, 1000); // Update every second
    return () => clearInterval(interval);
  }, []);

  // Fetch data when navigation expands
  useEffect(() => {
    if (isExpanded) {
      fetchNavigationData();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isExpanded]);

  const fetchNavigationData = async () => {
    try {
      // Fetch today's chats and build heatmap
      const sessionsResponse = await listSessions({ throwOnError: false });
      if (sessionsResponse.data) {
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        
        // Build heatmap data for last 30 days and count today's chats in one pass
        const heatmap: Record<string, number> = {};
        let todayCount = 0;
        
        sessionsResponse.data.sessions.forEach((session) => {
          const sessionDate = new Date(session.created_at);
          const dateKey = sessionDate.toISOString().split('T')[0];
          heatmap[dateKey] = (heatmap[dateKey] || 0) + 1;
          
          // Count today's chats
          const sessionDateOnly = new Date(session.created_at);
          sessionDateOnly.setHours(0, 0, 0, 0);
          if (sessionDateOnly.getTime() === today.getTime()) {
            todayCount++;
          }
        });
        
        setSessionHeatmapData(heatmap);
        setTodayChatsCount(todayCount);
      }

      // Fetch total sessions and tokens
      const insightsResponse = await getSessionInsights({ throwOnError: false });
      if (insightsResponse.data) {
        setTotalSessions(insightsResponse.data.totalSessions || 0);
        setTotalTokens(insightsResponse.data.totalTokens || 0);
      }

      // Fetch recipes count
      try {
        const recipes = await listSavedRecipes();
        setRecipesCount(recipes.length);
      } catch (error) {
        console.error('Failed to fetch recipes:', error);
        setRecipesCount(0);
      }

      // TODO: Fetch scheduled jobs when API is available
      // For now, set to 0 as placeholder
      setScheduledTodayCount(0);

      // Refresh extensions data
      try {
        await getExtensions(true);
      } catch (error) {
        console.error('Failed to fetch extensions:', error);
      }
    } catch (error) {
      console.error('Failed to fetch navigation data:', error);
    }
  };

  const navItemsBase: NavItem[] = [
    {
      id: 'home',
      path: '/',
      label: 'Home',
      icon: Home,
      getTag: () => currentTime,
    },
    {
      id: 'chat',
      path: '/pair',
      label: 'Chat',
      icon: ChatSmart,
      getTag: () => `${todayChatsCount} today`,
      tagAlign: 'left',
    },
    {
      id: 'history',
      path: '/sessions',
      label: 'History',
      icon: History,
      getTag: () => `${totalSessions} total`,
      tagAlign: 'left',
    },
    {
      id: 'recipes',
      path: '/recipes',
      label: 'Recipes',
      icon: FileText,
      getTag: () => `${recipesCount}`,
    },
    {
      id: 'scheduler',
      path: '/schedules',
      label: 'Scheduler',
      icon: Clock,
      getTag: () => `${scheduledTodayCount} today`,
      tagAlign: 'left',
    },
    {
      id: 'extensions',
      path: '/extensions',
      label: 'Extensions',
      icon: Puzzle,
      getTag: () => {
        if (!extensionsList || !Array.isArray(extensionsList)) {
          return '0 of 0 enabled';
        }
        const enabled = extensionsList.filter(ext => ext.enabled).length;
        const total = extensionsList.length;
        return `${enabled} of ${total} enabled`;
      },
    },
    {
      id: 'settings',
      path: '/settings',
      label: 'Settings',
      icon: SettingsIcon,
      getTag: () => '✓',
      tagAlign: 'left',
    },
    {
      id: 'clock-widget',
      label: 'Clock',
      isWidget: true,
      renderContent: () => <AnalogClock />,
    },
    {
      id: 'activity-widget',
      label: 'Activity',
      isWidget: true,
      renderContent: () => {
        // Get last 35 days for a 5x7 grid
        const days = 35;
        const today = new Date();
        const heatmapCells = [];
        
        for (let i = days - 1; i >= 0; i--) {
          const date = new Date(today);
          date.setDate(date.getDate() - i);
          const dateKey = date.toISOString().split('T')[0];
          const count = sessionHeatmapData[dateKey] || 0;
          
          // Calculate intensity (0-4 scale)
          const maxCount = Math.max(...Object.values(sessionHeatmapData), 1);
          const intensity = count === 0 ? 0 : Math.ceil((count / maxCount) * 4);
          
          heatmapCells.push({ date: dateKey, count, intensity });
        }
        
        return (
          <div className="w-full h-full flex flex-col items-center justify-center p-4">
            <div className="grid grid-cols-7 gap-1">
              {heatmapCells.map((cell, index) => (
                <div
                  key={index}
                  className={`w-3 h-3 rounded-sm ${
                    cell.intensity === 0 ? 'bg-background-muted' :
                    cell.intensity === 1 ? 'bg-green-200 dark:bg-green-900' :
                    cell.intensity === 2 ? 'bg-green-300 dark:bg-green-700' :
                    cell.intensity === 3 ? 'bg-green-400 dark:bg-green-600' :
                    'bg-green-500 dark:bg-green-500'
                  }`}
                  title={`${cell.date}: ${cell.count} sessions`}
                />
              ))}
            </div>
            <div className="mt-3 text-xs text-text-muted font-mono">
              Last 35 days
            </div>
          </div>
        );
      },
    },
    {
      id: 'tokens-widget',
      label: 'Tokens',
      isWidget: true,
      renderContent: () => {
        // Format tokens in millions
        const tokensInMillions = (totalTokens / 1000000).toFixed(2);
        
        return (
          <div className="w-full h-full flex flex-col items-center justify-center p-6">
            <div className="text-center">
              <div className="text-3xl font-mono font-light text-text-default mb-2">
                {tokensInMillions}M
              </div>
              <div className="text-xs text-text-muted font-mono">
                Total tokens
              </div>
            </div>
          </div>
        );
      },
    },
  ];

  // Initialize tile order on first render
  useEffect(() => {
    if (tileOrder.length === 0) {
      setTileOrder(navItemsBase.map(item => item.id));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Get ordered nav items
  const navItems = tileOrder.length > 0
    ? tileOrder.map(id => navItemsBase.find(item => item.id === id)!).filter(Boolean)
    : navItemsBase;

  // Track value changes for pulse animation
  useEffect(() => {
    const currentValues: Record<string, string> = {
      chat: `${todayChatsCount}`,
      history: `${totalSessions}`,
      recipes: `${recipesCount}`,
      scheduler: `${scheduledTodayCount}`,
      tokens: `${totalTokens}`,
    };

    Object.entries(currentValues).forEach(([key, value]) => {
      if (prevValues[key] && prevValues[key] !== value) {
        // Value changed, trigger pulse
        setPulsingItems(prev => new Set(prev).add(key));
        setTimeout(() => {
          setPulsingItems(prev => {
            const next = new Set(prev);
            next.delete(key);
            return next;
          });
        }, 2000); // Pulse for 2 seconds
      }
    });

    setPrevValues(currentValues);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [todayChatsCount, totalSessions, recipesCount, scheduledTodayCount, totalTokens]);

  // Drag and drop handlers
  const handleDragStart = (e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, itemId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    if (draggedItem && draggedItem !== itemId) {
      setDragOverItem(itemId);
    }
  };

  const handleDrop = (e: React.DragEvent, dropItemId: string) => {
    e.preventDefault();
    if (!draggedItem || draggedItem === dropItemId) return;

    const newOrder = [...tileOrder];
    const draggedIndex = newOrder.indexOf(draggedItem);
    const dropIndex = newOrder.indexOf(dropItemId);

    // Remove dragged item
    newOrder.splice(draggedIndex, 1);
    // Insert at new position
    newOrder.splice(dropIndex, 0, draggedItem);

    setTileOrder(newOrder);
    setDraggedItem(null);
    setDragOverItem(null);
  };

  const handleDragEnd = () => {
    setDraggedItem(null);
    setDragOverItem(null);
  };

  const isActive = (path: string) => {
    return location.pathname === path;
  };

  return (
    <div className="bg-background-muted overflow-hidden relative z-50">
      {/* Expanded Navigation Cards with Spring Animation */}
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ 
              height: "auto", 
              opacity: 1,
            }}
            exit={{ 
              height: 0, 
              opacity: 0,
            }}
            transition={{
              height: {
                type: "spring",
                stiffness: 300,
                damping: 30,
                mass: 0.8,
              },
              opacity: {
                duration: 0.3,
                ease: "easeInOut"
              }
            }}
            className="bg-background-muted overflow-hidden"
          >
            <motion.div
              initial={{ y: -20 }}
              animate={{ y: 0 }}
              exit={{ y: -20 }}
              transition={{
                type: "spring",
                stiffness: 400,
                damping: 25,
              }}
              className="pb-0.5 overflow-y-auto lg:max-h-[2000px] md:max-h-[calc(100vh-60px)] max-h-screen"
            >
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-5 gap-0.5">
            {navItems.map((item, index) => {
              const isPulsing = pulsingItems.has(item.id);
              const isDragging = draggedItem === item.id;
              const isDragOver = dragOverItem === item.id;

              // Widget tiles (non-clickable but draggable)
              if (item.isWidget) {
                return (
                  <motion.div
                    key={item.id}
                    draggable
                    onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                    onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                    onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                    onDragEnd={handleDragEnd}
                    initial={{ opacity: 0, y: 20, scale: 0.9 }}
                    animate={{ 
                      opacity: 1, 
                      y: 0, 
                      scale: isDragging ? 0.95 : 1,
                    }}
                    transition={{
                      type: "spring",
                      stiffness: 350,
                      damping: 25,
                      delay: index * 0.03, // 30ms stagger
                    }}
                    className={`
                      relative bg-background-default rounded-2xl aspect-square 
                      overflow-hidden cursor-move group
                      ${isDragOver ? 'ring-2 ring-blue-500' : ''}
                      ${isPulsing ? 'animate-pulse' : ''}
                    `}
                    style={{
                      opacity: isDragging ? 0.5 : 1,
                    }}
                  >
                    {/* Drag handle indicator */}
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>
                    {item.renderContent && item.renderContent()}
                  </motion.div>
                );
              }

              // Regular navigation tiles
              const IconComponent = item.icon!;
              const active = isActive(item.path!);

              return (
                <motion.div
                  key={item.id}
                  draggable
                  onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                  onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                  onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                  onDragEnd={handleDragEnd}
                  initial={{ opacity: 0, y: 20, scale: 0.9 }}
                  animate={{ 
                    opacity: 1, 
                    y: 0, 
                    scale: isDragging ? 0.95 : 1,
                  }}
                  transition={{
                    type: "spring",
                    stiffness: 350,
                    damping: 25,
                    delay: index * 0.03, // 30ms stagger
                  }}
                  className={`
                    relative cursor-move group
                    ${isDragOver ? 'ring-2 ring-blue-500 rounded-2xl' : ''}
                  `}
                  style={{
                    opacity: isDragging ? 0.5 : 1,
                  }}
                >
                  <motion.button
                    onClick={() => {
                      navigate(item.path!);
                      setIsExpanded(false);
                    }}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    className={`
                      w-full relative flex flex-col items-start justify-between
                      rounded-2xl
                      px-6 py-6 aspect-square
                      transition-colors duration-200
                      no-drag
                      ${active 
                        ? 'bg-background-accent text-text-on-accent' 
                        : 'bg-background-default hover:bg-background-medium'
                      }
                      ${isPulsing ? 'ring-2 ring-blue-400' : ''}
                    `}
                  >
                    {/* Drag handle indicator */}
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>

                    {/* Tag in top corner */}
                    {item.getTag && (
                      <div className={`absolute top-4 px-2 py-1 bg-background-muted rounded-full ${
                        item.tagAlign === 'left' ? 'left-4' : 'right-4'
                      }`}>
                        <span className="text-xs text-text-muted font-mono">{item.getTag()}</span>
                      </div>
                    )}
                    
                    {/* Icon and Label at bottom */}
                    <div className="mt-auto w-full">
                      <IconComponent className="w-6 h-6 mb-2" />
                      <h2 className="text-2xl font-light text-left">{item.label}</h2>
                    </div>
                  </motion.button>
                </motion.div>
              );
            })}
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
