import React from 'react';
import { useMultiChat, SessionStatus } from './useMultiChat';
import { ChatTab } from './ChatTab';
import { Plus, ChevronLeft, ChevronRight } from 'lucide-react';
import BaseChat2 from '../BaseChat2';
import { ChatType } from '../../types/chat';
import { Button } from '../ui/button';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { useNavExpanded } from '../Layout/AppLayout';
import { ChatState } from '../../types/chatState';

// Helper function to map ChatState to SessionStatus
const getChatStatus = (chatState?: ChatState): SessionStatus | undefined => {
  if (!chatState) return undefined;
  
  switch (chatState) {
    case ChatState.WaitingForUserInput:
      return 'waiting';
    case ChatState.Streaming:
    case ChatState.Thinking:
    case ChatState.Compacting:
    case ChatState.LoadingConversation:
      return 'working';
    case ChatState.Idle:
      return 'done';
    default:
      return 'done';
  }
};

interface ChatViewProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

export const ChatView: React.FC<ChatViewProps> = ({
  setChat,
  setIsGoosehintsModalOpen,
}) => {
  const {
    openSessions,
    activeSessionId,
    setActiveSessionId,
    closeSession,
    createNewSession,
    reorderSessions,
    updateSessionChatState,
  } = useMultiChat();

  const [draggedIndex, setDraggedIndex] = React.useState<number | null>(null);
  const tabBarRef = React.useRef<HTMLDivElement>(null);
  const [canScrollLeft, setCanScrollLeft] = React.useState(false);
  const [canScrollRight, setCanScrollRight] = React.useState(false);

  // Check scroll state
  const checkScroll = React.useCallback(() => {
    if (tabBarRef.current) {
      const { scrollLeft, scrollWidth, clientWidth } = tabBarRef.current;
      setCanScrollLeft(scrollLeft > 0);
      setCanScrollRight(scrollLeft < scrollWidth - clientWidth - 1);
    }
  }, []);

  // Scroll tabs
  const scrollTabs = React.useCallback((direction: 'left' | 'right') => {
    if (tabBarRef.current) {
      const scrollAmount = 200;
      tabBarRef.current.scrollBy({
        left: direction === 'left' ? -scrollAmount : scrollAmount,
        behavior: 'smooth',
      });
      setTimeout(checkScroll, 300);
    }
  }, [checkScroll]);

  // Drag and drop handlers
  const handleDragStart = React.useCallback((index: number) => (e: React.DragEvent) => {
    setDraggedIndex(index);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDragOver = React.useCallback((index: number) => (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDrop = React.useCallback((index: number) => (e: React.DragEvent) => {
    e.preventDefault();
    if (draggedIndex !== null && draggedIndex !== index) {
      reorderSessions(draggedIndex, index);
    }
    setDraggedIndex(null);
  }, [draggedIndex, reorderSessions]);

  // Keyboard shortcuts
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = window.electron?.platform === 'darwin';
      const modifier = isMac ? e.metaKey : e.ctrlKey;

      if (!modifier) return;

      // Cmd/Ctrl + 1-9: Switch to tab
      if (e.key >= '1' && e.key <= '9') {
        e.preventDefault();
        const index = parseInt(e.key) - 1;
        if (openSessions[index]) {
          setActiveSessionId(openSessions[index].sessionId);
        }
      }

      // Cmd/Ctrl + T: New tab
      if (e.key === 't') {
        e.preventDefault();
        createNewSession();
      }

      // Cmd/Ctrl + W: Close current tab
      if (e.key === 'w' && activeSessionId) {
        e.preventDefault();
        closeSession(activeSessionId);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openSessions, activeSessionId, setActiveSessionId, createNewSession, closeSession]);

  // Update scroll state
  React.useEffect(() => {
    checkScroll();
    const tabBar = tabBarRef.current;
    if (tabBar) {
      tabBar.addEventListener('scroll', checkScroll);
      return () => tabBar.removeEventListener('scroll', checkScroll);
    }
  }, [openSessions, checkScroll]);

  const activeSession = openSessions.find(s => s.sessionId === activeSessionId);
  
  // Debug logging
  console.log('ChatView render - activeSessionId:', activeSessionId);
  console.log('ChatView render - activeSession:', activeSession);
  console.log('ChatView render - openSessions:', openSessions);
  
  // Check if macOS for stoplight button spacing
  const isMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  
  // Get navigation expanded state
  const isNavExpanded = useNavExpanded();

  // Memoize the chat state change handler to prevent infinite loops
  // We need to include the activeSession.sessionId in the dependencies
  const handleChatStateChange = React.useCallback((chatState: ChatState) => {
    if (activeSession?.sessionId) {
      updateSessionChatState(activeSession.sessionId, chatState);
    }
  }, [activeSession?.sessionId, updateSessionChatState]);

  // Auto-create a session if there are none
  React.useEffect(() => {
    if (openSessions.length === 0) {
      createNewSession();
    }
  }, [openSessions.length, createNewSession]);

  // Always show tabs when there's at least one session
  const shouldShowTabs = openSessions.length > 0;

  return (
    <MainPanelLayout removeTopPadding={true} backgroundColor="bg-background-muted">
      <div className="flex flex-col h-full">
        {/* Tab Bar - Only show when there are messages or multiple tabs */}
        {shouldShowTabs && (
          <div className="bg-background-muted">
          <div className="flex items-center bg-background-muted mt-px mb-px relative">
            {/* Scroll Left Button */}
            {canScrollLeft && (
              <Button
                onClick={() => scrollTabs('left')}
                variant="ghost"
                size="sm"
                className={`absolute ${isMacOS ? 'left-20' : 'left-0'} z-10 bg-background-muted/90 backdrop-blur-sm rounded-none border-r border-border-default h-full px-2`}
              >
                <ChevronLeft className="w-4 h-4" />
              </Button>
            )}

            {/* Tabs Container */}
            <div
              ref={tabBarRef}
              className="flex flex-1 overflow-x-auto scrollbar-hide scroll-smooth transition-all duration-300 ease-in-out"
              style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}
            >
              {/* Spacer for macOS stoplight buttons - only when nav is closed */}
              {isMacOS && <div className="flex-shrink-0 bg-background-muted transition-all duration-300 ease-in-out" style={{ minWidth: isNavExpanded ? '0px' : '100px', width: isNavExpanded ? '0px' : '100px' }} />}
              
              {openSessions.map((openSession, index) => (
                <ChatTab
                  key={openSession.sessionId}
                  session={openSession.session}
                  isActive={activeSessionId === openSession.sessionId}
                  isLoading={openSession.isLoading}
                  onSelect={() => setActiveSessionId(openSession.sessionId)}
                  onClose={() => closeSession(openSession.sessionId)}
                  onDragStart={handleDragStart(index)}
                  onDragOver={handleDragOver(index)}
                  onDrop={handleDrop(index)}
                  hasUnread={false}
                  status={getChatStatus(openSession.chatState)}
                />
              ))}
              
              {/* New Tab Button - inline with tabs */}
              <button
                onClick={createNewSession}
                className="flex-shrink-0 flex items-center justify-center px-4 py-3 min-w-[60px] rounded-2xl mr-0.5 bg-background-default hover:bg-background-medium text-text-default transition-all duration-200"
                aria-label="New chat"
              >
                <Plus className="w-4 h-4" />
              </button>
              
              {/* Spacer for control buttons in top right - only when nav is closed */}
              {!isNavExpanded && <div className="flex-shrink-0 bg-background-muted transition-all duration-300 ease-in-out" style={{ minWidth: '175px', width: '175px' }} />}
            </div>

            {/* Scroll Right Button */}
            {canScrollRight && (
              <Button
                onClick={() => scrollTabs('right')}
                variant="ghost"
                size="sm"
                className="absolute right-0 z-10 bg-background-muted/90 backdrop-blur-sm rounded-none border-l border-border-default h-full px-2"
              >
                <ChevronRight className="w-4 h-4" />
              </Button>
            )}
          </div>
        </div>
        )}

        {/* Chat Content */}
        <div className="flex-1 min-h-0">
          {openSessions.length === 0 ? (
            // No sessions - create one and it will show the empty chat interface
            <div className="flex flex-col items-center justify-center h-full text-text-muted bg-background-default">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-text-default mb-4"></div>
              <p className="text-lg mb-2">Initializing chat...</p>
            </div>
          ) : activeSession && activeSession.sessionId ? (
            <BaseChat2
              key={activeSession.sessionId}
              sessionId={activeSession.sessionId}
              setChat={setChat}
              setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
              suppressEmptyState={false}
              onChatStateChange={handleChatStateChange}
            />
          ) : activeSessionId ? (
            <div className="flex flex-col items-center justify-center h-full text-text-muted bg-background-default">
              <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-text-default mb-4"></div>
              <p className="text-lg mb-2">Loading session...</p>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-text-muted bg-background-default">
              <Plus className="w-12 h-12 mb-4" />
              <p className="text-lg mb-2">No chat open</p>
              <p className="text-sm mb-4">Create a new chat to get started</p>
              <Button onClick={createNewSession} variant="default">
                New Chat
              </Button>
            </div>
          )}
        </div>
      </div>
    </MainPanelLayout>
  );
};
