import React, { useState, createContext, useContext } from 'react';
import { Outlet } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../ui/button';
import { TopNavigation } from './TopNavigation';

interface AppLayoutProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

// Create context for navigation state
const NavExpandedContext = createContext<boolean>(false);

export const useNavExpanded = () => useContext(NavExpandedContext);

// Inner component
const AppLayoutContent: React.FC<AppLayoutProps> = () => {
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const [isNavExpanded, setIsNavExpanded] = useState(false);

  const handleNewWindow = () => {
    window.electron.createChatWindow(
      undefined,
      window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
    );
  };

  return (
    <NavExpandedContext.Provider value={isNavExpanded}>
      <div className="flex flex-col flex-1 w-full h-full bg-background-muted">
        {/* Top Navigation Bar */}
        <TopNavigation isExpanded={isNavExpanded} setIsExpanded={setIsNavExpanded} />
        
        {/* Main Content Area */}
        <div className="flex-1 overflow-hidden">
          <Outlet />
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
    </NavExpandedContext.Provider>
  );
};

export const AppLayout: React.FC<AppLayoutProps> = () => {
  return <AppLayoutContent />;
};
