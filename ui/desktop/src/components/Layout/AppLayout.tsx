import React, { useState, createContext, useContext, useCallback, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp, ChevronLeft, ChevronRight } from 'lucide-react';
import { Button } from '../ui/button';
import { SidebarProvider } from '../ui/sidebar';
import { SidecarProvider, useSidecar } from '../SidecarLayout';
import { EnhancedBentoBox, SidecarContainer } from './EnhancedBentoBox';
import { ResizableSplitter } from './ResizableSplitter';
import MultiPanelSplitter, { LayoutMode, PanelConfig } from './MultiPanelSplitter';
import SidecarTabs from '../SidecarTabs';
import { FileViewer } from '../FileViewer';
import DocumentEditor from '../DocumentEditor';
import WebViewer from '../WebViewer';

import { TopNavigation } from './TopNavigation';
import { CondensedNavigation } from './CondensedNavigation';
import { NavigationPosition } from '../settings/app/NavigationPositionSelector';
import { NavigationStyle } from '../settings/app/NavigationStyleSelector';
import { NavigationMode } from '../settings/app/NavigationModeSelector';
import { NotificationTicker, useNotificationTicker } from '../NotificationTicker';

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
const AppLayoutContent: React.FC<AppLayoutProps> = ({ setIsGoosehintsModalOpen }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const sidecar = useSidecar();
  const [isNavExpanded, setIsNavExpanded] = useState(false);
  
  // Notification ticker
  const ticker = useNotificationTicker();
  
  // Debug log to ensure ticker is initialized (only once)
  useEffect(() => {
    console.log('🎯 AppLayout ticker initialized');
  }, []);
  const [navigationPosition, setNavigationPosition] = useState<NavigationPosition>(() => {
    const stored = localStorage.getItem('navigation_position');
    return (stored as NavigationPosition) || 'top';
  });
  const [navigationStyle, setNavigationStyle] = useState<NavigationStyle>(() => {
    const stored = localStorage.getItem('navigation_style');
    return (stored as NavigationStyle) || 'expanded';
  });
  const [navigationMode, setNavigationMode] = useState<NavigationMode>(() => {
    const stored = localStorage.getItem('navigation_mode');
    return (stored as NavigationMode) || 'push';
  });
  
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
  
  // Bento box state management
  const [bentoBoxContainers, setBentoBoxContainers] = useState<SidecarContainer[]>([]);
  
  // Multi-panel state management
  const [panels, setPanels] = useState<PanelConfig[]>([]);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('single');
  const [useMultiPanel, setUseMultiPanel] = useState(false); // Toggle between old and new system
  
  // Resizable splitter state
  const [chatWidth, setChatWidth] = useState(60); // Default 60% for chat, 40% for sidecars

  // Convert sidecar views to both bento box containers and panels
  React.useEffect(() => {
    if (!sidecar) return;

    const activeViews = sidecar.views.filter(view => sidecar.activeViews.includes(view.id));

    // Convert to bento box containers (existing system)
    const containers: SidecarContainer[] = activeViews.map(view => {
      // Extract content props based on view type
      let contentProps: SidecarContainer['contentProps'] = {};
      let contentType: SidecarContainer['contentType'] = null;

      if (view.id.startsWith('localhost-')) {
        contentType = 'localhost';
        contentProps = {
          initialUrl: view.fileName || 'http://localhost:3000',
          allowAllSites: true
        };
      } else if (view.id.startsWith('web-viewer-')) {
        contentType = 'web-viewer';
        contentProps = {
          initialUrl: view.fileName || 'https://google.com',
          allowAllSites: true
        };
      } else if (view.id.startsWith('file-')) {
        contentType = 'file';
        contentProps = {
          filePath: view.fileName || ''
        };
      } else if (view.id.startsWith('editor-')) {
        contentType = 'document-editor';
        contentProps = {
          filePath: view.fileName,
          placeholder: 'Start writing your document...'
        };
      } else if (view.id.startsWith('diff-')) {
        contentType = 'sidecar'; // Treat diff as generic sidecar
      } else {
        contentType = 'sidecar';
      }

      return {
        id: view.id,
        content: view.content,
        contentType,
        title: view.title,
        size: 'medium' as const,
        contentProps
      };
    });

    // Convert to panels (new multi-panel system)
    const newPanels: PanelConfig[] = activeViews.map((view, index) => {
      let content: React.ReactNode;

      // Render content based on view type
      if (view.id.startsWith('localhost-')) {
        content = <SidecarTabs initialUrl={view.fileName || 'http://localhost:3000'} />;
      } else if (view.id.startsWith('web-viewer-')) {
        content = <WebViewer initialUrl={view.fileName || 'https://google.com'} allowAllSites={true} />;
      } else if (view.id.startsWith('file-')) {
        content = <FileViewer filePath={view.fileName || ''} />;
      } else if (view.id.startsWith('editor-')) {
        content = <DocumentEditor filePath={view.fileName} placeholder="Start writing your document..." />;
      } else {
        content = view.content || (
          <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
            <p>Sidecar content</p>
          </div>
        );
      }

      return {
        id: view.id,
        content,
        title: view.title,
        minWidth: 200,
        minHeight: 150,
        size: { width: 50, height: 50 }, // Default size percentages
        position: { row: Math.floor(index / 2), col: index % 2 }
      };
    });

    setBentoBoxContainers(containers);
    setPanels(newPanels);

    // Auto-enable multi-panel mode when we have multiple panels
    if (newPanels.length > 1 && !useMultiPanel) {
      setUseMultiPanel(true);
      setLayoutMode('columns'); // Default to columns layout
    } else if (newPanels.length <= 1 && useMultiPanel) {
      setUseMultiPanel(false);
      setLayoutMode('single');
    }
  }, [sidecar?.views, sidecar?.activeViews, useMultiPanel]);

  // Bento box handlers
  const handleAddToBentoBox = useCallback((type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer', filePath?: string, url?: string, title?: string) => {
    if (!sidecar) return;

    // Use the sidecar system to create the view
    switch (type) {
      case 'localhost':
        sidecar.showLocalhostViewer(url || 'http://localhost:3000', title || 'Localhost Viewer');
        break;
      case 'file':
        if (filePath) {
          sidecar.showFileViewer(filePath);
        }
        break;
      case 'document-editor':
        sidecar.showDocumentEditor(filePath, undefined, title);
        break;
      case 'web-viewer':
        sidecar.showView({
          id: `web-viewer-${Date.now()}`,
          title: title || 'Web Viewer',
          icon: <div className="w-4 h-4 bg-cyan-500 rounded" />,
          content: null, // Will be rendered by contentType
          contentType: 'web-viewer',
          contentProps: {
            initialUrl: url || 'https://google.com',
            allowAllSites: true
          }
        });
        break;
      case 'sidecar':
      default:
        sidecar.showView({
          id: `sidecar-${Date.now()}`,
          title: title || 'Sidecar',
          icon: <div className="w-4 h-4 bg-blue-500 rounded" />,
          content: (
            <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
              <p>Sidecar content will go here</p>
            </div>
          ),
        });
        break;
    }
  }, [sidecar]);

  const handleRemoveFromBentoBox = useCallback((containerId: string) => {
    if (!sidecar) return;
    sidecar.hideView(containerId);
  }, [sidecar]);

  const handleReorderBentoBox = useCallback((containers: SidecarContainer[]) => {
    // For now, just update local state
    // The sidecar system doesn't support reordering, so we'll manage it locally
    setBentoBoxContainers(containers);
  }, []);

  // Multi-panel handlers
  const handleLayoutModeChange = useCallback((mode: LayoutMode) => {
    setLayoutMode(mode);
  }, []);

  const handlePanelResize = useCallback((panelId: string, size: { width: number; height: number }) => {
    setPanels(prevPanels => 
      prevPanels.map(panel => 
        panel.id === panelId ? { ...panel, size } : panel
      )
    );
  }, []);

  const handlePanelReorder = useCallback((newPanels: PanelConfig[]) => {
    setPanels(newPanels);
  }, []);

  const handleNewWindow = () => {
    window.electron.createChatWindow(
      undefined,
      window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
    );
  };

  const handleShowLocalhost = () => {
    console.log('Localhost viewer requested');
    console.log('Sidecar available:', !!sidecar);
    console.log('Current pathname:', location.pathname);

    if (sidecar) {
      console.log('Calling sidecar.showLocalhostViewer...');
      sidecar.showLocalhostViewer('http://localhost:3000', 'Localhost Viewer');
    } else {
      console.error('No sidecar available');
    }
  };

  const handleShowFileViewer = (filePath: string) => {
    console.log('File viewer requested for:', filePath);
    console.log('Sidecar available:', !!sidecar);

    if (sidecar) {
      console.log('Calling sidecar.showFileViewer...');
      sidecar.showFileViewer(filePath);
    } else {
      console.error('No sidecar available');
    }
  };

  const handleAddContainer = (type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer', filePath?: string) => {
    console.log('Add container requested:', type, filePath);
    // This will be handled by MainPanelLayout
    window.dispatchEvent(new CustomEvent('add-container', { detail: { type, filePath } }));
  };

  // Listen for programmatic request to show the sidecar localhost viewer
  React.useEffect(() => {
    const handler = (e: globalThis.Event) => {
      if (!sidecar) return;
      const ce = e as CustomEvent<{ url?: string }>;
      const url = ce.detail?.url || 'http://localhost:3000';
      sidecar.showLocalhostViewer(url, 'Localhost Viewer');
    };
    window.addEventListener('open-sidecar-localhost', handler);
    return () => window.removeEventListener('open-sidecar-localhost', handler);
  }, [sidecar]);

  // Add Matrix-related notifications and demo content
  useEffect(() => {
    // Simulate Matrix notifications for demo purposes
    const demoNotifications = [
      () => ticker.addSessionInvitation('project-gamma', 'david'),
      () => ticker.addMessageNotification('eve', 'design-team', 2),
      () => ticker.addSessionJoined('frank', 'standup-meeting'),
      () => ticker.addSessionWaiting('code-review-session', 3),
      () => ticker.addConnectionStatus('connected'),
    ];

    // Add initial demo notification
    setTimeout(() => {
      const randomNotification = demoNotifications[Math.floor(Math.random() * demoNotifications.length)];
      randomNotification();
    }, 2000);

    // Add periodic demo notifications
    const interval = setInterval(() => {
      const randomNotification = demoNotifications[Math.floor(Math.random() * demoNotifications.length)];
      randomNotification();
    }, 15000); // Every 15 seconds

    return () => clearInterval(interval);
  }, []); // Removed ticker from dependencies

  // Add route change notifications
  useEffect(() => {
    const routeNames: Record<string, string> = {
      '/': 'hub',
      '/pair': 'chat',
      '/tabs': 'tabs',
      '/settings': 'settings',
      '/sessions': 'sessions',
      '/recipes': 'recipes',
      '/extensions': 'extensions',
      '/peers': 'peers',
      '/channels': 'channels',
    };
    
    const currentRoute = routeNames[location.pathname] || 'unknown';
    
    // Only add route notifications for Matrix-related sections
    if (currentRoute === 'sessions' || currentRoute === 'peers' || currentRoute === 'channels') {
      ticker.addItem({
        text: `navigated to ${currentRoute}`,
        type: 'info'
      });
    }
  }, [location.pathname]); // Removed ticker from dependencies

  // Determine layout direction based on navigation position (only for push mode)
  const isHorizontalNav = navigationPosition === 'top' || navigationPosition === 'bottom';
  const flexDirection = isHorizontalNav ? 'flex-col' : 'flex-row';
  
  // Render the main content area
  const mainContent = (
    <div className="flex-1 overflow-hidden">
      <div className="h-full w-full bg-background-default rounded-2xl overflow-hidden">
        {panels.length > 0 ? (
          useMultiPanel && panels.length > 1 ? (
            <MultiPanelSplitter
              leftContent={<Outlet />}
              panels={panels}
              layoutMode={layoutMode}
              onLayoutModeChange={handleLayoutModeChange}
              onPanelResize={handlePanelResize}
              onPanelReorder={handlePanelReorder}
              initialLeftWidth={chatWidth}
              className="h-full"
            />
          ) : (
            <ResizableSplitter
              leftContent={<Outlet />}
              rightContent={
                <EnhancedBentoBox
                  containers={bentoBoxContainers}
                  onRemoveContainer={handleRemoveFromBentoBox}
                  onAddContainer={handleAddToBentoBox}
                  onReorderContainers={handleReorderBentoBox}
                />
              }
              initialLeftWidth={chatWidth}
              minLeftWidth={30}
              maxLeftWidth={80}
              onResize={setChatWidth}
              className="h-full"
              floatingRight={true}
            />
          )
        ) : (
          <Outlet />
        )}
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
        {/* Navigation content - full viewport */}
        {navigationComponent}
      </div>
    </div>
  );

  return (
    <NavigationContext.Provider value={{ isNavExpanded, setIsNavExpanded, navigationPosition }}>
      <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
        {/* Notification Ticker - Only show when navigation is open */}
        {isNavExpanded && (
          <NotificationTicker 
            items={ticker.items} 
            height={32} 
            className="z-[10001]" 
          />
        )}
        
        {navigationMode === 'overlay' ? (
          // Overlay Mode - Full screen content with floating navigation
          <div className="flex flex-1 w-full h-full bg-background-muted relative">
            {/* Main Content Area - Full Screen */}
            {mainContent}
            
            {/* Overlay Navigation - Only show when expanded */}
            {overlayNavigationComponent}
            
            {/* Control Buttons - Fixed position for overlay mode */}
            <div className="absolute z-[10002] flex gap-2 top-4 right-4">
              <Button
                onClick={() => setIsNavExpanded(!isNavExpanded)}
                className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm relative"
                variant="ghost"
                size="xs"
                title="Toggle navigation overlay"
              >
                <ChevronDown className="w-4 h-4" />
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
            {navigationPosition === 'top' && navigationComponent}
            {navigationPosition === 'left' && navigationComponent}
            
            {/* Main Content Area */}
            {mainContent}
            
            {/* Navigation placement for bottom and right - always render but let component handle visibility */}
            {navigationPosition === 'bottom' && navigationComponent}
            {navigationPosition === 'right' && navigationComponent}
            
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

export const AppLayout: React.FC<AppLayoutProps> = ({ setIsGoosehintsModalOpen }) => {
  return (
    <SidebarProvider>
      <SidecarProvider>
        <AppLayoutContent setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
      </SidecarProvider>
    </SidebarProvider>
  );
};
