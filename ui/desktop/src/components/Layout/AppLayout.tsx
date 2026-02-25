import React from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Menu } from 'lucide-react';
import { Button } from '../ui/button';
import ChatSessionsContainer from '../ChatSessionsContainer';
import { useChatContext } from '../../contexts/ChatContext';
import { NavigationProvider, useNavigationContext } from './NavigationContext';
import { Navigation } from './NavigationPanel';
import { NAV_DIMENSIONS, Z_INDEX } from './constants';
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
    isCondensedIconOnly,
  } = useNavigationContext();

  if (!chatContext) {
    throw new Error('AppLayoutContent must be used within ChatProvider');
  }

  const { setChat } = chatContext;

  // Hide the titlebar drag region when nav is at the top in push mode,
  // since the nav occupies that space and the drag region blocks interactions
  const isPushTopNav =
    effectiveNavigationMode === 'push' && navigationPosition === 'top' && isNavExpanded;
  React.useEffect(() => {
    const dragRegion = document.querySelector('.titlebar-drag-region') as HTMLElement | null;
    if (!dragRegion) return;
    if (isPushTopNav) {
      dragRegion.style.display = 'none';
    } else {
      dragRegion.style.display = '';
    }
    return () => {
      dragRegion.style.display = '';
    };
  }, [isPushTopNav]);

  // Calculate padding based on macOS traffic lights
  const headerPadding = safeIsMacOS ? 'pl-21' : 'pl-4';

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
    <div className="flex-1 overflow-hidden min-h-0">
      <div className="h-full w-full bg-background-primary rounded-lg overflow-hidden">
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
        'flex flex-1 w-full h-full relative animate-fade-in bg-background-secondary',
        getLayoutClass()
      )}
    >
      {/* Header controls */}
      <div
        style={{ zIndex: Z_INDEX.HEADER }}
        className={cn(
          'absolute flex items-center gap-1',
          effectiveNavigationStyle === 'condensed' &&
            navigationPosition === 'bottom' &&
            effectiveNavigationMode === 'push'
            ? 'bottom-4 right-6'
            : cn(
                headerPadding,
                'top-[11px]',
                navigationPosition === 'right' ? 'right-6 left-auto' : 'ml-1.5'
              )
        )}
      >
        {/* Navigation trigger */}
        <Button
          onClick={() => setIsNavExpanded(!isNavExpanded)}
          className="no-drag hover:!bg-background-tertiary"
          variant="ghost"
          size="xs"
          title={isNavExpanded ? 'Close navigation' : 'Open navigation'}
        >
          <Menu className="w-5 h-5" />
        </Button>
      </div>

      {/* Main content with navigation */}
      <div className={cn('flex flex-1 w-full h-full min-h-0 p-[2px]', getLayoutClass())}>
        {/* Push mode navigation (inline) with animation */}
        {effectiveNavigationMode === 'push' && (
          <motion.div
            key="push-nav"
            initial={false}
            animate={{
              width: isHorizontalNav
                ? '100%'
                : isNavExpanded
                  ? effectiveNavigationStyle === 'expanded'
                    ? '30%'
                    : isCondensedIconOnly
                      ? NAV_DIMENSIONS.CONDENSED_ICON_ONLY_WIDTH
                      : NAV_DIMENSIONS.CONDENSED_WIDTH
                  : 0,
              height: isHorizontalNav
                ? isNavExpanded
                  ? effectiveNavigationStyle === 'expanded'
                    ? NAV_DIMENSIONS.EXPANDED_HEIGHT
                    : NAV_DIMENSIONS.CONDENSED_HEIGHT
                  : 0
                : '100%',
            }}
            transition={{
              type: 'spring',
              stiffness: 400,
              damping: 40,
            }}
            style={{
              maxWidth:
                !isHorizontalNav && effectiveNavigationStyle === 'expanded' ? '400px' : undefined,
              minWidth:
                !isHorizontalNav && effectiveNavigationStyle === 'condensed' && isNavExpanded
                  ? isCondensedIconOnly
                    ? NAV_DIMENSIONS.CONDENSED_ICON_ONLY_WIDTH
                    : NAV_DIMENSIONS.CONDENSED_WIDTH
                  : undefined,
              minHeight:
                isHorizontalNav && isNavExpanded
                  ? effectiveNavigationStyle === 'expanded'
                    ? NAV_DIMENSIONS.EXPANDED_HEIGHT
                    : NAV_DIMENSIONS.CONDENSED_HEIGHT
                  : undefined,
              height: !isHorizontalNav ? '100%' : undefined,
            }}
            className={cn(
              'flex-shrink-0',
              effectiveNavigationStyle === 'condensed' && !isHorizontalNav
                ? 'overflow-visible'
                : 'overflow-hidden',
              isHorizontalNav ? 'w-full' : 'h-full'
            )}
          >
            <Navigation />
          </motion.div>
        )}

        {/* Main content */}
        {mainContent}
      </div>

      {/* Overlay mode navigation */}
      {effectiveNavigationMode === 'overlay' && <Navigation />}
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
