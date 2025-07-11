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
import { Message } from '../types/message';
import BaseChat from './BaseChat';
import ParameterInputModal from './ParameterInputModal';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { Recipe } from '../recipe';
import 'react-toastify/dist/ReactToastify.css';

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipeConfig?: Recipe | null; // Add recipe configuration to chat state
}

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
  const [hasProcessedInitialInput, setHasProcessedInitialInput] = useState(false);
  const [shouldAutoSubmit, setShouldAutoSubmit] = useState(false);
  const [initialMessage, setInitialMessage] = useState<string | null>(null);

  // Get recipe configuration and parameter handling
  const {
    recipeConfig,
    initialPrompt: recipeInitialPrompt,
    isParameterModalOpen,
    setIsParameterModalOpen,
    handleParameterSubmit,
  } = useRecipeManager(chat.messages, location.state);

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
      };
      setChat(newChat);

      // Clear the location state to prevent re-processing
      window.history.replaceState({}, '', '/pair');
    }
  }, [location.state, chat.id, setChat]);

  // Handle initial message from hub page
  useEffect(() => {
    const messageFromHub = location.state?.initialMessage;

    // Reset processing state when we have a new message from hub
    if (messageFromHub) {
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
  }, [location.state, hasProcessedInitialInput, initialMessage, chat]);

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

      // eslint-disable-next-line no-undef
      return () => clearTimeout(timer);
    }

    // Return undefined when condition is not met
    return undefined;
  }, [shouldAutoSubmit, initialMessage]);

  // Custom message submit handler
  const handleMessageSubmit = (message: string) => {
    // This is called after a message is submitted
    setShouldAutoSubmit(false);
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

  // Custom content before messages
  const renderBeforeMessages = () => {
    return <div>{/* Any Pair-specific content before messages can go here */}</div>;
  };

  return (
    <>
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
        contentClassName="pl-6 pl-4 pb-16 pt-2" // Add Pair-specific padding
      />

      {/* Recipe Parameter Modal */}
      {isParameterModalOpen && recipeConfig?.parameters && (
        <ParameterInputModal
          parameters={recipeConfig.parameters}
          onSubmit={handleParameterSubmit}
          onClose={() => setIsParameterModalOpen(false)}
        />
      )}
    </>
  );
}
