import React, { useState } from 'react';
import { Outlet } from 'react-router-dom';
import { AppWindowMac, AppWindow, ChevronDown, ChevronUp } from 'lucide-react';
import { Button } from '../ui/button';
import { TopNavigation } from './TopNavigation';

interface AppLayoutProps {
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

// Inner component
const AppLayoutContent: React.FC<AppLayoutProps> = ({ setIsGoosehintsModalOpen }) => {
  const safeIsMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  const [isNavExpanded, setIsNavExpanded] = useState(false);

  const handleNewWindow = () => {
    window.electron.createChatWindow(
      undefined,
      window.appConfig.get('GOOSE_WORKING_DIR') as string | undefined
    );
  };

  return (
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
          className="no-drag hover:!bg-background-medium bg-background-default rounded-xl shadow-sm"
          variant="ghost"
          size="xs"
          title="Toggle navigation"
        >
          {isNavExpanded ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
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
  return <AppLayoutContent setIsGoosehintsModalOpen={setIsGoosehintsModalOpen} />;
};
