import React, { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../ui/button';
import { SidebarProvider } from '../ui/sidebar';
import { SidecarProvider, useSidecar, Sidecar } from '../SidecarLayout';
import { SidecarInvoker } from './SidecarInvoker';
import { TopNavigation } from './TopNavigation';

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

  // Check if sidecar has any active views
  const hasSidecarContent = sidecar && sidecar.activeViews.length > 0;

  return (
    <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
      {/* Top Navigation Bar */}
      <TopNavigation isExpanded={isNavExpanded} setIsExpanded={setIsNavExpanded} />
      
      {/* Main Content Area */}
      <div className="flex flex-1 overflow-hidden relative">
        {/* New hover-triggered sidecar invoker */}
        <SidecarInvoker 
          onShowLocalhost={handleShowLocalhost}
          onShowFileViewer={handleShowFileViewer}
          onAddContainer={handleAddContainer}
          isVisible={shouldShowSidecarInvoker}
        />

        {/* Main content */}
        <div className={`${hasSidecarContent ? 'flex-1' : 'flex-1'} overflow-hidden`}>
          <Outlet />
        </div>

        {/* Sidecar - shows when there are active views */}
        {hasSidecarContent && (
          <div className="w-96 flex-shrink-0 border-l border-border-subtle">
            <Sidecar />
          </div>
        )}
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
