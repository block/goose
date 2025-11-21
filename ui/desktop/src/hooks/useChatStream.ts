import { useCallback, useEffect, useRef, useState } from 'react';
import { ChatState } from '../types/chatState';

import {
  Message,
  MessageEvent,
  resumeAgent,
  startAgent,
  Session,
  TokenState,
  // updateSessionUserRecipeValues, // TODO: Implement this API endpoint
} from '../api';
import { client } from '../api/client.gen';

import { createUserMessage, getCompactingMessage, getThinkingMessage } from '../types/message';

const resultsCache = new Map<string, { messages: Message[]; session: Session }>();

// Session creation tracking to prevent conflicts
const sessionCreationInProgress = new Set<string>();

// Debug logging - set to false in production
const DEBUG_CHAT_STREAM = true;

// Check if a message contains goose commands or mentions that shouldn't trigger AI response
// Returns an object with skipAI flag and new goose state
function checkForGooseCommands(message: string, currentGooseEnabled: boolean): { 
  skipAI: boolean; 
  newGooseEnabled: boolean;
  isGooseCommand: boolean;
} {
  const trimmedMessage = message.trim().toLowerCase();
  
  // Check for goose OFF commands (must be exact matches)
  const gooseOffCommands = ['@goose off', '@goose stop', '@goose quiet', '@goose pause'];
  for (const command of gooseOffCommands) {
    if (trimmedMessage === command) {
      console.log('🦆 Goose turned OFF with command:', command);
      return { skipAI: true, newGooseEnabled: false, isGooseCommand: true };
    }
  }
  
  // Check for @goose mention anywhere in the message (case-insensitive)
  const gooseMentionPattern = /@goose\b/i;
  const containsGooseMention = gooseMentionPattern.test(message);
  
  // If goose is disabled and message contains @goose, turn it back on
  if (!currentGooseEnabled && containsGooseMention) {
    console.log('🦆 Goose turned ON - detected @goose mention in message');
    return { skipAI: false, newGooseEnabled: true, isGooseCommand: false };
  }
  
  // If goose is disabled and no @goose mention, skip AI response
  if (!currentGooseEnabled) {
    console.log('🦆 Goose is OFF - skipping AI response');
    return { skipAI: true, newGooseEnabled: currentGooseEnabled, isGooseCommand: false };
  }
  
  // Check for friend mentions (Matrix user IDs or @username patterns)
  // Matrix user IDs start with @ and contain a colon (e.g., @user:domain.com)
  const matrixUserPattern = /@[a-zA-Z0-9._-]+:[a-zA-Z0-9.-]+/;
  if (matrixUserPattern.test(message.trim())) {
    console.log('👥 Detected Matrix user mention - skipping AI response');
    return { skipAI: true, newGooseEnabled: currentGooseEnabled, isGooseCommand: false };
  }
  
  // Check for simple @username mentions (without domain)
  const simpleMentionPattern = /^@[a-zA-Z0-9._-]+(\s|$)/;
  if (simpleMentionPattern.test(message.trim())) {
    // But allow @goose commands to pass through to be handled above
    if (!message.trim().toLowerCase().startsWith('@goose')) {
      console.log('👥 Detected user mention - skipping AI response');
      return { skipAI: true, newGooseEnabled: currentGooseEnabled, isGooseCommand: false };
    }
  }
  
  return { skipAI: false, newGooseEnabled: currentGooseEnabled, isGooseCommand: false };
}

const log = {
  session: (action: string, sessionId: string, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:session] ${action}`, {
      sessionId: sessionId.slice(0, 8),
      ...details,
    });
  },
  messages: (action: string, count: number, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:messages] ${action}`, {
      count,
      ...details,
    });
  },
  stream: (action: string, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:stream] ${action}`, details);
  },
  state: (newState: ChatState, details?: Record<string, unknown>) => {
    if (!DEBUG_CHAT_STREAM) return;
    console.log(`[useChatStream:state] → ${newState}`, details);
  },
  error: (context: string, error: unknown) => {
    console.error(`[useChatStream:error] ${context}`, error);
  },
};

interface UseChatStreamProps {
  sessionId: string;
  onStreamFinish: () => void;
  initialMessage?: string;
  onSessionIdChange?: (newSessionId: string) => void;
  isMatrixTab?: boolean; // Flag to indicate if this is a Matrix tab that should listen for Matrix messages
}

interface UseChatStreamReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  handleSubmit: (userMessage: string) => Promise<void>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState: TokenState;
  gooseEnabled: boolean;
}

function pushMessage(currentMessages: Message[], incomingMsg: Message): Message[] {
  const lastMsg = currentMessages[currentMessages.length - 1];

  if (lastMsg?.id && lastMsg.id === incomingMsg.id) {
    const lastContent = lastMsg.content[lastMsg.content.length - 1];
    const newContent = incomingMsg.content[incomingMsg.content.length - 1];

    if (
      lastContent?.type === 'text' &&
      newContent?.type === 'text' &&
      incomingMsg.content.length === 1
    ) {
      lastContent.text += newContent.text;
    } else {
      lastMsg.content.push(...incomingMsg.content);
    }
    return [...currentMessages];
  } else {
    return [...currentMessages, incomingMsg];
  }
}

// Parse SSE stream from a Response object
async function* parseSSEStreamFromResponse(response: Response): AsyncIterable<MessageEvent> {
  if (!response.body) {
    throw new Error('Response body is empty');
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    let running = true;
    while (running) {
      const { done, value } = await reader.read();
      if (done) {
        running = false;
        break;
      }

      // Decode the chunk and add it to our buffer
      buffer += decoder.decode(value, { stream: true });

      // Process complete SSE events
      const events = buffer.split('\n\n');
      buffer = events.pop() || ''; // Keep the last incomplete event in the buffer

      for (const event of events) {
        if (event.startsWith('data: ')) {
          try {
            const data = event.slice(6); // Remove 'data: ' prefix
            const parsedEvent = JSON.parse(data) as MessageEvent;
            yield parsedEvent;
          } catch (e) {
            console.error('Error parsing SSE event:', e);
            // Skip malformed events
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

async function streamFromResponse(
  stream: AsyncIterable<MessageEvent>,
  initialMessages: Message[],
  updateMessages: (messages: Message[]) => void,
  updateTokenState: (tokenState: TokenState) => void,
  updateChatState: (state: ChatState) => void,
  onFinish: (error?: string) => void
): Promise<void> {
  let messageEventCount = 0;
  let currentMessages = initialMessages;

  try {
    log.stream('reading-events');

    for await (const event of stream) {
      switch (event.type) {
        case 'Message': {
          messageEventCount++;
          const msg = event.message;
          currentMessages = pushMessage(currentMessages, msg);

          if (getCompactingMessage(msg)) {
            log.state(ChatState.Compacting, { reason: 'compacting notification' });
            updateChatState(ChatState.Compacting);
          } else if (getThinkingMessage(msg)) {
            log.state(ChatState.Thinking, { reason: 'thinking notification' });
            updateChatState(ChatState.Thinking);
          }

          if (messageEventCount % 10 === 0) {
            log.stream('message-chunk', {
              eventCount: messageEventCount,
              messageCount: currentMessages.length,
            });
          }

          updateTokenState(event.token_state);

          updateMessages(currentMessages);
          break;
        }
        case 'Error': {
          log.error('stream event error', event.error);
          onFinish('Stream error: ' + event.error);
          return;
        }
        case 'Finish': {
          log.stream('finish-event', { reason: event.reason });
          onFinish();
          return;
        }
        case 'ModelChange': {
          log.stream('model-change', {
            model: event.model,
            mode: event.mode,
          });
          break;
        }
        case 'UpdateConversation': {
          log.messages('conversation-update', event.conversation.length);
          // WARNING: Since Message handler uses this local variable, we need to update it here to avoid the client clobbering it.
          // Longterm fix is to only send the agent the new messages, not the entire conversation.
          currentMessages = event.conversation;
          updateMessages(event.conversation);
          break;
        }
        case 'Notification':
        case 'Ping':
          break;
      }
    }

    log.stream('events-complete', { messageEvents: messageEventCount });
    onFinish();
  } catch (error) {
    if (error instanceof Error && error.name !== 'AbortError') {
      log.error('stream read error', error);
      onFinish('Stream error: ' + error);
    }
  }
}

export function useChatStream({
  sessionId,
  onStreamFinish,
  initialMessage,
  onSessionIdChange,
  isMatrixTab = false,
}: UseChatStreamProps): UseChatStreamReturn {
  const [messages, setMessages] = useState<Message[]>([]);
  const messagesRef = useRef<Message[]>([]);
  const [session, setSession] = useState<Session>();
  const [sessionLoadError, setSessionLoadError] = useState<string>();
  const [chatState, setChatState] = useState<ChatState>(ChatState.Idle);
  const [gooseEnabled, setGooseEnabled] = useState<boolean>(true); // Goose starts enabled
  const [tokenState, setTokenState] = useState<TokenState>({
    inputTokens: 0,
    outputTokens: 0,
    totalTokens: 0,
    accumulatedInputTokens: 0,
    accumulatedOutputTokens: 0,
    accumulatedTotalTokens: 0,
  });
  const abortControllerRef = useRef<AbortController | null>(null);
  const initialSessionIdRef = useRef<string>(sessionId);
  const hasLoadedSessionRef = useRef<boolean>(false);

  useEffect(() => {
    if (session) {
      resultsCache.set(sessionId, { session, messages });
    }
  }, [sessionId, session, messages]);

  const renderCountRef = useRef(0);
  renderCountRef.current += 1;
  console.log(`useChatStream render #${renderCountRef.current}, ${session?.id}`);

  const setMessagesAndLog = useCallback((newMessages: Message[], logContext: string) => {
    log.messages(logContext, newMessages.length, {
      lastMessageRole: newMessages[newMessages.length - 1]?.role,
      lastMessageId: newMessages[newMessages.length - 1]?.id?.slice(0, 8),
    });
    setMessages(newMessages);
    messagesRef.current = newMessages;
  }, []);

  const onFinish = useCallback(
    (error?: string): void => {
      if (error) {
        setSessionLoadError(error);
      }
      setChatState(ChatState.Idle);
      onStreamFinish();
    },
    [onStreamFinish]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    // Reset state when sessionId changes
    log.session('loading', sessionId, {
      previousSessionId: session?.id,
      currentMessagesCount: messages.length,
      willReset: true
    });
    setMessagesAndLog([], 'session-reset');
    setSession(undefined);
    setSessionLoadError(undefined);
    setChatState(ChatState.Idle);

    // Check if this is a cached session first
    const cached = resultsCache.get(sessionId);
    if (cached) {
      log.session('loaded-from-cache', sessionId, {
        messageCount: cached.messages.length,
        sessionName: cached.session.name,
      });
      setSession(cached.session);
      setMessagesAndLog(cached.messages, 'load-cached');
      setChatState(ChatState.Idle);
      return;
    }

    // Try to load existing session from backend first
    const loadExistingSession = async () => {
      // Skip backend loading for sessions that start with 'new_' (these are truly new)
      if (sessionId.startsWith('new_')) {
        log.session('new-session', sessionId, { note: 'will create on first message' });
        setChatState(ChatState.Idle);
        setSession(undefined);
        setMessagesAndLog([], 'new-session');
        setSessionLoadError(undefined);
        return;
      }

      try {
        log.session('attempting-resume', sessionId);
        setChatState(ChatState.Thinking);

        console.log('🔄 About to call resumeAgent with sessionId:', sessionId);
        
        // Try to resume the existing session
        const resumeResponse = await resumeAgent({
          body: { session_id: sessionId },
          throwOnError: true,
        });

        console.log('🔄 resumeAgent response:', {
          hasData: !!resumeResponse.data,
          session: resumeResponse.data?.session,
          conversationLength: resumeResponse.data?.conversation?.length,
          fullResponse: resumeResponse
        });

        if (resumeResponse.data) {
          const loadedSession = resumeResponse.data;
          const conversation = loadedSession.conversation || [];
          
          // Check if we actually got valid session data
          if (!loadedSession || !loadedSession.id) {
            console.log('🔄 Resume response missing session data:', resumeResponse.data);
            throw new Error('Resume response missing session data');
          }
          
          console.log('🔄 Processing resumed session:', {
            sessionId: loadedSession.id,
            sessionDescription: loadedSession.description,
            conversationLength: conversation.length,
            firstFewMessages: conversation.slice(0, 3).map(m => ({
              role: m.role,
              content: m.content[0]?.text?.slice(0, 100)
            }))
          });
          
          log.session('resumed-existing', sessionId, {
            messageCount: conversation.length,
            sessionDescription: loadedSession.description,
            conversationPreview: conversation.slice(0, 2).map(m => `${m.role}: ${m.content[0]?.text?.slice(0, 50)}...`)
          });

          setSession(loadedSession);
          setMessagesAndLog(conversation, 'load-existing');
          
          // Cache the loaded session and messages immediately
          resultsCache.set(sessionId, { session: loadedSession, messages: conversation });
          
          setChatState(ChatState.Idle);
          
          console.log('🔄 Session loaded successfully, messages set to:', conversation.length);
        } else {
          log.session('resume-response-empty', sessionId);
          console.log('🔄 Resume response was empty');
        }
      } catch (error) {
        log.session('resume-failed', sessionId, { error });
        console.log('🔄 Resume failed:', error);
        
        // If resume fails, treat as new session
        log.session('treating-as-new', sessionId, { note: 'resume failed, will create on first message' });
        setSession(undefined);
        setMessagesAndLog([], 'new-after-resume-fail');
        setSessionLoadError(undefined);
        setChatState(ChatState.Idle);
      }
    };

    loadExistingSession();
  }, [sessionId, setMessagesAndLog]);

  const handleSubmit = useCallback(
    async (userMessage: string) => {
      log.messages('user-submit', messagesRef.current.length + 1, {
        userMessageLength: userMessage.length,
      });

      // Check if this is a goose control command or mention that shouldn't trigger AI response
      const commandResult = checkForGooseCommands(userMessage, gooseEnabled);
      
      const currentMessages = [...messagesRef.current, createUserMessage(userMessage)];
      setMessagesAndLog(currentMessages, 'user-entered');

      // Update goose enabled state if it changed
      if (commandResult.newGooseEnabled !== gooseEnabled) {
        setGooseEnabled(commandResult.newGooseEnabled);
        console.log('🦆 Goose state changed:', commandResult.newGooseEnabled ? 'ENABLED' : 'DISABLED');
      }

      // If this is a goose control command or mention, or goose is disabled, don't send to AI
      if (commandResult.skipAI) {
        const reason = commandResult.isGooseCommand 
          ? 'goose control command' 
          : !gooseEnabled 
            ? 'goose is disabled'
            : 'user mention detected';
            
        log.messages('skipping-ai-for-command', currentMessages.length, {
          reason,
          message: userMessage.slice(0, 50),
          gooseEnabled: commandResult.newGooseEnabled
        });
        setChatState(ChatState.Idle);
        return;
      }

      log.state(ChatState.Streaming, { reason: 'user submit' });
      setChatState(ChatState.Streaming);

      abortControllerRef.current = new AbortController();

      try {
        log.stream('request-start', { sessionId: sessionId.slice(0, 8) });

        // Check if client is configured before making API calls
        const config = client.getConfig();
        if (!config.baseUrl) {
          log.error('client not configured during submit', { config });
          throw new Error('API client not configured. Please refresh the page.');
        }

        // Ensure we have a valid session before sending the message
        let currentSession = session;
        if (!currentSession) {
          // Check if session creation is already in progress for this sessionId
          if (sessionCreationInProgress.has(sessionId)) {
            log.session('session-creation-already-in-progress', sessionId);
            throw new Error('Session creation already in progress. Please wait a moment and try again.');
          }

          log.session('creating-session-for-message', sessionId);
          sessionCreationInProgress.add(sessionId);
          
          try {
            // Add a small delay to prevent rapid session creation conflicts
            await new Promise(resolve => setTimeout(resolve, Math.random() * 200 + 100));
            
            const createResponse = await client.post({
              url: '/agent/start',
              body: {
                working_dir: window.appConfig?.get('GOOSE_WORKING_DIR') as string || process.cwd(),
              },
              throwOnError: true,
            });
            
            currentSession = createResponse.data;
            setSession(currentSession);
            log.session('session-created-for-message', currentSession.id, {
              originalSessionId: sessionId,
              actualSessionId: currentSession.id,
            });

            // Notify parent component that session ID has changed
            if (onSessionIdChange && currentSession.id !== sessionId) {
              log.session('notifying-session-id-change', currentSession.id, {
                from: sessionId,
                to: currentSession.id,
              });
              onSessionIdChange(currentSession.id);
            }
          } catch (createError) {
            log.error('failed-to-create-session-for-message', createError);
            
            // Check if it's a port conflict error
            if (createError instanceof Error && createError.message.includes('Address already in use')) {
              throw new Error('Server port conflict - please restart the development server');
            }
            
            throw new Error('Failed to create session: ' + (createError instanceof Error ? createError.message : String(createError)));
          } finally {
            // Always remove from tracking set
            sessionCreationInProgress.delete(sessionId);
          }
        }

        // Get the configured base URL and headers from the API client
        const baseUrl = config.baseUrl || '';
        const apiUrl = `${baseUrl}/reply`;

        // Get and log the secret key for debugging
        const secretKey = await window.electron.getSecretKey();
        log.stream('auth-debug', { 
          baseUrl, 
          apiUrl, 
          hasSecretKey: !!secretKey,
          secretKeyLength: secretKey?.length || 0,
          configHeaders: Object.keys(config.headers || {}),
          sessionId: currentSession.id.slice(0, 8)
        });

        // Make a direct fetch call to handle SSE streaming
        const response = await fetch(apiUrl, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': secretKey,
            ...config.headers,
          },
          body: JSON.stringify({
            messages: currentMessages,
            session_id: currentSession.id, // Use the actual session ID from the backend
          }),
          signal: abortControllerRef.current.signal,
        });

        if (!response.ok) {
          const errorText = await response.text().catch(() => 'Unknown error');
          log.error('http-error', { 
            status: response.status, 
            statusText: response.statusText, 
            errorText: errorText.slice(0, 200) 
          });
          
          if (response.status === 401) {
            throw new Error(`Authentication failed (401): Invalid or missing secret key. ${errorText}`);
          }
          
          throw new Error(`HTTP ${response.status}: ${response.statusText}. ${errorText}`);
        }

        log.stream('stream-started');

        // Parse the SSE stream from the response
        const stream = parseSSEStreamFromResponse(response);

        await streamFromResponse(
          stream,
          currentMessages,
          (messages: Message[]) => setMessagesAndLog(messages, 'streaming'),
          setTokenState,
          setChatState,
          onFinish
        );

        log.stream('stream-complete');
      } catch (error) {
        // AbortError is expected when user stops streaming
        if (error instanceof Error && error.name === 'AbortError') {
          log.stream('stream-aborted');
        } else {
          // Check for backend unavailable errors
          if (error && typeof error === 'object' && 'status' in error && (error.status === 404 || error.status === 0)) {
            log.error('backend unavailable during submit', error);
            
            // Add a mock response for UI testing
            const mockResponse: Message = {
              id: `mock-${Date.now()}`,
              role: 'assistant',
              content: [{
                type: 'text',
                text: '🔌 **Backend Unavailable**\n\nThe goose server is not running. To use the chat functionality:\n\n1. Start the goose server\n2. Ensure it\'s running on the correct port\n3. Try your message again\n\nFor now, you can still test the tabbed interface!'
              }],
              created_at: new Date().toISOString(),
            };
            
            const updatedMessages = [...currentMessages, mockResponse];
            setMessagesAndLog(updatedMessages, 'mock-response');
            onFinish();
          } else {
            // Other unexpected errors
            log.error('submit failed', error);
            onFinish('Submit error: ' + (error instanceof Error ? error.message : String(error)));
          }
        }
      }
    },
    [sessionId, session, gooseEnabled, setMessagesAndLog, onFinish, onSessionIdChange]
  );

  const setRecipeUserParams = useCallback(
    async (user_recipe_values: Record<string, string>) => {
      if (session) {
        // TODO: Implement updateSessionUserRecipeValues API endpoint
        console.warn('setRecipeUserParams: API endpoint not implemented yet', user_recipe_values);
        
        // Temporary workaround: just update local state
        setSession({
          ...session,
          user_recipe_values,
        } as any); // Type assertion needed since user_recipe_values doesn't exist on Session type yet
      } else {
        setSessionLoadError("can't call setRecipeParams without a session");
      }
    },
    [sessionId, session, setSessionLoadError]
  );

  useEffect(() => {
    // Session sync with server - this functionality may need to be implemented
    // when proper session management is required. For now, basic session loading
    // via resumeAgent is sufficient for the tabbed chat interface.
    if (session) {
      log.session('session-loaded', session.id, {
        name: session.name,
        messageCount: session.conversation?.length || 0,
      });
    }
  }, [session]);

  useEffect(() => {
    if (initialMessage && session && messages.length === 0 && chatState === ChatState.Idle) {
      log.messages('auto-submit-initial', 0, { initialMessage: initialMessage.slice(0, 50) });
      handleSubmit(initialMessage);
    }
  }, [initialMessage, session, messages.length, chatState, handleSubmit]);

  const stopStreaming = useCallback(() => {
    log.stream('stop-requested');
    abortControllerRef.current?.abort();
    log.state(ChatState.Idle, { reason: 'user stopped streaming' });
    setChatState(ChatState.Idle);
  }, []);

  // Listen for Matrix messages from BaseChat2's append function - ONLY FOR MATRIX TABS
  useEffect(() => {
    // CRITICAL SECURITY: Only Matrix tabs should listen for Matrix messages
    // This prevents Matrix messages from appearing in non-Matrix tabs
    if (!isMatrixTab) {
      console.log('🚫 useChatStream: Not a Matrix tab, skipping Matrix message listener setup:', {
        sessionId: sessionId.substring(0, 8),
        isMatrixTab
      });
      return;
    }

    console.log('✅ useChatStream: Setting up Matrix message listener for Matrix tab:', {
      sessionId: sessionId.substring(0, 8),
      isMatrixTab
    });

    const handleMatrixMessage = (event: CustomEvent) => {
      const { message, targetSessionId, timestamp } = event.detail;
      
      // CRITICAL: Only process messages intended for THIS specific session
      if (targetSessionId !== sessionId) {
        console.log('🚫 useChatStream ignoring matrix message for different session:', {
          eventTargetSessionId: targetSessionId?.substring(0, 8),
          thisSessionId: sessionId.substring(0, 8),
          messageId: message.id,
          sender: message.sender?.displayName || message.sender?.userId || 'unknown'
        });
        return;
      }
      
      console.log('✅ useChatStream received SESSION-SPECIFIC matrix-message-received event (MATRIX TAB ONLY):', {
        sessionId: sessionId.substring(0, 8),
        messageId: message.id,
        role: message.role,
        sender: message.sender?.displayName || message.sender?.userId || 'unknown',
        content: Array.isArray(message.content) ? message.content[0]?.text?.substring(0, 50) + '...' : 'N/A',
        timestamp,
        isMatrixTab
      });
      
      // Add the Matrix message to our current messages
      const currentMessages = [...messagesRef.current, message];
      setMessagesAndLog(currentMessages, 'matrix-message-added');
      
      console.log('✅ Matrix message added to stream for Matrix tab:', {
        sessionId: sessionId.substring(0, 8),
        totalMessages: currentMessages.length,
        messageId: message.id
      });
    };

    // Type assertion to handle the mismatch between CustomEvent and EventListener
    const eventListener = handleMatrixMessage as (event: globalThis.Event) => void;
    window.addEventListener('matrix-message-received', eventListener);

    return () => {
      window.removeEventListener('matrix-message-received', eventListener);
      console.log('🧹 useChatStream: Cleaned up Matrix message listener for Matrix tab:', {
        sessionId: sessionId.substring(0, 8),
        isMatrixTab
      });
    };
  }, [setMessagesAndLog, sessionId, isMatrixTab]); // IMPORTANT: Include isMatrixTab in dependencies

  const cached = resultsCache.get(sessionId);
  
  // CRITICAL FIX: Only use cache if it's for the EXACT same session ID
  // and we don't have current messages loaded yet
  const shouldUseCachedMessages = cached && 
    cached.session.id === sessionId && 
    messages.length === 0 && 
    !session;
    
  const finalMessages = shouldUseCachedMessages ? cached.messages : messages;
  const finalSession = session ?? (shouldUseCachedMessages ? cached.session : undefined);

  console.log('>> returning', sessionId, Date.now(), {
    messagesLength: finalMessages.length,
    hasSession: !!finalSession,
    chatState,
    cached: !!cached,
    cachedMessageCount: cached?.messages?.length || 0,
    shouldUseCachedMessages,
    sessionMatch: cached?.session.id === sessionId,
    currentSessionId: session?.id,
    cachedSessionId: cached?.session.id
  });

  return {
    sessionLoadError,
    messages: finalMessages,
    session: finalSession,
    chatState,
    handleSubmit,
    stopStreaming,
    setRecipeUserParams,
    tokenState,
    gooseEnabled,
  };
}
