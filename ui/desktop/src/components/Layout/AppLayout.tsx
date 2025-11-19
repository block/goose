import React, { useState, createContext, useContext, useCallback } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../ui/button';
import { SidebarProvider } from '../ui/sidebar';
import { SidecarProvider, useSidecar } from '../SidecarLayout';
import { EnhancedBentoBox, SidecarContainer } from './EnhancedBentoBox';

import { TopNavigation } from './TopNavigation';

// Create context for navigation state
const NavigationContext = createContext<{
  isNavExpanded: boolean;
  setIsNavExpanded: (expanded: boolean) => void;
}>({
  isNavExpanded: false,
  setIsNavExpanded: () => {}
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
  
  // Bento box state management
  const [bentoBoxContainers, setBentoBoxContainers] = useState<SidecarContainer[]>([]);

  // Convert sidecar views to bento box containers
  React.useEffect(() => {
    if (!sidecar) return;

    const containers: SidecarContainer[] = sidecar.views
      .filter(view => sidecar.activeViews.includes(view.id))
      .map(view => {
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

    setBentoBoxContainers(containers);
  }, [sidecar?.views, sidecar?.activeViews]);

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

  return (
    <NavigationContext.Provider value={{ isNavExpanded, setIsNavExpanded }}>
      <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
        {/* Top Navigation Bar */}
        <TopNavigation isExpanded={isNavExpanded} setIsExpanded={setIsNavExpanded} />
        
        {/* Main Content Area */}
        <div className="flex flex-1 overflow-hidden relative">
          {/* Main content without sidebar */}
          <div className="flex-1 overflow-hidden">
            <Outlet />
          </div>
          
          {/* Enhanced Bento Box - positioned as sibling to main content */}
          {bentoBoxContainers.length > 0 && (
            <div className="w-[600px] border-l border-border-subtle">
              <EnhancedBentoBox
                containers={bentoBoxContainers}
                onRemoveContainer={handleRemoveFromBentoBox}
                onAddContainer={handleAddToBentoBox}
                onReorderContainers={handleReorderBentoBox}
              />
            </div>
          )}
        </div>
        
        {/* Control Buttons - floating in top right */}
        <div className="absolute top-4 right-4 z-[9999] flex gap-2">
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
