import React from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import AppSidebar from '../GooseSidebar/AppSidebar';
import { View, ViewOptions } from '../../App';
import { AppWindowMac, AppWindow } from 'lucide-react';
import { Button } from '../ui/button';
import { Sidebar, SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from '../ui/sidebar';

interface AppLayoutProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

// Inner component that uses useSidebar within SidebarProvider context
const AppLayoutContent: React.FC<AppLayoutProps> = ({ setIsGoosehintsModalOpen }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const { isMobile, openMobile } = useSidebar();

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

  return (
    <div className="flex flex-1 w-full relative animate-fade-in">
      {!shouldHideButtons && (
        <div className={`${headerPadding} absolute top-3 z-100 flex items-center`}>
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
      <AppLayoutContent setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />
    </SidebarProvider>
  );
};
