import React, { useState, createContext, useContext, useEffect } from 'react';
import { Outlet } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp, ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from '../ui/button';
import { SidebarProvider } from '../ui/sidebar';
import { TopNavigation } from './TopNavigation';
import { CondensedNavigation } from './CondensedNavigation';
import { NavigationPosition } from '../settings/app/NavigationPositionSelector';
import { NavigationStyle } from '../settings/app/NavigationStyleSelector';
import { NavigationMode } from '../settings/app/NavigationModeSelector';

// Create context for navigation state
const NavigationContext = createContext<{
  isNavExpanded: boolean;
  setIsNavExpanded: (expanded: boolean) => void;
  navigationPosition: NavigationPosition;
}>({
  isNavExpanded: false,
  setIsNavExpanded: () => {},
  navigationPosition: 'top'
});

export const useNavigation = () => useContext(NavigationContext);

interface AppLayoutProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

// Inner component 
const AppLayoutContent: React.FC<AppLayoutProps> = () => {
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const [isNavExpanded, setIsNavExpanded] = useState(false);
  const [shouldUseOverlayOnSmallScreen, setShouldUseOverlayOnSmallScreen] = useState(false);
  
  const [navigationPosition, setNavigationPosition] = useState<NavigationPosition>(() => {
    const stored = localStorage.getItem('navigation_position');
    return (stored as NavigationPosition) || 'left';
  });
  const [navigationStyle, setNavigationStyle] = useState<NavigationStyle>(() => {
    const stored = localStorage.getItem('navigation_style');
    return (stored as NavigationStyle) || 'condensed';
  });
  const [navigationMode, setNavigationMode] = useState<NavigationMode>(() => {
    const stored = localStorage.getItem('navigation_mode');
    return (stored as NavigationMode) || 'push';
  });
  
  // Check screen size to determine if we should use overlay mode on small screens
  useEffect(() => {
    const checkScreenSize = () => {
      setShouldUseOverlayOnSmallScreen(window.innerWidth < 900);
    };
    
    checkScreenSize();
    window.addEventListener('resize', checkScreenSize);
    
    return () => window.removeEventListener('resize', checkScreenSize);
  }, []);
  
  // Listen for navigation position changes
  useEffect(() => {
    const handlePositionChange = (e: Event) => {
      const customEvent = e as CustomEvent<{ position: NavigationPosition }>;
      setNavigationPosition(customEvent.detail.position);
    };
    
    window.addEventListener('navigation-position-changed', handlePositionChange);
    return () => window.removeEventListener('navigation-position-changed', handlePositionChange);
  }, []);
  
  // Listen for navigation style changes
  useEffect(() => {
    const handleStyleChange = (e: Event) => {
      const customEvent = e as CustomEvent<{ style: NavigationStyle }>;
      setNavigationStyle(customEvent.detail.style);
    };
    
    window.addEventListener('navigation-style-changed', handleStyleChange);
    return () => window.removeEventListener('navigation-style-changed', handleStyleChange);
  }, []);
  
  // Listen for navigation mode changes
  useEffect(() => {
    const handleModeChange = (e: Event) => {
      const customEvent = e as CustomEvent<{ mode: NavigationMode }>;
      setNavigationMode(customEvent.detail.mode);
    };
    
    window.addEventListener('navigation-mode-changed', handleModeChange);
    return () => window.removeEventListener('navigation-mode-changed', handleModeChange);
  }, []);

  const handleNewWindow = () => {
    window.electron.createChatWindow(
      undefined,
      window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
    );
  };

  // Determine layout direction based on navigation position (only for push mode)
  const isHorizontalNav = navigationPosition === 'top' || navigationPosition === 'bottom';
  const flexDirection = isHorizontalNav ? 'flex-col' : 'flex-row';
  
  // On small screens, treat all push mode positions as overlay mode
  const shouldUseOverlayLayout = navigationMode === 'overlay' || 
    (navigationMode === 'push' && shouldUseOverlayOnSmallScreen);
  
  // Render the main content area
  const mainContent = (
    <div className="flex-1 overflow-hidden">
      <div className="h-full w-full bg-background-default rounded-2xl overflow-hidden">
        <Outlet />
      </div>
    </div>
  );

  // Render navigation component based on style
  const navigationComponent = navigationStyle === 'expanded' ? (
    <TopNavigation 
      isExpanded={isNavExpanded} 
      setIsExpanded={setIsNavExpanded}
      position={navigationPosition}
      isOverlayMode={navigationMode === 'overlay'}
    />
  ) : (
    <CondensedNavigation 
      isExpanded={isNavExpanded} 
      setIsExpanded={setIsNavExpanded}
      position={navigationPosition}
      isOverlayMode={navigationMode === 'overlay'}
    />
  );

  // Overlay navigation component (full screen)
  const overlayNavigationComponent = (
    <div className={`
      fixed inset-0 z-[10000] pointer-events-none
      ${isNavExpanded ? 'pointer-events-auto' : ''}
    `}>
      {/* Overlay background with blur - click to close */}
      {isNavExpanded && (
        <div 
          className="absolute inset-0 bg-black/20 backdrop-blur-md" 
          onClick={() => setIsNavExpanded(false)}
        />
      )}
      
      {/* Navigation overlay - Full screen without container */}
      <div className={`
        absolute inset-0
        transition-all duration-300 ease-out pointer-events-auto
        ${isNavExpanded 
          ? 'opacity-100 scale-100' 
          : 'opacity-0 scale-95 pointer-events-none'
        }
      `}>
        {/* Navigation content - let TopNavigation handle positioning */}
        {navigationComponent}
      </div>
    </div>
  );

  return (
    <NavigationContext.Provider value={{ isNavExpanded, setIsNavExpanded, navigationPosition }}>
      <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
        
        {shouldUseOverlayLayout ? (
          // Overlay Mode - Full screen content with floating navigation
          // (Used for: overlay mode OR any push mode on small screens < 900px)
          <div className="flex flex-1 w-full h-full bg-background-muted relative">
            {/* Main Content Area - Full Screen */}
            {mainContent}
            
            {/* Overlay Navigation - Only show when expanded */}
            {overlayNavigationComponent}
            
            {/* Control Buttons - Position based on navigation position setting */}
            <div className={`absolute z-[10002] flex gap-2 ${
              navigationPosition === 'top' ? 'top-4 right-4' :
              navigationPosition === 'bottom' ? 'bottom-4 right-4' :
              navigationPosition === 'left' ? (safeIsMacOS ? 'top-4 left-20' : 'top-4 left-4') :
              'top-4 right-4'
            }`}>
              <Button
                onClick={() => setIsNavExpanded(!isNavExpanded)}
                className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm relative"
                variant="ghost"
                size="xs"
                title="Toggle navigation overlay"
              >
                {navigationPosition === 'left' ? (
                  isNavExpanded ? <ChevronLeft className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />
                ) : navigationPosition === 'right' ? (
                  isNavExpanded ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />
                ) : (
                  isNavExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />
                )}
                <span className="ml-2 text-xs text-text-muted font-mono">
                  {isNavExpanded ? 'Hide launcher' : 'Show launcher'}
                </span>
              </Button>
              <Button
                onClick={handleNewWindow}
                className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm"
                variant="ghost"
                size="xs"
                title="Start a new session in a new window"
              >
                {safeIsMacOS ? <AppWindowMac className="w-4 h-4" /> : <AppWindow className="w-4 h-4" />}
              </Button>
            </div>
          </div>
        ) : (
          // Push Mode - Traditional layout with positioned navigation
          <div className={`flex ${flexDirection} flex-1 w-full h-full bg-background-muted`}>
            {/* Navigation placement based on position - always render but let component handle visibility */}
            {navigationPosition === 'top' && (
              <div className="flex flex-col max-h-[50vh]">
                {navigationComponent}
              </div>
            )}
            {navigationPosition === 'left' && (
              <div className="flex flex-col h-full">
                {navigationComponent}
              </div>
            )}
            
            {/* Main Content Area */}
            {mainContent}
            
            {/* Navigation placement for bottom and right - always render but let component handle visibility */}
            {navigationPosition === 'bottom' && (
              <div className="flex flex-col max-h-[50vh]">
                {navigationComponent}
              </div>
            )}
            {navigationPosition === 'right' && (
              <div className="flex flex-col h-full">
                {navigationComponent}
              </div>
            )}
            
            {/* Control Buttons - position based on nav location */}
            <div className={`absolute z-[10002] flex gap-2 ${
              navigationPosition === 'top' ? 'top-4 right-4' :
              navigationPosition === 'bottom' ? 'bottom-4 right-4' :
              navigationPosition === 'left' ? (safeIsMacOS ? 'top-4 left-20' : 'top-4 left-4') :
              'top-4 right-4'
            }`}>
              <Button
                onClick={() => setIsNavExpanded(!isNavExpanded)}
                className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm relative"
                variant="ghost"
                size="xs"
                title="Toggle navigation"
              >
                {navigationPosition === 'left' ? (
                  isNavExpanded ? <ChevronLeft className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />
                ) : navigationPosition === 'right' ? (
                  isNavExpanded ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />
                ) : (
                  isNavExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />
                )}
                <span className="ml-2 text-xs text-text-muted font-mono">
                  {isNavExpanded ? 'Hide menu' : 'Show menu'}
                </span>
              </Button>
              <Button
                onClick={handleNewWindow}
                className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm"
                variant="ghost"
                size="xs"
                title="Start a new session in a new window"
              >
                {safeIsMacOS ? <AppWindowMac className="w-4 h-4" /> : <AppWindow className="w-4 h-4" />}
              </Button>
            </div>
          </div>
        )}
      </div>
    </NavigationContext.Provider>
  );
};

export const AppLayout: React.FC = () => {
  return (
    <SidebarProvider>
      <AppLayoutContent />
    </SidebarProvider>
  );
};
