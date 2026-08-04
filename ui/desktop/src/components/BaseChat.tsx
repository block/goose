import { AppEvents } from '../constants/events';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { defineMessages, useIntl } from '../i18n';
import { useLocation, useNavigate } from 'react-router-dom';
import { SearchView } from './conversation/SearchView';
import LoadingGoose from './LoadingGoose';
import ProgressiveMessageList from './ProgressiveMessageList';
import { MainPanelLayout } from './Layout/MainPanelLayout';
import ChatInput from './ChatInput';
import { ChatInputCard } from './ChatInputCard';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { useFileDrop } from '../hooks/useFileDrop';
import { ChatState } from '../types/chatState';
import { ChatType } from '../types/chat';
import { useIsMobile } from '../hooks/use-mobile';
import { useNavigationContextSafe } from './Layout/NavigationContext';
import { cn } from '../utils';
import { useChatSession } from '../hooks/useChatSession';
import { acpDeleteSession, acpUpdateWorkingDir } from '../acp/sessions';
import { useNavigation } from '../hooks/useNavigation';
import { RecipeHeader } from './RecipeHeader';
import { RecipeWarningModal } from './ui/RecipeWarningModal';
import { scanRecipe } from '../recipe';
import type { Recipe } from '../recipe';
import RecipeActivities from './recipes/RecipeActivities';
import {
  getTextAndImageContent,
  type ImageData,
  type Message,
  type UserInput,
} from '../types/message';
import { substituteParameters } from '../utils/parameterSubstitution';
import { useAutoSubmit } from '../hooks/useAutoSubmit';
import { Goose } from './icons';
import EnvironmentBadge from './GooseSidebar/EnvironmentBadge';
import SessionActionsHeader from './SessionActionsHeader';
import { isAcpRecovering, subscribeToAcpRecovery } from '../acp/acpConnection';

const i18n = defineMessages({
  failedToLoadSession: {
    id: 'baseChat.failedToLoadSession',
    defaultMessage: 'Failed to Load Session',
  },
  goHome: {
    id: 'baseChat.goHome',
    defaultMessage: 'Go home',
  },
  reconnecting: {
    id: 'baseChat.reconnecting',
    defaultMessage: 'Connection lost. Reconnecting…',
  },
});

interface BaseChatProps {
  setChat: (chat: ChatType) => void;
  onMessageSubmit?: (message: string) => void;
  renderHeader?: () => React.ReactNode;
  customChatInputProps?: Record<string, unknown>;
  customMainLayoutProps?: Record<string, unknown>;
  contentClassName?: string;
  disableSearch?: boolean;
  suppressEmptyState: boolean;
  sessionId: string;
  isActiveSession: boolean;
  initialMessage?: UserInput;
  noAutoSubmit?: boolean;
}

export default function BaseChat({
  setChat,
  renderHeader,
  customChatInputProps = {},
  customMainLayoutProps = {},
  sessionId,
  initialMessage,
  noAutoSubmit,
  isActiveSession,
}: BaseChatProps) {
  const intl = useIntl();
  const location = useLocation();
  const navigate = useNavigate();
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const chatInputRef = useRef<HTMLTextAreaElement>(null);
  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [hasNotAcceptedRecipe, setHasNotAcceptedRecipe] = useState<boolean>();
  const [hasRecipeSecurityWarnings, setHasRecipeSecurityWarnings] = useState(false);
  const [acpRecovering, setAcpRecovering] = useState(isAcpRecovering);
  const isMobile = useIsMobile();
  const navContext = useNavigationContextSafe();
  const setView = useNavigation();
  const isNavCollapsed = !navContext?.isNavExpanded;
  // Top padding clears the session title / watermark; bottom padding is handled by
  // the composer-height spacer so messages can scroll under the floating input.
  const contentClassName = cn('pr-1 pt-12', (isMobile || isNavCollapsed) && 'pt-16');
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();
  const onStreamFinish = useCallback(() => {}, []);
  const composerRef = useRef<HTMLDivElement>(null);
  const [composerHeight, setComposerHeight] = useState(160);

  useEffect(() => subscribeToAcpRecovery(setAcpRecovering), []);

  useEffect(() => {
    const el = composerRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;

    const observer = new ResizeObserver(([entry]) => {
      setComposerHeight(entry.contentRect.height);
    });
    observer.observe(el);
    setComposerHeight(el.getBoundingClientRect().height);
    return () => observer.disconnect();
  }, []);

  const {
    session,
    messages,
    chatState,
    progressMessage,
    updateSession,
    handleSubmit,
    onSteerQueuedMessage,
    submitElicitationResponse,
    stopStreaming,
    sessionLoadError,
    tokenState,
    notifications: toolCallNotifications,
    pauseQueueOnStop,
    queueProcessingBlocked,
    onMessageUpdate,
  } = useChatSession({
    sessionId,
    onStreamFinish,
  });

  const handleWorkingDirChange = useCallback(
    async (newDir: string) => {
      if (!session) {
        throw new Error('Cannot update working directory before ACP session is loaded');
      }
      await acpUpdateWorkingDir(session.id, newDir);
      updateSession((currentSession) => ({ ...currentSession, working_dir: newDir }));
    },
    [session, updateSession]
  );

  const recipe = session?.recipe as Recipe | null | undefined;

  const resolvedInitialMessage = useMemo((): UserInput | undefined => {
    if (!initialMessage) return undefined;
    if (recipe?.prompt && session?.user_recipe_values) {
      return {
        ...initialMessage,
        msg: substituteParameters(initialMessage.msg, session.user_recipe_values),
      };
    }
    return initialMessage;
  }, [initialMessage, recipe?.prompt, session?.user_recipe_values]);

  // noAutoSubmit only suppresses auto-submitting the initial prompt of a fresh session
  // (goose://new-session?prompt=...). Once the conversation has messages, later flows
  // such as forks or resumes should auto-submit normally.
  const suppressInitialAutoSubmit = noAutoSubmit && messages.length === 0;
  const canAutoSubmit =
    !acpRecovering &&
    !suppressInitialAutoSubmit &&
    (session?.session_type === 'scheduled' || !recipe || hasNotAcceptedRecipe === false);

  useAutoSubmit({
    sessionId,
    session,
    messages,
    chatState,
    initialMessage: resolvedInitialMessage,
    canAutoSubmit,
    handleSubmit,
  });

  useEffect(() => {
    let streamState: 'idle' | 'loading' | 'streaming' | 'error' = 'idle';
    if (chatState === ChatState.LoadingConversation) {
      streamState = 'loading';
    } else if (
      chatState === ChatState.Streaming ||
      chatState === ChatState.Thinking ||
      chatState === ChatState.Compacting
    ) {
      streamState = 'streaming';
    } else if (sessionLoadError) {
      streamState = 'error';
    }

    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_STATUS_UPDATE, {
        detail: {
          sessionId,
          streamState,
          messageCount: messages.length,
        },
      })
    );
  }, [sessionId, chatState, messages.length, sessionLoadError]);

  // Generate command history from user messages (most recent first)
  const commandHistory = useMemo(() => {
    return messages
      .reduce<string[]>((history, message) => {
        if (message.role === 'user') {
          const text = getTextAndImageContent(message).textContent.trim();
          if (text) {
            history.push(text);
          }
        }
        return history;
      }, [])
      .reverse();
  }, [messages]);

  const chatInputSubmit = (input: UserInput) => {
    if (recipe && input.msg.trim()) {
      setHasStartedUsingRecipe(true);
    }
    handleSubmit(input);
  };

  const sessionModel = session?.model_config?.model_name ?? null;
  const sessionProvider = session?.provider_name ?? null;
  const sessionLoaded = session !== undefined;
  const latestInference = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      const message = messages[i];
      if (
        message.role === 'assistant' &&
        message.metadata.userVisible &&
        message.metadata.inference
      ) {
        return message.metadata.inference;
      }
    }
    return null;
  }, [messages]);

  useEffect(() => {
    if (!recipe || !isActiveSession || session?.session_type === 'scheduled') return;

    (async () => {
      const accepted = await window.electron.hasAcceptedRecipeBefore(recipe);
      setHasNotAcceptedRecipe(!accepted);

      if (!accepted) {
        const scanResult = await scanRecipe(recipe);
        setHasRecipeSecurityWarnings(scanResult.has_security_warnings);
      }
    })();
  }, [recipe, isActiveSession, session?.session_type]);

  const handleRecipeAccept = async (accept: boolean) => {
    if (recipe && accept) {
      await window.electron.recordRecipeHash(recipe);
      setHasNotAcceptedRecipe(false);
      return;
    }

    if (sessionId) {
      try {
        await acpDeleteSession(sessionId);
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_DELETED, { detail: { sessionId } }));
      } catch (error) {
        console.error('Failed to delete declined recipe session:', error);
      }
    }
    setView('chat');
  };

  const [showScrollToBottom, setShowScrollToBottom] = useState(false);
  const [scrollButtonMounted, setScrollButtonMounted] = useState(false);
  const [scrollButtonLeaving, setScrollButtonLeaving] = useState(false);
  const handleScrollChange = useCallback((isAtBottom: boolean) => {
    setShowScrollToBottom(!isAtBottom);
  }, []);

  // Keep the scroll-to-bottom control mounted through a fade-out so hide
  // isn't an instant pop (fade-in alone looked jarring).
  useEffect(() => {
    if (showScrollToBottom) {
      setScrollButtonMounted(true);
      setScrollButtonLeaving(false);
      return;
    }
    if (!scrollButtonMounted) return;

    const prefersReducedMotion =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (prefersReducedMotion) {
      setScrollButtonMounted(false);
      setScrollButtonLeaving(false);
      return;
    }

    setScrollButtonLeaving(true);
  }, [showScrollToBottom, scrollButtonMounted]);

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

  // Listen for global scroll-to-bottom requests (e.g., from MCP App message actions)
  useEffect(() => {
    const handleGlobalScrollRequest = () => {
      // Add a small delay to ensure content has been rendered
      setTimeout(() => {
        if (scrollRef.current?.scrollToBottom) {
          scrollRef.current.scrollToBottom();
        }
      }, 200);
    };

    window.addEventListener(AppEvents.SCROLL_CHAT_TO_BOTTOM, handleGlobalScrollRequest);
    return () =>
      window.removeEventListener(AppEvents.SCROLL_CHAT_TO_BOTTOM, handleGlobalScrollRequest);
  }, []);

  useEffect(() => {
    if (
      isActiveSession &&
      sessionId &&
      chatInputRef.current &&
      chatState !== ChatState.LoadingConversation
    ) {
      const timeoutId = setTimeout(() => {
        chatInputRef.current?.focus();
      }, 100);
      return () => clearTimeout(timeoutId);
    }
    return undefined;
  }, [isActiveSession, sessionId, chatState]);

  useEffect(() => {
    const handleSessionForked = (event: Event) => {
      const customEvent = event as CustomEvent<{
        newSessionId: string;
        shouldStartAgent: boolean;
        editedMessage: string;
        editedImages: ImageData[];
      }>;
      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      const { newSessionId, shouldStartAgent, editedMessage, editedImages } = customEvent.detail;

      const params = new URLSearchParams();
      params.set('resumeSessionId', newSessionId);
      if (shouldStartAgent) {
        params.set('shouldStartAgent', 'true');
      }

      navigate(`/pair?${params.toString()}`, {
        state: {
          disableAnimation: true,
          initialMessage: { msg: editedMessage, images: editedImages },
        },
      });
    };

    window.addEventListener(AppEvents.SESSION_FORKED, handleSessionForked);

    return () => {
      window.removeEventListener(AppEvents.SESSION_FORKED, handleSessionForked);
    };
  }, [location.pathname, navigate]);

  const lastSetNameRef = useRef<string>('');

  useEffect(() => {
    const currentSessionName = session?.name;
    if (currentSessionName && currentSessionName !== lastSetNameRef.current) {
      lastSetNameRef.current = currentSessionName;
      setChat({
        messages,
        recipe,
        sessionId,
        name: currentSessionName,
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.name, setChat]);

  // If we have a recipe prompt and user recipe values, substitute parameters
  let recipePrompt = '';
  if (messages.length === 0 && recipe?.prompt) {
    recipePrompt = session?.user_recipe_values
      ? substituteParameters(recipe.prompt, session.user_recipe_values)
      : recipe.prompt;
  }

  const initialPrompt =
    noAutoSubmit && messages.length === 0 && resolvedInitialMessage?.msg
      ? resolvedInitialMessage.msg
      : recipePrompt;

  if (sessionLoadError) {
    return (
      <div className="h-full flex flex-col min-h-0">
        <MainPanelLayout
          backgroundColor={'bg-background-primary'}
          removeTopPadding={true}
          {...customMainLayoutProps}
        >
          {renderHeader && renderHeader()}
          <div className="flex flex-col flex-1 min-h-0 relative">
            <div className="flex-1 flex items-center justify-center">
              <div className="flex flex-col items-center justify-center p-8">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg mb-4 max-w-md">
                  <h3 className="font-semibold mb-2">
                    {intl.formatMessage(i18n.failedToLoadSession)}
                  </h3>
                  <p className="text-sm">{sessionLoadError}</p>
                </div>
                <button
                  onClick={() => {
                    setView('chat');
                  }}
                  className="px-4 py-2 text-center cursor-pointer text-text-primary border border-border-primary hover:bg-background-secondary rounded-lg transition-all duration-150"
                >
                  {intl.formatMessage(i18n.goHome)}
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
        backgroundColor={'bg-background-primary'}
        removeTopPadding={true}
        {...customMainLayoutProps}
      >
        {/* Custom header */}
        {renderHeader && renderHeader()}

        {/* Chat canvas: scroll fills the pane; composer floats over the bottom. */}
        <div className="relative flex flex-1 min-h-0 flex-col">
          {/* Goose watermark - top right */}
          <div className="absolute top-[14px] right-4 z-[60] flex flex-row items-center gap-1">
            <a
              href="https://goose-docs.ai"
              target="_blank"
              rel="noopener noreferrer"
              className="no-drag flex flex-row items-center gap-1 hover:opacity-80 transition-opacity"
            >
              <Goose className="size-5 goose-icon-animation" />
              <span className="text-sm leading-none text-text-secondary -translate-y-px">
                goose
              </span>
            </a>
            <EnvironmentBadge className="translate-y-px" />
          </div>

          <SessionActionsHeader session={session} onSessionChange={updateSession} />

          <ScrollArea
            ref={scrollRef}
            className={cn('relative z-0 min-h-0 flex-1 basis-0', contentClassName)}
            autoScroll
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            data-drop-zone="true"
            onScrollChange={handleScrollChange}
            paddingX={6}
            paddingY={0}
          >
            {recipe?.title && (
              <div className="sticky top-0 z-10 bg-background-primary px-0 -mx-6 mb-6 pt-6">
                <RecipeHeader title={recipe.title} />
              </div>
            )}

            {recipe && (
              <div className={hasStartedUsingRecipe ? 'mb-6' : ''}>
                <RecipeActivities
                  append={(text: string) => handleSubmit({ msg: text, images: [] })}
                  activities={Array.isArray(recipe.activities) ? recipe.activities : null}
                  title={recipe.title}
                  parameterValues={session?.user_recipe_values || {}}
                />
              </div>
            )}

            {messages.length > 0 || recipe ? (
              <SearchView>
                <ProgressiveMessageList
                  messages={messages}
                  chat={{ sessionId }}
                  toolCallNotifications={toolCallNotifications}
                  append={(text: string) => handleSubmit({ msg: text, images: [] })}
                  isUserMessage={(m: Message) => m.role === 'user'}
                  isStreamingMessage={chatState !== ChatState.Idle}
                  onRenderingComplete={handleRenderingComplete}
                  onMessageUpdate={onMessageUpdate}
                  submitElicitationResponse={submitElicitationResponse}
                />
              </SearchView>
            ) : null}

            {/* Clears the floating composer so the last message can sit just above it. */}
            <div aria-hidden className="pointer-events-none" style={{ height: composerHeight }} />
          </ScrollArea>


          <div className="pointer-events-none absolute inset-x-0 bottom-0 z-20 flex flex-col">
            <div ref={composerRef} className="relative bg-background-primary px-4 pb-3">
              {scrollButtonMounted && (
                <button
                  type="button"
                  onClick={() => {
                    if (scrollButtonLeaving) return;
                    scrollRef.current?.scrollToBottom();
                  }}
                  onAnimationEnd={(event) => {
                    if (event.animationName !== 'fadeout' || !scrollButtonLeaving) return;
                    setScrollButtonMounted(false);
                    setScrollButtonLeaving(false);
                  }}
                  className={cn(
                    'absolute left-1/2 top-0 z-30 -translate-x-1/2 -translate-y-1/2 p-1.5 rounded-full bg-background-secondary border border-border-primary shadow-lg hover:bg-background-tertiary transition-colors duration-200',
                    scrollButtonLeaving
                      ? 'pointer-events-none animate-[fadeout_200ms_ease-out_forwards]'
                      : 'pointer-events-auto animate-[fadein_200ms_ease-out]'
                  )}
                  title={intl.formatMessage({
                    id: 'baseChat.scrollToBottom',
                    defaultMessage: 'Scroll to bottom',
                  })}
                  aria-label={intl.formatMessage({
                    id: 'baseChat.scrollToBottom',
                    defaultMessage: 'Scroll to bottom',
                  })}
                  aria-hidden={scrollButtonLeaving}
                  tabIndex={scrollButtonLeaving ? -1 : 0}
                >
                  <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M12 5v14M19 12l-7 7-7-7" />
                  </svg>
                </button>
              )}

              {chatState !== ChatState.Idle && (
                <div className="relative z-10 mb-2">
                  <LoadingGoose chatState={chatState} message={progressMessage} />
                </div>
              )}

              {acpRecovering && (
                <div role="status" className="relative z-10 mb-2 text-sm text-text-secondary">
                  {intl.formatMessage(i18n.reconnecting)}
                </div>
              )}

              <ChatInputCard
                className={cn(
                  'pointer-events-auto relative z-10',
                  !disableAnimation && 'animate-[fadein_400ms_ease-in_forwards]'
                )}
              >
                <ChatInput
                  inputRef={chatInputRef}
                  sessionId={sessionId}
                  handleSubmit={chatInputSubmit}
                  chatState={chatState}
                  onStop={stopStreaming}
                  onSteerQueuedMessage={onSteerQueuedMessage}
                  pauseQueueOnStop={pauseQueueOnStop}
                  queueProcessingBlocked={queueProcessingBlocked || acpRecovering}
                  commandHistory={commandHistory}
                  initialValue={initialPrompt}
                  setView={setView}
                  totalTokens={tokenState?.totalTokens ?? session?.usage?.total_tokens ?? undefined}
                  accumulatedInputTokens={
                    tokenState?.accumulatedInputTokens ??
                    session?.accumulated_usage?.input_tokens ??
                    undefined
                  }
                  accumulatedOutputTokens={
                    tokenState?.accumulatedOutputTokens ??
                    session?.accumulated_usage?.output_tokens ??
                    undefined
                  }
                  accumulatedCost={
                    tokenState?.accumulatedCost ?? session?.accumulated_cost ?? undefined
                  }
                  droppedFiles={droppedFiles}
                  onFilesProcessed={() => setDroppedFiles([])}
                  messages={messages}
                  disableAnimation={disableAnimation}
                  recipe={recipe}
                  recipeAccepted={!hasNotAcceptedRecipe}
                  initialPrompt={initialPrompt}
                  sessionModel={sessionModel}
                  sessionProvider={sessionProvider}
                  sessionLoaded={sessionLoaded}
                  workingDir={session?.working_dir}
                  onWorkingDirChange={handleWorkingDirChange}
                  latestInference={latestInference}
                  {...customChatInputProps}
                />
              </ChatInputCard>
            </div>
          </div>
        </div>
      </MainPanelLayout>

      {recipe && isActiveSession && session?.session_type !== 'scheduled' && (
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
    </div>
  );
}
