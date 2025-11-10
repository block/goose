import React, { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import AppSidebar from '../GooseSidebar/AppSidebar';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../ui/button';
import { Sidebar, SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from '../ui/sidebar';
import { SidecarProvider, useSidecar } from '../SidecarLayout';
import { SidecarInvoker } from './SidecarInvoker';
import { TopNavigation } from './TopNavigation';

interface AppLayoutProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

// Inner component that uses useSidebar within SidebarProvider context
const AppLayoutContent: React.FC<AppLayoutProps> = ({ setIsGoosehintsModalOpen }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const { isMobile, openMobile } = useSidebar();
  const sidecar = useSidecar();
  const [isNavExpanded, setIsNavExpanded] = useState(false);

  // Calculate padding based on sidebar state and macOS
  const headerPadding = safeIsMacOS ? 'pl-21' : 'pl-4';
  // const headerPadding = '';

  // Hide buttons when mobile sheet is showing
  const shouldHideButtons = isMobile && openMobile;

  const setView = (view: View, viewOptions?: ViewOptions) => {
    // Convert view-based navigation to route-based navigation
    switch (view) {
      case 'chat':
        navigate('/');
        break;
      case 'pair':
        navigate('/pair');
        break;
      case 'settings':
        navigate('/settings', { state: viewOptions });
        break;
      case 'extensions':
        navigate('/extensions', { state: viewOptions });
        break;
      case 'sessions':
        navigate('/sessions');
        break;
      case 'schedules':
        navigate('/schedules');
        break;
      case 'recipes':
        navigate('/recipes');
        break;
      case 'permission':
        navigate('/permission', { state: viewOptions });
        break;
      case 'ConfigureProviders':
        navigate('/configure-providers');
        break;
      case 'sharedSession':
        navigate('/shared-session', { state: viewOptions });
        break;
      case 'recipeEditor':
        navigate('/recipe-editor', { state: viewOptions });
        break;
      case 'welcome':
        navigate('/welcome');
        break;
      default:
        navigate('/');
    }
  };

  const handleSelectSession = async (sessionId: string) => {
    // Navigate to chat with session data
    navigate('/', { state: { sessionId } });
  };

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

  const handleAddContainer = (type: 'sidecar' | 'localhost' | 'file', filePath?: string) => {
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

  // Show sidecar invoker on chat-related pages - now always show it since we support multiple sidecars
  const shouldShowSidecarInvoker = 
    (location.pathname === '/' || location.pathname === '/chat' || location.pathname === '/pair');

  return (
    <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
      {/* Top Navigation Bar */}
      <TopNavigation isExpanded={isNavExpanded} setIsExpanded={setIsNavExpanded} />
      
      {/* Main Content Area with Sidebar and Sidecar */}
      <div className="flex flex-1 overflow-hidden relative">
        {!shouldHideButtons && (
          <>
            {/* Left side buttons */}
            <div className={`${headerPadding} absolute top-3 z-100 flex items-center gap-1`}>
              <SidebarTrigger
                className={`no-drag hover:border-border-strong hover:text-text-default hover:!bg-background-medium hover:scale-105`}
              />
              <Button
                onClick={handleNewWindow}
                className="no-drag hover:!bg-background-medium"
                variant="ghost"
                size="xs"
                title="Start a new session in a new window"
              >
                {safeIsMacOS ? (
                  <AppWindowMac className="w-4 h-4" />
                ) : (
                  <AppWindow className="w-4 h-4" />
                )}
              </Button>
            </div>
          </>
        )}

        {/* New hover-triggered sidecar invoker */}
        <SidecarInvoker 
          onShowLocalhost={handleShowLocalhost}
          onShowFileViewer={handleShowFileViewer}
          onAddContainer={handleAddContainer}
          isVisible={shouldShowSidecarInvoker}
        />

        <Sidebar variant="inset" collapsible="offcanvas">
          <AppSidebar
            onSelectSession={handleSelectSession}
            setView={setView}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            currentPath={location.pathname}
          />
        </Sidebar>
        <SidebarInset>
          <Outlet />
        </SidebarInset>
      </div>
      
      {/* Control Buttons - floating in top right */}
      <div className="absolute top-4 right-4 z-50 flex gap-2">
        <Button
          onClick={() => setIsNavExpanded(!isNavExpanded)}
          className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm relative"
          variant="ghost"
          size="xs"
          title="Toggle navigation"
        >
          {isNavExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
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
