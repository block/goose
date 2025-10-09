import React, { useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import LoadingGoose from './LoadingGoose';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import { View, ViewOptions } from '../utils/navigationUtils';
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
import { useSessionStream } from '../hooks/useSessionStream';

interface BaseChatProps {
  chat: ChatType;
  setChat: (chat: ChatType) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageStreamFinish?: () => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  renderBeforeMessages?: () => React.ReactNode;
  renderAfterMessages?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  contentClassName?: string;
  disableSearch?: boolean;
  showPopularTopics?: boolean;
  suppressEmptyState?: boolean;
  autoSubmit?: boolean;
  resumeSessionId?: string; // Optional session ID to resume on mount
}

function BaseChatContent({
  chat,
  setChat,
  setView,
  setIsGoosehintsModalOpen,
  renderHeader,
  renderBeforeMessages,
  renderAfterMessages,
  customChatInputProps = {},
  customMainLayoutProps = {},
  disableSearch = false,
  resumeSessionId,
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const disableAnimation = location.state?.disableAnimation || false;
  // const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  // const [currentRecipeTitle, setCurrentRecipeTitle] = React.useState<string | null>(null);
  // const { isCompacting, handleManualCompaction } = useContextManager();
  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();

  const contentClassName = cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11');

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  // Use shared cost tracking
  // const { sessionCosts } = useCostTracking({
  //   sessionInputTokens,
  //   sessionOutputTokens,
  //   localInputTokens,
  //   localOutputTokens,
  //   session: sessionMetadata,
  // });

  // Keep session streaming active for multi-window sync, but track initial load
  const [sessionLoadComplete, setSessionLoadComplete] = useState(!resumeSessionId);

  const {
    session: streamedSession,
    isLoading: sessionLoading,
    error: sessionStreamError,
    // isConnected: sessionStreamConnected, // maybe we show an indicator somewhere?
  } = useSessionStream(resumeSessionId || undefined);

  const [messages, setMessages] = useState(chat.messages || []);
  const isStreamingRef = useRef(false);
  const lastStreamedMessageCountRef = useRef(0);

  // Update chat when streamed session data arrives
  useEffect(() => {
    if (streamedSession && resumeSessionId) {
      const conversation = streamedSession.conversation || [];

      // Mark initial load as complete
      if (!sessionLoadComplete) {
        setSessionLoadComplete(true);
      }

      // Only update if we're not actively streaming a response AND
      // the message count has actually changed (to avoid unnecessary re-renders)
      if (!isStreamingRef.current && conversation.length !== lastStreamedMessageCountRef.current) {
        lastStreamedMessageCountRef.current = conversation.length;

        const loadedChat: ChatType = {
          sessionId: streamedSession.id,
          title: streamedSession.description || 'Untitled Chat',
          messageHistoryIndex: 0,
          messages: conversation,
          recipe: null,
          recipeParameters: null,
        };
        setChat(loadedChat);
        setMessages(conversation);

        // Log for debugging
        window.electron.logInfo(
          `Session updated from stream: ${streamedSession.id}, messages: ${conversation.length}`
        );
      }
    }
  }, [streamedSession, resumeSessionId, sessionLoadComplete, setChat]);

  // Update messages when chat.sessionId changes (switching sessions)
  // but NOT when just chat.messages changes (to avoid conflicts with streaming)
  const prevSessionIdRef = useRef(chat.sessionId);
  useEffect(() => {
    if (chat.sessionId !== prevSessionIdRef.current) {
      prevSessionIdRef.current = chat.sessionId;
      if (chat.messages && !isStreamingRef.current) {
        setMessages(chat.messages);
      }
    }
  }, [chat.sessionId, chat.messages]);

  // Use chat.sessionId as the source of truth - it gets updated by the effect above
  // when streamedSession arrives
  const { chatState, handleSubmit, stopStreaming } = useChatStream({
    sessionId: chat.sessionId || '',
    messages,
    setMessages,
    onStreamFinish: () => {},
  });

  // Track streaming state to prevent session updates from overwriting messages
  useEffect(() => {
    isStreamingRef.current = chatState !== ChatState.Idle;
  }, [chatState]);

  const handleFormSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const textValue = customEvent.detail?.value || '';

    // if (recipe && textValue.trim()) {
    //   setHasStartedUsingRecipe(true);
    // }
    //
    // if (onMessageSubmit && textValue.trim()) {
    //   onMessageSubmit(textValue);
    // }

    handleSubmit(textValue);
  };

  // TODO(Douwe): send this to the chatbox instead, possibly autosubmit? or backend
  const append = (_txt: string) => {};

  useEffect(() => {
    window.electron.logInfo(
      'Initial messages when resuming session: ' + JSON.stringify(messages, null, 2)
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Track if this is the initial render for session resuming
  const initialRenderRef = useRef(true);

  const recipe = chat?.recipe;

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

  //const toolCount = useToolCount(chat.sessionId);

  // Wrapper for append that tracks recipe usage
  // const appendWithTracking = (text: string | Message) => {
  //   // Mark that user has started using the recipe when they use append
  //   if (recipe) {
  //     setHasStartedUsingRecipe(true);
  //   }
  //   append(text);
  // };

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

  const showPopularTopics = messages.length === 0;
  // TODO(Douwe): get this from the backend
  const isCompacting = false;

  const initialPrompt = messages.length == 0 && recipe?.prompt ? recipe.prompt : '';
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
            {/*/!* Recipe agent header - sticky at top of chat container *!/*/}
            {/*{recipe?.title && (*/}
            {/*  <div className="sticky top-0 z-10 bg-background-default px-0 -mx-6 mb-6 pt-6">*/}
            {/*    <AgentHeader*/}
            {/*      title={recipe.title}*/}
            {/*      profileInfo={*/}
            {/*        recipe.profile ? `${recipe.profile} - ${recipe.mcps || 12} MCPs` : undefined*/}
            {/*      }*/}
            {/*      onChangeProfile={() => {*/}
            {/*        console.log('Change profile clicked');*/}
            {/*      }}*/}
            {/*      showBorder={true}*/}
            {/*    />*/}
            {/*  </div>*/}
            {/*)}*/}

            {/* Custom content before messages */}
            {renderBeforeMessages && renderBeforeMessages()}

            {/*/!* Recipe Activities - always show when recipe is active and accepted *!/*/}
            {/*{recipe && recipeAccepted && !suppressEmptyState && (*/}
            {/*  <div className={hasStartedUsingRecipe ? 'mb-6' : ''}>*/}
            {/*    <RecipeActivities*/}
            {/*      append={(text: string) => appendWithTracking(text)}*/}
            {/*      activities={Array.isArray(recipe.activities) ? recipe.activities : null}*/}
            {/*      title={recipe.title}*/}
            {/*      parameterValues={recipeParameters || {}}*/}
            {/*    />*/}
            {/*  </div>*/}
            {/*)}*/}

            {sessionStreamError && (
              <div className="flex flex-col items-center justify-center p-8">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg mb-4 max-w-md">
                  <h3 className="font-semibold mb-2">Failed to Load Session</h3>
                  <p className="text-sm">{sessionStreamError}</p>
                </div>
              </div>
            )}

            {/* Messages or Popular Topics */}
            {
              messages.length > 0 || recipe ? (
                <>
                  {disableSearch ? (
                    renderProgressiveMessageList(chat)
                  ) : (
                    // Render messages with SearchView wrapper when search is enabled
                    <SearchView>{renderProgressiveMessageList(chat)}</SearchView>
                  )}

                  {/*{error && (*/}
                  {/*  <>*/}
                  {/*    <div className="flex flex-col items-center justify-center p-4">*/}
                  {/*      <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">*/}
                  {/*        {error.message || 'Honk! Goose experienced an error while responding'}*/}
                  {/*      </div>*/}

                  {/*      /!* Action buttons for all errors including token limit errors *!/*/}
                  {/*      <div className="flex gap-2 mt-2">*/}
                  {/*        <div*/}
                  {/*          className="px-3 py-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"*/}
                  {/*          onClick={async () => {*/}
                  {/*            clearError();*/}

                  {/*            await handleManualCompaction(*/}
                  {/*              messages,*/}
                  {/*              setMessages,*/}
                  {/*              append,*/}
                  {/*              chat.sessionId*/}
                  {/*            );*/}
                  {/*          }}*/}
                  {/*        >*/}
                  {/*          Summarize Conversation*/}
                  {/*        </div>*/}
                  {/*        <div*/}
                  {/*          className="px-3 py-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"*/}
                  {/*          onClick={async () => {*/}
                  {/*            // Find the last user message*/}
                  {/*            const lastUserMessage = messages.reduceRight(*/}
                  {/*              (found, m) => found || (m.role === 'user' ? m : null),*/}
                  {/*              null as Message | null*/}
                  {/*            );*/}
                  {/*            if (lastUserMessage) {*/}
                  {/*              await append(lastUserMessage);*/}
                  {/*            }*/}
                  {/*          }}*/}
                  {/*        >*/}
                  {/*          Retry Last Message*/}
                  {/*        </div>*/}
                  {/*      </div>*/}
                  {/*    </div>*/}
                  {/*  </>*/}
                  {/*)}*/}

                  <div className="block h-8" />
                </>
              ) : !recipe && showPopularTopics ? (
                /* Show PopularChatTopics when no messages, no recipe, and showPopularTopics is true (Pair view) */
                <PopularChatTopics append={(text: string) => append(text)} />
              ) : null /* Show nothing when messages.length === 0 && suppressEmptyState === true */
            }

            {/* Custom content after messages */}
            {renderAfterMessages && renderAfterMessages()}
          </ScrollArea>

          {/* Fixed loading indicator at bottom left of chat container */}
          {((sessionLoading && !sessionLoadComplete) ||
            (messages.length === 0 && !resumeSessionId) ||
            isCompacting) &&
            !sessionStreamError && (
              <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
                <LoadingGoose
                  message={
                    (sessionLoading && !sessionLoadComplete) ||
                    (messages.length === 0 && !resumeSessionId)
                      ? 'loading conversation...'
                      : isCompacting
                        ? 'goose is compacting the conversation...'
                        : undefined
                  }
                  chatState={chatState}
                />
              </div>
            )}
        </div>

        <div
          className={`relative z-10 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        >
          <ChatInput
            sessionId={chat?.sessionId || ''}
            handleSubmit={handleFormSubmit}
            chatState={chatState}
            onStop={stopStreaming}
            //commandHistory={commandHistory}
            initialValue={initialPrompt}
            setView={setView}
            // numTokens={sessionTokenCount}
            // inputTokens={sessionInputTokens || localInputTokens}
            // outputTokens={sessionOutputTokens || localOutputTokens}
            droppedFiles={droppedFiles}
            onFilesProcessed={() => setDroppedFiles([])} // Clear dropped files after processing
            messages={messages}
            setMessages={(_m) => {}}
            disableAnimation={disableAnimation}
            //sessionCosts={sessionCosts}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            recipe={recipe}
            //recipeAccepted={recipeAccepted}
            initialPrompt={initialPrompt}
            //toolCount={toolCount || 0}
            toolCount={0}
            //autoSubmit={autoSubmit}
            autoSubmit={false}
            //append={append}
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

      {/*/!* Recipe Warning Modal *!/*/}
      {/*<RecipeWarningModal*/}
      {/*  isOpen={isRecipeWarningModalOpen}*/}
      {/*  onConfirm={handleRecipeAccept}*/}
      {/*  onCancel={handleRecipeCancel}*/}
      {/*  recipeDetails={{*/}
      {/*    title: recipe?.title,*/}
      {/*    description: recipe?.description,*/}
      {/*    instructions: recipe?.instructions || undefined,*/}
      {/*  }}*/}
      {/*  hasSecurityWarnings={hasSecurityWarnings}*/}
      {/*/>*/}

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
