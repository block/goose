import React from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { AppWindowMac, AppWindow } from 'lucide-react';
import { Goose } from '../icons/Goose';
import { Button } from '../ui/button';
import ChatSessionsContainer from '../ChatSessionsContainer';
import { useChatContext } from '../../contexts/ChatContext';
import { NavigationProvider, useNavigationContext } from './NavigationContext';
import { ExpandedNavigation } from './ExpandedNavigation';
import { CondensedNavigation } from './CondensedNavigation';
import { cn } from '../../utils';

interface AppLayoutContentProps {
  activeSessions: Array<{ sessionId: string; initialMessage?: string }>;
}

const AppLayoutContent: React.FC<AppLayoutContentProps> = ({ activeSessions }) => {
  const location = useLocation();
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const chatContext = useChatContext();
  const isOnPairRoute = location.pathname === '/pair';
  
  const {
    isNavExpanded,
    setIsNavExpanded,
    navigationMode,
    navigationStyle,
    navigationPosition,
    isHorizontalNav,
  } = useNavigationContext();

  if (!chatContext) {
    throw new Error('AppLayoutContent must be used within ChatProvider');
  }

  const { setChat } = chatContext;

  // Calculate padding based on macOS traffic lights
  const headerPadding = safeIsMacOS ? 'pl-21' : 'pl-4';

  const handleNewWindow = () => {
    window.electron.createChatWindow(
      undefined,
      window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
    );
  };

  // Render the appropriate navigation component based on style
  const renderNavigation = () => {
    if (navigationStyle === 'expanded') {
      return <ExpandedNavigation activeSessions={activeSessions} />;
    }
    return <CondensedNavigation activeSessions={activeSessions} />;
  };

  // Determine flex direction based on navigation position (for push mode)
  const getLayoutClass = () => {
    if (navigationMode === 'overlay') {
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
      <div className="h-full w-full bg-background-default rounded-2xl overflow-hidden">
        {isOnPairRoute ? (
          <>
            <Outlet />
            <ChatSessionsContainer setChat={setChat} activeSessions={activeSessions} />
          </>
        ) : (
          <Outlet />
        )}
      </div>
    </div>
  );

  return (
    <div className={cn('flex flex-1 w-full h-full relative animate-fade-in bg-background-muted', getLayoutClass())}>
      {/* Header controls */}
      <div className={cn(
        'absolute z-[100] flex items-center gap-1',
        // Bottom right for bottom condensed push mode
        navigationStyle === 'condensed' && navigationPosition === 'bottom' && navigationMode === 'push'
          ? 'bottom-4 right-4'
          : cn(
              headerPadding,
              'top-3 mt-[2px]',
              navigationStyle === 'condensed' && navigationPosition === 'right' 
                ? 'right-4 left-auto' 
                : 'ml-1.5'
            )
      )}>
        {/* Navigation trigger */}
        <Button
          onClick={() => setIsNavExpanded(!isNavExpanded)}
          className="no-drag hover:!bg-background-medium"
          variant="ghost"
          size="xs"
          title={isNavExpanded ? 'Close navigation' : 'Open navigation'}
        >
          <Goose className="w-6 h-6" />
        </Button>
        
        {/* New window button */}
        <Button
          onClick={handleNewWindow}
          className="no-drag hover:!bg-background-medium"
          variant="ghost"
          size="xs"
          title="Start a new session in a new window"
        >
          {safeIsMacOS ? <AppWindowMac className="w-4 h-4" /> : <AppWindow className="w-4 h-4" />}
        </Button>
      </div>

      {/* Main content with navigation */}
      <div className={cn('flex flex-1 w-full h-full p-2', getLayoutClass())}>
        {/* Push mode navigation (inline) with animation */}
        <AnimatePresence mode="wait">
          {navigationMode === 'push' && isNavExpanded && (
            <motion.div
              key="push-nav"
              initial={{ 
                width: isHorizontalNav ? '100%' : 0,
                height: isHorizontalNav ? 0 : '100%',
                opacity: 0 
              }}
              animate={{ 
                width: isHorizontalNav ? '100%' : 'auto',
                height: isHorizontalNav ? 'auto' : '100%',
                opacity: 1 
              }}
              exit={{ 
                width: isHorizontalNav ? '100%' : 0,
                height: isHorizontalNav ? 0 : '100%',
                opacity: 0 
              }}
              transition={{ 
                type: 'spring',
                stiffness: 300,
                damping: 30,
                opacity: { duration: 0.2 }
              }}
              className={cn(
                'flex-shrink-0 overflow-hidden',
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
      {navigationMode === 'overlay' && renderNavigation()}
    </div>
  );
};

interface AppLayoutProps {
  activeSessions: Array<{ sessionId: string; initialMessage?: string }>;
}

export const AppLayout: React.FC<AppLayoutProps> = ({ activeSessions }) => {
  return (
    <NavigationProvider>
      <AppLayoutContent activeSessions={activeSessions} />
    </NavigationProvider>
  );
};
