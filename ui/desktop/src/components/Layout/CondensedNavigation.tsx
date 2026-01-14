import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Home, History, FileText, Puzzle, Settings as SettingsIcon, GripVertical, Clock } from 'lucide-react';
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

interface CondensedNavigationProps {
  isExpanded: boolean;
  setIsExpanded: (expanded: boolean) => void;
  position?: NavigationPosition;
  isOverlayMode?: boolean;
}

export const CondensedNavigation: React.FC<CondensedNavigationProps> = ({ 
  isExpanded, 
  setIsExpanded, 
  position = 'top',
  isOverlayMode = false
}) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { extensionsList, getExtensions } = useConfig();
  const { preferences, updatePreferences } = useNavigationCustomization();
  const [forceUpdate, setForceUpdate] = useState(0);
  const [currentTime, setCurrentTime] = useState('');
  const [todayChatsCount, setTodayChatsCount] = useState(0);
  const [totalSessions, setTotalSessions] = useState(0);
  const [recipesCount, setRecipesCount] = useState(0);
  const [scheduledTodayCount, setScheduledTodayCount] = useState(0);
  const [totalTokens, setTotalTokens] = useState(0);
  const [isClosing, setIsClosing] = useState(false);
  
  const [sessionHeatmapData, setSessionHeatmapData] = useState<Record<string, number>>({});
  
  // Track previous values for pulse animation
  const [prevValues, setPrevValues] = useState<Record<string, string>>({});
  const [pulsingItems, setPulsingItems] = useState<Set<string>>(new Set());
  
  // Drag and drop state
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  
  // Track if we should use blade overlay (push mode on small screens)
  const [shouldUseBladeOverlay, setShouldUseBladeOverlay] = useState(false);

  // Check screen size for blade overlay mode
  useEffect(() => {
    const checkScreenSize = () => {
      setShouldUseBladeOverlay(window.innerWidth < 900);
    };
    
    checkScreenSize();
    window.addEventListener('resize', checkScreenSize);
    
    return () => window.removeEventListener('resize', checkScreenSize);
  }, []);

  // Handle close with animation
  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      setIsExpanded(false);
      setIsClosing(false);
    }, 250);
  };

  // Handle escape key to close overlay or blade overlay
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const shouldCloseOnEscape = isOverlayMode || (!isOverlayMode && shouldUseBladeOverlay);
      if (event.key === 'Escape' && isExpanded && shouldCloseOnEscape) {
        event.preventDefault();
        event.stopPropagation();
        handleClose();
      }
    };

    const shouldListen = (isOverlayMode || shouldUseBladeOverlay) && isExpanded;
    if (shouldListen) {
      document.addEventListener('keydown', handleKeyDown, { capture: true });
      return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
    }
    return undefined;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isExpanded, isOverlayMode, shouldUseBladeOverlay]);

  // Update time every second
  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setCurrentTime(now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: true }));
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
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
        <div className="relative w-24 h-24">
          {/* Hour markers */}
          {[...Array(12)].map((_, i) => (
            <div
              key={i}
              className="absolute w-0.5 h-2 bg-text-muted/40 top-2 left-1/2 -translate-x-1/2"
              style={{
                transformOrigin: '50% 46px',
                transform: `translateX(-50%) rotate(${i * 30}deg)`,
              }}
            />
          ))}
          
          {/* Hour hand */}
          <div
            className="absolute w-1.5 h-9 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
            style={{
              transform: `translateX(-50%) translateY(-100%) rotate(${hourAngle}deg)`,
            }}
          />
          
          {/* Minute hand */}
          <div
            className="absolute w-1 h-11 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
            style={{
              transform: `translateX(-50%) translateY(-100%) rotate(${minuteAngle}deg)`,
            }}
          />
          
          {/* Second hand */}
          <div
            className="absolute w-0.5 h-12 bg-red-500 rounded-full top-1/2 left-1/2 -translate-x-1/2 origin-bottom transition-transform duration-1000 ease-linear"
            style={{
              transform: `translateX(-50%) translateY(-100%) rotate(${secondAngle}deg)`,
            }}
          />
          
          {/* Center dot */}
          <div className="absolute w-2 h-2 bg-text-default rounded-full top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" />
        </div>
      </div>
    );
  };

  // Filter and order navigation items based on user preferences
  const navItems = React.useMemo(() => {
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
        getTag: () => `${todayChatsCount}`,
        tagAlign: 'left',
      },
      {
        id: 'history',
        path: '/sessions',
        label: 'History',
        icon: History,
        getTag: () => `${totalSessions}`,
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
            return '0/0';
          }
          const enabled = extensionsList.filter(ext => ext.enabled).length;
          const total = extensionsList.length;
          return `${enabled}/${total}`;
        },
      },
      {
        id: 'settings',
        path: '/settings',
        label: 'Settings',
        icon: SettingsIcon,
      },
      // Widget tiles for overlay mode
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
    
    // Filter enabled items
    const enabledItems = navItemsBase.filter(item => 
      preferences.enabledItems.includes(item.id)
    );

    // Filter widgets based on overlay mode
    const filteredItems = enabledItems.filter(item => 
      isOverlayMode || !item.isWidget
    );

    // Order items according to user preferences
    const orderedItems = preferences.itemOrder
      .map(id => filteredItems.find(item => item.id === id))
      .filter(Boolean) as NavItem[];

    // Add any new items that aren't in the order yet (for backwards compatibility)
    const itemsNotInOrder = filteredItems.filter(item => 
      !preferences.itemOrder.includes(item.id)
    );

    return [...orderedItems, ...itemsNotInOrder];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preferences, forceUpdate, isOverlayMode, currentTime, todayChatsCount, totalSessions, recipesCount, totalTokens]);

  // Listen for navigation preferences updates
  useEffect(() => {
    const handlePreferencesUpdate = () => {
      // Force re-render when preferences change
      setForceUpdate(prev => prev + 1);
    };

    window.addEventListener('navigation-preferences-updated', handlePreferencesUpdate);
    return () => {
      window.removeEventListener('navigation-preferences-updated', handlePreferencesUpdate);
    };
  }, []);

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

  // Determine layout based on position
  const isVertical = position === 'left' || position === 'right';
  
  // Blade overlay mode: push mode on small screens becomes overlay-style (transparent but full height)
  const isBladeOverlay = !isOverlayMode && shouldUseBladeOverlay;
  
  // Position-aware container classes
  const containerClasses = isOverlayMode
    ? `w-full h-full flex ${
        position === 'top' ? 'items-start justify-center' :
        position === 'bottom' ? 'items-end justify-center' :
        position === 'left' ? 'items-center justify-start' :
        'items-center justify-end'
      }`
    : isVertical
      ? 'h-full'
      : 'w-full overflow-hidden';

  return (
    <div className={`${isOverlayMode || isBladeOverlay ? 'bg-transparent' : 'bg-background-default'} ${containerClasses} relative z-[9998]`}>
      {(isExpanded || isClosing) && (
        <div
          className={`${isClosing ? 'nav-tile-exit' : 'nav-tile'} transition-all duration-300 ${isOverlayMode || isBladeOverlay ? 'bg-transparent' : 'bg-background-default overflow-hidden'} ${
            !isOverlayMode && isVertical ? 'h-full lg:relative lg:z-auto absolute z-[10000] top-0 shadow-lg lg:shadow-none' : ''
          } ${
            !isOverlayMode && isVertical && position === 'left' ? 'left-0' : ''
          } ${
            !isOverlayMode && isVertical && position === 'right' ? 'right-0' : ''
          }`}
        >
          <div
            className={`transition-all duration-200 ${isOverlayMode ? '' : ''} ${
              !isOverlayMode && isVertical 
                ? position === 'right' 
                  ? 'h-full pl-0.5' 
                  : 'h-full pr-0.5'
                : !isOverlayMode && position === 'top'
                  ? 'pr-0.5 pb-0.5'
                  : !isOverlayMode ? 'pr-0.5' : ''
            }`}
          >
{isOverlayMode ? (
                // Overlay Mode: Responsive flex layout with scrolling
                <div className={`flex flex-col md:flex-row gap-2 md:gap-4 max-h-[90vh] overflow-y-auto px-4 sm:px-6 md:px-8 py-4 sm:py-6 md:py-8 ${
                  position === 'left' || position === 'right' ? 'items-center' : 
                  position === 'bottom' ? 'items-end' : 'items-start'
                }`}>
                  {/* Menu Container: Windows Start Menu style - Responsive width */}
                  <div 
                    className="bg-background-card backdrop-blur-xl rounded-2xl shadow-2xl border border-border-default overflow-hidden w-full md:w-80 flex-shrink-0"
                    style={{ maxHeight: '500px' }}
                  >
                    {/* Navigation Rows */}
                    <div className="flex flex-col gap-[1px] p-4 overflow-y-auto max-h-[500px]">
                    {navItems.filter(item => !item.isWidget).map((item, index) => {
                      const isPulsing = pulsingItems.has(item.id);
                      const isDragging = draggedItem === item.id;
                      const isDragOver = dragOverItem === item.id;
                      const IconComponent = item.icon;
                      const active = item.path ? isActive(item.path) : false;

                      return (
                        <div
                          key={item.id}
                          draggable
                          onDragStart={(e) => handleDragStart(e, item.id)}
                          onDragOver={(e) => handleDragOver(e, item.id)}
                          onDrop={(e) => handleDrop(e, item.id)}
                          onDragEnd={handleDragEnd}
                          className={`
                            ${isClosing ? 'nav-tile-exit' : 'nav-tile'} transition-all duration-300
                            relative cursor-move group w-full
                            ${isDragOver ? 'ring-2 ring-blue-500 rounded-lg' : ''}
                            ${isDragging ? 'scale-95' : ''}
                          `}
                          style={{
                            opacity: isDragging ? 0.5 : 1,
                            animationDelay: `${index * 30}ms`,
                          }}
                        >
                          <button
                            onClick={() => {
                              if (item.path) {
                                navigate(item.path);
                                handleClose();
                              }
                            }}
                            className={`
                              flex flex-row items-center gap-2 w-full
                              relative rounded-lg transition-all duration-200 no-drag
                              pl-2 pr-4 py-2.5
                              transition-transform hover:scale-[1.02] active:scale-[0.98]
                              ${active 
                                ? 'bg-background-accent text-text-on-accent backdrop-blur-md' 
                                : 'bg-background-default text-text-default hover:bg-background-medium backdrop-blur-md'
                              }
                            `}
                          >
                            {/* Drag handle indicator */}
                            <div className="opacity-0 group-hover:opacity-100 transition-opacity">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>

                            {/* Icon */}
                            {IconComponent && <IconComponent className="w-5 h-5 flex-shrink-0" />}
                            
                            {/* Label */}
                            <span className="text-sm font-medium flex-1 text-left">{item.label}</span>

                            {/* Tag/Badge */}
                            {item.getTag && (
                              <div className="flex items-center gap-1">
                                <span className={`text-xs font-mono px-2 py-0.5 rounded-full ${
                                  active 
                                    ? 'bg-background-default/20 text-text-on-accent/80 backdrop-blur-sm' 
                                    : 'bg-background-muted text-text-muted backdrop-blur-sm'
                                }`}>
                                  {item.getTag()}
                                </span>
                              </div>
                            )}

                            {/* Update indicator dot */}
                            {isPulsing && (
                              <div className="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                            )}
                          </button>
                        </div>
                      );
                    })}
                  </div>
                  </div>

                  {/* Widget Tiles: Responsive 2-column Grid */}
                  <div 
                    className="grid grid-cols-2 gap-2 w-full md:w-auto md:max-w-[520px]"
                  >
                    {navItems.filter(item => item.isWidget).map((item, index) => {
                      const isDragging = draggedItem === item.id;
                      const isDragOver = dragOverItem === item.id;

                      return (
                        <div
                          key={item.id}
                          draggable
                          onDragStart={(e) => handleDragStart(e, item.id)}
                          onDragOver={(e) => handleDragOver(e, item.id)}
                          onDrop={(e) => handleDrop(e, item.id)}
                          onDragEnd={handleDragEnd}
                          className={`
                            ${isClosing ? 'nav-tile-exit' : 'nav-tile'} transition-all duration-300
                            relative cursor-move group
                            ${isDragOver ? 'ring-2 ring-blue-500 rounded-2xl' : ''}
                            ${isDragging ? 'scale-95' : ''}
                            aspect-square
                          `}
                          style={{
                            opacity: isDragging ? 0.5 : 1,
                            animationDelay: `${(index + 7) * 30}ms`,
                          }}
                        >
                          <div className="w-full h-full bg-background-card backdrop-blur-xl rounded-2xl shadow-2xl border border-border-default overflow-hidden relative group">
                            {/* Drag handle indicator */}
                            <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>
                            {item.renderContent && item.renderContent()}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : (
                // Push Mode: Original single-column layout
                <div className={`${isVertical ? 'flex flex-col w-full gap-[1px] h-full' : 'flex flex-row w-full gap-[1px]'}`}>
                  {/* Top spacer for vertical layout */}
                  {(isVertical || position === 'top') && (
                    <div className={`${isVertical ? 'h-[60px] w-full' : 'w-[100px]'} bg-background-default rounded-lg flex items-center justify-center py-2.5`} />
                  )}
                  
                  {navItems.filter(item => !item.isWidget).map((item, index) => {
                    const isPulsing = pulsingItems.has(item.id);
                    const isDragging = draggedItem === item.id;
                    const isDragOver = dragOverItem === item.id;
                    const IconComponent = item.icon;
                    const active = item.path ? isActive(item.path) : false;

                    return (
                      <div
                        key={item.id}
                        draggable
                        onDragStart={(e) => handleDragStart(e, item.id)}
                        onDragOver={(e) => handleDragOver(e, item.id)}
                        onDrop={(e) => handleDrop(e, item.id)}
                        onDragEnd={handleDragEnd}
                        className={`
                          ${isClosing ? 'nav-tile-exit' : 'nav-tile'} transition-all duration-300
                          relative cursor-move group
                          ${isVertical ? 'w-full' : 'flex-1'}
                          ${isDragOver ? 'ring-2 ring-blue-500 rounded-lg' : ''}
                          ${isDragging ? 'scale-95' : ''}
                        `}
                        style={{
                          opacity: isDragging ? 0.5 : 1,
                          animationDelay: `${index * 30}ms`,
                        }}
                      >
                        <button
                          onClick={() => {
                            if (item.path) {
                              navigate(item.path);
                              handleClose();
                            }
                          }}
                          className={`
                            flex flex-row items-center gap-2 w-full
                            relative rounded-lg transition-all duration-200 no-drag
                            transition-transform hover:scale-[1.02] active:scale-[0.98]
                            ${!isVertical ? 'px-2 pt-[18px] pb-[14px] min-[1800px]:pl-2 min-[1800px]:pr-4' : 'pl-2 pr-4 py-2.5'}
                            ${active 
                              ? 'bg-background-accent text-text-on-accent' 
                              : 'bg-background-default hover:bg-background-medium'
                            }
                          `}
                        >
                          {/* Drag handle indicator */}
                          <div className={`opacity-0 group-hover:opacity-100 transition-opacity ${!isVertical ? 'hidden min-[1800px]:block' : ''}`}>
                            <GripVertical className="w-4 h-4 text-text-muted" />
                          </div>

                          {/* Icon */}
                          {IconComponent && <IconComponent className={`w-5 h-5 flex-shrink-0 ${!isVertical ? 'mx-auto min-[1800px]:mx-0' : ''}`} />}
                          
                          {/* Label */}
                          <span className={`text-sm font-medium flex-1 text-left ${!isVertical ? 'hidden min-[1800px]:block' : ''}`}>{item.label}</span>

                          {/* Tag/Badge */}
                          {item.getTag && (
                            <div className={`flex items-center gap-1 ${!isVertical ? 'hidden min-[1800px]:flex' : ''}`}>
                              <span className={`text-xs font-mono px-2 py-0.5 rounded-full ${
                                active 
                                  ? 'bg-background-default/20 text-text-on-accent/80' 
                                  : 'bg-background-muted text-text-muted'
                              }`}>
                                {item.getTag()}
                              </span>
                            </div>
                          )}

                          {/* Update indicator dot */}
                          {isPulsing && (
                            <div className="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                          )}
                        </button>
                      </div>
                    );
                  })}
                  
                  {/* Bottom spacer */}
                  {(isVertical || position === 'top' || position === 'bottom') && (
                    <div className={`${
                      isVertical 
                        ? 'flex-1 w-full min-h-[60px]' 
                        : position === 'top'
                          ? 'w-[160px]'
                          : 'flex-1 min-w-[60px]'
                    } bg-background-default rounded-lg flex items-center justify-center py-2.5`} />
                  )}
                </div>
              )}
          </div>
        </div>
      )}
    </div>
  );
};
