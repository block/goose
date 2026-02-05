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
import { Send, RefreshCw, AlertTriangle } from 'lucide-react';
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
      }
    },
    [recipe.prompt, setRecipeUserParams]
  );

  useEffect(() => {
    if (!sessionId || !recipe.prompt || inputValue) return;
    if (hasParameters && !session?.user_recipe_values) return;

    const prompt = session?.user_recipe_values
      ? substituteParameters(recipe.prompt, session.user_recipe_values)
      : recipe.prompt;
    setInputValue(prompt);
  }, [sessionId, recipe.prompt, hasParameters, session?.user_recipe_values, inputValue]);

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

  // Show loading state
  if (isStarting) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <LoadingGoose />
      </div>
    );
  }

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
    <div className="flex flex-col h-full">
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

      {/* Chat area */}
      <ScrollArea ref={scrollRef} className="flex-1 min-h-0 px-4" autoScroll>
        <div className="py-6">
          {messages.length === 0 ? (
            <div className="text-center text-textSubtle py-8">
              <p className="text-sm">Send a message to test your recipe.</p>
            </div>
          ) : (
            <ProgressiveMessageList
              messages={messages}
              chat={{ sessionId: sessionId! }}
              isUserMessage={isUserMessage}
            />
          )}
          {isStreaming && (
            <div className="flex justify-center py-4">
              <LoadingGoose chatState={chatState} />
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Input area */}
      <div className="p-4 border-t border-borderSubtle">
        <div className="flex gap-2">
          <textarea
            ref={inputRef}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Test your recipe..."
            className="flex-1 p-3 border border-borderSubtle rounded-lg bg-background-default text-textStandard resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
            rows={2}
            disabled={isStreaming}
          />
          <div className="flex flex-col gap-2">
            {isStreaming ? (
              <Button onClick={stopStreaming} variant="outline" size="sm" className="h-full">
                Stop
              </Button>
            ) : (
              <Button
                onClick={handleSubmit}
                disabled={!inputValue.trim()}
                size="sm"
                className="h-full"
              >
                <Send className="w-4 h-4" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
