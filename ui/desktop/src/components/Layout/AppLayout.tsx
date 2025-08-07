import React from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import AppSidebar from '../GooseSidebar/AppSidebar';
import { View, ViewOptions } from '../../App';
import { AppWindowMac, AppWindow, Globe } from 'lucide-react';
import { Button } from '../ui/button';
import { Sidebar, SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from '../ui/sidebar';
import LocalhostButton from '../LocalhostButton';
import { SidecarProvider, useSidecar } from '../SidecarLayout';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';

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

  const handleGlobeClick = () => {
    console.log('Right globe button clicked');
    console.log('Sidecar available:', !!sidecar);
    console.log('Current pathname:', location.pathname);
    
    if (sidecar) {
      console.log('Calling sidecar.showLocalhostViewer...');
      sidecar.showLocalhostViewer('http://localhost:3000', 'Localhost Viewer');
    } else {
      console.error('No sidecar available');
    }
  };

  return (
    <div className="flex flex-1 w-full relative animate-fade-in">
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
              {safeIsMacOS ? <AppWindowMac className="w-4 h-4" /> : <AppWindow className="w-4 h-4" />}
            </Button>
          </div>
          
          {/* Right side globe button - show on chat-related pages (not home/hub) and hide when sidecar is open */}
          {(location.pathname === '/chat' || location.pathname === '/pair') && 
           !(sidecar?.activeView && sidecar?.views.find((v) => v.id === sidecar.activeView)) && (
            <div className="absolute top-3 right-4 z-100">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    onClick={handleGlobeClick}
                    className="no-drag hover:!bg-background-medium"
                    variant="ghost"
                    size="xs"
                  >
                    <Globe className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="top">Open Localhost Site</TooltipContent>
              </Tooltip>
            </div>
          )}
        </>
      )}
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
