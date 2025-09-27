import { useEffect, useState, useRef } from 'react';
import { View, ViewOptions } from '../utils/navigationUtils';
import BaseChat from './BaseChat';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useIsMobile } from '../hooks/use-mobile';
import { useSidebar } from './ui/sidebar';
import { AgentState, InitializationContext } from '../hooks/useAgent';
import 'react-toastify/dist/ReactToastify.css';
import { cn } from '../utils';
import { Recipe } from '../api';

import { ChatType } from '../types/chat';
import { useSearchParams } from 'react-router-dom';

export interface PairRouteState {
  resumeSessionId?: string;
  initialMessage?: string;
  recipeConfig?: Recipe;
}

interface PairProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  setFatalError: (value: ((prevState: string | null) => string | null) | string | null) => void;
  setAgentWaitingMessage: (msg: string | null) => void;
  agentState: AgentState;
  loadCurrentChat: (context: InitializationContext) => Promise<ChatType>;
}

export default function Pair({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  setFatalError,
  setAgentWaitingMessage,
  agentState,
  loadCurrentChat,
  resumeSessionId,
  initialMessage,
  recipeConfig,
}: PairProps & PairRouteState) {
  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const [hasProcessedInitialInput, setHasProcessedInitialInput] = useState(false);
  const [shouldAutoSubmit, setShouldAutoSubmit] = useState(false);
  const [messageToSubmit, setMessageToSubmit] = useState<string | null>(null);
  const [isTransitioningFromHub, setIsTransitioningFromHub] = useState(false);
  const [loadingChat, setLoadingChat] = useState(false);
  const [_searchParams, setSearchParams] = useSearchParams();

  // Use refs to track current chat state to avoid dependency issues
  const chatRef = useRef(chat);
  const prevRecipeConfigRef = useRef<Recipe | null>(null);
  chatRef.current = chat;

  useEffect(() => {
    const initializeFromState = async () => {
      setLoadingChat(true);

      const currentChat = chatRef.current;
      const prevRecipeConfig = prevRecipeConfigRef.current;

      // If we have a recipe config from navigation, always create a new chat with the recipe
      // This ensures we start fresh when loading a recipe from the recipes view
      // Compare using JSON.stringify to detect any changes in the recipe object
      const recipeChanged =
        recipeConfig &&
        (!prevRecipeConfig || JSON.stringify(prevRecipeConfig) !== JSON.stringify(recipeConfig));

      if (recipeConfig && recipeChanged) {
        prevRecipeConfigRef.current = recipeConfig;

        try {
          // Load a fresh chat session with forced reset
          const newChat = await loadCurrentChat({
            resumeSessionId: undefined,
            recipeConfig: recipeConfig,
            setAgentWaitingMessage,
            forceReset: true,
          });

          // Set the chat with the recipe config and ensure messages are cleared
          const chatWithRecipe = {
            ...newChat,
            recipeConfig: recipeConfig,
            recipeParameters: null,
            messages: [],
          };

          setChat(chatWithRecipe);
          setSearchParams((prev) => {
            prev.set('resumeSessionId', newChat.sessionId);
            return prev;
          });
        } catch (error) {
          setFatalError(
            `Agent init failure: ${error instanceof Error ? error.message : '' + error}`
          );
        } finally {
          setLoadingChat(false);
        }
        return;
      }

      // If we don't have a recipe config from navigation but had one before, clear it
      if (!recipeConfig && prevRecipeConfig) {
        prevRecipeConfigRef.current = null;
      }

      try {
        const loadedChat = await loadCurrentChat({
          resumeSessionId,
          setAgentWaitingMessage,
        });

        // If the loaded chat doesn't have a recipe config but we have one in the current chat, preserve it
        // BUT only if we're resuming the same session (not starting a new chat)
        let finalChat = loadedChat;
        if (
          !loadedChat.recipeConfig &&
          currentChat?.recipeConfig &&
          resumeSessionId === currentChat.sessionId
        ) {
          finalChat = {
            ...loadedChat,
            recipeConfig: currentChat.recipeConfig,
            recipeParameters: currentChat.recipeParameters || null,
          };
        }

        setChat(finalChat);
        setSearchParams((prev) => {
          prev.set('resumeSessionId', loadedChat.sessionId);
          return prev;
        });
      } catch (error) {
        setFatalError(`Agent init failure: ${error instanceof Error ? error.message : '' + error}`);
      } finally {
        setLoadingChat(false);
      }
    };
    initializeFromState();
  }, [
    agentState,
    setChat,
    setFatalError,
    setAgentWaitingMessage,
    loadCurrentChat,
    resumeSessionId,
    setSearchParams,
    recipeConfig,
  ]);

  // Followed by sending the initialMessage if we have one. This will happen
  // only once, unless we reset the chat in step one.
  useEffect(() => {
    if (agentState !== AgentState.INITIALIZED || !initialMessage || hasProcessedInitialInput) {
      return;
    }

    setIsTransitioningFromHub(true);
    setHasProcessedInitialInput(true);
    setMessageToSubmit(initialMessage);
    setShouldAutoSubmit(true);
  }, [agentState, initialMessage, hasProcessedInitialInput]);

  useEffect(() => {
    if (agentState === AgentState.NO_PROVIDER) {
      setView('welcome');
    }
  }, [agentState, setView]);

  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(
    chat,
    recipeConfig || chat.recipeConfig || null
  );

  const handleMessageSubmit = () => {
    // Clean up any auto submit state:
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false);
    setMessageToSubmit(null);
  };

  const recipePrompt =
    agentState === 'initialized' && chat.messages.length === 0 && recipeInitialPrompt;

  const initialValue = messageToSubmit || recipePrompt || undefined;

  const customChatInputProps = {
    // Pass initial message from Hub or recipe prompt
    initialValue,
  };

  return (
    <BaseChat
      chat={chat}
      loadingChat={loadingChat}
      autoSubmit={shouldAutoSubmit}
      setChat={setChat}
      setView={setView}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
      onMessageSubmit={handleMessageSubmit}
      customChatInputProps={customChatInputProps}
      contentClassName={cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11')} // Use dynamic content class with mobile margin and sidebar state
      showPopularTopics={!isTransitioningFromHub} // Don't show popular topics while transitioning from Hub
      suppressEmptyState={isTransitioningFromHub} // Suppress all empty state content while transitioning from Hub
    />
  );
}
