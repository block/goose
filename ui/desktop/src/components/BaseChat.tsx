import { AppEvents } from '../constants/events';
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
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
import { useNavigationContextSafe } from './Layout/NavigationContext';
import { cn } from '../utils';
import { useChatStream } from '../hooks/useChatStream';
import { useNavigation } from '../hooks/useNavigation';
import { RecipeHeader } from './RecipeHeader';
import { RecipeWarningModal } from './ui/RecipeWarningModal';
import { scanRecipe } from '../recipe';
import { UserInput } from '../types/message';
import { useCostTracking } from '../hooks/useCostTracking';
import RecipeActivities from './recipes/RecipeActivities';
import { useToolCount } from './alerts/useToolCount';
import { getThinkingMessage, getTextAndImageContent } from '../types/message';
import ParameterInputModal from './ParameterInputModal';
import { substituteParameters } from '../utils/parameterSubstitution';
import { useModelAndProvider } from './ModelAndProviderContext';
import CreateRecipeFromSessionModal from './recipes/CreateRecipeFromSessionModal';
import { toastSuccess } from '../toasts';
import { Recipe } from '../recipe';
import { useAutoSubmit } from '../hooks/useAutoSubmit';
import { ForwardedToolCall, ForwardedToolResult } from '../hooks/useChatStream';
import BrowserPanel from './BrowserPanel';
import TurndownService from 'turndown';
import { Goose } from './icons';
import EnvironmentBadge from './GooseSidebar/EnvironmentBadge';

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
  isActiveSession: boolean;
  initialMessage?: UserInput;
}

export default function BaseChat({
  setChat,
  renderHeader,
  customChatInputProps = {},
  customMainLayoutProps = {},
  sessionId,
  initialMessage,
  isActiveSession,
}: BaseChatProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const chatInputRef = useRef<HTMLTextAreaElement>(null);
  const disableAnimation = location.state?.disableAnimation || false;
  const [hasStartedUsingRecipe, setHasStartedUsingRecipe] = React.useState(false);
  const [hasNotAcceptedRecipe, setHasNotAcceptedRecipe] = useState<boolean>();
  const [hasRecipeSecurityWarnings, setHasRecipeSecurityWarnings] = useState(false);
  const isMobile = useIsMobile();
  const navContext = useNavigationContextSafe();
  const setView = useNavigation();
  const isNavCollapsed = !navContext?.isNavExpanded;
  const contentClassName = cn('pr-1 pb-10 pt-10', (isMobile || isNavCollapsed) && 'pt-14');
  const { droppedFiles, setDroppedFiles, handleDrop, handleDragOver } = useFileDrop();
  const onStreamFinish = useCallback(() => {}, []);
  const [isCreateRecipeModalOpen, setIsCreateRecipeModalOpen] = useState(false);

  // Browser panel state
  const webviewRef = useRef<Electron.WebviewTag | null>(null);
  const [isBrowserOpen, setIsBrowserOpen] = useState(false);
  const [browserUrl, setBrowserUrl] = useState('');

  // JS that runs inside the webview to inspect the page structure
  const inspectJs = `(() => {
    const els = document.querySelectorAll('a, button, [role="button"], [onclick], input, select, textarea, [contenteditable="true"]');
    const items = [];
    let idx = 0;
    els.forEach(el => {
      if (el.offsetParent === null && el.type !== 'hidden') return;
      const tag = el.tagName.toLowerCase();
      const role = el.getAttribute('role') || '';
      const type = el.getAttribute('type') || '';
      const name = el.getAttribute('name') || '';
      const label = el.getAttribute('aria-label') || el.getAttribute('placeholder') || '';
      const text = (el.innerText || '').trim().substring(0, 80);
      const href = el.href || '';
      const value = el.value || '';

      let kind = '';
      if (tag === 'a') kind = 'link';
      else if (tag === 'button' || role === 'button') kind = 'button';
      else if (tag === 'select') kind = 'select';
      else if (tag === 'textarea') kind = 'textarea';
      else if (tag === 'input') kind = 'input[' + (type || 'text') + ']';
      else if (el.getAttribute('contenteditable')) kind = 'editable';
      else if (el.getAttribute('onclick')) kind = 'clickable';
      else kind = tag;

      el.setAttribute('data-goose-idx', String(idx));
      const desc = text || label || name || href.substring(0, 60) || '(no label)';
      items.push({ idx, kind, desc, value: value ? value.substring(0, 60) : undefined, href: kind === 'link' ? href : undefined });
      idx++;
    });
    const title = document.title;
    const url = location.href;
    const headings = Array.from(document.querySelectorAll('h1,h2,h3')).slice(0, 10).map(h => h.tagName.toLowerCase() + ': ' + (h.innerText || '').trim().substring(0, 80));
    return { title, url, headings, elements: items };
  })()`;

  // Resolve a target that is either a CSS selector or [index] to a JS expression
  const resolveTarget = (target: string) => {
    const idxMatch = target.match(/^\[(\d+)\]$/);
    if (idxMatch) {
      return `document.querySelector('[data-goose-idx="${idxMatch[1]}"]')`;
    }
    return `document.querySelector(${JSON.stringify(target)})`;
  };

  const formatInspectResult = (data: { title: string; url: string; headings: string[]; elements: { idx: number; kind: string; desc: string; value?: string; href?: string }[] }) => {
    const lines: string[] = [];
    lines.push(`Page: ${data.title}`);
    lines.push(`URL: ${data.url}`);
    if (data.headings.length > 0) {
      lines.push('', 'Headings:');
      data.headings.forEach(h => lines.push(`  ${h}`));
    }
    if (data.elements.length > 0) {
      lines.push('', 'Interactive elements:');
      data.elements.forEach(el => {
        let line = `  [${el.idx}] ${el.kind} "${el.desc}"`;
        if (el.value) line += ` value="${el.value}"`;
        if (el.href) line += ` → ${el.href}`;
        lines.push(line);
      });
    }
    return lines.join('\n');
  };

  const onForwardedToolCall = useCallback(async (tool: ForwardedToolCall): Promise<ForwardedToolResult> => {
    const { toolName: command, arguments: args } = tool;

    const text = (msg: string) => ({ content: [{ type: 'text' as const, text: msg }] });

    if (command === 'close') {
      setIsBrowserOpen(false);
      setBrowserUrl('');
      return text('Browser closed');
    }

    if (command === 'navigate') {
      const url = (args.url as string) || 'about:blank';
      if (!isBrowserOpen) {
        setBrowserUrl(url);
        setIsBrowserOpen(true);
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      const wv = webviewRef.current;
      if (wv) {
        await wv.loadURL(url).catch(() => {});
        setBrowserUrl(url);
        await new Promise((resolve) => setTimeout(resolve, 500));
        const pageData = await wv.executeJavaScript(inspectJs);
        return text(formatInspectResult(pageData));
      }
      return text(`Navigated to ${url}`);
    }

    const wv = webviewRef.current;
    if (!wv) {
      return { content: [{ type: 'text', text: 'Browser is not open' }], isError: true };
    }

    try {
      switch (command) {
        case 'inspect': {
          const pageData = await wv.executeJavaScript(inspectJs);
          return text(formatInspectResult(pageData));
        }
        case 'screenshot': {
          const image = await wv.capturePage();
          const dataUrl = image.toDataURL();
          const base64 = dataUrl.replace(/^data:image\/png;base64,/, '');
          return { content: [{ type: 'image', data: base64, mimeType: 'image/png' }] };
        }
        case 'click': {
          const target = (args.selector as string) || (args.target as string);
          const el = resolveTarget(target);
          await wv.executeJavaScript(`${el}?.click()`);
          await new Promise((resolve) => setTimeout(resolve, 300));
          return text(`Clicked ${target}`);
        }
        case 'type': {
          const target = (args.selector as string) || (args.target as string);
          const el = resolveTarget(target);
          const typeText = args.text as string;
          await wv.executeJavaScript(`(() => { const el = ${el}; if(el) { el.focus(); el.value = ${JSON.stringify(typeText)}; el.dispatchEvent(new Event('input', {bubbles:true})); el.dispatchEvent(new Event('change', {bubbles:true})); } })()`);
          return text(`Typed "${typeText}" into ${target}`);
        }
        case 'get': {
          const selector = (args.selector as string) || 'body';
          const format = (args.format as string) || 'text';
          if (format === 'html') {
            const html = await wv.executeJavaScript(`document.querySelector(${JSON.stringify(selector)})?.outerHTML || ''`);
            return text(html);
          } else if (format === 'markdown') {
            const html = await wv.executeJavaScript(`document.querySelector(${JSON.stringify(selector)})?.innerHTML || ''`);
            const turndown = new TurndownService({ headingStyle: 'atx', codeBlockStyle: 'fenced' });
            return text(turndown.turndown(html));
          } else {
            const pageText = await wv.executeJavaScript(`document.querySelector(${JSON.stringify(selector)})?.innerText || ''`);
            return text(pageText);
          }
        }
        case 'evaluate': {
          const evalResult = await wv.executeJavaScript(args.script as string);
          return text(typeof evalResult === 'string' ? evalResult : JSON.stringify(evalResult));
        }
        case 'scroll': {
          const dir = (args.direction as string) || 'down';
          const amount = (args.amount as number) || 0;
          let scrollJs: string;
          if (dir === 'top') scrollJs = 'window.scrollTo(0, 0)';
          else if (dir === 'bottom') scrollJs = 'window.scrollTo(0, document.body.scrollHeight)';
          else if (dir === 'up') scrollJs = `window.scrollBy(0, -${amount || 'window.innerHeight'})`;
          else scrollJs = `window.scrollBy(0, ${amount || 'window.innerHeight'})`;
          await wv.executeJavaScript(scrollJs);
          return text(`Scrolled ${dir}`);
        }
        default:
          return { content: [{ type: 'text', text: `Unknown command: ${command}` }], isError: true };
      }
    } catch (err) {
      return { content: [{ type: 'text', text: String(err) }], isError: true };
    }
  }, []);

  const {
    session,
    messages,
    chatState,
    setChatState,
    handleSubmit,
    submitElicitationResponse,
    stopStreaming,
    sessionLoadError,
    setRecipeUserParams,
    tokenState,
    notifications: toolCallNotifications,
    onMessageUpdate,
  } = useChatStream({
    sessionId,
    onStreamFinish,
    onForwardedToolCall,
  });

  const recipe = session?.recipe;

  useAutoSubmit({
    sessionId,
    session,
    messages,
    chatState,
    initialMessage,
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

  const { sessionCosts } = useCostTracking({
    sessionInputTokens: session?.accumulated_input_tokens || 0,
    sessionOutputTokens: session?.accumulated_output_tokens || 0,
    localInputTokens: 0,
    localOutputTokens: 0,
    session,
  });

  const { setProviderAndModel } = useModelAndProvider();

  useEffect(() => {
    if (session?.provider_name && session?.model_config?.model_name) {
      setProviderAndModel(session.provider_name, session.model_config.model_name);
    }
  }, [session?.provider_name, session?.model_config?.model_name, setProviderAndModel]);

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
      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      const { newSessionId, shouldStartAgent, editedMessage } = customEvent.detail;

      const params = new URLSearchParams();
      params.set('resumeSessionId', newSessionId);
      if (shouldStartAgent) {
        params.set('shouldStartAgent', 'true');
      }

      navigate(`/pair?${params.toString()}`, {
        state: {
          disableAnimation: true,
          initialMessage: editedMessage ? { msg: editedMessage, images: [] } : undefined,
        },
      });
    };

    window.addEventListener(AppEvents.SESSION_FORKED, handleSessionForked);

    return () => {
      window.removeEventListener(AppEvents.SESSION_FORKED, handleSessionForked);
    };
  }, [location.pathname, navigate]);

  const handleRecipeCreated = (recipe: Recipe) => {
    toastSuccess({
      title: 'Recipe created successfully!',
      msg: `"${recipe.title}" has been saved and is ready to use.`,
    });
  };

  const showPopularTopics =
    messages.length === 0 && !initialMessage && chatState === ChatState.Idle;

  const chat: ChatType = {
    messages,
    recipe,
    sessionId,
    name: session?.name || 'No Session',
  };

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

  const initialPrompt = recipePrompt;

  if (sessionLoadError) {
    return (
      <div className="h-full flex flex-col min-h-0">
        <MainPanelLayout
          backgroundColor={'bg-background-secondary'}
          removeTopPadding={true}
          {...customMainLayoutProps}
        >
          {renderHeader && renderHeader()}
          <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
            <div className="flex-1 bg-background-primary rounded-b-2xl flex items-center justify-center">
              <div className="flex flex-col items-center justify-center p-8">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg mb-4 max-w-md">
                  <h3 className="font-semibold mb-2">Failed to Load Session</h3>
                  <p className="text-sm">{sessionLoadError}</p>
                </div>
                <button
                  onClick={() => {
                    setView('chat');
                  }}
                  className="px-4 py-2 text-center cursor-pointer text-text-primary border border-border-primary hover:bg-background-secondary rounded-lg transition-all duration-150"
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
    <div className="h-full flex flex-row min-h-0">
    <div className={`h-full flex flex-col min-h-0 ${isBrowserOpen ? 'flex-1' : 'w-full'}`}>
      <MainPanelLayout
        backgroundColor={'bg-background-secondary'}
        removeTopPadding={true}
        {...customMainLayoutProps}
      >
        {/* Custom header */}
        {renderHeader && renderHeader()}

        {/* Chat container with sticky recipe header */}
        <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
          {/* Goose watermark - top right */}
          <div className="absolute top-3 right-4 z-[60] flex flex-row items-center gap-1">
            <a
              href="https://block.github.io/goose"
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

          <ScrollArea
            ref={scrollRef}
            className={`flex-1 bg-background-primary rounded-b-2xl min-h-0 relative ${contentClassName}`}
            autoScroll
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            data-drop-zone="true"
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
              <>
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

                <div className="block h-8" />
              </>
            ) : !recipe && showPopularTopics ? (
              <PopularChatTopics
                append={(text: string) => handleSubmit({ msg: text, images: [] })}
              />
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
            inputRef={chatInputRef}
            sessionId={sessionId}
            handleSubmit={chatInputSubmit}
            chatState={chatState}
            setChatState={setChatState}
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
          initialValues={
            (window.appConfig?.get('recipeParameters') as Record<string, string> | undefined) ||
            undefined
          }
        />
      )}

      <CreateRecipeFromSessionModal
        isOpen={isCreateRecipeModalOpen}
        onClose={() => setIsCreateRecipeModalOpen(false)}
        sessionId={chat.sessionId}
        onRecipeCreated={handleRecipeCreated}
      />
    </div>

    {isBrowserOpen && (
      <BrowserPanel
        webviewRef={webviewRef}
        url={browserUrl}
        onClose={() => {
          setIsBrowserOpen(false);
          setBrowserUrl('');
        }}
      />
    )}
    </div>
  );
}
