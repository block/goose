import { useCallback, useEffect, useRef, useState } from 'react';
import { clawReply, clawSession, Message } from '../api';
import { ChatState } from '../types/chatState';
import { UserInput } from '../types/message';
import { useChatStream, CustomReplyFn } from '../hooks/useChatStream';
import ProgressiveMessageList from './ProgressiveMessageList';
import ChatInput from './ChatInput';
import LoadingGoose from './LoadingGoose';
import { ScrollArea, ScrollAreaHandle } from './ui/scroll-area';
import { View, ViewOptions } from '../utils/navigationUtils';

const STALENESS_THRESHOLD_MS = 60 * 60 * 1000; // 1 hour

/**
 * Custom reply function that routes through /claw/reply instead of /sessions/{id}/reply.
 * This ensures the claw agent's system prompt and extensions are always set up.
 */
const clawReplyFn: CustomReplyFn = async ({ requestId, userMessage, overrideConversation, signal }) => {
  await clawReply({
    body: {
      request_id: requestId,
      user_message: userMessage,
      override_conversation: overrideConversation,
    },
    signal,
    throwOnError: true,
  });
};

/**
 * Inner component that renders once the claw session ID is known.
 * Uses useChatStream with a custom replyFn to route all messages through /claw/reply.
 */
function ActiveAgentChat({
  setView,
  sessionId,
  shouldAutoPrompt,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  sessionId: string;
  shouldAutoPrompt: boolean;
}) {
  const scrollRef = useRef<ScrollAreaHandle>(null);
  const hasAutoPrompted = useRef(false);

  const {
    messages,
    session,
    chatState,
    handleSubmit: streamHandleSubmit,
    submitElicitationResponse,
    stopStreaming,
    notifications: toolCallNotifications,
    onMessageUpdate,
  } = useChatStream({
    sessionId,
    onStreamFinish: () => {
      scrollRef.current?.scrollToBottom();
    },
    replyFn: clawReplyFn,
  });

  // Auto-scroll on new messages
  useEffect(() => {
    if (chatState === ChatState.Streaming) {
      scrollRef.current?.scrollToBottom();
    }
  }, [messages, chatState]);

  // Fire auto-prompt once session is loaded and idle.
  useEffect(() => {
    if (!session || hasAutoPrompted.current || !shouldAutoPrompt) return;
    if (chatState !== ChatState.Idle) return;

    hasAutoPrompted.current = true;
    streamHandleSubmit({
      msg: 'Check in.',
      images: [],
      metadata: { userVisible: false, agentVisible: true },
    }).catch((err) => {
      console.error('Auto-prompt failed:', err);
    });
  }, [session, chatState, shouldAutoPrompt, streamHandleSubmit]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      await streamHandleSubmit(input);
    },
    [streamHandleSubmit]
  );

  return (
    <div className="flex flex-col h-full min-h-0 bg-background-primary">
      <ScrollArea ref={scrollRef} className="flex-1 min-h-0">
        <div className="px-4 py-4">
          {messages.length === 0 && chatState === ChatState.Idle ? (
            <div className="flex flex-col items-center justify-center text-text-secondary text-sm py-12">
              <p>The active agent is ready. It will check in periodically.</p>
              <p className="mt-1">You can also ask it anything below.</p>
            </div>
          ) : (
            <ProgressiveMessageList
              messages={messages}
              chat={{ sessionId }}
              toolCallNotifications={toolCallNotifications}
              append={(text: string) => streamHandleSubmit({ msg: text, images: [] })}
              isUserMessage={(m: Message) => m.role === 'user'}
              isStreamingMessage={chatState !== ChatState.Idle}
              onMessageUpdate={onMessageUpdate}
              submitElicitationResponse={submitElicitationResponse}
            />
          )}
          {(chatState === ChatState.Streaming || chatState === ChatState.Thinking) && (
            <div className="mt-2">
              <LoadingGoose chatState={chatState} />
            </div>
          )}
        </div>
      </ScrollArea>

      <div className="flex-shrink-0 px-4 pb-4">
        <ChatInput
          sessionId={sessionId}
          handleSubmit={handleSubmit}
          chatState={chatState}
          onStop={stopStreaming}
          initialValue=""
          setView={setView}
          totalTokens={0}
          accumulatedInputTokens={0}
          accumulatedOutputTokens={0}
          droppedFiles={[]}
          onFilesProcessed={() => {}}
          messages={messages}
          disableAnimation={false}
          sessionCosts={undefined}
          toolCount={0}
          hideBottomBar={true}
        />
      </div>
    </div>
  );
}

/**
 * ActiveAgentView — the "Active" mode of the home screen.
 *
 * On mount it calls POST /claw/session to get (or create) the claw session ID,
 * then renders ActiveAgentChat which connects via useChatStream.
 */
export default function ActiveAgentView({
  setView,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}) {
  const [state, setState] = useState<'loading' | 'ready' | 'error'>('loading');
  const [clawSessionId, setClawSessionId] = useState<string | null>(null);
  const [shouldAutoPrompt, setShouldAutoPrompt] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const initialize = useCallback(async () => {
    setState('loading');
    setErrorMsg(null);
    try {
      const response = await clawSession({ throwOnError: true });
      const sid = response.data.session_id;
      setClawSessionId(sid);

      // Check staleness by loading the session
      const { getSession } = await import('../api');
      const sessionResponse = await getSession({
        path: { session_id: sid },
        throwOnError: false,
      });
      const messages = sessionResponse.data?.conversation ?? [];
      const stale =
        messages.length === 0 ||
        Date.now() - messages[messages.length - 1].created * 1000 > STALENESS_THRESHOLD_MS;
      setShouldAutoPrompt(stale);
      setState('ready');
    } catch (err) {
      console.error('Failed to initialize active agent:', err);
      setErrorMsg(String(err));
      setState('error');
    }
  }, []);

  useEffect(() => {
    initialize();
  }, [initialize]);

  if (state === 'loading') {
    return (
      <div className="flex flex-col h-full items-center justify-center bg-background-secondary">
        <LoadingGoose chatState={ChatState.LoadingConversation} />
        <p className="text-text-secondary mt-2 text-sm">Waking up the active agent...</p>
      </div>
    );
  }

  if (state === 'error') {
    return (
      <div className="flex flex-col h-full items-center justify-center bg-background-secondary">
        <p className="text-text-secondary text-sm">Could not connect to the active agent.</p>
        {errorMsg && (
          <p className="text-text-tertiary text-xs mt-1 max-w-md text-center">{errorMsg}</p>
        )}
        <button
          className="mt-4 px-4 py-2 bg-background-primary text-text-primary rounded-lg border border-border-primary hover:bg-background-secondary transition-colors text-sm"
          onClick={initialize}
        >
          Try again
        </button>
      </div>
    );
  }

  return (
    <ActiveAgentChat
      setView={setView}
      sessionId={clawSessionId!}
      shouldAutoPrompt={shouldAutoPrompt}
    />
  );
}
