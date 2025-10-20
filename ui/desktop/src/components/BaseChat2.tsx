import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import LoadingGoose from './LoadingGoose';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import { ContextManagerProvider } from './context_management/ContextManager';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { useFileDrop } from '../hooks/useFileDrop';
import { Message } from '../api';
import { ChatState } from '../types/chatState';
import { ChatType } from '../types/chat';
import { useIsMobile } from '../hooks/use-mobile';
import { useSidebar } from './ui/sidebar';
import { cn } from '../utils';
import { useChatStream } from '../hooks/useChatStream';
import { useNavigation } from '../hooks/useNavigation';
import { RecipeHeader } from './RecipeHeader';
import { RecipeWarningModal } from './ui/RecipeWarningModal';
import { scanRecipe } from '../recipe';
import { useCostTracking } from '../hooks/useCostTracking';
import RecipeActivities from './recipes/RecipeActivities';
import { useToolCount } from './alerts/useToolCount';
import { useChatState } from '../contexts/ChatStateContext';
import { createDebugLogger } from '../utils/debugLogger';

const logger = createDebugLogger('BaseChat2');

interface BaseChatProps {
  setChat: (chat: ChatType | ((prevChat: ChatType) => ChatType)) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  suppressEmptyState: boolean;
  sessionId: string;
  initialMessage?: string;
}

function BaseChatContent({
  setChat,
  setIsGoosehintsModalOpen,
  renderHeader,
  customChatInputProps = {},
  customMainLayoutProps = {},
  sessionId,
  initialMessage,
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [hasAcceptedRecipe, setHasAcceptedRecipe] = useState<boolean>();
  const [hasRecipeSecurityWarnings, setHasRecipeSecurityWarnings] = useState(false);

  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const setView = useNavigation();

  const contentClassName = cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11');

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  // Get chat state from context
  const { getCachedSession, updateSessionCache, lastActiveSessionId, setLastActiveSessionId } =
    useChatState();

  // Resolve which session to use: explicit sessionId prop, or fallback to last active session
  const [resolvedSessionId, setResolvedSessionId] = useState(sessionId);

  useEffect(() => {
    if (!sessionId && lastActiveSessionId) {
      logger.tracking('No sessionId provided, loading last active session', lastActiveSessionId);
      setResolvedSessionId(lastActiveSessionId);
    } else if (sessionId) {
      setResolvedSessionId(sessionId);
    }
  }, [sessionId, lastActiveSessionId]);

  const onStreamFinish = useCallback(() => {}, []);

  const {
    session,
    messages,
    chatState,
    handleSubmit,
    stopStreaming,
    sessionLoadError,
    loadedFromCache,
  } = useChatStream({
    sessionId: resolvedSessionId,
    onStreamFinish,
    initialMessage,
    sessionCache: getCachedSession,
  });

  const handleFormSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const textValue = customEvent.detail?.value || '';

    if (recipe && textValue.trim()) {
      setHasStartedUsingRecipe(true);
    }
    handleSubmit(textValue);
  };

  const { sessionCosts } = useCostTracking({
    sessionInputTokens: session?.accumulated_input_tokens || 0,
    sessionOutputTokens: session?.accumulated_output_tokens || 0,
    localInputTokens: 0,
    localOutputTokens: 0,
    session,
  });

  const recipe = session?.recipe;

  // Update global session cache when session/messages change (but NOT when loaded from cache)
  useEffect(() => {
    // Only update cache if we got fresh data from the server, not from cache
    if (session && resolvedSessionId && !loadedFromCache) {
      updateSessionCache(resolvedSessionId, {
        session,
        messages,
        cachedAt: Date.now(),
      });
    }
  }, [resolvedSessionId, session, messages, loadedFromCache, updateSessionCache]);

  // Track the last active session
  useEffect(() => {
    if (resolvedSessionId && session) {
      logger.tracking('Setting last active session', resolvedSessionId);
      setLastActiveSessionId(resolvedSessionId);
    }
  }, [resolvedSessionId, session, setLastActiveSessionId]);

  // Update parent chat state when session/messages change
  useEffect(() => {
    setChat((prevChat) => ({
      ...prevChat,
      sessionId,
      title: session?.description || 'No Session',
      messages,
      recipe,
    }));
  }, [sessionId, session, messages, recipe, setChat]);

  useEffect(() => {
    if (!recipe) return;

    (async () => {
      const accepted = await window.electron.hasAcceptedRecipeBefore(recipe);
      setHasAcceptedRecipe(accepted);

      if (!accepted) {
        const scanResult = await scanRecipe(recipe);
        setHasRecipeSecurityWarnings(scanResult.has_security_warnings);
      }
    })();
  }, [recipe]);

  const handleRecipeAccept = async (accept: boolean) => {
    if (recipe && accept) {
      await window.electron.recordRecipeHash(recipe);
      setHasAcceptedRecipe(true);
    } else {
      setView('chat');
    }
  };

  // Track if this is the initial render for session resuming
  const initialRenderRef = useRef(true);

  // Auto-scroll when messages are loaded (for session resuming)
  const handleRenderingComplete = React.useCallback(() => {
    // Only force scroll on the very first render
    if (initialRenderRef.current && messages.length > 0) {
      initialRenderRef.current = false;
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    } else if (scrollRef.current?.isFollowing) {
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  }, [messages.length]);

  const toolCount = useToolCount(sessionId);

  // Listen for global scroll-to-bottom requests (e.g., from MCP UI prompt actions)
  useEffect(() => {
    const handleGlobalScrollRequest = () => {
      // Add a small delay to ensure content has been rendered
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 200);
    };

    window.addEventListener('scroll-chat-to-bottom', handleGlobalScrollRequest);
    return () => window.removeEventListener('scroll-chat-to-bottom', handleGlobalScrollRequest);
  }, []);

  const renderProgressiveMessageList = (chat: ChatType) => (
    <>
      <ProgressiveMessageList
        messages={messages}
        chat={chat}
        // toolCallNotifications={toolCallNotifications}
        // appendMessage={(newMessage) => {
        //   const updatedMessages = [...messages, newMessage];
        //   setMessages(updatedMessages);
        // }}
        isUserMessage={(m: Message) => m.role === 'user'}
        isStreamingMessage={chatState !== ChatState.Idle}
        // onMessageUpdate={onMessageUpdate}
        onRenderingComplete={handleRenderingComplete}
      />
    </>
  );

  const showPopularTopics =
    messages.length === 0 && !initialMessage && chatState === ChatState.Idle;
  // TODO(Douwe): get this from the backend
  const isCompacting = false;

  const chat: ChatType = {
    messageHistoryIndex: 0,
    messages,
    recipe,
    sessionId,
    title: session?.description || 'No Session',
  };

  const initialPrompt = messages.length == 0 && recipe?.prompt ? recipe.prompt : '';

  // Map chatState to LoadingGoose message
  const getLoadingMessage = (): string | undefined => {
    if (isCompacting) return 'goose is compacting the conversation...';
    if (messages.length === 0 && chatState === ChatState.Thinking) {
      return 'loading conversation...';
    }
    return undefined;
  };
  return (
    <div className="h-full flex flex-col min-h-0">
      <h2>Warning: BaseChat2!</h2>
      <MainPanelLayout
        backgroundColor={'bg-background-muted'}
        removeTopPadding={true}
        {...customMainLayoutProps}
      >
        {/* Custom header */}
        {renderHeader && renderHeader()}

        {/* Chat container with sticky recipe header */}
        <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
          <ScrollArea
            ref={scrollRef}
            className={`flex-1 bg-background-default rounded-b-2xl min-h-0 relative ${contentClassName}`}
            autoScroll
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            data-drop-zone="true"
            paddingX={6}
            paddingY={0}
          >
            {recipe?.title && (
              <div className="sticky top-0 z-10 bg-background-default px-0 -mx-6 mb-6 pt-6">
                <RecipeHeader title={recipe.title} />
              </div>
            )}

            {recipe && (
              <div className={hasStartedUsingRecipe ? 'mb-6' : ''}>
                <RecipeActivities
                  append={(text: string) => handleSubmit(text)}
                  activities={Array.isArray(recipe.activities) ? recipe.activities : null}
                  title={recipe.title}
                  //parameterValues={recipeParameters || {}}
                />
              </div>
            )}

            {sessionLoadError && (
              <div className="flex flex-col items-center justify-center p-8">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg mb-4 max-w-md">
                  <h3 className="font-semibold mb-2">Failed to Load Session</h3>
                  <p className="text-sm">{sessionLoadError}</p>
                </div>
                <button
                  onClick={() => {
                    setView('chat');
                  }}
                  className="px-4 py-2 text-center cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-lg transition-all duration-150"
                >
                  Go home
                </button>
              </div>
            )}

            {/* Messages or Popular Topics */}
            {messages.length > 0 || recipe ? (
              <>
                <SearchView>{renderProgressiveMessageList(chat)}</SearchView>

                <div className="block h-8" />
              </>
            ) : !recipe && showPopularTopics ? (
              <PopularChatTopics append={(text: string) => handleSubmit(text)} />
            ) : null}
          </ScrollArea>

          {(chatState !== ChatState.Idle || isCompacting) && !sessionLoadError && (
            <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
              <LoadingGoose message={getLoadingMessage()} chatState={chatState} />
            </div>
          )}
        </div>

        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          <ChatInput
            sessionId={sessionId}
            handleSubmit={handleFormSubmit}
            chatState={chatState}
            onStop={stopStreaming}
            //commandHistory={commandHistory}
            initialValue={initialPrompt}
            setView={setView}
            numTokens={session?.total_tokens || undefined}
            inputTokens={session?.input_tokens || undefined}
            outputTokens={session?.output_tokens || undefined}
            droppedFiles={droppedFiles}
            onFilesProcessed={() => setDroppedFiles([])} // Clear dropped files after processing
            messages={messages}
            setMessages={(_m) => {}}
            disableAnimation={disableAnimation}
            sessionCosts={sessionCosts}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            recipe={recipe}
            recipeAccepted={hasAcceptedRecipe}
            initialPrompt={initialPrompt}
            toolCount={toolCount || 0}
            autoSubmit={false}
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

      {recipe && (
        <RecipeWarningModal
          isOpen={!hasAcceptedRecipe}
          onConfirm={() => handleRecipeAccept(true)}
          onCancel={() => handleRecipeAccept(false)}
          recipeDetails={{
            title: recipe.title,
            description: recipe.description,
            instructions: recipe.instructions || undefined,
          }}
          hasSecurityWarnings={hasRecipeSecurityWarnings}
        />
      )}

      {/*/!* Recipe Parameter Modal *!/*/}
      {/*{isParameterModalOpen && filteredParameters.length > 0 && (*/}
      {/*  <ParameterInputModal*/}
      {/*    parameters={filteredParameters}*/}
      {/*    onSubmit={handleParameterSubmit}*/}
      {/*    onClose={() => setIsParameterModalOpen(false)}*/}
      {/*  />*/}
      {/*)}*/}

      {/*/!* Create Recipe from Session Modal *!/*/}
      {/*<CreateRecipeFromSessionModal*/}
      {/*  isOpen={isCreateRecipeModalOpen}*/}
      {/*  onClose={() => setIsCreateRecipeModalOpen(false)}*/}
      {/*  sessionId={chat.sessionId}*/}
      {/*  onRecipeCreated={handleRecipeCreated}*/}
      {/*/>*/}
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return (
    <ContextManagerProvider>
      <BaseChatContent {...props} />
    </ContextManagerProvider>
  );
}
