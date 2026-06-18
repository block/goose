import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { RequestPermissionRequest, SessionNotification } from '@agentclientprotocol/sdk';
import type { Message, Session, TokenState } from '../api';
import { ChatState } from '../types/chatState';
import type { NotificationEvent } from '../types/message';
import {
  createAcpSessionNotificationAdapter,
  type AcpChatStateChange,
  type AcpSessionNotificationAdapter,
} from './sessionNotificationAdapter';
import type { ElicitationStatus } from './adapter/elicitations';
import { cloneMessage } from './adapter/shared';
import type { AcpElicitationRequest } from './elicitationRequests';

export interface AcpChatSessionSnapshot {
  session: Session | undefined;
  messages: Message[];
  tokenState: TokenState;
  notifications: NotificationEvent[];
  chatState: ChatState;
  sessionLoadError: string | undefined;
  activePromptAttemptId: string | null;
}

type SnapshotListener = (snapshot: AcpChatSessionSnapshot) => void;

interface StoreEntry extends AcpChatSessionSnapshot {
  adapter: AcpSessionNotificationAdapter;
}

const initialTokenState: TokenState = {
  inputTokens: 0,
  outputTokens: 0,
  totalTokens: 0,
  accumulatedInputTokens: 0,
  accumulatedOutputTokens: 0,
  accumulatedTotalTokens: 0,
};

export interface AcpChatSessionStore {
  getSnapshot(sessionId: string): AcpChatSessionSnapshot | undefined;
  subscribe(sessionId: string, listener: (snapshot: AcpChatSessionSnapshot) => void): () => void;
  deleteSnapshot(sessionId: string): void;
  setLoadedSession(
    sessionId: string,
    session: Session,
    tokenState?: TokenState
  ): AcpChatSessionSnapshot;
  setSessionMetadata(sessionId: string, session: Session | undefined): AcpChatSessionSnapshot;
  setMessages(sessionId: string, messages: Message[]): AcpChatSessionSnapshot;
  setTokenState(sessionId: string, tokenState: TokenState): AcpChatSessionSnapshot;
  setChatState(sessionId: string, chatState: ChatState): AcpChatSessionSnapshot;
  setSessionLoadError(
    sessionId: string,
    sessionLoadError: string | undefined
  ): AcpChatSessionSnapshot;
  startPromptAttempt(sessionId: string, promptAttemptId: string): AcpChatSessionSnapshot;
  finishPromptAttemptIfCurrent(sessionId: string, promptAttemptId: string, error?: string): boolean;
  clearActivePromptAttempt(sessionId: string): AcpChatSessionSnapshot | undefined;
  isCurrentPromptAttempt(sessionId: string, promptAttemptId: string): boolean;
  applyAcpSessionNotification(notification: SessionNotification): AcpChatSessionSnapshot;
  applyAcpGooseSessionNotification(
    notification: GooseSessionNotification_unstable
  ): AcpChatSessionSnapshot;
  applyPermissionRequest(request: RequestPermissionRequest): AcpChatSessionSnapshot;
  applyElicitationRequest(request: AcpElicitationRequest): AcpChatSessionSnapshot;
  setElicitationStatus(
    sessionId: string,
    elicitationId: string,
    status: ElicitationStatus
  ): AcpChatSessionSnapshot | undefined;
}

export function createAcpChatSessionStore(): AcpChatSessionStore {
  const sessionsById = new Map<string, StoreEntry>();
  const listenersBySessionId = new Map<string, Set<SnapshotListener>>();

  const getSnapshot: AcpChatSessionStore['getSnapshot'] = (sessionId) => {
    const entry = sessionsById.get(sessionId);
    return entry ? snapshotFromEntry(entry) : undefined;
  };

  const subscribe: AcpChatSessionStore['subscribe'] = (sessionId, listener) => {
    const listeners = listenersBySessionId.get(sessionId) ?? new Set<SnapshotListener>();
    listeners.add(listener);
    listenersBySessionId.set(sessionId, listeners);

    let subscribed = true;
    return () => {
      if (!subscribed) {
        return;
      }

      subscribed = false;
      const currentListeners = listenersBySessionId.get(sessionId);
      if (!currentListeners) {
        return;
      }

      currentListeners.delete(listener);
      if (currentListeners.size === 0) {
        listenersBySessionId.delete(sessionId);
      }
    };
  };

  const deleteSnapshot: AcpChatSessionStore['deleteSnapshot'] = (sessionId) => {
    sessionsById.delete(sessionId);
  };

  const getOrCreateEntry = (sessionId: string): StoreEntry => {
    const existing = sessionsById.get(sessionId);
    if (existing) {
      return existing;
    }

    const entry: StoreEntry = {
      session: undefined,
      messages: [],
      tokenState: { ...initialTokenState },
      notifications: [],
      chatState: ChatState.Idle,
      sessionLoadError: undefined,
      activePromptAttemptId: null,
      adapter: createAcpSessionNotificationAdapter(),
    };
    sessionsById.set(sessionId, entry);
    return entry;
  };

  const notify = (sessionId: string, entry: StoreEntry): AcpChatSessionSnapshot => {
    const snapshot = snapshotFromEntry(entry);
    const listeners = listenersBySessionId.get(sessionId);
    if (listeners) {
      for (const listener of listeners) {
        listener(snapshot);
      }
    }
    return snapshot;
  };

  const setLoadedSession: AcpChatSessionStore['setLoadedSession'] = (
    sessionId,
    session,
    tokenState = tokenStateFromSession(session)
  ) => {
    const entry = getOrCreateEntry(sessionId);
    entry.session = session;
    entry.messages = cloneMessages(session.conversation ?? []);
    entry.tokenState = { ...tokenState };
    entry.chatState = entry.activePromptAttemptId ? ChatState.Streaming : ChatState.Idle;
    entry.sessionLoadError = undefined;
    entry.adapter = createAcpSessionNotificationAdapter(entry.messages);
    return notify(sessionId, entry);
  };

  const setSessionMetadata: AcpChatSessionStore['setSessionMetadata'] = (sessionId, session) => {
    const entry = getOrCreateEntry(sessionId);
    entry.session = session;
    return notify(sessionId, entry);
  };

  const setMessages: AcpChatSessionStore['setMessages'] = (sessionId, messages) => {
    const entry = getOrCreateEntry(sessionId);
    entry.messages = cloneMessages(messages);
    entry.adapter = createAcpSessionNotificationAdapter(entry.messages);
    return notify(sessionId, entry);
  };

  const setTokenState: AcpChatSessionStore['setTokenState'] = (sessionId, tokenState) => {
    const entry = getOrCreateEntry(sessionId);
    entry.tokenState = { ...tokenState };
    return notify(sessionId, entry);
  };

  const setChatState: AcpChatSessionStore['setChatState'] = (sessionId, chatState) => {
    const entry = getOrCreateEntry(sessionId);
    entry.chatState = chatState;
    return notify(sessionId, entry);
  };

  const setSessionLoadError: AcpChatSessionStore['setSessionLoadError'] = (
    sessionId,
    sessionLoadError
  ) => {
    const entry = getOrCreateEntry(sessionId);
    entry.sessionLoadError = sessionLoadError;
    return notify(sessionId, entry);
  };

  const startPromptAttempt: AcpChatSessionStore['startPromptAttempt'] = (
    sessionId,
    promptAttemptId
  ) => {
    const entry = getOrCreateEntry(sessionId);
    entry.activePromptAttemptId = promptAttemptId;
    entry.chatState = ChatState.Streaming;
    entry.sessionLoadError = undefined;
    entry.notifications = [];
    return notify(sessionId, entry);
  };

  const finishPromptAttemptIfCurrent: AcpChatSessionStore['finishPromptAttemptIfCurrent'] = (
    sessionId,
    promptAttemptId,
    error
  ) => {
    const entry = sessionsById.get(sessionId);
    if (!entry || entry.activePromptAttemptId !== promptAttemptId) {
      return false;
    }

    entry.activePromptAttemptId = null;
    entry.chatState = ChatState.Idle;
    entry.sessionLoadError = error;
    notify(sessionId, entry);
    return true;
  };

  const clearActivePromptAttempt: AcpChatSessionStore['clearActivePromptAttempt'] = (sessionId) => {
    const entry = sessionsById.get(sessionId);
    if (!entry) {
      return undefined;
    }

    entry.activePromptAttemptId = null;
    entry.chatState = ChatState.Idle;
    return notify(sessionId, entry);
  };

  const isCurrentPromptAttempt: AcpChatSessionStore['isCurrentPromptAttempt'] = (
    sessionId,
    promptAttemptId
  ) => sessionsById.get(sessionId)?.activePromptAttemptId === promptAttemptId;

  const applyAcpSessionNotification: AcpChatSessionStore['applyAcpSessionNotification'] = (
    notification
  ) => {
    const entry = getOrCreateEntry(notification.sessionId);
    const changes = entry.adapter.apply(notification);
    applyChatStateChanges(entry, changes);
    return notify(notification.sessionId, entry);
  };

  const applyAcpGooseSessionNotification: AcpChatSessionStore['applyAcpGooseSessionNotification'] =
    (notification) => {
      const entry = getOrCreateEntry(notification.sessionId);
      const changes = entry.adapter.applyGoose(notification);
      applyChatStateChanges(entry, changes);
      return notify(notification.sessionId, entry);
    };

  const applyPermissionRequest: AcpChatSessionStore['applyPermissionRequest'] = (request) => {
    const entry = getOrCreateEntry(request.sessionId);
    const changes = entry.adapter.applyPermissionRequest(request);
    applyChatStateChanges(entry, changes);
    entry.chatState = ChatState.WaitingForUserInput;
    return notify(request.sessionId, entry);
  };

  const applyElicitationRequest: AcpChatSessionStore['applyElicitationRequest'] = (request) => {
    const entry = getOrCreateEntry(request.sessionId);
    const changes = entry.adapter.applyElicitationRequest(request);
    applyChatStateChanges(entry, changes);
    entry.chatState = ChatState.WaitingForUserInput;
    return notify(request.sessionId, entry);
  };

  const setElicitationStatus: AcpChatSessionStore['setElicitationStatus'] = (
    sessionId,
    elicitationId,
    status
  ) => {
    const entry = sessionsById.get(sessionId);
    if (!entry) {
      return undefined;
    }

    const changes = entry.adapter.applyElicitationStatus(elicitationId, status);
    if (changes.length === 0) {
      return snapshotFromEntry(entry);
    }

    applyChatStateChanges(entry, changes);
    return notify(sessionId, entry);
  };

  return {
    getSnapshot,
    subscribe,
    deleteSnapshot,
    setLoadedSession,
    setSessionMetadata,
    setMessages,
    setTokenState,
    setChatState,
    setSessionLoadError,
    startPromptAttempt,
    finishPromptAttemptIfCurrent,
    clearActivePromptAttempt,
    isCurrentPromptAttempt,
    applyAcpSessionNotification,
    applyAcpGooseSessionNotification,
    applyPermissionRequest,
    applyElicitationRequest,
    setElicitationStatus,
  };
}

export const acpChatSessionStore = createAcpChatSessionStore();

export function tokenStateFromSession(session: Session | undefined): TokenState {
  return {
    inputTokens: session?.input_tokens ?? 0,
    outputTokens: session?.output_tokens ?? 0,
    totalTokens: session?.total_tokens ?? 0,
    accumulatedInputTokens: session?.accumulated_input_tokens ?? 0,
    accumulatedOutputTokens: session?.accumulated_output_tokens ?? 0,
    accumulatedTotalTokens: session?.accumulated_total_tokens ?? 0,
    ...(session?.accumulated_cost !== undefined
      ? { accumulatedCost: session.accumulated_cost }
      : {}),
  };
}

function applyChatStateChanges(entry: StoreEntry, changes: AcpChatStateChange[]): void {
  for (const change of changes) {
    switch (change.type) {
      case 'messages':
        entry.messages = cloneMessages(change.messages);
        break;
      case 'tokenState':
        entry.tokenState = { ...entry.tokenState, ...change.tokenState };
        break;
      case 'sessionInfo':
        if (change.name && entry.session) {
          entry.session = { ...entry.session, name: change.name };
        }
        break;
      case 'notification':
        entry.notifications = [...entry.notifications, change.notification];
        break;
    }
  }
}

function snapshotFromEntry(entry: StoreEntry): AcpChatSessionSnapshot {
  return {
    session: entry.session,
    messages: cloneMessages(entry.messages),
    tokenState: { ...entry.tokenState },
    notifications: [...entry.notifications],
    chatState: entry.chatState,
    sessionLoadError: entry.sessionLoadError,
    activePromptAttemptId: entry.activePromptAttemptId,
  };
}

function cloneMessages(messages: Message[]): Message[] {
  return messages.map(cloneMessage);
}
