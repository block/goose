import React from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { Goose } from '../icons/Goose';
import { Button } from '../ui/button';
import ChatSessionsContainer from '../ChatSessionsContainer';
import { useChatContext } from '../../contexts/ChatContext';
import { NavigationProvider, useNavigationContext } from './NavigationContext';
import { ExpandedNavigation } from './ExpandedNavigation';
import { CondensedNavigation } from './CondensedNavigation';
import { cn } from '../../utils';
import { UserInput } from '../../types/message';

interface AppLayoutContentProps {
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
}

const AppLayoutContent: React.FC<AppLayoutContentProps> = ({ activeSessions }) => {
  const location = useLocation();
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const chatContext = useChatContext();
  const isOnPairRoute = location.pathname === '/pair';

  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    effectiveNavigationStyle,
    navigationPosition,
    isHorizontalNav,
  } = useNavigationContext();

  if (!chatContext) {
    throw new Error('AppLayoutContent must be used within ChatProvider');
  }

  const { setChat } = chatContext;

  // Calculate padding based on macOS traffic lights
  const headerPadding = safeIsMacOS ? 'pl-21' : 'pl-4';

  // Render the appropriate navigation component based on style
  const renderNavigation = () => {
    if (effectiveNavigationStyle === 'expanded') {
      return <ExpandedNavigation />;
    }
    return <CondensedNavigation />;
  };

  // Determine flex direction based on navigation position (for push mode)
  const getLayoutClass = () => {
    if (effectiveNavigationMode === 'overlay') {
      return 'flex-row';
    }

    switch (navigationPosition) {
      case 'top':
        return 'flex-col';
      case 'bottom':
        return 'flex-col-reverse';
      case 'left':
        return 'flex-row';
      case 'right':
        return 'flex-row-reverse';
      default:
        return 'flex-row';
    }
  };

  // Main content area
  const mainContent = (
    <div className="flex-1 overflow-hidden">
      <div className="h-full w-full bg-background-default rounded-lg overflow-hidden">
        <Outlet />
        {/* Always render ChatSessionsContainer to keep SSE connections alive.
            When navigating away from /pair, hide it with CSS */}
        <div className={isOnPairRoute ? 'contents' : 'hidden'}>
          <ChatSessionsContainer setChat={setChat} activeSessions={activeSessions} />
        </div>
      </div>
    </div>
  );

  return (
    <div
      className={cn(
        'flex flex-1 w-full h-full relative animate-fade-in bg-background-muted',
        getLayoutClass()
      )}
    >
      {/* Header controls */}
      <div
        className={cn(
          'absolute z-[100] flex items-center gap-1',
          // Bottom right for bottom condensed push mode
          effectiveNavigationStyle === 'condensed' &&
            navigationPosition === 'bottom' &&
            effectiveNavigationMode === 'push'
            ? 'bottom-4 right-6'
            : cn(
                headerPadding,
                'top-3 mt-[2px]',
                // Right position (both condensed and expanded) - 24px from right
                navigationPosition === 'right' ? 'right-6 left-auto' : 'ml-1.5'
              )
        )}
      >
        {/* Navigation trigger */}
        <Button
          onClick={() => setIsNavExpanded(!isNavExpanded)}
          className="no-drag hover:!bg-background-medium gap-1.5"
          variant="ghost"
          size="xs"
          title={isNavExpanded ? 'Close navigation' : 'Open navigation'}
        >
          <Goose className="w-6 h-6" />
          <span className="text-xs font-mono text-text-muted">menu</span>
        </Button>
      </div>

      {/* Main content with navigation */}
      <div className={cn('flex flex-1 w-full h-full min-h-0 p-[2px]', getLayoutClass())}>
        {/* Push mode navigation (inline) with animation */}
        <AnimatePresence mode="wait">
          {effectiveNavigationMode === 'push' && isNavExpanded && (
            <motion.div
              key="push-nav"
              initial={{
                width: isHorizontalNav ? '100%' : 0,
                height: isHorizontalNav ? 0 : '100%',
                opacity: 0,
                minWidth: 0,
              }}
              animate={{
                width: isHorizontalNav
                  ? '100%'
                  : effectiveNavigationStyle === 'expanded'
                    ? '30%'
                    : 'auto',
                height: isHorizontalNav ? 'auto' : '100%',
                opacity: 1,
                minWidth: !isHorizontalNav && effectiveNavigationStyle === 'expanded' ? 200 : 0,
              }}
              exit={{
                width: isHorizontalNav ? '100%' : 0,
                height: isHorizontalNav ? 0 : '100%',
                opacity: 0,
                minWidth: 0,
              }}
              transition={{
                type: 'spring',
                stiffness: 300,
                damping: 30,
                opacity: { duration: 0.15 },
              }}
              style={{
                // For expanded left/right, use percentage width that scales with window
                maxWidth:
                  !isHorizontalNav && effectiveNavigationStyle === 'expanded' ? '400px' : undefined,
                // Ensure full height for left/right positions
                height: !isHorizontalNav ? '100%' : undefined,
              }}
              className={cn(
                'flex-shrink-0',
                // Only hide overflow for expanded style (needs it for animations)
                // Condensed style needs overflow-visible for floating new chat button
                effectiveNavigationStyle === 'expanded' ? 'overflow-hidden' : 'overflow-visible',
                isHorizontalNav ? 'w-full' : 'h-full'
              )}
            >
              {renderNavigation()}
            </motion.div>
          )}
        </AnimatePresence>

        {/* Main content */}
        {mainContent}
      </div>

      {/* Overlay mode navigation */}
      {effectiveNavigationMode === 'overlay' && renderNavigation()}
    </div>
  );
};

interface AppLayoutProps {
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
}

export const AppLayout: React.FC<AppLayoutProps> = ({ activeSessions }) => {
  return (
    <NavigationProvider>
      <AppLayoutContent activeSessions={activeSessions} />
    </NavigationProvider>
  );
};
