import React from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import AppSidebar from '../GooseSidebar/AppSidebar';
import { View, ViewOptions } from '../../utils/navigationUtils';
import { Sidebar, SidebarInset, SidebarProvider } from '../ui/sidebar';
import ChatSessionsContainer from '../ChatSessionsContainer';
import { useChatContext } from '../../contexts/ChatContext';
import { UserInput } from '../../types/message';
import { ReasoningDetailProvider } from '../../contexts/ReasoningDetailContext';
import ReasoningDetailPanel from '../ReasoningDetailPanel';
import { UnifiedInputProvider } from '../../contexts/UnifiedInputContext';
import PromptBar from '../prompt_bar/PromptBar';

interface AppLayoutContentProps {
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
}

const AppLayoutContent: React.FC<AppLayoutContentProps> = ({ activeSessions }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const chatContext = useChatContext();
  const isOnPairRoute = location.pathname === '/pair';

  if (!chatContext) {
    throw new Error('AppLayoutContent must be used within ChatProvider');
  }

  const { setChat } = chatContext;

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
      case 'welcome':
        navigate('/welcome');
        break;
      default:
        navigate('/');
    }
  };

  const handleSelectSession = async (sessionId: string) => {
    navigate('/', { state: { sessionId } });
  };

  return (
    <div className="flex flex-1 w-full min-h-0 relative animate-fade-in">
      <Sidebar variant="inset" collapsible="icon">
        <AppSidebar
          onSelectSession={handleSelectSession}
          setView={setView}
          currentPath={location.pathname}
        />
      </Sidebar>
      <SidebarInset>
        {/* Non-pair routes: standard page content */}
        <div className={isOnPairRoute ? 'hidden' : 'flex-1 overflow-auto pb-20'}>
          <Outlet />
        </div>
        {/* Pair route: chat sessions or WelcomeState */}
        <div className={isOnPairRoute ? 'flex-1 overflow-auto pb-20' : 'hidden'}>
          <ChatSessionsContainer setChat={setChat} activeSessions={activeSessions} />
        </div>
        <PromptBar />
      </SidebarInset>
      <ReasoningDetailPanel />
    </div>
  );
};

interface AppLayoutProps {
  activeSessions: Array<{
    sessionId: string;
    initialMessage?: UserInput;
  }>;
}

export const AppLayout: React.FC<AppLayoutProps> = ({ activeSessions }) => {
  return (
    <ReasoningDetailProvider>
      <SidebarProvider>
        <UnifiedInputProvider>
          <AppLayoutContent activeSessions={activeSessions} />
        </UnifiedInputProvider>
      </SidebarProvider>
    </ReasoningDetailProvider>
  );
};
