import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { View, ViewOptions } from '../utils/navigationUtils';
import BaseChat from './BaseChat';
import { useRecipeManager } from '../hooks/useRecipeManager';
import { useIsMobile } from '../hooks/use-mobile';
import { useSidebar } from './ui/sidebar';
import { AgentState, InitializationContext } from '../hooks/useAgent';
import 'react-toastify/dist/ReactToastify.css';
import { cn } from '../utils';

import { ChatType } from '../types/chat';
import { Recipe } from '../recipe';

export interface PairRouteState {
  resumeSessionId?: string;
  recipeConfig?: Recipe;
  initialMessage?: string;
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
  routeState,
}: {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  setFatalError: (value: ((prevState: string | null) => string | null) | string | null) => void;
  setAgentWaitingMessage: (msg: string | null) => void;
  agentState: AgentState;
  loadCurrentChat: (context: InitializationContext) => Promise<ChatType>;
  routeState: PairRouteState;
}) {
  const location = useLocation();
  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const [hasProcessedInitialInput, setHasProcessedInitialInput] = useState(false);
  const [shouldAutoSubmit, setShouldAutoSubmit] = useState(false);
  const [messageToSubmit, setMessageToSubmit] = useState<string | null>(null);
  const [isTransitioningFromHub, setIsTransitioningFromHub] = useState(false);
  const [loadingChat, setLoadingChat] = useState(false);

  const { initialPrompt: recipeInitialPrompt } = useRecipeManager(chat, location.state);

  const recipeJson = JSON.stringify(routeState.recipeConfig);

  useEffect(() => {
    const initializeFromState = async () => {
      setLoadingChat(true);
      try {
        const chat = await loadCurrentChat({
          recipeConfig: routeState.recipeConfig,
          resumeSessionId: routeState.resumeSessionId,
          setAgentWaitingMessage,
        });
        setChat(chat);
      } catch (error) {
        setFatalError(`Agent init failure: ${error instanceof Error ? error.message : '' + error}`);
      } finally {
        setLoadingChat(false);
      }
    };

    initializeFromState();
  }, [
    setChat,
    setFatalError,
    setAgentWaitingMessage,
    loadCurrentChat,
    routeState.resumeSessionId,
    routeState.recipeConfig,
    recipeJson, // TODO: Hacky object comparison, but works for now
  ]);

  // Followed by sending the initialMessage if we have one. This will happen
  // only once, unless we reset the chat in step one.
  useEffect(() => {
    if (
      agentState !== AgentState.INITIALIZED ||
      !routeState.initialMessage ||
      hasProcessedInitialInput
    ) {
      return;
    }

    setIsTransitioningFromHub(true);
    setHasProcessedInitialInput(true);
    setMessageToSubmit(routeState.initialMessage);
    setShouldAutoSubmit(true);
  }, [agentState, routeState.initialMessage, hasProcessedInitialInput]);

  useEffect(() => {
    if (agentState === AgentState.NO_PROVIDER) {
      setView('welcome');
    }
  }, [agentState, setView]);

  const handleMessageSubmit = (message: string) => {
    // Clean up any auto submit state:
    setShouldAutoSubmit(false);
    setIsTransitioningFromHub(false);
    setMessageToSubmit(null);
    console.log('Message submitted:', message);
  };

  const initialValue = messageToSubmit || recipeInitialPrompt || undefined;

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
