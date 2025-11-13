import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
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
import { getThinkingMessage } from '../types/message';
import ParameterInputModal from './ParameterInputModal';
import { substituteParameters } from '../utils/providerUtils';
import CreateRecipeFromSessionModal from './recipes/CreateRecipeFromSessionModal';
import { toastSuccess } from '../toasts';
import { Recipe } from '../recipe';

// Context for sharing current model info
const CurrentModelContext = createContext<{ model: string; mode: string } | null>(null);
export const useCurrentModelInfo = () => useContext(CurrentModelContext);

interface BaseChatProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  contentClassName?: string;
  disableSearch?: boolean;
  showPopularTopics?: boolean;
  suppressEmptyState: boolean;
  sessionId: string;
  initialMessage?: string;
}

function BaseChatContent({
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
  const [hasNotAcceptedRecipe, setHasNotAcceptedRecipe] = useState<boolean>();
  const [hasRecipeSecurityWarnings, setHasRecipeSecurityWarnings] = useState(false);

  const isMobile = useIsMobile();
  const { state: sidebarState } = useSidebar();
  const setView = useNavigation();

  const contentClassName = cn('pr-1 pb-10', (isMobile || sidebarState === 'collapsed') && 'pt-11');

  // Use shared file drop
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();

  const onStreamFinish = useCallback(() => {}, []);

  const [sessionLoaded, setSessionLoaded] = useState(false);
  const [hasSubmittedInitialMessage, setHasSubmittedInitialMessage] = useState(false);
  const [isCreateRecipeModalOpen, setIsCreateRecipeModalOpen] = useState(false);

  useEffect(() => {
    setSessionLoaded(false);
    setHasSubmittedInitialMessage(false);
  }, [sessionId]);

  const handleSessionLoaded = useCallback(() => {
    setSessionLoaded(true);
  }, []);

  const {
    session,
    messages,
    chatState,
    handleSubmit,
    stopStreaming,
    sessionLoadError,
    setRecipeUserParams,
    tokenState,
    notifications,
    appendMessage,
  } = useChatStream({
    sessionId,
    onStreamFinish,
    onSessionLoaded: handleSessionLoaded,
  });

  // Handle auto-submission when session is loaded and we have an initial message
  useEffect(() => {
    if (sessionLoaded && initialMessage && !hasSubmittedInitialMessage) {
      setHasSubmittedInitialMessage(true);
      handleSubmit(initialMessage);
    }
  }, [sessionLoaded, initialMessage, hasSubmittedInitialMessage, handleSubmit]);

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

  useEffect(() => {
    const handleMakeAgent = () => {
      setIsCreateRecipeModalOpen(true);
    };

    window.addEventListener('make-agent-from-chat', handleMakeAgent);
    return () => window.removeEventListener('make-agent-from-chat', handleMakeAgent);
  }, []);

  const handleRecipeCreated = (recipe: Recipe) => {
    toastSuccess({
      title: 'Recipe created successfully!',
      msg: `"${recipe.title}" has been saved and is ready to use.`,
    });
  };

  const toolCallNotifications = useMemo(() => {
    return notifications.reduce((map, notification) => {
      const key = notification.request_id;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(notification);
      return map;
    }, new Map());
  }, [notifications]);

  const renderProgressiveMessageList = (chat: ChatType) => (
    <>
      <ProgressiveMessageList
        messages={messages}
        chat={chat}
        toolCallNotifications={toolCallNotifications}
        appendMessage={appendMessage}
        isUserMessage={(m: Message) => {
          const hasToolConfirmation = m.content.some(
            (content) => content.type === 'toolConfirmationRequest'
          );
          return m.role === 'user' && !hasToolConfirmation;
        }}
        isStreamingMessage={chatState !== ChatState.Idle}
        onRenderingComplete={handleRenderingComplete}
      />
    </>
  );

  const showPopularTopics =
    messages.length === 0 && !initialMessage && chatState === ChatState.Idle;

  const chat: ChatType = {
    messageHistoryIndex: 0,
    messages,
    recipe,
    sessionId,
    name: session?.name || 'No Session',
  };

  // Only use initialMessage for the prompt if it hasn't been submitted yet
  // If we have a recipe prompt and user recipe values, substitute parameters
  let recipePrompt = '';
  if (messages.length === 0 && recipe?.prompt) {
    recipePrompt = session?.user_recipe_values
      ? substituteParameters(recipe.prompt, session.user_recipe_values)
      : recipe.prompt;
  }

  const initialPrompt =
    (initialMessage && !hasSubmittedInitialMessage ? initialMessage : '') || recipePrompt;

  if (sessionLoadError) {
    return (
      <div className="h-full flex flex-col min-h-0">
        <MainPanelLayout
          backgroundColor={'bg-background-muted'}
          removeTopPadding={true}
          {...customMainLayoutProps}
        >
          {renderHeader && renderHeader()}
          <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
            <div className="flex-1 bg-background-default rounded-b-2xl flex items-center justify-center">
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
            </div>
          </div>
        </MainPanelLayout>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col min-h-0">
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

          {chatState !== ChatState.Idle && (
            <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
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
            {...customChatInputProps}
          />
        </div>
      </MainPanelLayout>

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

      <CreateRecipeFromSessionModal
        isOpen={isCreateRecipeModalOpen}
        onClose={() => setIsCreateRecipeModalOpen(false)}
        sessionId={chat.sessionId}
        onRecipeCreated={handleRecipeCreated}
      />
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return <BaseChatContent {...props} />;
}
