/**
 * Pair Component
 *
 * The Pair component represents the active conversation mode in the Goose Desktop application.
 * This is where users engage in ongoing conversations with the AI assistant after transitioning
 * from the Hub's initial welcome screen.
 *
 * Key Responsibilities:
 * - Manages active chat sessions with full message history
 * - Handles transitions from Hub with initial input processing
 * - Provides the main conversational interface for extended interactions
 * - Enables local storage persistence for conversation continuity
 * - Supports all advanced chat features like file attachments, tool usage, etc.
 *
 * Navigation Flow:
 * Hub (initial message) → Pair (active conversation) → Hub (new session)
 *
 * The Pair component is essentially a specialized wrapper around BaseChat that:
 * - Processes initial input from the Hub transition
 * - Enables conversation persistence
 * - Provides the full-featured chat experience
 *
 * Unlike Hub, Pair assumes an active conversation state and focuses on
 * maintaining conversation flow rather than onboarding new users.
 */

import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { type View, ViewOptions } from '../App';
import BaseChat from './BaseChat';
import GlobalBackground from './GlobalBackground';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useIsMobile } from '../hooks/use-mobile';
import { useSidebar } from './ui/sidebar';
import 'react-toastify/dist/ReactToastify.css';
import { cn } from '../utils';

import { ChatType } from '../types/chat';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';

export default function Pair({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const location = useLocation();
  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const [hasProcessedInitialInput, setHasProcessedInitialInput] = useState(false);
  const [shouldAutoSubmit, setShouldAutoSubmit] = useState(false);
  const [initialMessage, setInitialMessage] = useState<string | null>(null);
  const [isTransitioningFromHub, setIsTransitioningFromHub] = useState(false);

  // Get recipe configuration and parameter handling
  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(chat.messages, location.state);

  // Get sidebar state for background adjustments
  const { state: currentSidebarState } = useSidebar();
  const isSidebarCollapsed = currentSidebarState === 'collapsed';

  // Get system theme preference
  const prefersDarkMode = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;

  // Override backgrounds to allow our gradient to show through
  useEffect(() => {
    // Target the specific SidebarInset component with the complex class
    const sidebarInset = document.querySelector('[data-slot="sidebar-inset"]') as HTMLElement;
    if (sidebarInset) {
      sidebarInset.style.background = 'transparent';
      sidebarInset.style.backgroundColor = 'transparent';
      // Remove the bg-background class that might be causing the issue
      sidebarInset.classList.remove('bg-background');
      // Add bg-transparent class
      sidebarInset.classList.add('bg-transparent');
    }

    // Target the sidebar element itself
    const sidebar = document.querySelector('[data-slot="sidebar"]') as HTMLElement;
    if (sidebar) {
      // Add a style to make sure it doesn't block our background
      sidebar.style.pointerEvents = 'auto';
      sidebar.style.zIndex = '20';
      // Make the sidebar background transparent
      sidebar.style.background = 'transparent';
      sidebar.style.backgroundColor = 'transparent';
    }

    // Target the sidebar wrapper
    const sidebarWrapper = document.querySelector('[data-slot="sidebar-wrapper"]') as HTMLElement;
    if (sidebarWrapper) {
      sidebarWrapper.style.background = 'transparent';
      sidebarWrapper.style.backgroundColor = 'transparent';
    }

    // Target the sidebar container
    const sidebarContainer = document.querySelector('[data-slot="sidebar-container"]') as HTMLElement;
    if (sidebarContainer) {
      sidebarContainer.style.background = 'transparent';
      sidebarContainer.style.backgroundColor = 'transparent';
    }

    // Target the sidebar inner
    const sidebarInner = document.querySelector('[data-slot="sidebar-inner"]') as HTMLElement;
    if (sidebarInner) {
      // Keep the sidebar's own background but make sure it doesn't extend
      sidebarInner.style.width = '100%';
      sidebarInner.style.height = '100%';
    }

    // Override MainPanelLayout background
    const mainPanels = document.querySelectorAll('.bg-background-default, .bg-background-muted') as NodeListOf<HTMLElement>;
    mainPanels.forEach(panel => {
      if (panel) {
        panel.style.background = 'transparent';
        panel.style.backgroundColor = 'transparent';
      }
    });

    // Override ChatInput background to be transparent with glass effect
    const chatInputContainer = document.querySelector('[data-drop-zone="true"]') as HTMLElement;
    if (chatInputContainer) {
      chatInputContainer.style.background = 'rgba(255, 255, 255, 0.05)';
      chatInputContainer.style.backdropFilter = 'blur(10px)';
      // @ts-expect-error - webkitBackdropFilter is a valid CSS property
      chatInputContainer.style.webkitBackdropFilter = 'blur(10px)';
      chatInputContainer.style.border = '1px solid rgba(255, 255, 255, 0.1)';
    }
    
    // Cleanup on unmount
    return () => {
      if (sidebarInset) {
        sidebarInset.style.background = '';
        sidebarInset.style.backgroundColor = '';
        sidebarInset.classList.remove('bg-transparent');
        // Restore the original class if needed
        if (!sidebarInset.classList.contains('bg-background')) {
          sidebarInset.classList.add('bg-background');
        }
      }
      if (sidebar) {
        sidebar.style.pointerEvents = '';
        sidebar.style.zIndex = '';
        sidebar.style.background = '';
        sidebar.style.backgroundColor = '';
      }
      if (sidebarWrapper) {
        sidebarWrapper.style.background = '';
        sidebarWrapper.style.backgroundColor = '';
      }
      if (sidebarContainer) {
        sidebarContainer.style.background = '';
        sidebarContainer.style.backgroundColor = '';
      }
      if (sidebarInner) {
        sidebarInner.style.width = '';
        sidebarInner.style.height = '';
      }
      mainPanels.forEach(panel => {
        if (panel) {
          panel.style.background = '';
          panel.style.backgroundColor = '';
        }
      });
      if (chatInputContainer) {
        chatInputContainer.style.background = '';
        chatInputContainer.style.backdropFilter = '';
        // @ts-expect-error - webkitBackdropFilter is a valid CSS property
        chatInputContainer.style.webkitBackdropFilter = '';
        chatInputContainer.style.border = '';
      }
    };
  }, []);

  // Handle recipe loading from recipes view - reset chat if needed
  useEffect(() => {
    if (location.state?.resetChat && location.state?.recipeConfig) {
      // Reset the chat to start fresh with the recipe
      const newChat = {
        id: chat.id, // Keep the same ID to maintain the session
        title: location.state.recipeConfig.title || 'Recipe Chat',
        messages: [], // Clear messages to start fresh
        messageHistoryIndex: 0,
        recipeConfig: location.state.recipeConfig, // Set the recipe config in chat state
        recipeParameters: null, // Clear parameters for new recipe
      };
      setChat(newChat);

      // Clear the location state to prevent re-processing
      window.history.replaceState({}, '', '/pair');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state, chat.id]);

  // Handle initial message from hub page
  useEffect(() => {
    const messageFromHub = location.state?.initialMessage;
    const resetChat = location.state?.resetChat;

    // If we have a resetChat flag from Hub, clear any existing recipe config
    // This scenario occurs when a user navigates from Hub to start a new chat,
    // ensuring any previous recipe configuration is cleared for a fresh start
    if (resetChat) {
      const newChat: ChatType = {
        ...chat,
        recipeConfig: null,
        recipeParameters: null,
        title: DEFAULT_CHAT_TITLE,
        messages: [], // Clear messages for fresh start
        messageHistoryIndex: 0,
      };
      setChat(newChat);
    }

    // Reset processing state when we have a new message from hub
    if (messageFromHub) {
      // Set transitioning state to prevent showing popular topics
      setIsTransitioningFromHub(true);

      // If this is a different message than what we processed before, reset the flag
      if (messageFromHub !== initialMessage) {
        setHasProcessedInitialInput(false);
      }

      if (!hasProcessedInitialInput) {
        setHasProcessedInitialInput(true);
        setInitialMessage(messageFromHub);
        setShouldAutoSubmit(true);

        // Clear the location state to prevent re-processing
        window.history.replaceState({}, '', '/pair');
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.state, hasProcessedInitialInput, initialMessage]);

  // Auto-submit the initial message after it's been set and component is ready
  useEffect(() => {
    if (shouldAutoSubmit && initialMessage) {
      // Wait for the component to be fully rendered
      const timer = setTimeout(() => {
        // Try to trigger form submission programmatically
        const textarea = document.querySelector(
          'textarea[data-testid="chat-input"]'
        ) as HTMLTextAreaElement;
        const form = textarea?.closest('form');

        if (textarea && form) {
          // Set the textarea value
          textarea.value = initialMessage;
          // eslint-disable-next-line no-undef
          textarea.dispatchEvent(new Event('input', { bubbles: true }));

          // Focus the textarea
          textarea.focus();

          // Simulate Enter key press to trigger submission
          const enterEvent = new KeyboardEvent('keydown', {
            key: 'Enter',
            code: 'Enter',
            keyCode: 13,
            which: 13,
            bubbles: true,
          });
          textarea.dispatchEvent(enterEvent);

          setShouldAutoSubmit(false);
        }
      }, 500); // Give more time for the component to fully mount

      return () => clearTimeout(timer);
    }

    // Return undefined when condition is not met
    return undefined;
  }, [shouldAutoSubmit, initialMessage]);

  // Custom message submit handler
  const handleMessageSubmit = (message: string) => {
    // This is called after a message is submitted
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false); // Clear transitioning state once message is submitted
    console.log('Message submitted:', message);
  };

  // Custom message stream finish handler to handle recipe auto-execution
  const handleMessageStreamFinish = () => {
    // This will be called with the proper append function from BaseChat
    // For now, we'll handle auto-execution in the BaseChat component
  };

  // Determine the initial value for the chat input
  // Priority: Hub message > Recipe prompt > empty
  const initialValue = initialMessage || recipeInitialPrompt || undefined;

  // Custom chat input props for Pair-specific behavior
  const customChatInputProps = {
    // Pass initial message from Hub or recipe prompt
    initialValue,
  };

  // Custom main layout props to override background completely
  const customMainLayoutProps = {
    backgroundColor: 'transparent', // Use transparent instead of empty string
    style: { 
      backgroundColor: 'transparent',
      background: 'transparent'
    }, // Force transparent background with inline style
  };

  // Custom content before messages
  const renderBeforeMessages = () => {
    return <div>{/* Any Pair-specific content before messages can go here */}</div>;
  };

  // Fixed blur intensity and background color based on theme - matching home page styling
  const blurIntensity = 20; // Consistent blur for chat mode
  
  // Get the actual theme from document.documentElement
  const isDarkTheme = document.documentElement.classList.contains('dark');
  
  const backgroundColor = isDarkTheme
    ? 'rgba(0, 0, 0, 0.7)' // Dark mode: 70% black overlay
    : 'rgba(255, 255, 255, 0.7)'; // Light mode: 70% white overlay

  return (
    <div className="flex flex-col h-full relative bg-transparent">
      {/* Image background implementation */}
      <div className="fixed inset-0 -z-10" 
        style={{
          backgroundImage: `url('/background.jpg')`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          backgroundRepeat: 'no-repeat',
        }}
      />
      
      {/* Fixed blur overlay - always present with consistent intensity */}
      <div 
        className="fixed inset-0 -z-5 pointer-events-none"
        style={{ 
          backdropFilter: `blur(${blurIntensity}px)`,
          backgroundColor: backgroundColor
        }}
      />
      
      {/* Centered chat content */}
      <div className="relative z-10 flex justify-center h-full bg-transparent">
        <div className="w-full max-w-[1000px] h-full bg-transparent">
          <BaseChat
            chat={chat}
            setChat={setChat}
            setView={setView}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            enableLocalStorage={true} // Enable local storage for Pair mode
            onMessageSubmit={handleMessageSubmit}
            onMessageStreamFinish={handleMessageStreamFinish}
            renderBeforeMessages={renderBeforeMessages}
            customChatInputProps={customChatInputProps}
            customMainLayoutProps={customMainLayoutProps} // Override background
            contentClassName={cn('pr-1 pb-10', (isMobile || currentSidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
            showPopularTopics={!isTransitioningFromHub} // Don't show popular topics while transitioning from Hub
            suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
          />
        </div>
      </div>
    </div>
  );
}
