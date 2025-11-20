import React, { useCallback, useEffect, useRef, useState, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import LoadingGoose from './LoadingGoose';
import PopularChatTopics from './PopularChatTopics';
import ProgressiveMessageList from './ProgressiveMessageList';
import { MainPanelLayout } from './Layout/MainPanelLayout';
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
import { SidecarInvoker } from './Layout/SidecarInvoker';
import { useSidecar } from './SidecarLayout';
import ParticipantsBar from './ParticipantsBar';
import PendingInvitesInHistory from './PendingInvitesInHistory';

interface BaseChatProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  suppressEmptyState: boolean;
  sessionId: string;
  initialMessage?: string;
  // Matrix integration props
  showParticipantsBar?: boolean;
  matrixRoomId?: string;
  showPendingInvites?: boolean;
  // Sidecar and UI props
  contentClassName?: string;
  disableSearch?: boolean;
  showPopularTopics?: boolean;
  loadingChat?: boolean;
}

function BaseChatContent({
  setChat,
  setIsGoosehintsModalOpen,
  renderHeader,
  customChatInputProps = {},
  customMainLayoutProps = {},
  sessionId,
  initialMessage,
  showParticipantsBar = false,
  matrixRoomId,
  showPendingInvites = false,
  contentClassName: customContentClassName,
  disableSearch = false,
  showPopularTopics = true,
  loadingChat = false,
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

  // Hover state for sidecar dock
  const [isHoveringChatInput, setIsHoveringChatInput] = useState(false);

  // Sidecar functionality
  const sidecar = useSidecar();

  const handleShowLocalhost = () => {
    console.log('Localhost viewer requested');
    if (sidecar) {
      sidecar.showLocalhostViewer('http://localhost:3000', 'Localhost Viewer');
    }
  };

  const handleShowFileViewer = (filePath: string) => {
    console.log('File viewer requested for:', filePath);
    if (sidecar) {
      sidecar.showFileViewer(filePath);
    }
  };

  const handleAddContainer = (type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer', filePath?: string) => {
    console.log('Add container requested:', type, filePath);
    
    if (!sidecar) {
      console.error('No sidecar context available');
      return;
    }

    // Use sidecar context directly instead of dispatching events
    switch (type) {
      case 'sidecar':
        // Show a generic sidecar view
        sidecar.showView({
          id: `sidecar-${Date.now()}`,
          title: 'Sidecar',
          icon: <div className="w-4 h-4 bg-blue-500 rounded" />,
          content: (
            <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
              <p>Sidecar content will go here</p>
            </div>
          ),
        });
        break;
      case 'localhost':
        sidecar.showLocalhostViewer('http://localhost:3000', 'Localhost Viewer');
        break;
      case 'file':
        if (filePath) {
          sidecar.showFileViewer(filePath);
        }
        break;
      case 'document-editor':
        sidecar.showDocumentEditor(filePath);
        break;
      case 'web-viewer':
        // Show a proper web viewer
        sidecar.showView({
          id: `web-viewer-${Date.now()}`,
          title: 'Web Viewer',
          icon: <div className="w-4 h-4 bg-cyan-500 rounded" />,
          content: null, // Will be rendered by contentType
          contentType: 'web-viewer',
          contentProps: {
            initialUrl: 'https://google.com',
            allowAllSites: true
          }
        });
        break;
      case 'app-installer':
        // Show a generic app installer view
        sidecar.showView({
          id: `app-installer-${Date.now()}`,
          title: 'App Installer',
          icon: <div className="w-4 h-4 bg-green-500 rounded" />,
          content: (
            <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
              <p>App installer will go here</p>
            </div>
          ),
        });
        break;
      default:
        console.warn('Unknown container type:', type);
    }
  };

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
  } = useChatStream({
    sessionId,
    onStreamFinish,
    initialMessage,
  });

  // Create append function for adding messages programmatically
  const append = useCallback((text: string) => {
    streamHandleSubmit(text);
  }, [streamHandleSubmit]);

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

  // Use the showPopularTopics prop, but also check the current state
  const shouldShowPopularTopics = showPopularTopics && 
    messages.length === 0 && !initialMessage && chatState === ChatState.Idle;

  // Memoize the chat object to prevent infinite re-renders
  const chat: ChatType = useMemo(() => ({
    messageHistoryIndex: 0,
    messages,
    recipe,
    sessionId: session?.id || sessionId, // Use actual session ID if available
    name: session?.name || 'No Session',
    title: session?.description || (messages.length > 0 ? 'Chat' : 'New Chat'),
  }), [messages, recipe, session?.id, sessionId, session?.name, session?.description]);

  // Update parent with chat state whenever it changes
  useEffect(() => {
    setChat(chat);
  }, [setChat, chat]);

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
      <div className="absolute inset-0 bg-background-muted">
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
                      messages={messages}
                      chat={chat}
                      toolCallNotifications={toolCallNotifications}
                      append={append}
                      appendMessage={(newMessage) => {
                        // Note: useChatStream doesn't expose setMessages, so this is a placeholder
                        console.log('appendMessage called with:', newMessage);
                      }}
                      isUserMessage={(m: Message) => m.role === 'user'}
                      isStreamingMessage={chatState !== ChatState.Idle}
                      onMessageUpdate={onMessageUpdate}
                      onRenderingComplete={handleRenderingComplete}
                    />
                  ) : (
                    // Render messages with SearchView wrapper when search is enabled
                    <SearchView>
                      <ProgressiveMessageList
                        messages={messages}
                        chat={chat}
                        toolCallNotifications={toolCallNotifications}
                        append={append}
                        appendMessage={(newMessage) => {
                          // Note: useChatStream doesn't expose setMessages, so this is a placeholder
                          console.log('appendMessage called with:', newMessage);
                        }}
                        isUserMessage={(m: Message) => m.role === 'user'}
                        isStreamingMessage={chatState !== ChatState.Idle}
                        onMessageUpdate={onMessageUpdate}
                        onRenderingComplete={handleRenderingComplete}
                      />
                    </SearchView>
                  )}

                  {/* Inline loading indicator below messages */}
                  {chatState !== ChatState.Idle && (
                    <div className="px-6 py-2">
                      <LoadingGoose
                        chatState={chatState}
                        message={
                          messages.length > 0
                            ? getThinkingMessage(messages[messages.length - 1])
                            : undefined
                        }
                      />
                    </div>
                  )}

                  {/* Extra spacing at bottom to prevent overlap with floating input */}
                  <div className="block h-32" />
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
              <div className="px-6 py-4">
                <LoadingGoose
                  message="loading conversation..."
                  chatState={ChatState.Idle}
                />
              </div>
            )}
          </div>
        </ScrollArea>
      </div>

      {/* Floating Chat Input - positioned absolutely at bottom */}
      <div
        className={`absolute bottom-0 left-0 right-0 z-20 ${disableAnimation ? '' : 'animate-[fadein_400ms_ease-in_forwards]'}`}
      >
        {/* Combined hover zone for both dock and chat input */}
        <div
          onMouseEnter={() => setIsHoveringChatInput(true)}
          onMouseLeave={() => setIsHoveringChatInput(false)}
        >
          {/* Sidecar Invoker Dock - positioned above ChatInput with proper spacing */}
          <div className="relative max-w-4xl mx-auto w-full">
            <SidecarInvoker 
              onShowLocalhost={handleShowLocalhost}
              onShowFileViewer={handleShowFileViewer}
              onAddContainer={handleAddContainer}
              isVisible={isHoveringChatInput}
            />
          </div>

          <ChatInput
            sessionId={sessionId}
            handleSubmit={handleFormSubmit}
            chatState={chatState}
            onStop={stopStreaming}
            commandHistory={commandHistory}
            initialValue={initialPrompt}
            setView={setView}
            totalTokens={tokenState?.totalTokens ?? session?.total_tokens ?? undefined}
            accumulatedInputTokens={
              tokenState?.accumulatedInputTokens ?? session?.accumulated_input_tokens ?? undefined
            }
            accumulatedOutputTokens={
              tokenState?.accumulatedOutputTokens ?? session?.accumulated_output_tokens ?? undefined
            }
            droppedFiles={droppedFiles}
            onFilesProcessed={() => setDroppedFiles([])} // Clear dropped files after processing
            messages={messages}
            disableAnimation={disableAnimation}
            sessionCosts={sessionCosts}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            recipe={recipe}
            recipeAccepted={!hasNotAcceptedRecipe}
            initialPrompt={initialPrompt}
            toolCount={toolCount || 0}
            autoSubmit={false}
            append={append}
            {...customChatInputProps}
          />
        </div>
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

      {recipe?.parameters && recipe.parameters.length > 0 && !session?.user_recipe_values && (
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
