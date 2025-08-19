/**
 * Hub Component - Dashboard/OS Style
 *
 * The Hub is now redesigned as a dashboard/OS-style homepage with draggable widgets
 * on a canvas. This provides a more interactive and customizable experience.
 *
 * Key Features:
 * - Canvas-based layout with draggable widgets
 * - Metric cards (sessions, tokens) as moveable widgets
 * - Recent chats widget
 * - Centered greeting element (not on canvas) using original Greeting component
 * - Grid-based background for visual organization
 * - Floating chat input at the bottom with max-width constraint
 *
 * Navigation Flow:
 * Hub (dashboard interaction + input submission) → Pair (new conversation)
 */

import { useState, useEffect } from 'react';
import FlappyGoose from './FlappyGoose';
import { type View, ViewOptions } from '../App';
import ChatInput from './ChatInput';
import { generateSessionId } from '../sessions';
import { ChatState } from '../types/chatState';
import { ChatContextManagerProvider } from './context_management/ChatContextManager';
import { DashboardCanvas } from './dashboard/DashboardCanvas';
import { useDashboard } from '../hooks/useDashboard';
import { Goose } from './icons/Goose';
import { Greeting } from './common/Greeting';
import 'react-toastify/dist/ReactToastify.css';

import { ChatType } from '../types/chat';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';

export default function Hub({
  chat: _chat,
  setChat: _setChat,
  setPairChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  readyForAutoUserPrompt: boolean;
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setPairChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [showGame, setShowGame] = useState(false);
  const { dashboardState, isLoading, error, moveWidget, resizeWidget, resetDashboardLayout } = useDashboard();

  // Override backgrounds to allow our gradient to show through (same as Hub)
  useEffect(() => {
    // Override SidebarInset background
    const sidebarInset = document.querySelector('[data-slot="sidebar-inset"]') as HTMLElement;
    if (sidebarInset) {
      sidebarInset.style.background = 'transparent';
    }

    // Override ChatInput background to be transparent with glass effect
    const chatInputContainer = document.querySelector('[data-drop-zone="true"]') as HTMLElement;
    if (chatInputContainer) {
      chatInputContainer.style.background = 'rgba(255, 255, 255, 0.05)';
      chatInputContainer.style.backdropFilter = 'blur(20px)';
      // @ts-expect-error - webkitBackdropFilter is a valid CSS property
      chatInputContainer.style.webkitBackdropFilter = 'blur(20px)';
      chatInputContainer.style.border = '1px solid rgba(255, 255, 255, 0.1)';
    }
    
    // Cleanup on unmount
    return () => {
      if (sidebarInset) {
        sidebarInset.style.background = '';
      }
      if (chatInputContainer) {
        chatInputContainer.style.background = '';
        chatInputContainer.style.backdropFilter = '';
        // @ts-expect-error - webkitBackdropFilter is a valid CSS property
        chatInputContainer.style.webkitBackdropFilter = '';
        chatInputContainer.style.border = '';
      }
    };
  });

  // Handle chat input submission - create new chat and navigate to pair
  const handleSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      // Always create a completely new chat session with a unique ID for the PAIR
      const newChatId = generateSessionId();
      const newPairChat = {
        id: newChatId,
        title: DEFAULT_CHAT_TITLE,
        messages: [],
        messageHistoryIndex: 0,
        recipeConfig: null,
        recipeParameters: null,
      };

      // Update the PAIR chat state immediately to prevent flashing
      setPairChat(newPairChat);

      // Navigate to pair page with the message to be submitted immediately
      setView('pair', {
        disableAnimation: true,
        initialMessage: combinedTextFromInput,
        resetChat: true,
      });
    }

    // Prevent default form submission
    e.preventDefault();
  };

  if (isLoading) {
    return (
      <ChatContextManagerProvider>
        <div className="flex flex-col h-full relative">
          {/* Enhanced monochromatic gradient with better light mode contrast */}
          <div 
            className="absolute inset-0 animate-gradient-slow"
            style={{
              background: `
                radial-gradient(circle at 20% 80%, rgba(100, 100, 110, 0.25) 0%, transparent 50%),
                radial-gradient(circle at 80% 20%, rgba(120, 120, 125, 0.22) 0%, transparent 50%),
                radial-gradient(circle at 40% 40%, rgba(90, 95, 100, 0.18) 0%, transparent 50%),
                linear-gradient(135deg, 
                  rgba(0, 0, 0, 0.02) 0%, 
                  rgba(0, 0, 0, 0.04) 25%, 
                  rgba(0, 0, 0, 0.02) 50%, 
                  rgba(0, 0, 0, 0.06) 75%, 
                  rgba(0, 0, 0, 0.03) 100%
                )
              `,
              backgroundSize: '400% 400%',
            }}
          />
          
          {/* Loading state with Goose icon */}
          <div className="flex-1 flex items-center justify-center relative z-10">
            <div className="text-center space-y-4">
              <div className="origin-center goose-icon-animation">
                <Goose className="size-12 mx-auto text-text-muted" />
              </div>
              <div className="text-text-muted">Loading dashboard...</div>
            </div>
          </div>
          
          {/* Floating Chat Input */}
          <div className="absolute bottom-0 left-0 right-0 z-20">
            <div className="flex justify-center px-4">
              <div className="w-full max-w-[1000px]">
                <ChatInput
                  handleSubmit={handleSubmit}
                  chatState={ChatState.Idle}
                  onStop={() => {}}
                  commandHistory={[]}
                  initialValue=""
                  setView={setView}
                  numTokens={0}
                  inputTokens={0}
                  outputTokens={0}
                  droppedFiles={[]}
                  onFilesProcessed={() => {}}
                  messages={[]}
                  setMessages={() => {}}
                  disableAnimation={false}
                  sessionCosts={undefined}
                  setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                />
              </div>
            </div>
          </div>
        </div>
      </ChatContextManagerProvider>
    );
  }

  // Filter out the greeting widget from the canvas widgets
  const canvasWidgets = dashboardState.widgets.filter(widget => widget.type !== 'greeting');

  return (
    <ChatContextManagerProvider>
      <div className="flex flex-col h-full relative">
        {/* Dashboard Canvas with enhanced monochromatic gradient - now takes full height */}
        <div className="absolute inset-0">
          <DashboardCanvas
            widgets={canvasWidgets}
            onWidgetMove={moveWidget}
            onWidgetResize={resizeWidget}
          />
        </div>

        {/* Centered Greeting Section - using original Greeting component */}
        <div className="absolute inset-0 flex items-center justify-center z-10 pointer-events-none">
          <div className="text-center space-y-4 pointer-events-auto">
            <div className="origin-center">
              <Goose className="size-8 mx-auto text-text-muted mb-4" />
            </div>
            <Greeting className="text-4xl font-light animate-in fade-in duration-300" />
          </div>
        </div>

        {/* Error overlay if data failed to load */}
        {error && (
          <div className="absolute top-4 left-4 right-4 z-10">
            <div className="px-4 py-2 bg-orange-50 dark:bg-orange-950/20 border border-orange-200 dark:border-orange-800/30 rounded-xl">
              <div className="flex items-center space-x-2">
                <div className="w-2 h-2 bg-orange-400 rounded-full flex-shrink-0"></div>
                <span className="text-xs text-orange-700 dark:text-orange-300">
                  Failed to load dashboard data
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Dashboard controls - positioned at top right */}
        <div className="absolute top-4 right-4 z-10">
          <div className="flex items-center gap-2">
            <button
              onClick={resetDashboardLayout}
              className="px-3 py-1 text-xs bg-white/10 hover:bg-white/20 backdrop-blur-sm border border-white/20 rounded-md text-text-muted hover:text-text-default transition-all duration-200"
              title="Reset widget positions to default"
            >
              Reset Layout
            </button>
          </div>
        </div>

        {/* Floating Chat Input - positioned absolutely at bottom */}
        <div className="absolute bottom-0 left-0 right-0 z-20">
          <div className="flex justify-center px-4">
            <div className="w-full max-w-[1000px]">
              <ChatInput
                handleSubmit={handleSubmit}
                chatState={ChatState.Idle}
                onStop={() => {}}
                commandHistory={[]}
                initialValue=""
                setView={setView}
                numTokens={0}
                inputTokens={0}
                outputTokens={0}
                droppedFiles={[]}
                onFilesProcessed={() => {}}
                messages={[]}
                setMessages={() => {}}
                disableAnimation={false}
                sessionCosts={undefined}
                setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
              />
            </div>
          </div>
        </div>

        {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
      </div>
    </ChatContextManagerProvider>
  );
}
