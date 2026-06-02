import { useCallback, useEffect, useMemo, useReducer, useRef } from 'react';
import type { SessionConfigOption } from '@agentclientprotocol/sdk';
import type { RequestPermissionRequest, RequestPermissionResponse } from '@agentclientprotocol/sdk';
import { defineMessages, useIntl } from '../i18n';
import { v7 as uuidv7 } from 'uuid';
import { AppEvents } from '../constants/events';
import { ChatState } from '../types/chatState';

import {
  getSession,
  Message,
  Session,
  sessionCancel,
  sessionReply,
  TokenState,
  updateFromSession,
  updateSessionUserRecipeValues,
  listApps,
  ExtensionLoadResult,
  Permission,
} from '../api';

import {
  createUserMessage,
  createElicitationResponseMessage,
  generateMessageId,
  getCompactingMessage,
  getThinkingMessage,
  NotificationEvent,
  UserInput,
} from '../types/message';
import { errorMessage } from '../utils/conversionUtils';
import { showExtensionLoadResults } from '../utils/extensionErrorUtils';
import { maybeHandlePlatformEvent } from '../utils/platform_events';
import { useSessionEvents, type SessionEvent } from './useSessionEvents';
import {
  acpCancelPrompt,
  acpLoadSession,
  acpLoadSessionMeta,
  acpPromptSession,
} from '../acp/sessions';
import { DEFAULT_CHAT_TITLE } from '../contexts/ChatContext';
import { getInitialWorkingDir } from '../utils/workingDir';
import {
  createAcpSessionNotificationAdapter,
  type AcpSessionUpdate,
} from '../acp/sessionNotificationAdapter';
import { type AcpCreditsExhaustedError, parseAcpCreditsExhaustedError } from '../acp/errors';
import {
  getAcpClient,
  setAcpPermissionHandler,
  subscribeToAcpGooseSession,
  subscribeToAcpSession,
} from '../acp/acpConnection';

const resultsCache = new Map<
  string,
  { messages: Message[]; session: Session; acpConfigOptions?: SessionConfigOption[] | null }
>();

const sessionCwdHints = new Map<string, string>();

export function setSessionCwdHint(sessionId: string, cwd: string): void {
  sessionCwdHints.set(sessionId, cwd);
}

export function getSessionCwdHint(sessionId: string): string | undefined {
  return sessionCwdHints.get(sessionId);
}

export function seedSessionCwdHints(items: Array<{ id: string; workingDir: string }>): void {
  for (const s of items) {
    sessionCwdHints.set(s.id, s.workingDir);
  }
}

export function useSessionCwdChangeListener(
  handler: (detail: { sessionId: string; newCwd: string }) => void
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  useEffect(() => {
    const wrap = (event: Event) => {
      const detail = (event as CustomEvent<{ sessionId: string; newCwd: string }>).detail;
      if (detail) handlerRef.current(detail);
    };
    window.addEventListener(AppEvents.SESSION_CWD_CHANGED, wrap);
    return () => window.removeEventListener(AppEvents.SESSION_CWD_CHANGED, wrap);
  }, []);
}

interface UseChatStreamProps {
  sessionId: string;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

interface UseChatStreamReturn {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  setChatState: (state: ChatState) => void;
  handleSubmit: (input: UserInput) => Promise<void>;
  submitElicitationResponse: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<void>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState: TokenState;
  acpConfigOptions?: SessionConfigOption[] | null;
  setAcpConfigOptions: (configOptions: SessionConfigOption[] | null | undefined) => void;
  notifications: Map<string, NotificationEvent[]>;
  onMessageUpdate: (
    messageId: string,
    newContent: string,
    editType?: 'fork' | 'edit'
  ) => Promise<void>;
  onAcpPermissionDecision: (toolCallId: string, action: Permission) => Promise<boolean>;
}

interface PendingAcpPermissionRequest {
  request: RequestPermissionRequest;
  resolve: (response: RequestPermissionResponse) => void;
}

interface StreamState {
  messages: Message[];
  session: Session | undefined;
  chatState: ChatState;
  sessionLoadError: string | undefined;
  tokenState: TokenState;
  acpConfigOptions?: SessionConfigOption[] | null;
  notifications: NotificationEvent[];
}

type StreamAction =
  | { type: 'SET_MESSAGES'; payload: Message[] }
  | { type: 'SET_SESSION'; payload: Session | undefined }
  | { type: 'SET_CHAT_STATE'; payload: ChatState }
  | { type: 'SET_SESSION_LOAD_ERROR'; payload: string | undefined }
  | { type: 'SET_TOKEN_STATE'; payload: TokenState }
  | { type: 'SET_ACP_CONFIG_OPTIONS'; payload: SessionConfigOption[] | null | undefined }
  | { type: 'ADD_NOTIFICATION'; payload: NotificationEvent }
  | { type: 'CLEAR_NOTIFICATIONS' }
  | {
      type: 'SESSION_LOADED';
      payload: {
        session: Session;
        messages: Message[];
        tokenState: TokenState;
        acpConfigOptions?: SessionConfigOption[] | null;
      };
    }
  | { type: 'RESET_FOR_NEW_SESSION' }
  | { type: 'START_STREAMING' }
  | { type: 'STREAM_ERROR'; payload: string }
  | { type: 'STREAM_FINISH'; payload?: string }
  | { type: 'SET_SESSION_CWD'; payload: string };

const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

const initialState: StreamState = {
  messages: [],
  session: undefined,
  chatState: ChatState.Idle,
  sessionLoadError: undefined,
  tokenState: initialTokenState,
  acpConfigOptions: undefined,
  notifications: [],
};

function tokenStateFromSession(session: Session | undefined): TokenState {
  return {
    inputTokens: session?.input_tokens ?? 0,
    outputTokens: session?.output_tokens ?? 0,
    totalTokens: session?.total_tokens ?? 0,
    accumulatedInputTokens: session?.accumulated_input_tokens ?? 0,
    accumulatedOutputTokens: session?.accumulated_output_tokens ?? 0,
    accumulatedTotalTokens: session?.accumulated_total_tokens ?? 0,
    accumulatedCost: session?.accumulated_cost,
  };
}

function mergeTokenState(tokenState: TokenState, update: Partial<TokenState>): TokenState {
  return {
    ...tokenState,
    ...update,
  };
}

function acpPermissionOptionId(
  request: RequestPermissionRequest,
  action: Permission
): string | undefined {
  const preferredKind = {
    allow_once: 'allow_once',
    always_allow: 'allow_always',
    deny_once: 'reject_once',
    always_deny: 'reject_always',
    cancel: undefined,
  } satisfies Record<Permission, string | undefined>;

  const kind = preferredKind[action];
  return kind ? request.options.find((option) => option.kind === kind)?.optionId : undefined;
}

function createAcpLoadSessionSnapshot(sessionId: string): Session {
  const now = new Date().toISOString();
  const workingDir =
    sessionCwdHints.get(sessionId) ??
    resultsCache.get(sessionId)?.session.working_dir ??
    getInitialWorkingDir();
  return {
    id: sessionId,
    name: DEFAULT_CHAT_TITLE,
    working_dir: workingDir,
    created_at: now,
    updated_at: now,
    message_count: 0,
    extension_data: {},
    conversation: [],
  };
}

function createAcpCreditsExhaustedMessage(error: AcpCreditsExhaustedError): Message {
  return {
    id: generateMessageId(),
    role: 'assistant',
    created: Math.floor(Date.now() / 1000),
    content: [
      {
        type: 'systemNotification',
        notificationType: 'creditsExhausted',
        msg: error.message,
        ...(error.url ? { data: { top_up_url: error.url } } : {}),
      },
    ],
    metadata: { userVisible: true, agentVisible: false },
  };
}

interface AcpLoadController {
  load(): Promise<{
    session: Session;
    tokenState: TokenState;
    extensionResults: ExtensionLoadResult[] | null | undefined;
    acpConfigOptions?: SessionConfigOption[] | null;
  }>;
  dispose(): void;
}

function createAcpLoadController(sessionId: string, sessionSnapshot: Session): AcpLoadController {
  const adapter = createAcpSessionNotificationAdapter();
  let tokenState = tokenStateFromSession(sessionSnapshot);
  let sessionName = sessionSnapshot.name;
  let disposed = false;

  const applyUpdates = (updates: AcpSessionUpdate[]) => {
    if (disposed) {
      return;
    }

    for (const update of updates) {
      switch (update.type) {
        case 'sessionInfo':
          sessionName = update.name ?? sessionName;
          break;
        case 'tokenState':
          tokenState = mergeTokenState(tokenState, update.tokenState);
          break;
        case 'configOptions':
          break;
        case 'messages':
          break;
      }
    }
  };

  const unsubscribeSession = subscribeToAcpSession(sessionId, (notification) => {
    if (!disposed) {
      applyUpdates(adapter.apply(notification));
    }
  });
  const unsubscribeGooseSession = subscribeToAcpGooseSession(sessionId, (notification) => {
    if (!disposed) {
      applyUpdates(adapter.applyGoose(notification));
    }
  });

  return {
    async load() {
      const response = await acpLoadSession(sessionId, sessionSnapshot.working_dir);
      const meta = acpLoadSessionMeta(response);
      return {
        session: {
          ...sessionSnapshot,
          name: sessionName,
          working_dir: meta.workingDir ?? sessionSnapshot.working_dir,
          recipe: meta.recipe ?? sessionSnapshot.recipe,
          user_recipe_values: meta.userRecipeValues ?? sessionSnapshot.user_recipe_values,
          conversation: adapter.snapshot().messages,
        },
        tokenState,
        extensionResults: meta.extensionResults,
        acpConfigOptions: meta.configOptions,
      };
    },
    dispose() {
      if (disposed) {
        return;
      }
      disposed = true;
      unsubscribeSession();
      unsubscribeGooseSession();
    },
  };
}

function streamReducer(state: StreamState, action: StreamAction): StreamState {
  switch (action.type) {
    case 'SET_MESSAGES':
      return { ...state, messages: action.payload };

    case 'SET_SESSION':
      return { ...state, session: action.payload };

    case 'SET_CHAT_STATE':
      return { ...state, chatState: action.payload };

    case 'SET_SESSION_LOAD_ERROR':
      return { ...state, sessionLoadError: action.payload };

    case 'SET_TOKEN_STATE':
      return { ...state, tokenState: action.payload };

    case 'SET_ACP_CONFIG_OPTIONS':
      return { ...state, acpConfigOptions: action.payload };

    case 'ADD_NOTIFICATION':
      return { ...state, notifications: [...state.notifications, action.payload] };

    case 'CLEAR_NOTIFICATIONS':
      return { ...state, notifications: [] };

    case 'SESSION_LOADED':
      return {
        ...state,
        session: action.payload.session,
        messages: action.payload.messages,
        tokenState: action.payload.tokenState,
        acpConfigOptions: action.payload.acpConfigOptions,
        chatState: ChatState.Idle,
        sessionLoadError: undefined,
      };

    case 'RESET_FOR_NEW_SESSION':
      return {
        ...state,
        messages: [],
        session: undefined,
        sessionLoadError: undefined,
        acpConfigOptions: undefined,
        chatState: ChatState.LoadingConversation,
      };

    case 'START_STREAMING':
      return {
        ...state,
        chatState: ChatState.Streaming,
        notifications: [],
      };

    case 'STREAM_ERROR':
      return {
        ...state,
        sessionLoadError: action.payload,
        chatState: ChatState.Idle,
      };

    case 'STREAM_FINISH':
      return {
        ...state,
        sessionLoadError: action.payload,
        chatState: ChatState.Idle,
      };

    case 'SET_SESSION_CWD':
      if (!state.session) return state;
      return {
        ...state,
        session: { ...state.session, working_dir: action.payload },
      };

    default:
      return state;
  }
}

function pushMessage(currentMessages: Message[], incomingMsg: Message): Message[] {
  const lastMsg = currentMessages[currentMessages.length - 1];

  if (lastMsg?.id && lastMsg.id === incomingMsg.id) {
    const lastContent = lastMsg.content[lastMsg.content.length - 1];
    const newContent = incomingMsg.content[incomingMsg.content.length - 1];

    if (incomingMsg.metadata?.inference) {
      lastMsg.metadata = {
        ...lastMsg.metadata,
        inference: incomingMsg.metadata.inference,
      };
    }

    if (
      lastContent?.type === 'text' &&
      newContent?.type === 'text' &&
      incomingMsg.content.length === 1
    ) {
      lastContent.text += newContent.text;
    } else if (
      lastContent?.type === 'thinking' &&
      newContent?.type === 'thinking' &&
      incomingMsg.content.length === 1 &&
      'thinking' in lastContent &&
      'thinking' in newContent
    ) {
      // For thinking blocks: if the new block has a signature, it's the complete
      // block from content_block_stop — replace entirely. Otherwise append the delta.
      if ('signature' in newContent && newContent.signature) {
        lastContent.thinking = newContent.thinking;
        lastContent.signature = newContent.signature;
      } else {
        lastContent.thinking += newContent.thinking;
      }
    } else {
      lastMsg.content.push(...incomingMsg.content);
    }
    return [...currentMessages];
  } else {
    return [...currentMessages, incomingMsg];
  }
}

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

const REDUCED_MOTION_BATCH_INTERVAL = 1000;

/**
 * Creates an event processor that handles individual SSE events for a request.
 * Returns an unsubscribe function and a handler to process events.
 */
function createEventProcessor(
  initialMessages: Message[],
  dispatch: React.Dispatch<StreamAction>,
  onFinish: (error?: string) => void,
  sessionId: string,
  onReloadNeeded?: () => void
) {
  let currentMessages = initialMessages;
  const reduceMotion = prefersReducedMotion();
  let latestTokenState: TokenState | null = null;
  let latestChatState: ChatState = ChatState.Streaming;
  let lastBatchUpdate = Date.now();
  let hasPendingUpdate = false;
  let pendingInference: Message['metadata']['inference'] | undefined;

  const flushBatchedUpdates = () => {
    if (reduceMotion && hasPendingUpdate) {
      if (latestTokenState) {
        dispatch({ type: 'SET_TOKEN_STATE', payload: latestTokenState });
      }
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: latestChatState });
      hasPendingUpdate = false;
      lastBatchUpdate = Date.now();
    }
  };

  const maybeUpdateUI = (tokenState: TokenState, chatState: ChatState, forceImmediate = false) => {
    if (!reduceMotion) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: tokenState });
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: chatState });
    } else if (forceImmediate) {
      dispatch({ type: 'SET_TOKEN_STATE', payload: tokenState });
      dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      dispatch({ type: 'SET_CHAT_STATE', payload: chatState });
      hasPendingUpdate = false;
      lastBatchUpdate = Date.now();
    } else {
      latestTokenState = tokenState;
      latestChatState = chatState;
      hasPendingUpdate = true;
      const now = Date.now();
      if (now - lastBatchUpdate >= REDUCED_MOTION_BATCH_INTERVAL) {
        flushBatchedUpdates();
      }
    }
  };

  const flushPendingInference = () => {
    if (!pendingInference) {
      return;
    }

    for (let i = currentMessages.length - 1; i >= 0; i--) {
      const message = currentMessages[i];
      if (message.role === 'assistant' && message.metadata.userVisible) {
        currentMessages = [
          ...currentMessages.slice(0, i),
          {
            ...message,
            metadata: {
              ...message.metadata,
              inference: message.metadata.inference ?? pendingInference,
            },
          },
          ...currentMessages.slice(i + 1),
        ];
        break;
      }
    }
    pendingInference = undefined;
  };

  // Returns true if the event is terminal (Finish or Error)
  const processEvent = (event: SessionEvent): boolean => {
    switch (event.type) {
      case 'Message': {
        let msg = (event as Record<string, unknown>).message as Message;
        const tokenState = (event as Record<string, unknown>).token_state as TokenState;

        if (msg.content.length === 0 && msg.metadata?.inference) {
          pendingInference = msg.metadata.inference;
          return false;
        }

        if (pendingInference && msg.role === 'assistant' && msg.metadata.userVisible) {
          msg = {
            ...msg,
            metadata: {
              ...msg.metadata,
              inference: msg.metadata.inference ?? pendingInference,
            },
          };
          pendingInference = undefined;
        }

        currentMessages = pushMessage(currentMessages, msg);

        const hasToolConfirmation = msg.content.some(
          (content) =>
            content.type === 'actionRequired' && content.data.actionType === 'toolConfirmation'
        );

        const hasElicitation = msg.content.some(
          (content) =>
            content.type === 'actionRequired' && content.data.actionType === 'elicitation'
        );

        if (hasToolConfirmation || hasElicitation) {
          maybeUpdateUI(tokenState, ChatState.WaitingForUserInput, true);
        } else if (getCompactingMessage(msg)) {
          maybeUpdateUI(tokenState, ChatState.Compacting);
        } else if (getThinkingMessage(msg)) {
          maybeUpdateUI(tokenState, ChatState.Thinking);
        } else {
          maybeUpdateUI(tokenState, ChatState.Streaming);
        }
        return false;
      }
      case 'Error': {
        flushPendingInference();
        flushBatchedUpdates();
        dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
        const errorMsg = String((event as Record<string, unknown>).error ?? '');
        if (errorMsg.includes('too far behind') && onReloadNeeded) {
          // Server indicated we missed events — end streaming without setting
          // an error (which would show a blocking error screen), then reload
          // the full conversation so the UI reflects the actual state.
          onFinish();
          onReloadNeeded();
        } else {
          onFinish('Stream error: ' + errorMsg);
        }
        return true;
      }
      case 'Finish': {
        flushPendingInference();
        flushBatchedUpdates();
        dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
        onFinish();
        return true;
      }
      case 'UpdateConversation': {
        const conversation = (event as Record<string, unknown>).conversation as Message[];
        currentMessages = conversation;
        if (!reduceMotion) {
          dispatch({ type: 'SET_MESSAGES', payload: conversation });
        } else {
          hasPendingUpdate = true;
        }
        return false;
      }
      case 'Notification': {
        dispatch({ type: 'ADD_NOTIFICATION', payload: event as unknown as NotificationEvent });
        maybeHandlePlatformEvent((event as Record<string, unknown>).message, sessionId);
        return false;
      }
      case 'Ping':
        return false;
      default:
        return false;
    }
  };

  return processEvent;
}

const i18n = defineMessages({
  notificationTitle: {
    id: 'chat.notification.taskComplete.title',
    defaultMessage: 'Goose finished the task.',
  },
  notificationBody: {
    id: 'chat.notification.taskComplete.body',
    defaultMessage: 'Click here to bring Goose back into focus.',
  },
});

export function useChatStream({
  sessionId,
  onStreamFinish,
  onSessionLoaded,
}: UseChatStreamProps): UseChatStreamReturn {
  const intl = useIntl();
  const [state, dispatch] = useReducer(streamReducer, initialState);

  // Long-lived SSE connection for this session
  const { addListener, setActiveRequestsHandler } = useSessionEvents(sessionId);

  // Track the active request for cancellation (includes the session that started it)
  const activeRequestIdRef = useRef<string | null>(null);
  const activeRequestSessionIdRef = useRef<string | null>(null);
  const activeUnsubscribeRef = useRef<(() => void) | null>(null);
  const activeAcpPromptSessionRef = useRef<string | null>(null);
  const pendingAcpPermissionRequestsRef = useRef<Map<string, PendingAcpPermissionRequest>>(
    new Map()
  );
  // When ActiveRequests fires before session load populates messages (cold mount),
  // defer the reattach until the session is loaded so the event processor has
  // the full conversation history. Events are buffered in the meantime.
  const pendingReattachRequestIdRef = useRef<string | null>(null);
  const pendingReattachBufferRef = useRef<SessionEvent[]>([]);
  const namePollingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Ref to access latest state in callbacks (avoids stale closures)
  const stateRef = useRef(state);
  stateRef.current = state;
  const doReattachRef = useRef<((requestId: string, messages: Message[]) => void) | null>(null);

  useEffect(() => {
    return () => {
      if (namePollingRef.current) {
        clearTimeout(namePollingRef.current);
        namePollingRef.current = null;
      }
    };
  }, [sessionId]);

  useEffect(() => {
    if (state.session) {
      resultsCache.set(sessionId, {
        session: state.session,
        messages: state.messages,
        acpConfigOptions: state.acpConfigOptions,
      });
      sessionCwdHints.set(sessionId, state.session.working_dir);
    }
  }, [sessionId, state.session, state.messages, state.acpConfigOptions]);

  useSessionCwdChangeListener((detail) => {
    if (!sessionId || detail.sessionId !== sessionId) return;
    dispatch({ type: 'SET_SESSION_CWD', payload: detail.newCwd });
  });

  const onFinish = useCallback(
    async (error?: string): Promise<void> => {
      // Note: SSE listener/ref cleanup is handled by the terminal-event
      // handler in each listener closure (which guards on requestId) so
      // that overlapping requests don't clobber each other's state.

      if (namePollingRef.current) {
        clearTimeout(namePollingRef.current);
        namePollingRef.current = null;
      }

      dispatch({ type: 'STREAM_FINISH', payload: error });

      if (!error) {
        try {
          const [notificationsEnabled, anyWindowFocused] = await Promise.all([
            window.electron.getSetting('enableNotifications'),
            window.electron.isAnyWindowFocused(),
          ]);
          if (notificationsEnabled === true && !anyWindowFocused) {
            window.electron.showNotification({
              title: intl.formatMessage(i18n.notificationTitle),
              body: intl.formatMessage(i18n.notificationBody),
            });
          }
        } catch (notifyError) {
          console.warn('Failed to show task completion notification:', notifyError);
        }
      }

      const isNewSession = sessionId && sessionId.match(/^\d{8}_\d{6}$/);
      if (isNewSession) {
        window.dispatchEvent(new CustomEvent(AppEvents.MESSAGE_STREAM_FINISHED));
      }

      // Refresh session name after each reply for the first 3 user messages
      if (!error && sessionId) {
        const currentState = stateRef.current;
        const userMessageCount = currentState.messages.filter((m) => m.role === 'user').length;

        if (userMessageCount <= 3) {
          try {
            const response = await getSession({
              path: { session_id: sessionId },
              throwOnError: true,
            });
            if (response.data?.name) {
              dispatch({
                type: 'SET_SESSION',
                payload: currentState.session
                  ? { ...currentState.session, name: response.data.name }
                  : undefined,
              });
              window.dispatchEvent(
                new CustomEvent(AppEvents.SESSION_RENAMED, {
                  detail: { sessionId, newName: response.data.name },
                })
              );
            }
          } catch (refreshError) {
            console.warn('Failed to refresh session name:', refreshError);
          }
        }
      }

      onStreamFinish();
    },
    [intl, onStreamFinish, sessionId]
  );

  // Reload the full conversation from the server, e.g. after the SSE
  // stream indicates the client fell too far behind the replay buffer.
  const reloadConversation = useCallback(() => {
    getSession({
      path: { session_id: sessionId },
      throwOnError: true,
    })
      .then((response) => {
        const session = response.data as Session;
        if (session?.conversation) {
          dispatch({ type: 'SET_MESSAGES', payload: session.conversation });
        }
      })
      .catch((e) => {
        console.warn('Failed to reload conversation after buffer overflow:', e);
      });
  }, [sessionId]);

  // Perform the actual reattach: wire up an event processor and listener
  // for a request that is already in-flight on the server.
  const doReattach = useCallback(
    (requestId: string, messages: Message[]) => {
      activeRequestIdRef.current = requestId;
      activeRequestSessionIdRef.current = sessionId;
      pendingReattachRequestIdRef.current = null;

      dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Streaming });
      dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: undefined });

      const processEvent = createEventProcessor(
        messages,
        dispatch,
        onFinish,
        sessionId,
        reloadConversation
      );

      // Replay any events that were buffered during cold-mount wait
      const buffered = pendingReattachBufferRef.current;
      pendingReattachBufferRef.current = [];
      let finished = false;
      for (const event of buffered) {
        if (processEvent(event)) {
          finished = true;
          break;
        }
      }

      if (finished) {
        // The reply already completed while we were waiting for session load.
        // Clean up — the buffering listener will be replaced below but the
        // old one captured into activeUnsubscribeRef should be removed.
        if (activeUnsubscribeRef.current) {
          activeUnsubscribeRef.current();
          activeUnsubscribeRef.current = null;
        }
        activeRequestIdRef.current = null;
        activeRequestSessionIdRef.current = null;
        return;
      }

      // Replace the buffering listener with a real processing listener
      if (activeUnsubscribeRef.current) {
        activeUnsubscribeRef.current();
      }
      const unsubscribe = addListener(requestId, (event) => {
        const isTerminal = processEvent(event);
        if (isTerminal) {
          unsubscribe();
          if (activeRequestIdRef.current === requestId) {
            activeUnsubscribeRef.current = null;
            activeRequestIdRef.current = null;
            activeRequestSessionIdRef.current = null;
          }
        }
      });
      activeUnsubscribeRef.current = unsubscribe;
    },
    [sessionId, addListener, onFinish, reloadConversation]
  );
  doReattachRef.current = doReattach;

  // Reattach to in-flight replies discovered via the SSE ActiveRequests event.
  // This handles the case where the chat view remounts while a reply is still
  // running on the server — the new hook instance picks up the existing request
  // and starts processing its events.
  useEffect(() => {
    setActiveRequestsHandler((requestIds: string[]) => {
      // Only reattach if we don't already have an active request
      if (activeRequestIdRef.current) return;
      if (requestIds.length === 0) return;

      // Reattach to the first (most recent) active request.
      // Multiple concurrent requests per session aren't supported in the UI.
      const requestId = requestIds[0];
      const currentMessages = stateRef.current.messages;

      if (currentMessages.length === 0) {
        // Cold mount: session load hasn't populated messages yet.
        // Defer event processing until session load completes so the
        // processor starts with the full conversation history.
        // Register a buffering listener NOW so replayed events aren't
        // lost while we wait.
        pendingReattachRequestIdRef.current = requestId;
        pendingReattachBufferRef.current = [];
        activeRequestIdRef.current = requestId;
        activeRequestSessionIdRef.current = sessionId;
        dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Streaming });
        dispatch({ type: 'SET_SESSION_LOAD_ERROR', payload: undefined });

        const unsubscribe = addListener(requestId, (event) => {
          pendingReattachBufferRef.current.push(event);
        });
        activeUnsubscribeRef.current = unsubscribe;
        return;
      }

      doReattach(requestId, currentMessages);
    });

    return () => {
      setActiveRequestsHandler(null);
    };
  }, [sessionId, addListener, onFinish, reloadConversation, setActiveRequestsHandler, doReattach]);

  const onAcpPermissionDecision = useCallback(
    async (toolCallId: string, action: Permission): Promise<boolean> => {
      const pending = pendingAcpPermissionRequestsRef.current.get(toolCallId);
      if (!pending) {
        return false;
      }

      pendingAcpPermissionRequestsRef.current.delete(toolCallId);
      const optionId = acpPermissionOptionId(pending.request, action);
      pending.resolve(
        optionId
          ? { outcome: { outcome: 'selected', optionId } }
          : { outcome: { outcome: 'cancelled' } }
      );
      dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Streaming });
      return true;
    },
    []
  );

  const submitToSession = useCallback(
    async (targetSessionId: string, userMessage: Message, currentMessages: Message[]) => {
      const adapter = createAcpSessionNotificationAdapter(currentMessages);
      activeRequestSessionIdRef.current = targetSessionId;
      activeAcpPromptSessionRef.current = targetSessionId;
      pendingAcpPermissionRequestsRef.current.clear();

      const applyUpdates = (updates: AcpSessionUpdate[]) => {
        for (const update of updates) {
          switch (update.type) {
            case 'messages':
              dispatch({ type: 'SET_MESSAGES', payload: update.messages });
              break;
            case 'sessionInfo': {
              const currentSession = stateRef.current.session;
              if (currentSession && update.name) {
                dispatch({
                  type: 'SET_SESSION',
                  payload: { ...currentSession, name: update.name },
                });
                window.dispatchEvent(
                  new CustomEvent(AppEvents.SESSION_RENAMED, {
                    detail: { sessionId: targetSessionId, newName: update.name },
                  })
                );
              }
              break;
            }
            case 'tokenState':
              dispatch({
                type: 'SET_TOKEN_STATE',
                payload: mergeTokenState(stateRef.current.tokenState, update.tokenState),
              });
              break;
            case 'configOptions':
              dispatch({ type: 'SET_ACP_CONFIG_OPTIONS', payload: update.configOptions });
              break;
          }
        }
      };

      const unsubscribeSession = subscribeToAcpSession(targetSessionId, (notification) => {
        applyUpdates(adapter.apply(notification));
      });
      const unsubscribeGooseSession = subscribeToAcpGooseSession(
        targetSessionId,
        (notification) => {
          applyUpdates(adapter.applyGoose(notification));
        }
      );
      const unsubscribe = () => {
        unsubscribeSession();
        unsubscribeGooseSession();
      };
      activeUnsubscribeRef.current = unsubscribe;

      setAcpPermissionHandler((request) => {
        if (request.sessionId !== targetSessionId) {
          return Promise.resolve({ outcome: { outcome: 'cancelled' } });
        }

        applyUpdates(adapter.applyPermissionRequest(request));
        dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.WaitingForUserInput });

        return new Promise<RequestPermissionResponse>((resolve) => {
          pendingAcpPermissionRequestsRef.current.set(request.toolCall.toolCallId, {
            request,
            resolve,
          });
        });
      });

      try {
        const response = await acpPromptSession(targetSessionId, userMessage);
        if (response.stopReason === 'cancelled') {
          dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
        } else {
          onFinish();
        }
      } catch (error) {
        if (activeAcpPromptSessionRef.current === null) {
          return;
        }

        const creditsExhaustedError = parseAcpCreditsExhaustedError(error);
        if (creditsExhaustedError) {
          dispatch({
            type: 'SET_MESSAGES',
            payload: [
              ...stateRef.current.messages,
              createAcpCreditsExhaustedMessage(creditsExhaustedError),
            ],
          });
          onFinish();
          return;
        }

        onFinish('Submit error: ' + errorMessage(error));
      } finally {
        setAcpPermissionHandler(null);
        for (const pending of pendingAcpPermissionRequestsRef.current.values()) {
          pending.resolve({ outcome: { outcome: 'cancelled' } });
        }
        pendingAcpPermissionRequestsRef.current.clear();
        unsubscribe();
        if (activeAcpPromptSessionRef.current === targetSessionId) {
          activeAcpPromptSessionRef.current = null;
          activeRequestSessionIdRef.current = null;
          activeUnsubscribeRef.current = null;
        } else {
          activeUnsubscribeRef.current = null;
        }
      }
    },
    [onFinish]
  );

  // Load session on mount or sessionId change
  useEffect(() => {
    if (!sessionId) return;

    const cached = resultsCache.get(sessionId);
    if (cached) {
      dispatch({
        type: 'SESSION_LOADED',
        payload: {
          session: cached.session,
          messages: cached.messages,
          acpConfigOptions: cached.acpConfigOptions,
          tokenState: {
            inputTokens: cached.session?.input_tokens ?? 0,
            outputTokens: cached.session?.output_tokens ?? 0,
            totalTokens: cached.session?.total_tokens ?? 0,
            accumulatedInputTokens: cached.session?.accumulated_input_tokens ?? 0,
            accumulatedOutputTokens: cached.session?.accumulated_output_tokens ?? 0,
            accumulatedTotalTokens: cached.session?.accumulated_total_tokens ?? 0,
          },
        },
      });
      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED));
      onSessionLoaded?.();
      return;
    }

    dispatch({ type: 'RESET_FOR_NEW_SESSION' });

    let cancelled = false;
    let loadController: AcpLoadController | null = null;

    (async () => {
      try {
        let loadedSession: Session | undefined;
        let loadedTokenState: TokenState | undefined;
        let extensionResults: ExtensionLoadResult[] | null | undefined;
        let loadedAcpConfigOptions: SessionConfigOption[] | null | undefined;

        const sessionSnapshot = createAcpLoadSessionSnapshot(sessionId);

        if (cancelled) {
          return;
        }

        loadController = createAcpLoadController(sessionId, sessionSnapshot);
        const response = await loadController.load();

        if (cancelled) {
          return;
        }

        loadedSession = response.session;
        loadedTokenState = response.tokenState;
        extensionResults = response.extensionResults;
        loadedAcpConfigOptions = response.acpConfigOptions;

        showExtensionLoadResults(extensionResults);
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED));

        const pendingRequestId = pendingReattachRequestIdRef.current;
        const reattachedToActiveRequest = activeRequestIdRef.current !== null;

        if (pendingRequestId) {
          // Cold-mount reattach: ActiveRequests arrived before session load
          // returned. Load session state first, then complete the reattach
          // with the full conversation so the event processor has context.
          dispatch({
            type: 'SESSION_LOADED',
            payload: {
              session: loadedSession!,
              messages: loadedSession?.conversation || [],
              tokenState: loadedTokenState ?? tokenStateFromSession(loadedSession),
              acpConfigOptions: loadedAcpConfigOptions,
            },
          });
          // Now complete the deferred reattach with the loaded messages
          doReattachRef.current?.(pendingRequestId, loadedSession?.conversation || []);
        } else if (reattachedToActiveRequest) {
          // ActiveRequests already wired up an event processor with existing
          // messages — only load session metadata, don't overwrite messages
          // with the stale DB snapshot.
          dispatch({ type: 'SET_SESSION', payload: loadedSession });
          dispatch({ type: 'SET_ACP_CONFIG_OPTIONS', payload: loadedAcpConfigOptions });
          dispatch({
            type: 'SET_TOKEN_STATE',
            payload: loadedTokenState ?? tokenStateFromSession(loadedSession),
          });
        } else {
          dispatch({
            type: 'SESSION_LOADED',
            payload: {
              session: loadedSession!,
              messages: loadedSession?.conversation || [],
              tokenState: loadedTokenState ?? tokenStateFromSession(loadedSession),
              acpConfigOptions: loadedAcpConfigOptions,
            },
          });
        }

        listApps({
          throwOnError: true,
          query: { session_id: sessionId },
        }).catch((err) => {
          console.warn('Failed to populate apps cache:', err);
        });

        onSessionLoaded?.();
      } catch (error) {
        if (cancelled) return;

        dispatch({ type: 'STREAM_ERROR', payload: errorMessage(error) });
      } finally {
        loadController?.dispose();
        loadController = null;
      }
    })();

    return () => {
      cancelled = true;
      loadController?.dispose();
      loadController = null;
    };
  }, [sessionId, onSessionLoaded]);

  const handleSubmit = useCallback(
    async (input: UserInput) => {
      const { msg: userMessage, images } = input;
      const currentState = stateRef.current;

      if (
        !currentState.session ||
        currentState.chatState === ChatState.LoadingConversation ||
        currentState.chatState === ChatState.Streaming ||
        currentState.chatState === ChatState.Thinking ||
        currentState.chatState === ChatState.Compacting
      ) {
        return;
      }

      const hasExistingMessages = currentState.messages.length > 0;
      const hasNewMessage = userMessage.trim().length > 0 || images.length > 0;

      if (!hasNewMessage && !hasExistingMessages) {
        return;
      }

      // Emit session-created event for first message in a new session
      if (!hasExistingMessages && hasNewMessage) {
        window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));

        const pollForName = async (attempts = 0) => {
          if (attempts >= 20) return;

          try {
            const response = await getSession({
              path: { session_id: sessionId },
              throwOnError: true,
            });
            const currentState = stateRef.current;
            const currentName = currentState.session?.name;
            const newName = response.data?.name;

            if (newName && newName !== currentName) {
              dispatch({
                type: 'SET_SESSION',
                payload: currentState.session
                  ? { ...currentState.session, name: newName }
                  : undefined,
              });
              window.dispatchEvent(
                new CustomEvent(AppEvents.SESSION_RENAMED, {
                  detail: { sessionId, newName },
                })
              );
              return;
            }
          } catch {
            // Silently continue polling
          }

          const latestState = stateRef.current;
          if (
            latestState.chatState === ChatState.Streaming ||
            latestState.chatState === ChatState.Thinking ||
            latestState.chatState === ChatState.Compacting
          ) {
            namePollingRef.current = setTimeout(() => pollForName(attempts + 1), 500);
          }
        };

        namePollingRef.current = setTimeout(() => pollForName(0), 1000);
      }

      const newMessage = hasNewMessage
        ? createUserMessage(userMessage, images)
        : currentState.messages[currentState.messages.length - 1];
      const currentMessages = hasNewMessage
        ? [...currentState.messages, newMessage]
        : [...currentState.messages];

      if (hasNewMessage) {
        dispatch({ type: 'SET_MESSAGES', payload: currentMessages });
      }

      dispatch({ type: 'START_STREAMING' });

      await submitToSession(sessionId, newMessage, currentMessages);
    },
    [sessionId, submitToSession]
  );

  const submitElicitationResponse = useCallback(
    async (elicitationId: string, userData: Record<string, unknown>) => {
      const currentState = stateRef.current;

      if (!currentState.session || currentState.chatState === ChatState.LoadingConversation) {
        return;
      }

      // An elicitation response unblocks an in-flight tool call on the original
      // request's SSE stream — don't start a new stream or flip chat state.
      const responseMessage = createElicitationResponseMessage(elicitationId, userData);
      const nextMessages = [...currentState.messages, responseMessage];
      dispatch({ type: 'SET_MESSAGES', payload: nextMessages });

      try {
        const shouldUseAcp =
          activeAcpPromptSessionRef.current === sessionId ||
          currentState.acpConfigOptions !== undefined;

        if (shouldUseAcp) {
          const client = await getAcpClient();
          await client.goose.elicitationRespond_unstable({
            sessionId,
            elicitationId,
            userData,
          });
        } else {
          await sessionReply({
            path: { id: sessionId },
            body: {
              request_id: uuidv7(),
              user_message: responseMessage,
            },
            throwOnError: true,
          });
        }
      } catch (error) {
        onFinish('Submit error: ' + errorMessage(error));
      }
    },
    [sessionId, onFinish]
  );

  const setRecipeUserParams = useCallback(
    async (user_recipe_values: Record<string, string>) => {
      const currentState = stateRef.current;

      if (currentState.session) {
        await updateSessionUserRecipeValues({
          path: {
            session_id: sessionId,
          },
          body: {
            userRecipeValues: user_recipe_values,
          },
          throwOnError: true,
        });
        dispatch({
          type: 'SET_SESSION',
          payload: {
            ...currentState.session,
            user_recipe_values,
          },
        });
      } else {
        dispatch({
          type: 'SET_SESSION_LOAD_ERROR',
          payload: "can't call setRecipeParams without a session",
        });
      }
    },
    [sessionId]
  );

  useEffect(() => {
    if (state.session) {
      updateFromSession({
        body: {
          session_id: state.session.id,
        },
        throwOnError: true,
      });
    }
  }, [state.session]);

  const stopStreaming = useCallback(() => {
    const requestId = activeRequestIdRef.current;
    const requestSessionId = activeRequestSessionIdRef.current;
    const acpPromptSessionId = activeAcpPromptSessionRef.current;

    if (acpPromptSessionId) {
      activeAcpPromptSessionRef.current = null;
      acpCancelPrompt(acpPromptSessionId).catch((e) => {
        console.warn('Failed to cancel ACP prompt:', e);
      });
    }

    if (requestId && requestSessionId) {
      // Cancel against the session that originally started the request,
      // not the current sessionId (which may have changed if user navigated).
      sessionCancel({
        path: { id: requestSessionId },
        body: { request_id: requestId },
      }).catch((e) => {
        console.warn('Failed to cancel request:', e);
      });
    }

    // Clean up listener
    if (activeUnsubscribeRef.current) {
      activeUnsubscribeRef.current();
      activeUnsubscribeRef.current = null;
    }
    activeRequestIdRef.current = null;
    activeRequestSessionIdRef.current = null;
    activeAcpPromptSessionRef.current = null;

    dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
  }, []);

  const onMessageUpdate = useCallback(
    async (messageId: string, newContent: string, editType: 'fork' | 'edit' = 'fork') => {
      const currentState = stateRef.current;

      dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Thinking });

      try {
        const { forkSession } = await import('../api');
        const message = currentState.messages.find((m) => m.id === messageId);

        if (!message) {
          throw new Error(`Message with id ${messageId} not found in current messages`);
        }

        const response = await forkSession({
          path: {
            session_id: sessionId,
          },
          body: {
            timestamp: message.created,
            truncate: true,
            copy: editType === 'fork',
          },
          throwOnError: true,
        });

        const targetSessionId = response.data?.sessionId;
        if (!targetSessionId) {
          throw new Error('No session ID returned from fork');
        }

        if (editType === 'fork') {
          dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
          const event = new CustomEvent(AppEvents.SESSION_FORKED, {
            detail: {
              newSessionId: targetSessionId,
              shouldStartAgent: true,
              editedMessage: newContent,
            },
          });
          window.dispatchEvent(event);
          window.electron.logInfo(`Dispatched session-forked event for session ${targetSessionId}`);
        } else {
          const { getSession } = await import('../api');
          const sessionResponse = await getSession({
            path: { session_id: targetSessionId },
            throwOnError: true,
          });

          if (sessionResponse.data?.conversation) {
            const truncatedMessages = [...sessionResponse.data.conversation];
            const updatedUserMessage = createUserMessage(newContent);

            for (const content of message.content) {
              if (content.type === 'image') {
                updatedUserMessage.content.push(content);
              }
            }

            const messagesForUI = [...truncatedMessages, updatedUserMessage];
            dispatch({ type: 'SET_MESSAGES', payload: messagesForUI });
            dispatch({ type: 'START_STREAMING' });

            await submitToSession(targetSessionId, updatedUserMessage, messagesForUI);
          } else {
            await handleSubmit({ msg: newContent, images: [] });
          }
        }
      } catch (error) {
        dispatch({ type: 'SET_CHAT_STATE', payload: ChatState.Idle });
        const errorMsg = errorMessage(error);
        console.error('Failed to edit message:', error);
        const { toastError } = await import('../toasts');
        toastError({
          title: 'Failed to edit message',
          msg: errorMsg,
        });
      }
    },
    [sessionId, handleSubmit, submitToSession]
  );

  const setChatState = useCallback((newState: ChatState) => {
    dispatch({ type: 'SET_CHAT_STATE', payload: newState });
  }, []);

  const setAcpConfigOptions = useCallback(
    (configOptions: SessionConfigOption[] | null | undefined) => {
      dispatch({ type: 'SET_ACP_CONFIG_OPTIONS', payload: configOptions });
    },
    []
  );

  const cached = resultsCache.get(sessionId);
  const maybe_cached_messages = state.session ? state.messages : cached?.messages || [];
  const maybe_cached_session = state.session ?? cached?.session;
  const maybe_cached_acp_config_options = state.session
    ? state.acpConfigOptions
    : cached?.acpConfigOptions;

  const notificationsMap = useMemo(() => {
    return state.notifications.reduce((map, notification) => {
      const key = notification.request_id;
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(notification);
      return map;
    }, new Map<string, NotificationEvent[]>());
  }, [state.notifications]);

  return {
    sessionLoadError: state.sessionLoadError,
    messages: maybe_cached_messages,
    session: maybe_cached_session,
    chatState: state.chatState,
    setChatState,
    handleSubmit,
    submitElicitationResponse,
    stopStreaming,
    setRecipeUserParams,
    tokenState: state.tokenState,
    acpConfigOptions: maybe_cached_acp_config_options,
    setAcpConfigOptions,
    notifications: notificationsMap,
    onMessageUpdate,
    onAcpPermissionDecision,
  };
}
