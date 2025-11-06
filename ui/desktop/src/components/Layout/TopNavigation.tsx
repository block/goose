import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Home, History, FileText, Clock, Puzzle, Settings as SettingsIcon, GripVertical } from 'lucide-react';
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
  const [angles, setAngles] = useState(() => {
    const now = new Date();
    const hours = now.getHours() % 12;
    const minutes = now.getMinutes();
    const seconds = now.getSeconds();
    return {
      hour: (hours * 30) + (minutes * 0.5),
      minute: minutes * 6,
      second: seconds * 6,
    };
  });

  useEffect(() => {
    const interval = setInterval(() => {
      const now = new Date();
      const hours = now.getHours() % 12;
      const minutes = now.getMinutes();
      const seconds = now.getSeconds();
      setAngles({
        hour: (hours * 30) + (minutes * 0.5),
        minute: minutes * 6,
        second: seconds * 6,
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const hourAngle = angles.hour;
  const minuteAngle = angles.minute;
  const secondAngle = angles.second;

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
      {/* Expanded Navigation Cards */}
      <div 
        className={`bg-background-muted ${
          isExpanded 
            ? 'lg:max-h-[2000px] md:max-h-[calc(100vh-60px)] max-h-screen opacity-100 overflow-y-auto transition-all duration-700 ease-out' 
            : 'max-h-0 opacity-0 overflow-hidden transition-all duration-1000 ease-in-out'
        }`}
      >
        <div className="pb-0.5 pt-0.5">
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-5 gap-0.5">
            {navItems.map((item, index) => {
              const isPulsing = pulsingItems.has(item.id);
              const isDragging = draggedItem === item.id;
              const isDragOver = dragOverItem === item.id;

              // Widget tiles (non-clickable but draggable)
              if (item.isWidget) {
                return (
                  <div
                    key={item.id}
                    draggable
                    onDragStart={(e) => handleDragStart(e, item.id)}
                    onDragOver={(e) => handleDragOver(e, item.id)}
                    onDrop={(e) => handleDrop(e, item.id)}
                    onDragEnd={handleDragEnd}
                    className={`
                      relative bg-background-default rounded-2xl aspect-square 
                      animate-in slide-in-from-top-4 fade-in overflow-hidden
                      cursor-move group
                      transition-all duration-200
                      ${isDragging ? 'opacity-50 scale-95' : ''}
                      ${isDragOver ? 'ring-2 ring-blue-500' : ''}
                      ${isPulsing ? 'animate-pulse' : ''}
                    `}
                    style={{
                      animationDelay: `${index * 50}ms`,
                      animationDuration: '400ms',
                      animationFillMode: 'backwards',
                    }}
                  >
                    {/* Drag handle indicator */}
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>
                    {item.renderContent && item.renderContent()}
                  </div>
                );
              }

              // Regular navigation tiles
              const IconComponent = item.icon!;
              const active = isActive(item.path!);

              return (
                <div
                  key={item.id}
                  draggable
                  onDragStart={(e) => handleDragStart(e, item.id)}
                  onDragOver={(e) => handleDragOver(e, item.id)}
                  onDrop={(e) => handleDrop(e, item.id)}
                  onDragEnd={handleDragEnd}
                  className={`
                    relative cursor-move group
                    transition-all duration-200
                    ${isDragging ? 'opacity-50 scale-95' : ''}
                    ${isDragOver ? 'ring-2 ring-blue-500 rounded-2xl' : ''}
                  `}
                >
                  <button
                    onClick={() => {
                      navigate(item.path!);
                      setIsExpanded(false);
                    }}
                    className={`
                      w-full relative flex flex-col items-start justify-between
                      bg-background-default rounded-2xl
                      px-6 py-6 aspect-square
                      transition-all duration-200
                      hover:bg-background-medium
                      no-drag
                      animate-in slide-in-from-top-4 fade-in
                      ${active ? 'bg-background-medium' : ''}
                      ${isPulsing ? 'animate-pulse ring-2 ring-blue-400' : ''}
                    `}
                    style={{
                      animationDelay: `${index * 50}ms`,
                      animationDuration: '400ms',
                      animationFillMode: 'backwards',
                    }}
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
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};
