import { useEffect, useRef, useState, useCallback } from 'react';
import { Message, startAgent } from '../../../api';
import { Recipe } from '../../../recipe';
import { stripEmptyExtensions } from '../../../recipe';
import { useChatStream } from '../../../hooks/useChatStream';
import { ChatState } from '../../../types/chatState';
import { ScrollArea, ScrollAreaHandle } from '../../ui/scroll-area';
import ProgressiveMessageList from '../../ProgressiveMessageList';
import LoadingGoose from '../../LoadingGoose';
import { recipeBuilderRecipe } from './recipeBuilderRecipe';
import { getInitialWorkingDir } from '../../../utils/workingDir';
import { UserInput } from '../../../types/message';
import { Send } from '../../icons';
import Stop from '../../ui/Stop';
import { Button } from '../../ui/button';
import { RecipeBuilderChatProps } from './types';

export default function RecipeBuilderChat({
  recipe,
  onRecipeChange,
  recipeEditedInEditView,
  onRecipeEditSynced,
}: RecipeBuilderChatProps) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(true);
  const [inputValue, setInputValue] = useState('');
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const workingDir = getInitialWorkingDir();
        const response = await startAgent({
          body: {
            working_dir: workingDir,
            recipe: stripEmptyExtensions(recipeBuilderRecipe),
          },
          throwOnError: true,
        });

        if (cancelled) return;

        setSessionId(response.data.id);
        setIsInitializing(false);
      } catch (error) {
        if (cancelled) return;
        console.error('Failed to start recipe builder session:', error);
        setSessionError(error instanceof Error ? error.message : 'Failed to start session');
        setIsInitializing(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const onStreamFinish = useCallback(() => {}, []);

  const {
    messages,
    chatState,
    handleSubmit: submitToChat,
    sessionLoadError,
    stopStreaming,
  } = useChatStream({
    sessionId: sessionId || '',
    onStreamFinish,
    getExtraReplyBody: () => {
      if (recipeEditedInEditView && recipe) {
        onRecipeEditSynced();
        return {
          ui_state: {
            recipe_builder_draft: recipe,
          },
        };
      }
      return undefined;
    },
    onSessionUiStateUpdate: (state) => {
      if (
        state &&
        typeof state === 'object' &&
        'recipe_builder_draft' in state &&
        state.recipe_builder_draft
      ) {
        onRecipeChange(state.recipe_builder_draft as Recipe);
      }
    },
  });

  const isUserMessage = useCallback((message: Message) => message.role === 'user', []);

  useEffect(() => {
    if (!isInitializing && chatState === ChatState.Idle && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isInitializing, chatState]);

  const isLoading = isInitializing || chatState === ChatState.LoadingConversation;
  const isStreaming =
    chatState === ChatState.Streaming ||
    chatState === ChatState.Thinking ||
    chatState === ChatState.Compacting;

  const handleSubmit = useCallback(async () => {
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

  if (sessionError || sessionLoadError) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-4 rounded-lg max-w-md">
          <h3 className="font-semibold mb-2">Failed to Start Chat</h3>
          <p className="text-sm">{sessionError || sessionLoadError}</p>
        </div>
      </div>
    );
  }

  const showLoadingIndicator = isLoading || isStreaming;

  return (
    <>
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
              <p className="text-lg mb-2">Recipe Builder</p>
              <p className="text-sm">
                Tell me what kind of recipe you'd like to create, and I'll help you build it.
              </p>
            </div>
          ) : (
            <>
              <ProgressiveMessageList
                messages={messages}
                chat={{ sessionId: sessionId! }}
                isUserMessage={isUserMessage}
              />
              <div className="block h-8" />
            </>
          )}
        </ScrollArea>

        {showLoadingIndicator && (
          <div className="absolute bottom-1 left-4 z-20 pointer-events-none">
            <LoadingGoose chatState={isLoading ? ChatState.LoadingConversation : chatState} />
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
              placeholder="Describe the recipe you want to create..."
              className="w-full outline-none border-none focus:ring-0 bg-transparent px-3 pt-3 pb-1.5 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
              rows={2}
              disabled={isLoading || isStreaming}
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
                  disabled={isLoading || !inputValue.trim()}
                  className={`rounded-full px-10 py-2 flex items-center gap-2 ${
                    isLoading || !inputValue.trim()
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
