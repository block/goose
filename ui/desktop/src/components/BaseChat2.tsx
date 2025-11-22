import React, { useCallback, useEffect, useRef, useState, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import LoadingGoose from './LoadingGoose';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import ChatInput from './ChatInput';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { useFileDrop } from '../hooks/useFileDrop';
import { Message } from '../api';
import { ChatState } from '../types/chatState';
import { ChatType } from '../types/chat';
import { useIsMobile } from '../hooks/use-mobile';
import { cn } from '../utils';
import { useChatStream } from '../hooks/useChatStream';
import { useNavigation } from '../hooks/useNavigation';
import { RecipeHeader } from './RecipeHeader';
import { RecipeWarningModal } from './ui/RecipeWarningModal';
import { scanRecipe } from '../recipe';
import { useCostTracking } from '../hooks/useCostTracking';
import RecipeActivities from './recipes/RecipeActivities';
import { useToolCount } from './alerts/useToolCount';
import { getThinkingMessage } from '../types/message';
import ParameterInputModal from './ParameterInputModal';
import ParticipantsBar from './ParticipantsBar';
import PendingInvitesInHistory from './PendingInvitesInHistory';
import { useComments } from '../hooks/useComments';

interface BaseChatProps {
  setChat?: (chat: ChatType) => void; // Made optional for inactive tabs
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  suppressEmptyState: boolean;
  sessionId: string;
  initialMessage?: string;
  onSessionIdChange?: (newSessionId: string) => void;
  // Matrix integration props
  showParticipantsBar?: boolean;
  matrixRoomId?: string;
  showPendingInvites?: boolean;
  // Sidecar and UI props
  contentClassName?: string;
  disableSearch?: boolean;
  showPopularTopics?: boolean;
  loadingChat?: boolean;
  // Tab-specific sidecar props
  tabId?: string;
  // Tab persistence prop
  isTabActive?: boolean; // Whether this tab is currently active/visible
}

function BaseChatContent({
  setChat,
  setIsGoosehintsModalOpen,
  renderHeader,
  customChatInputProps = {},
  sessionId,
  initialMessage,
  onSessionIdChange,
  showParticipantsBar = false,
  matrixRoomId,
  showPendingInvites = false,
  contentClassName: customContentClassName,
  disableSearch = false,
  showPopularTopics = true,
  loadingChat = false,
  tabId,
}: BaseChatProps) {
  const location = useLocation();
  const scrollRef = useRef<ScrollAreaHandle>(null);

  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [hasNotAcceptedRecipe, setHasNotAcceptedRecipe] = useState<boolean>();
  const [hasRecipeSecurityWarnings, setHasRecipeSecurityWarnings] = useState(false);

  const isMobile = useIsMobile();
  const setView = useNavigation();

  // Use custom content class name if provided, otherwise default
  const contentClassName = customContentClassName || cn('pr-1 pb-10', isMobile && 'pt-11');

  // Comment state management
  const commentState = useComments(sessionId);

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  const onStreamFinish = useCallback(() => {}, []);

  const {
    session,
    messages,
    chatState,
    handleSubmit: streamHandleSubmit,
    stopStreaming,
    sessionLoadError,
    setRecipeUserParams,
    tokenState,
    gooseEnabled,
  } = useChatStream({
    sessionId,
    onStreamFinish,
    initialMessage,
    onSessionIdChange,
    isMatrixTab: !!matrixRoomId, // Pass Matrix tab flag based on whether we have a matrixRoomId
  });

  // Create append function for adding messages programmatically
  const append = useCallback((textOrMessage: string | Message) => {
    if (typeof textOrMessage === 'string') {
      // Handle string input (for existing functionality)
      streamHandleSubmit(textOrMessage);
    } else {
      // Handle Message object input (for Matrix integration)
      const message = textOrMessage;
      console.log('📥 BaseChat2 append called with Message object:', {
        sessionId: sessionId.substring(0, 8),
        id: message.id,
        role: message.role,
        content: Array.isArray(message.content) && message.content[0]?.type === 'text' 
          ? message.content[0].text?.substring(0, 50) + '...' 
          : 'N/A',
        sender: (message as any).sender?.displayName || (message as any).sender?.userId || 'unknown'
      });
      
      // FIXED: Make Matrix message events SESSION-SPECIFIC to prevent cross-tab contamination
      // Include sessionId in the event detail so only the correct useChatStream instance processes it
      const messageEvent = new CustomEvent('matrix-message-received', {
        detail: { 
          message,
          targetSessionId: sessionId, // CRITICAL: Only this session should process this message
          timestamp: new Date().toISOString()
        }
      });
      window.dispatchEvent(messageEvent);
      
      console.log('📥 BaseChat2 dispatched SESSION-SPECIFIC matrix-message-received event:', {
        messageId: message.id,
        targetSessionId: sessionId.substring(0, 8),
        sender: (message as any).sender?.displayName || (message as any).sender?.userId || 'unknown'
      });
    }
  }, [streamHandleSubmit, sessionId]);

  // Create simple command history from messages
  const commandHistory = useMemo(() => {
    return messages
      .filter(m => m.role === 'user')
      .map(m => {
        const textContent = Array.isArray(m.content) 
          ? m.content.find(c => c.type === 'text')?.text 
          : '';
        return textContent || '';
      })
      .filter(text => text.trim())
      .reverse();
  }, [messages]);

  // Simple tool call notifications (empty for now, can be enhanced later)
  const toolCallNotifications = useMemo(() => new Map(), []);

  // Simple message update handler (for future enhancement)
  const onMessageUpdate = useCallback((messageId: string, newContent: string) => {
    console.log('Message update requested:', messageId, newContent);
    // TODO: Implement message editing functionality
  }, []);

  const handleFormSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const textValue = customEvent.detail?.value || '';

    if (recipe && textValue.trim()) {
      setHasStartedUsingRecipe(true);
    }
    streamHandleSubmit(textValue);
  };

  const { sessionCosts } = useCostTracking({
    sessionInputTokens: session?.accumulated_input_tokens || 0,
    sessionOutputTokens: session?.accumulated_output_tokens || 0,
    localInputTokens: 0,
    localOutputTokens: 0,
    session,
  });

  const recipe = session?.recipe;

  useEffect(() => {
    if (!recipe) return;

    (async () => {
      const accepted = await window.electron.hasAcceptedRecipeBefore(recipe);
      setHasNotAcceptedRecipe(!accepted);

      if (!accepted) {
        const scanResult = await scanRecipe(recipe);
        setHasRecipeSecurityWarnings(scanResult.has_security_warnings);
      }
    })();
  }, [recipe]);

  const handleRecipeAccept = async (accept: boolean) => {
    if (recipe && accept) {
      await window.electron.recordRecipeHash(recipe);
      setHasNotAcceptedRecipe(false);
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
    } else if (scrollRef.current?.scrollToBottom) {
      scrollRef.current.scrollToBottom();
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
        messages={messages as any}
        chat={chat}
        // toolCallNotifications={toolCallNotifications}
        // appendMessage={(newMessage) => {
        //   const updatedMessages = [...messages, newMessage];
        //   setMessages(updatedMessages);
        // }}
        isUserMessage={(m: any) => m.role === 'user'}
        isStreamingMessage={chatState !== ChatState.Idle}
        // onMessageUpdate={onMessageUpdate}
        onRenderingComplete={handleRenderingComplete}
      />
    </>
  );

  // Use the showPopularTopics prop, but also check the current state
  const shouldShowPopularTopics = showPopularTopics && 
    messages.length === 0 && !initialMessage && chatState === ChatState.Idle;

  // Debug logging for empty state
  console.log('BaseChat2 render state:', {
    sessionId: sessionId, // Show full session ID for debugging
    sessionIdShort: sessionId.slice(0, 8), // Also show truncated for readability
    messagesLength: messages.length,
    chatState,
    shouldShowPopularTopics,
    loadingChat,
    hasSession: !!session,
    sessionName: (session as any)?.name,
    sessionDescription: session?.description
  });

  // Memoize the chat object to prevent infinite re-renders
  const chat: ChatType = useMemo(() => ({
    messageHistoryIndex: 0,
    messages: messages as any,
    recipe,
    sessionId: session?.id || sessionId, // Use actual session ID if available
    name: (session as any)?.name || 'No Session',
    title: session?.description || (messages.length > 0 ? 'Chat' : 'New Chat'),
  }), [messages, recipe, session?.id, sessionId, session?.description]);

  // Update parent only when session ID or title changes (to avoid infinite loops)
  // Only call setChat if it's provided (active tabs)
  useEffect(() => {
    if (setChat) {
      setChat(chat);
    }
  }, [setChat, chat.sessionId, chat.title]);

  const initialPrompt = messages.length == 0 && recipe?.prompt ? recipe.prompt : '';

  return (
    <div className="h-full flex flex-col min-h-0 relative">
      {/* BaseChat2 - Tabbed Chat Ready */}
      
      {/* Custom header */}
      {renderHeader && renderHeader()}

      {/* Participants Bar - shows who's in the conversation for Matrix sessions */}
      {showParticipantsBar && matrixRoomId && (
        <ParticipantsBar matrixRoomId={matrixRoomId} />
      )}

      {/* Chat container - full height, extends behind floating input */}
      <div className="absolute inset-0">
        <ScrollArea
          ref={scrollRef}
          className={`h-full bg-background-default relative ${contentClassName}`}
          autoScroll
          onDrop={handleDrop}
          onDragOver={handleDragOver}
          data-drop-zone="true"
          paddingX={6}
          paddingY={0}
        >
          {/* Chat thread container with max width */}
          <div className="max-w-4xl mx-auto w-full">
            {/* Recipe agent header - sticky at top of chat container */}
            {recipe?.title && (
              <div className="sticky top-0 z-10 bg-background-default px-0 -mx-6 mb-6 pt-6">
                <RecipeHeader title={recipe.title} />
              </div>
            )}

            {/* Recipe Activities - always show when recipe is active and accepted */}
            {recipe && !hasNotAcceptedRecipe && (
              <div className={hasStartedUsingRecipe ? 'mb-6' : ''}>
                <RecipeActivities
                  append={(text: string) => append(text)}
                  activities={Array.isArray(recipe.activities) ? recipe.activities : null}
                  title={recipe.title}
                  //parameterValues={recipeParameters || {}}
                />
              </div>
            )}

            {/* Session Load Error */}
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
            {
              loadingChat ? null : messages.length > 0 ||
                (recipe && !hasNotAcceptedRecipe && hasStartedUsingRecipe) ? (
                <>
                  {/* Spacer above first message */}
                  <div className="h-[50px]"></div>
                  
                  {disableSearch ? (
                    // Render messages without SearchView wrapper when search is disabled
                    <ProgressiveMessageList
                      messages={messages as any}
                      chat={chat}
                      toolCallNotifications={toolCallNotifications}
                      append={append}
                      appendMessage={(newMessage) => {
                        // Note: useChatStream doesn't expose setMessages, so this is a placeholder
                        console.log('appendMessage called with:', newMessage);
                      }}
                      isUserMessage={(m: any) => m.role === 'user'}
                      isStreamingMessage={chatState !== ChatState.Idle}
                      onMessageUpdate={onMessageUpdate}
                      onRenderingComplete={handleRenderingComplete}
                      // Comment props
                      comments={commentState.comments}
                      activeSelection={commentState.activeSelection}
                      activePosition={commentState.activePosition}
                      activeMessageId={commentState.activeMessageId}
                      isCreatingComment={commentState.isCreatingComment}
                      onSelectionChange={commentState.setActiveSelection}
                      onCreateComment={commentState.createComment}
                      onUpdateComment={commentState.updateComment}
                      onDeleteComment={commentState.deleteComment}
                      onReplyToComment={commentState.replyToComment}
                      onResolveComment={commentState.resolveComment}
                      onCancelComment={() => commentState.setActiveSelection(null)}
                      onFocusComment={commentState.focusComment}
                    />
                  ) : (
                    // Render messages with SearchView wrapper when search is enabled
                    <SearchView>
                      <ProgressiveMessageList
                        messages={messages as any}
                        chat={chat}
                        toolCallNotifications={toolCallNotifications}
                        append={append}
                        appendMessage={(newMessage) => {
                          // Note: useChatStream doesn't expose setMessages, so this is a placeholder
                          console.log('appendMessage called with:', newMessage);
                        }}
                        isUserMessage={(m: any) => m.role === 'user'}
                        isStreamingMessage={chatState !== ChatState.Idle}
                        onMessageUpdate={onMessageUpdate}
                        onRenderingComplete={handleRenderingComplete}
                        // Comment props
                        comments={commentState.comments}
                        activeSelection={commentState.activeSelection}
                        activePosition={commentState.activePosition}
                        activeMessageId={commentState.activeMessageId}
                        isCreatingComment={commentState.isCreatingComment}
                        onSelectionChange={commentState.setActiveSelection}
                        onCreateComment={commentState.createComment}
                        onUpdateComment={commentState.updateComment}
                        onDeleteComment={commentState.deleteComment}
                        onReplyToComment={commentState.replyToComment}
                        onResolveComment={commentState.resolveComment}
                        onCancelComment={() => commentState.setActiveSelection(null)}
                        onFocusComment={commentState.focusComment}
                      />
                    </SearchView>
                  )}

                  {/* Inline loading indicator below messages */}
                  {chatState !== ChatState.Idle && (
                    <div className="px-6 py-2">
                      <LoadingGoose
                        chatState={chatState}
                        message={
                          messages.length > 0 && messages[messages.length - 1].id != null
                            ? getThinkingMessage(messages[messages.length - 1] as any)
                            : undefined
                        }
                      />
                    </div>
                  )}

                  {/* Extra spacing at bottom to prevent overlap with floating input */}
                  <div className="block h-56" />
                </>
              ) : !recipe && shouldShowPopularTopics ? (
                /* Show PopularChatTopics when no messages, no recipe, and showPopularTopics is true */
                <div className="absolute bottom-0 left-0 right-0 flex justify-start pb-32">
                  <div className="max-w-4xl mx-auto w-full flex flex-col-reverse">
                    <PopularChatTopics append={(text: string) => append(text)} />
                    
                    {/* Show pending invites above popular topics if enabled */}
                    {showPendingInvites && (
                      <PendingInvitesInHistory showInChatHistory={false} />
                    )}
                  </div>
                </div>
              ) : showPendingInvites ? (
                /* Show only pending invites when no messages and showPendingInvites is true */
                <div className="absolute bottom-0 left-0 right-0 flex justify-start pb-32">
                  <div className="max-w-4xl mx-auto w-full">
                    <PendingInvitesInHistory showInChatHistory={false} />
                  </div>
                </div>
              ) : null /* Show nothing when messages.length === 0 && suppressEmptyState === true */
            }

            {/* Loading indicator for initial chat loading */}
            {loadingChat && (
              <div className="flex items-center justify-center h-full min-h-[400px]">
                <div className="text-center">
                  <LoadingGoose
                    message="Loading conversation..."
                    chatState={ChatState.Idle}
                  />
                  <p className="text-text-muted text-sm mt-4">
                    Fetching message history...
                  </p>
                </div>
              </div>
            )}
          </div>
        </ScrollArea>
      </div>

      {/* Floating Chat Input - positioned absolutely at bottom */}
      <div
        className={`absolute left-0 right-0 z-20 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
        style={{ bottom: '0px' }}
      >
        <ChatInput
          sessionId={sessionId}
          handleSubmit={handleFormSubmit}
            chatState={chatState}
            onStop={stopStreaming}
            commandHistory={commandHistory}
            initialValue={initialPrompt}
            setView={setView}
            numTokens={tokenState?.totalTokens ?? session?.total_tokens ?? undefined}
            inputTokens={
              tokenState?.accumulatedInputTokens ?? session?.accumulated_input_tokens ?? undefined
            }
            outputTokens={
              tokenState?.accumulatedOutputTokens ?? session?.accumulated_output_tokens ?? undefined
            }
            droppedFiles={droppedFiles}
            onFilesProcessed={() => setDroppedFiles([])} // Clear dropped files after processing
            messages={messages as any}
            setMessages={() => {}} // Placeholder - useChatStream doesn't expose setMessages
            disableAnimation={disableAnimation}
            sessionCosts={sessionCosts}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            recipeConfig={recipe}
            recipeAccepted={!hasNotAcceptedRecipe}
            initialPrompt={initialPrompt}
            toolCount={toolCount || 0}
            autoSubmit={false}
            append={append as any}
            gooseEnabled={gooseEnabled}
            {...customChatInputProps}
          />
      </div>

      {recipe && (
        <RecipeWarningModal
          isOpen={!!hasNotAcceptedRecipe}
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

      {recipe?.parameters && recipe.parameters.length > 0 && !(session as any)?.user_recipe_values && (
        <ParameterInputModal
          parameters={recipe.parameters}
          onSubmit={setRecipeUserParams}
          onClose={() => setView('chat')}
        />
      )}

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
  return <BaseChatContent {...props} />;
}
