import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Home, History, FileText, Clock, Puzzle, Settings as SettingsIcon, GripVertical, Users, Hash, ShoppingBag } from 'lucide-react';
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

export type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

interface CondensedNavigationProps {
  isExpanded: boolean;
  setIsExpanded: (expanded: boolean) => void;
  position?: NavigationPosition;
}

export const CondensedNavigation: React.FC<CondensedNavigationProps> = ({ 
  isExpanded, 
  setIsExpanded, 
  position = 'top' 
}) => {
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
  const [itemOrder, setItemOrder] = useState<string[]>([]);

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
      label: 'Marketplace',
      icon: ShoppingBag,
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
      id: 'peers',
      path: '/peers',
      label: 'Peers',
      icon: Users,
      getTag: () => '3',
      tagAlign: 'left',
    },
    {
      id: 'channels',
      path: '/channels',
      label: 'Channels',
      icon: Hash,
      getTag: () => '7',
      tagAlign: 'left',
    },
    {
      id: 'settings',
      path: '/settings',
      label: 'Settings',
      icon: SettingsIcon,
    },
  ];

  // Initialize item order on first render
  useEffect(() => {
    if (itemOrder.length === 0) {
      setItemOrder(navItemsBase.map(item => item.id));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Get ordered nav items (exclude widgets for condensed view)
  const navItems = itemOrder.length > 0
    ? itemOrder.map(id => navItemsBase.find(item => item.id === id)!).filter(item => item && !item.isWidget)
    : navItemsBase.filter(item => !item.isWidget);

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

    const newOrder = [...itemOrder];
    const draggedIndex = newOrder.indexOf(draggedItem);
    const dropIndex = newOrder.indexOf(dropItemId);

    newOrder.splice(draggedIndex, 1);
    newOrder.splice(dropIndex, 0, draggedItem);

    setItemOrder(newOrder);
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

  return (
    <div className={`bg-background-muted relative z-[9998] ${isVertical ? 'h-full' : 'w-full'}`}>
      <AnimatePresence initial={false}>
        {isExpanded && (
          <motion.div
            initial={{ 
              [isVertical ? 'width' : 'height']: 0, 
              opacity: 0 
            }}
            animate={{ 
              [isVertical ? 'width' : 'height']: "auto", 
              opacity: 1,
            }}
            exit={{ 
              [isVertical ? 'width' : 'height']: 0, 
              opacity: 0,
            }}
            transition={{
              [isVertical ? 'width' : 'height']: {
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
            className={`bg-background-muted overflow-hidden ${isVertical ? 'h-full lg:relative lg:z-auto absolute z-[10000] top-0 shadow-lg lg:shadow-none' : ''} ${
              isVertical && position === 'left' ? 'left-0' : ''
            } ${
              isVertical && position === 'right' ? 'right-0' : ''
            }`}
          >
            <motion.div
              initial={{ [isVertical ? 'x' : 'y']: -20 }}
              animate={{ [isVertical ? 'x' : 'y']: 0 }}
              exit={{ [isVertical ? 'x' : 'y']: -20 }}
              transition={{
                type: "spring",
                stiffness: 400,
                damping: 25,
              }}
              className={`${
                isVertical 
                  ? position === 'right' 
                    ? 'h-full pl-0.5' 
                    : 'h-full pr-0.5'
                  : position === 'top'
                    ? 'pr-0.5 pb-0.5'
                    : 'pr-0.5'
              }`}
              style={{ width: isVertical ? '240px' : undefined }}
            >
              <div className={`${isVertical ? 'flex flex-col w-full gap-[1px] h-full' : 'flex flex-row w-full gap-[1px]'}`}>
                {/* Left spacer - 100px for top nav, 60px height for left/right nav */}
                {(isVertical || position === 'top') && (
                  <div className={`${isVertical ? 'h-[60px] w-full' : 'w-[100px]'} bg-background-default rounded-lg flex items-center justify-center py-2.5`} />
                )}
                
                {navItems.map((item, index) => {
                  const isPulsing = pulsingItems.has(item.id);
                  const isDragging = draggedItem === item.id;
                  const isDragOver = dragOverItem === item.id;
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
                      initial={{ opacity: 0, [isVertical ? 'x' : 'y']: 20, scale: 0.9 }}
                      animate={{ 
                        opacity: 1, 
                        [isVertical ? 'x' : 'y']: 0, 
                        scale: isDragging ? 0.95 : 1,
                      }}
                      transition={{
                        type: "spring",
                        stiffness: 350,
                        damping: 25,
                        delay: index * 0.02,
                      }}
                      className={`
                        relative cursor-move group
                        ${isVertical ? 'w-full' : 'flex-1'}
                        ${isDragOver ? 'ring-2 ring-blue-500 rounded-lg' : ''}
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
                          flex flex-row items-center gap-2 w-full
                          relative
                          rounded-lg
                          transition-colors duration-200
                          no-drag
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
                        <IconComponent className={`w-5 h-5 flex-shrink-0 ${!isVertical ? 'mx-auto min-[1800px]:mx-0' : ''}`} />
                        
                        {/* Label - hidden on small screens for horizontal nav */}
                        <span className={`text-sm font-medium flex-1 text-left ${!isVertical ? 'hidden min-[1800px]:block' : ''}`}>{item.label}</span>

                        {/* Tag/Badge - hidden on small screens for horizontal nav */}
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
                          <motion.div
                            initial={{ scale: 0, opacity: 0 }}
                            animate={{ scale: 1, opacity: 1 }}
                            exit={{ scale: 0, opacity: 0 }}
                            className="absolute -top-1 -right-1 w-2 h-2 bg-blue-500 rounded-full animate-pulse"
                          />
                        )}
                      </motion.button>
                    </motion.div>
                  );
                })}
                
                {/* Right/Bottom spacer - 160px for top nav, 60px for bottom nav, flexible for left/right nav */}
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
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
