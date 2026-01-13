import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Home, History, FileText, Clock, Puzzle, Settings as SettingsIcon, GripVertical } from 'lucide-react';
import { ChatSmart } from '../icons';
import { listSessions, getSessionInsights } from '../../api';
import { useConfig } from '../ConfigContext';
import { listSavedRecipes } from '../../recipe/recipe_management';
import { useNavigationCustomization } from '../settings/app/NavigationCustomizationSettings';

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

export type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

interface TopNavigationProps {
  isExpanded: boolean;
  setIsExpanded: (expanded: boolean) => void;
  position?: NavigationPosition;
  isOverlayMode?: boolean;
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
      <div className="relative w-20 h-20 sm:w-24 sm:h-24 md:w-28 md:h-28 lg:w-32 lg:h-32 xl:w-36 xl:h-36">
        {/* Hour markers (ticks) - hidden on small/medium viewports, visible on large+ */}
        {[...Array(12)].map((_, i) => (
          <div
            key={i}
            className="absolute w-0.5 h-2 bg-text-muted/40 left-1/2 -translate-x-1/2 hidden lg:block"
            style={{
              top: '0.5rem',
              transformOrigin: 'center calc(50% + 3.5rem)',
              transform: `translateX(-50%) rotate(${i * 30}deg)`,
            }}
          />
        ))}
        
        {/* Hour hand - always visible */}
        <div
          className="absolute w-1 h-10 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${hourAngle}deg)`,
          }}
        />
        
        {/* Minute hand - always visible */}
        <div
          className="absolute w-0.5 h-14 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${minuteAngle}deg)`,
          }}
        />
        
        {/* Second hand - always visible */}
        <div
          className="absolute w-px h-16 bg-red-500 rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
          style={{
            transform: `translateX(-50%) translateY(-100%) rotate(${secondAngle}deg)`,
          }}
        />
        
        {/* Center dot - always visible */}
        <div className="absolute w-3 h-3 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" />
      </div>
    </div>
  );
};

export const TopNavigation: React.FC<TopNavigationProps> = ({ isExpanded, setIsExpanded, position = 'top', isOverlayMode = false }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { extensionsList, getExtensions } = useConfig();
  const { preferences, updatePreferences } = useNavigationCustomization();
  const [forceUpdate, setForceUpdate] = useState(0);
  const [currentTime, setCurrentTime] = useState('');
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recipesCount, setRecipesCount] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [isClosing, setIsClosing] = useState(false);

  // Handle close with animation
  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setIsExpanded(false);
      setIsClosing(false);
    }, 250);
  };

  // Handle escape key to close overlay
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isExpanded && isOverlayMode) {
        event.preventDefault();
        event.stopPropagation();
        handleClose();
      }
    };

    if (isOverlayMode && isExpanded) {
      document.addEventListener('keydown', handleKeyDown, { capture: true });
      return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isExpanded, isOverlayMode]);

  const [sessionHeatmapData, setSessionHeatmapData] = useState<Record<string, number>>({});
  
  const [prevValues, setPrevValues] = useState<Record<string, string>>({});
  const [pulsingItems, setPulsingItems] = useState<Set<string>>(new Set());
  
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  
  const [isUltraWide, setIsUltraWide] = useState(false);

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true }));
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const checkUltraWide = () => {
      setIsUltraWide(window.innerWidth >= 2536);
    };
    
    checkUltraWide();
    window.addEventListener('resize', checkUltraWide);
    
    return () => window.removeEventListener('resize', checkUltraWide);
  }, []);

  useEffect(() => {
    if (isExpanded) {
      fetchNavigationData();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isExpanded]);

  const fetchNavigationData = async () => {
    try {
      const sessionsResponse = await listSessions({ throwOnError: false });
      if (sessionsResponse.data) {
        const today = new Date();
        today.setHours(0, 0, 0, 0);
        
        const heatmap: Record<string, number> = {};
        let todayCount = 0;
        
        sessionsResponse.data.sessions.forEach((session) => {
          const sessionDate = new Date(session.created_at);
          const dateKey = sessionDate.toISOString().split('T')[0];
          heatmap[dateKey] = (heatmap[dateKey] || 0) + 1;
          
          const sessionDateOnly = new Date(session.created_at);
          sessionDateOnly.setHours(0, 0, 0, 0);
          if (sessionDateOnly.getTime() === today.getTime()) {
            todayCount++;
          }
        });
        
        setSessionHeatmapData(heatmap);
        setTodayChatsCount(todayCount);
      }

      const insightsResponse = await getSessionInsights({ throwOnError: false });
      if (insightsResponse.data) {
        setTotalSessions(insightsResponse.data.totalSessions || 0);
        setTotalTokens(insightsResponse.data.totalTokens || 0);
      }

      try {
        const recipes = await listSavedRecipes();
        setRecipesCount(recipes.length);
      } catch (error) {
        console.error('Failed to fetch recipes:', error);
        setRecipesCount(0);
      }

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
        const days = 35;
        const today = new Date();
        const heatmapCells = [];
        
        for (let i = days - 1; i >= 0; i--) {
          const date = new Date(today);
          date.setDate(date.getDate() - i);
          const dateKey = date.toISOString().split('T')[0];
          const count = sessionHeatmapData[dateKey] || 0;
          
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

  const navItems = React.useMemo(() => {
    const enabledItems = navItemsBase.filter(item => 
      preferences.enabledItems.includes(item.id)
    );

    const orderedItems = preferences.itemOrder
      .map(id => enabledItems.find(item => item.id === id))
      .filter(Boolean) as NavItem[];

    const itemsNotInOrder = enabledItems.filter(item => 
      !preferences.itemOrder.includes(item.id)
    );

    return [...orderedItems, ...itemsNotInOrder];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [navItemsBase, preferences.enabledItems, preferences.itemOrder, forceUpdate, currentTime, todayChatsCount, totalSessions, recipesCount, totalTokens]);

  useEffect(() => {
    const handlePreferencesUpdate = () => {
      setForceUpdate(prev => prev + 1);
    };

    window.addEventListener('navigation-preferences-updated', handlePreferencesUpdate);
    return () => {
      window.removeEventListener('navigation-preferences-updated', handlePreferencesUpdate);
    };
  }, []);

  useEffect(() => {
    const currentValues: Record<string, string> = {
      chat: `${todayChatsCount}`,
      history: `${totalSessions}`,
      recipes: `${recipesCount}`,
      tokens: `${totalTokens}`,
    };

    Object.entries(currentValues).forEach(([key, value]) => {
      if (prevValues[key] && prevValues[key] !== value) {
        setPulsingItems(prev => new Set(prev).add(key));
        setTimeout(() => {
          setPulsingItems(prev => {
            const next = new Set(prev);
            next.delete(key);
            return next;
          });
        }, 2000);
      }
    });

    setPrevValues(currentValues);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [todayChatsCount, totalSessions, recipesCount, totalTokens]);

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

    // Get current order from preferences
    const currentOrder = [...preferences.itemOrder];
    const draggedIndex = currentOrder.indexOf(draggedItem);
    const dropIndex = currentOrder.indexOf(dropItemId);

    // Reorder
    currentOrder.splice(draggedIndex, 1);
    currentOrder.splice(dropIndex, 0, draggedItem);

    // Save to preferences
    updatePreferences({
      ...preferences,
      itemOrder: currentOrder,
    });

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

  const isVertical = position === 'left' || position === 'right';
  
  const gridClasses = isOverlayMode
    ? 'grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-[2px] w-full max-w-7xl mx-auto px-4 sm:px-6 md:px-8'
    : isVertical
      ? 'grid grid-cols-1 gap-[2px] h-full overflow-y-auto'
      : 'grid grid-cols-2 md:grid-cols-5 2xl:grid-cols-10 gap-[2px]'; // Push mode: 2 cols → 5 cols (tablet) → 10 cols (large desktop)
  
  const containerClasses = isOverlayMode
    ? 'w-full h-full flex items-center justify-center overflow-hidden' // Always centered for overlay mode
    : isVertical
      ? 'h-full'
      : 'w-full overflow-y-auto'; // Enable vertical scrolling for horizontal push mode

  return (
    <div className={`${isOverlayMode ? 'bg-transparent' : 'bg-background-muted'} ${containerClasses} relative z-[9998]`}>
        {(isExpanded || isClosing) && (
          <div
            className={`${isOverlayMode ? 'bg-transparent' : 'bg-background-muted overflow-hidden'} ${isVertical && !isOverlayMode ? 'h-full' : ''} ${isClosing ? 'nav-overlay-exit' : 'nav-overlay-enter'} transition-all duration-300`}
          >
            <div
              className={`${isOverlayMode ? 'overflow-y-auto max-h-[90vh] py-4 sm:py-6 md:py-8' : isVertical ? 'p-1 h-full' : 'pb-0.5 lg:max-h-[2000px] md:max-h-[calc(100vh-60px)] max-h-screen'} transition-all duration-300`}
              style={{ width: isVertical && !isOverlayMode ? '360px' : undefined }}
            >
              <div 
                className={gridClasses} 
                style={{ 
                  ...(!isVertical && !isOverlayMode && isUltraWide ? { gridTemplateColumns: 'repeat(12, minmax(0, 1fr))' } : {}),
                  ...(isVertical && !isOverlayMode ? { width: 'auto' } : {})
                }}
              >
            {navItems.map((item, index) => {
              const isPulsing = pulsingItems.has(item.id);
              const isDragging = draggedItem === item.id;
              const isDragOver = dragOverItem === item.id;

              if (item.isWidget) {
                return (
                  <div
                    key={item.id}
                    draggable
                    onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                    onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                    onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                    onDragEnd={handleDragEnd}
                    className={`
                      ${isClosing ? 'nav-tile-exit' : 'nav-tile'}
                      relative ${isOverlayMode ? 'bg-background-default backdrop-blur-md' : 'bg-background-default'} rounded-2xl 
                      overflow-hidden cursor-move group
                      ${isDragOver ? 'ring-2 ring-blue-500' : ''}
                      ${isPulsing ? 'animate-pulse' : ''}
                      aspect-square
                      transition-all duration-300
                    `}
                    style={{
                      opacity: isDragging ? 0.5 : 1,
                      animationDelay: `${index * 30}ms`,
                    }}
                  >
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>
                    {item.renderContent && item.renderContent()}
                  </div>
                );
              }

              const IconComponent = item.icon!;
              const active = isActive(item.path!);

              return (
                <div
                  key={item.id}
                  draggable
                  onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                  onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                  onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                  onDragEnd={handleDragEnd}
                  className={`
                    ${isClosing ? 'nav-tile-exit' : 'nav-tile'}
                    relative cursor-move group
                    ${isDragOver ? 'ring-2 ring-blue-500 rounded-2xl' : ''}
                    transition-all duration-300
                  `}
                  style={{
                    opacity: isDragging ? 0.5 : 1,
                    animationDelay: `${index * 30}ms`,
                  }}
                >
                  <button
                    onClick={() => {
                      navigate(item.path!);
                      handleClose();
                    }}
                    className={`
                      w-full relative flex flex-col items-start justify-between
                      rounded-2xl
                      ${isOverlayMode ? 'px-5 py-5' : 'px-6 py-6'}
                      transition-all duration-300
                      no-drag
                      ${isOverlayMode 
                        ? (active 
                          ? 'bg-background-accent text-text-on-accent backdrop-blur-md' 
                          : 'bg-background-default text-text-default hover:bg-background-medium backdrop-blur-md'
                        )
                        : (active 
                          ? 'bg-background-accent text-text-on-accent' 
                          : 'bg-background-default hover:bg-background-medium'
                        )
                      }
                      aspect-square
                      transition-transform hover:scale-[1.02] active:scale-[0.98]
                    `}
                  >
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>

                    {isPulsing && (
                      <div
                        className="absolute bottom-3 right-3 w-2 h-2 bg-blue-500 rounded-full animate-pulse"
                      />
                    )}

                    {item.getTag && (
                      <div className={`absolute ${isOverlayMode ? 'top-3' : 'top-4'} px-2 py-1 rounded-full ${
                        item.tagAlign === 'left' ? 'left-3' : 'right-3'
                      } ${isOverlayMode ? 'bg-background-muted backdrop-blur-sm' : 'bg-background-muted'}`}>
                        <span className={`${isOverlayMode ? 'text-[10px]' : 'text-xs'} font-mono ${isOverlayMode ? 'text-text-muted' : 'text-text-muted'}`}>{item.getTag()}</span>
                      </div>
                    )}
                    
                    <div className="mt-auto w-full">
                      {IconComponent && <IconComponent className={`${isOverlayMode ? 'w-6 h-6 mb-2' : 'w-6 h-6 mb-2'}`} />}
                      {item.label && <h2 className={`font-light text-left ${isOverlayMode ? 'text-xl' : 'text-2xl'}`}>{item.label}</h2>}
                    </div>
                  </button>
                </div>
              );
            })}
              </div>
            </div>
          </div>
        )}
    </div>
  );
};
