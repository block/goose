import { useEffect, useRef, useState, useCallback } from 'react';
import { Message, startAgent, stopAgent } from '../../../api';
import { stripEmptyExtensions } from '../../../recipe';
import { useChatStream } from '../../../hooks/useChatStream';
import { ChatState } from '../../../types/chatState';
import { ScrollArea, ScrollAreaHandle } from '../../ui/scroll-area';
import ProgressiveMessageList from '../../ProgressiveMessageList';
import LoadingGoose from '../../LoadingGoose';
import { getInitialWorkingDir } from '../../../utils/workingDir';
import { UserInput } from '../../../types/message';
import { RefreshCw, AlertTriangle } from 'lucide-react';
import { Send } from '../../icons';
import Stop from '../../ui/Stop';
import { Button } from '../../ui/button';
import { RecipeBuilderTestProps } from './types';
import ParameterInputModal from '../../ParameterInputModal';
import { substituteParameters } from '../../../utils/providerUtils';

export default function RecipeBuilderTest({
  recipe,
  testRecipeSnapshot,
  onStart,
  onRestart,
  isOpen,
}: RecipeBuilderTestProps) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [inputValue, setInputValue] = useState('');
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const hasPopulatedPromptRef = useRef(false);

  // Detect if recipe has changed since test started
  const recipeChanged =
    testRecipeSnapshot !== null && JSON.stringify(recipe) !== JSON.stringify(testRecipeSnapshot);

  // Cleanup session when panel closes
  useEffect(() => {
    if (!isOpen && sessionId) {
      stopAgent({ body: { session_id: sessionId } }).catch((error) => {
        console.error('Failed to stop test session:', error);
      });
      setSessionId(null);
      setSessionError(null);
    }
  }, [isOpen, sessionId]);

  const handleStartTest = useCallback(async () => {
    setIsStarting(true);
    setSessionError(null);

    try {
      const workingDir = getInitialWorkingDir();
      const response = await startAgent({
        body: {
          working_dir: workingDir,
          recipe: stripEmptyExtensions(recipe),
        },
        throwOnError: true,
      });

      setSessionId(response.data.id);
      onStart();
    } catch (error) {
      console.error('Failed to start test session:', error);
      setSessionError(error instanceof Error ? error.message : 'Failed to start test');
    } finally {
      setIsStarting(false);
    }
  }, [recipe, onStart]);

  useEffect(() => {
    if (isOpen && !sessionId && !isStarting && !sessionError) {
      handleStartTest();
    }
  }, [isOpen, sessionId, isStarting, sessionError, handleStartTest]);

  const handleRestart = useCallback(async () => {
    if (sessionId) {
      try {
        await stopAgent({ body: { session_id: sessionId } });
      } catch (error) {
        console.error('Failed to stop previous test session:', error);
      }
    }

    setSessionId(null);
    setSessionError(null);
    setInputValue('');
    hasPopulatedPromptRef.current = false;

    setIsStarting(true);
    try {
      const workingDir = getInitialWorkingDir();
      const response = await startAgent({
        body: {
          working_dir: workingDir,
          recipe: stripEmptyExtensions(recipe),
        },
        throwOnError: true,
      });

      setSessionId(response.data.id);
      onRestart();
    } catch (error) {
      console.error('Failed to restart test session:', error);
      setSessionError(error instanceof Error ? error.message : 'Failed to restart test');
    } finally {
      setIsStarting(false);
    }
  }, [recipe, sessionId, onRestart]);

  const onStreamFinish = useCallback(() => {}, []);

  const {
    messages,
    chatState,
    session,
    handleSubmit: submitToChat,
    sessionLoadError,
    stopStreaming,
    setRecipeUserParams,
  } = useChatStream({
    sessionId: sessionId || '',
    onStreamFinish,
  });

  const hasParameters = recipe.parameters && recipe.parameters.length > 0;
  const needsParameterInput = hasParameters && !session?.user_recipe_values;

  const handleParameterSubmit = useCallback(
    async (values: Record<string, string>) => {
      await setRecipeUserParams(values);
      if (recipe.prompt) {
        setInputValue(substituteParameters(recipe.prompt, values));
        hasPopulatedPromptRef.current = true;
      }
    },
    [recipe.prompt, setRecipeUserParams]
  );

  useEffect(() => {
    if (!sessionId || !recipe.prompt) return;
    if (hasPopulatedPromptRef.current) return;
    if (hasParameters && !session?.user_recipe_values) return;

    const prompt = session?.user_recipe_values
      ? substituteParameters(recipe.prompt, session.user_recipe_values)
      : recipe.prompt;
    setInputValue(prompt);
    hasPopulatedPromptRef.current = true;
  }, [sessionId, recipe.prompt, hasParameters, session?.user_recipe_values]);

  const isUserMessage = useCallback((message: Message) => message.role === 'user', []);

  useEffect(() => {
    if (sessionId && chatState === ChatState.Idle && inputRef.current) {
      inputRef.current.focus();
    }
  }, [sessionId, chatState]);

  const isStreaming =
    chatState === ChatState.Streaming ||
    chatState === ChatState.Thinking ||
    chatState === ChatState.Compacting;

  const handleSubmit = useCallback(() => {
    if (!inputValue.trim() || isStreaming) return;

    const input: UserInput = { msg: inputValue, images: [] };
    submitToChat(input);
    setInputValue('');
  }, [inputValue, isStreaming, submitToChat]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit]
  );

  // Show error state
  if (sessionError || sessionLoadError) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg max-w-md">
          <h3 className="font-semibold mb-2">Failed to Start Test</h3>
          <p className="text-sm mb-4">{sessionError || sessionLoadError}</p>
          <Button onClick={handleStartTest} variant="outline" size="sm">
            Try Again
          </Button>
        </div>
      </div>
    );
  }

  if (needsParameterInput) {
    return (
      <ParameterInputModal
        parameters={recipe.parameters!}
        onSubmit={handleParameterSubmit}
        onClose={() => {}}
      />
    );
  }

  return (
    <>
      {recipeChanged && (
        <div className="flex items-center justify-between gap-2 px-4 py-2 bg-yellow-100 dark:bg-yellow-900/30 border-b border-yellow-300 dark:border-yellow-700">
          <div className="flex items-center gap-2 text-yellow-800 dark:text-yellow-200 text-sm">
            <AlertTriangle className="w-4 h-4" />
            <span>Recipe has changed</span>
          </div>
          <Button onClick={handleRestart} variant="outline" size="sm" className="gap-1">
            <RefreshCw className="w-3 h-3" />
            Restart
          </Button>
        </div>
      )}

      <div className="flex flex-col flex-1 mb-0.5 min-h-0 relative">
        <ScrollArea
          ref={scrollRef}
          className="flex-1 bg-background-default min-h-0 relative"
          autoScroll
          paddingX={4}
          paddingY={0}
        >
          {messages.length === 0 ? (
            <div className="text-center text-textSubtle py-8">
              <p className="text-sm">Send a message to test your recipe.</p>
            </div>
          ) : (
            <>
              <ProgressiveMessageList
                messages={messages}
                chat={{ sessionId: sessionId! }}
                isUserMessage={isUserMessage}
              />
              <div className="block h-12" />
            </>
          )}
        </ScrollArea>

        {(isStarting || isStreaming) && (
          <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
            <LoadingGoose chatState={isStarting ? ChatState.LoadingConversation : chatState} />
          </div>
        )}
      </div>

      <div className="relative z-10 p-4 bg-background-default border-t border-borderSubtle">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSubmit();
          }}
          className="relative"
        >
          <div className="relative">
            <textarea
              ref={inputRef}
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Test your recipe..."
              className="w-full outline-none border-none focus:ring-0 bg-transparent px-3 pt-3 pb-1.5 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
              rows={2}
              disabled={isStarting || isStreaming}
              style={{ paddingRight: '120px' }}
            />
            <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
              {isStreaming ? (
                <Button
                  type="button"
                  onClick={stopStreaming}
                  size="sm"
                  shape="round"
                  variant="outline"
                  className="bg-slate-600 text-white hover:bg-slate-700 border-slate-600 rounded-full px-6 py-2"
                >
                  <Stop />
                </Button>
              ) : (
                <Button
                  type="submit"
                  size="sm"
                  shape="round"
                  variant="outline"
                  disabled={isStarting || !inputValue.trim()}
                  className={`rounded-full px-10 py-2 flex items-center gap-2 ${
                    isStarting || !inputValue.trim()
                      ? 'bg-slate-600 text-white cursor-not-allowed opacity-50 border-slate-600'
                      : 'bg-slate-600 text-white hover:bg-slate-700 border-slate-600 hover:cursor-pointer'
                  }`}
                >
                  <Send />
                  <span className="text-sm">Send</span>
                </Button>
              )}
            </div>
          </div>
        </form>
      </div>
    </>
  );
}
