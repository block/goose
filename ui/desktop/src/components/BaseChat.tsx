import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom';
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
import { getThinkingMessage, getTextContent } from '../types/message';
import ParameterInputModal from './ParameterInputModal';
import { substituteParameters } from '../utils/providerUtils';
import CreateRecipeFromSessionModal from './recipes/CreateRecipeFromSessionModal';
import { toastSuccess } from '../toasts';
import { Recipe } from '../recipe';
import { EditConversationBanner } from './EditConversationBanner';
import { ContextUsageUpdateModal } from './ContextUsageUpdateModal';
import { getApiUrl } from '../config';

// Context for sharing current model info
const CurrentModelContext = createContext<{ model: string; mode: string } | null>(null);
export const useCurrentModelInfo = () => useContext(CurrentModelContext);

interface BaseChatProps {
  setChat: (chat: ChatType) => void;
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
  renderHeader,
  customChatInputProps = {},
  customMainLayoutProps = {},
  sessionId,
  initialMessage,
}: BaseChatProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
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

  const [isCreateRecipeModalOpen, setIsCreateRecipeModalOpen] = useState(false);
  const hasAutoSubmittedRef = useRef(false);

  // Conversation editing state
  const [isEditingConversation, setIsEditingConversation] = useState(false);
  const [messageCheckboxStates, setMessageCheckboxStates] = useState<Map<string, boolean>>(new Map());
  const [isContextUsageModalOpen, setIsContextUsageModalOpen] = useState(false);
  const [contextUsageData, setContextUsageData] = useState<{ before: number; after: number } | null>(null);

  // Reset auto-submit flag when session changes
  useEffect(() => {
    hasAutoSubmittedRef.current = false;
  }, [sessionId]);

  const {
    session,
    messages,
    chatState,
    handleSubmit,
    stopStreaming,
    sessionLoadError,
    setRecipeUserParams,
    tokenState,
    notifications: toolCallNotifications,
    onMessageUpdate,
    reloadSession,
  } = useChatStream({
    sessionId,
    onStreamFinish,
  });

  // Generate command history from user messages (most recent first)
  const commandHistory = useMemo(() => {
    return messages
      .reduce<string[]>((history, message) => {
        if (message.role === 'user') {
          const text = getTextContent(message).trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [messages]);

  useEffect(() => {
    if (!session || hasAutoSubmittedRef.current) {
      return;
    }

    const shouldStartAgent = searchParams.get('shouldStartAgent') === 'true';

    if (initialMessage) {
      // Submit the initial message (e.g., from fork)
      hasAutoSubmittedRef.current = true;
      handleSubmit(initialMessage);
    } else if (shouldStartAgent) {
      // Trigger agent to continue with existing conversation
      hasAutoSubmittedRef.current = true;
      handleSubmit('');
    }
  }, [session, initialMessage, searchParams, handleSubmit]);

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

  useEffect(() => {
    const handleSessionForked = (event: Event) => {
      const customEvent = event as CustomEvent<{
        newSessionId: string;
        shouldStartAgent?: boolean;
        editedMessage?: string;
      }>;
      const { newSessionId, shouldStartAgent, editedMessage } = customEvent.detail;

      const params = new URLSearchParams();
      params.set('resumeSessionId', newSessionId);
      if (shouldStartAgent) {
        params.set('shouldStartAgent', 'true');
      }

      navigate(`/pair?${params.toString()}`, {
        state: {
          disableAnimation: true,
          initialMessage: editedMessage,
        },
      });
    };

    window.addEventListener('session-forked', handleSessionForked);

    return () => {
      window.removeEventListener('session-forked', handleSessionForked);
    };
  }, [location.pathname, navigate]);

  const handleRecipeCreated = (recipe: Recipe) => {
    toastSuccess({
      title: 'Recipe created successfully!',
      msg: `"${recipe.title}" has been saved and is ready to use.`,
    });
  };

  // Calculate tokens for agent-visible messages
  const calculateAgentVisibleTokens = useCallback((messagesToCount: Message[]): number => {
    let totalTokens = 0;
    messagesToCount.forEach((message) => {
      // Only count messages that are visible to the agent
      if (message.metadata?.agentVisible !== false) {
        const textContent = getTextContent(message);
        if (textContent) {
          // Rough token estimation: ~4 characters per token
          totalTokens += Math.ceil(textContent.length / 4);
        }
      }
    });
    return totalTokens;
  }, []);

  // Handle checkbox change for conversation editing
  const handleCheckboxChange = useCallback(
    (messageId: string, checked: boolean) => {
      setMessageCheckboxStates((prev) => {
        const newMap = new Map(prev);
        newMap.set(messageId, checked);

        // If a user message is checked/unchecked, update all subsequent messages until next user message
        const messageIndex = messages.findIndex((msg) => msg.id === messageId);
        if (messageIndex !== -1 && messages[messageIndex].role === 'user') {
          // Find all messages after this user message until the next user message
          for (let i = messageIndex + 1; i < messages.length; i++) {
            const msg = messages[i];
            if (msg.role === 'user') {
              // Stop at next user message
              break;
            }
            // Update all assistant messages and tool calls to match the user message state
            if (msg.id) {
              newMap.set(msg.id, checked);
            }
          }
        }

        return newMap;
      });
    },
    [messages]
  );

  // Initialize checkbox states when entering edit mode
  useEffect(() => {
    if (isEditingConversation) {
      const initialState = new Map<string, boolean>();
      messages.forEach((msg) => {
        if (msg.id) {
          // Initialize with current agentVisible state (default to true if not set)
          initialState.set(msg.id, msg.metadata?.agentVisible !== false);
        }
      });
      setMessageCheckboxStates(initialState);
    }
  }, [isEditingConversation, messages]);

  // Handle saving conversation with updated metadata
  const handleSaveConversation = useCallback(async () => {
    try {
      // Calculate tokens before update
      const beforeTokens = calculateAgentVisibleTokens(messages);

      // Update messages with agentVisible=false for unchecked messages
      const updatedMessages = messages.map((msg) => {
        if (!msg.id) return msg;

        // Check if this message is unchecked (default to true if not in map)
        const isChecked = messageCheckboxStates.get(msg.id) ?? (msg.metadata?.agentVisible !== false);

        if (!isChecked) {
          // Set agentVisible=false for unchecked messages
          return {
            ...msg,
            metadata: {
              ...msg.metadata,
              agentVisible: false,
            },
          };
        }

        // Ensure agentVisible is true for checked messages
        return {
          ...msg,
          metadata: {
            ...msg.metadata,
            agentVisible: true,
          },
        };
      });

      // Calculate tokens after update
      const afterTokens = calculateAgentVisibleTokens(updatedMessages);

      // Persist to database via API endpoint
      const apiUrl = getApiUrl(`/sessions/${sessionId}/conversation`);
      const secretKey = await window.electron.getSecretKey();

      const response = await fetch(apiUrl, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify({
          conversation: updatedMessages,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text().catch(() => 'Unknown error');
        throw new Error(`Failed to update conversation: HTTP ${response.status} - ${errorText}`);
      }

      // Reload session to get updated messages
      await reloadSession();

      // Show modal with token usage update
      setContextUsageData({ before: beforeTokens, after: afterTokens });
      setIsContextUsageModalOpen(true);

      // Clear checkbox states and exit edit mode
      setMessageCheckboxStates(new Map());
      setIsEditingConversation(false);
    } catch (error) {
      console.error('Error in Save handler:', error);
      // Show error in console, user can see it there
    }
  }, [messages, messageCheckboxStates, sessionId, calculateAgentVisibleTokens, reloadSession]);

  const renderProgressiveMessageList = (chat: ChatType) => (
    <>
      <ProgressiveMessageList
        messages={messages}
        chat={chat}
        toolCallNotifications={toolCallNotifications}
        isUserMessage={(m: Message) => m.role === 'user'}
        isStreamingMessage={chatState !== ChatState.Idle}
        onRenderingComplete={handleRenderingComplete}
        onMessageUpdate={onMessageUpdate}
        isEditingConversation={isEditingConversation}
        messageCheckboxStates={messageCheckboxStates}
        onCheckboxChange={handleCheckboxChange}
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
    (initialMessage && !hasAutoSubmittedRef.current ? initialMessage : '') || recipePrompt;

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
        {isEditingConversation && (
          <div className="pt-12">
            <EditConversationBanner onSave={handleSaveConversation} />
          </div>
        )}
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
            recipe={recipe}
            recipeAccepted={!hasNotAcceptedRecipe}
            initialPrompt={initialPrompt}
            toolCount={toolCount || 0}
            isEditingConversation={isEditingConversation}
            onEditingConversationChange={setIsEditingConversation}
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

      {contextUsageData && (
        <ContextUsageUpdateModal
          isOpen={isContextUsageModalOpen}
          onClose={() => {
            setIsContextUsageModalOpen(false);
            setContextUsageData(null);
          }}
          beforeTokens={contextUsageData.before}
          afterTokens={contextUsageData.after}
        />
      )}
    </div>
  );
}

export default function BaseChat(props: BaseChatProps) {
  return <BaseChatContent {...props} />;
}
