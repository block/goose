import React, { useState, useCallback, useRef } from 'react';
import { Plus, ChevronLeft, ChevronRight } from 'lucide-react';
import { ChatTab } from './ChatTab';
import { useMultiChat } from './useMultiChat';
import BaseChat2 from '../BaseChat2';
import { ChatType } from '../../types/chat';
import { Button } from '../ui/button';

interface MultiChatViewProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}

export const MultiChatView: React.FC<MultiChatViewProps> = ({
  setChat,
  setIsGoosehintsModalOpen,
}) => {
  const {
    openSessions,
    activeSessionId,
    setActiveSessionId,
    openSession,
    closeSession,
    createNewSession,
    reorderSessions,
  } = useMultiChat();

  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);
  const tabBarRef = useRef<HTMLDivElement>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  // Check scroll state
  const checkScroll = useCallback(() => {
    if (tabBarRef.current) {
      const { scrollLeft, scrollWidth, clientWidth } = tabBarRef.current;
      setCanScrollLeft(scrollLeft > 0);
      setCanScrollRight(scrollLeft < scrollWidth - clientWidth - 1);
    }
  }, []);

  // Scroll tabs
  const scrollTabs = useCallback((direction: 'left' | 'right') => {
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
  const handleDragStart = useCallback((index: number) => (e: React.DragEvent) => {
    setDraggedIndex(index);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDragOver = useCallback((index: number) => (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDragOverIndex(index);
  }, []);

  const handleDrop = useCallback((index: number) => (e: React.DragEvent) => {
    e.preventDefault();
    if (draggedIndex !== null && draggedIndex !== index) {
      reorderSessions(draggedIndex, index);
    }
    setDraggedIndex(null);
    setDragOverIndex(null);
  }, [draggedIndex, reorderSessions]);

  const handleDragEnd = useCallback(() => {
    setDraggedIndex(null);
    setDragOverIndex(null);
  }, []);

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

      // Cmd/Ctrl + Tab: Next tab
      if (e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        const currentIndex = openSessions.findIndex(s => s.sessionId === activeSessionId);
        const nextIndex = (currentIndex + 1) % openSessions.length;
        if (openSessions[nextIndex]) {
          setActiveSessionId(openSessions[nextIndex].sessionId);
        }
      }

      // Cmd/Ctrl + Shift + Tab: Previous tab
      if (e.key === 'Tab' && e.shiftKey) {
        e.preventDefault();
        const currentIndex = openSessions.findIndex(s => s.sessionId === activeSessionId);
        const prevIndex = currentIndex === 0 ? openSessions.length - 1 : currentIndex - 1;
        if (openSessions[prevIndex]) {
          setActiveSessionId(openSessions[prevIndex].sessionId);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openSessions, activeSessionId, setActiveSessionId, createNewSession, closeSession]);

  // Update scroll state on mount and when tabs change
  React.useEffect(() => {
    checkScroll();
    const tabBar = tabBarRef.current;
    if (tabBar) {
      tabBar.addEventListener('scroll', checkScroll);
      return () => tabBar.removeEventListener('scroll', checkScroll);
    }
  }, [openSessions, checkScroll]);

  const activeSession = openSessions.find(s => s.sessionId === activeSessionId);
  
  // Check if macOS for stoplight button spacing
  const isMacOS = (window?.electron?.platform || 'darwin') === 'darwin';
  
  // Debug log
  React.useEffect(() => {
    console.log('MultiChatView - isMacOS:', isMacOS);
    console.log('MultiChatView - platform:', window?.electron?.platform);
  }, [isMacOS]);

  return (
    <div className="flex flex-col h-full">
      {/* Tab Bar */}
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
          className="flex flex-1 overflow-x-auto scrollbar-hide scroll-smooth"
          style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}
        >
          {/* Spacer for macOS stoplight buttons */}
          {isMacOS && <div className="flex-shrink-0 w-20 bg-background-muted" style={{ minWidth: '80px' }} />}
          
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
              hasUnread={false} // TODO: Implement unread tracking
            />
          ))}
          
          {/* New Tab Button - inline with tabs */}
          <button
            onClick={createNewSession}
            className="flex-shrink-0 flex items-center gap-2 px-4 py-3 min-w-[140px] rounded-2xl mr-0.5 bg-background-default hover:bg-background-medium text-text-default transition-all duration-200"
            aria-label="New chat"
          >
            <Plus className="w-4 h-4" />
            <span className="text-sm">New Chat</span>
          </button>
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

      {/* Chat Content */}
      <div className="flex-1 min-h-0 rounded-t-2xl overflow-hidden">
        {activeSession ? (
          <BaseChat2
            key={activeSession.sessionId}
            sessionId={activeSession.sessionId}
            setChat={setChat}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            suppressEmptyState={false}
          />
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
  );
};
