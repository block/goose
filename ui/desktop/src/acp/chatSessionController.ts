import { v7 as uuidv7 } from 'uuid';
import { updateSessionUserRecipeValues, type Message, type Session } from '../api';
import type { GooseExtension } from '@aaif/goose-sdk';
import { AppEvents } from '../constants/events';
import { ChatState } from '../types/chatState';
import { errorMessage } from '../utils/conversionUtils';
import { showExtensionLoadResults } from '../utils/extensionErrorUtils';
import { createUserMessage } from '../types/message';
import {
  acpChatSessionActions,
  acpChatSessionStore,
  type AcpChatSessionSnapshot,
} from './chatSessionStore';
import { cancelAcpElicitationRequestsForSession } from './elicitationRequests';
import { parseAcpCreditsExhaustedError, type AcpCreditsExhaustedError } from './errors';
import { cancelAcpPermissionRequestsForSession } from './permissionRequests';
import { acpCancelPrompt, acpPromptSession } from './prompt';
import {
  acpForkSession,
  acpLoadSession,
  acpNewSession,
  acpTruncateSessionConversation,
  isAcpSessionLoadInFlight,
  sessionInfoToSession,
} from './sessions';

export interface AcpLoadSessionOptions {
  onSessionLoaded?: () => void;
}

export interface AcpSnapshotOptions {
  getCurrentSnapshot(): AcpChatSessionSnapshot | undefined;
}

export interface AcpSubmitMessageOptions extends AcpSnapshotOptions {
  onFinish(error?: string): void | Promise<void>;
}

export interface AcpChatSessionController {
  createSession(cwd: string, gooseExtensions: GooseExtension[]): Promise<Session>;
  loadSession(sessionId: string, options?: AcpLoadSessionOptions): Promise<void>;
  submitMessage(
    sessionId: string,
    userMessage: Message,
    options: AcpSubmitMessageOptions
  ): Promise<void>;
  stop(sessionId: string): void;
  updateMessage(
    sessionId: string,
    messageId: string,
    newContent: string,
    editType: 'fork' | 'edit' | undefined,
    options: AcpSubmitMessageOptions
  ): Promise<void>;
  setRecipeUserParams(
    sessionId: string,
    userRecipeValues: Record<string, string>,
    options: AcpSnapshotOptions
  ): Promise<void>;
}

function createAcpCreditsExhaustedMessage(error: AcpCreditsExhaustedError): Message {
  return {
    id: uuidv7(),
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

async function createSession(cwd: string, gooseExtensions: GooseExtension[]): Promise<Session> {
  const { sessionId, sessionInfo, meta } = await acpNewSession(cwd, gooseExtensions);
  const session = sessionInfoToSession(sessionInfo, meta);

  showExtensionLoadResults(meta.extensionResults);
  window.dispatchEvent(
    new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
  );
  acpChatSessionActions.finishSessionLoad(sessionId, session);

  return session;
}

async function loadSession(sessionId: string, options: AcpLoadSessionOptions = {}): Promise<void> {
  const cached = acpChatSessionStore.getSnapshot(sessionId);
  if (cached?.session) {
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
    );
    options.onSessionLoaded?.();
    return;
  }

  if (!isAcpSessionLoadInFlight(sessionId)) {
    acpChatSessionActions.startSessionLoad(sessionId);
  }

  try {
    const { sessionInfo, meta } = await acpLoadSession(sessionId);

    showExtensionLoadResults(meta.extensionResults);
    window.dispatchEvent(
      new CustomEvent(AppEvents.SESSION_EXTENSIONS_LOADED, { detail: { sessionId } })
    );
    acpChatSessionActions.finishSessionLoad(sessionId, sessionInfoToSession(sessionInfo, meta));
    options.onSessionLoaded?.();
  } catch (error) {
    acpChatSessionActions.failSessionLoad(sessionId, errorMessage(error));
  }
}

async function submitMessage(
  sessionId: string,
  userMessage: Message,
  options: AcpSubmitMessageOptions
): Promise<void> {
  if (acpChatSessionStore.getSnapshot(sessionId)?.activePromptAttemptId) {
    return;
  }

  const promptAttemptId = uuidv7();
  acpChatSessionActions.startPromptAttempt(sessionId, promptAttemptId);

  try {
    await acpPromptSession(sessionId, userMessage);
    if (acpChatSessionActions.finishPromptAttemptIfCurrent(sessionId, promptAttemptId)) {
      void options.onFinish();
    }
  } catch (error) {
    const creditsExhaustedError = parseAcpCreditsExhaustedError(error);
    if (creditsExhaustedError) {
      if (!acpChatSessionActions.isCurrentPromptAttempt(sessionId, promptAttemptId)) {
        return;
      }

      const messages = [
        ...(options.getCurrentSnapshot()?.messages ?? []),
        createAcpCreditsExhaustedMessage(creditsExhaustedError),
      ];
      acpChatSessionActions.setMessages(sessionId, messages);
      if (acpChatSessionActions.finishPromptAttemptIfCurrent(sessionId, promptAttemptId)) {
        void options.onFinish();
      }
      return;
    }

    const submitError = 'Submit error: ' + errorMessage(error);
    if (
      acpChatSessionActions.finishPromptAttemptIfCurrent(sessionId, promptAttemptId, submitError)
    ) {
      void options.onFinish(submitError);
    }
  }
}

function stop(sessionId: string): void {
  const storedPromptAttemptId = acpChatSessionStore.getSnapshot(sessionId)?.activePromptAttemptId;
  const hasStoredAcpPrompt = storedPromptAttemptId !== null && storedPromptAttemptId !== undefined;

  if (hasStoredAcpPrompt) {
    acpChatSessionActions.clearActivePromptAttempt(sessionId);
    cancelAcpPermissionRequestsForSession(sessionId);
    cancelAcpElicitationRequestsForSession(sessionId);
    acpCancelPrompt(sessionId).catch((error) => {
      console.warn('Failed to cancel ACP prompt:', error);
    });
    return;
  }

  acpChatSessionActions.setChatState(sessionId, ChatState.Idle);
}

async function updateMessage(
  sessionId: string,
  messageId: string,
  newContent: string,
  editType: 'fork' | 'edit' | undefined,
  options: AcpSubmitMessageOptions
): Promise<void> {
  const resolvedEditType = editType ?? 'fork';
  const currentSnapshot = options.getCurrentSnapshot();

  acpChatSessionActions.setChatState(sessionId, ChatState.Thinking);

  try {
    const currentMessages = currentSnapshot?.messages ?? [];
    const message = currentMessages.find((m) => m.id === messageId);

    if (!message) {
      throw new Error(`Message with id ${messageId} not found in current messages`);
    }

    if (resolvedEditType === 'fork') {
      const targetSessionId = await acpForkSession(sessionId, message.created);

      acpChatSessionActions.setChatState(sessionId, ChatState.Idle);
      const event = new CustomEvent(AppEvents.SESSION_FORKED, {
        detail: {
          newSessionId: targetSessionId,
          shouldStartAgent: true,
          editedMessage: newContent,
        },
      });
      window.dispatchEvent(event);
      window.electron.logInfo(`Dispatched session-forked event for session ${targetSessionId}`);
      return;
    }

    await acpTruncateSessionConversation(sessionId, message.created);

    const truncatedMessages = currentMessages.filter((m) => m.created < message.created);
    const updatedUserMessage = createUserMessage(newContent);

    for (const content of message.content) {
      if (content.type === 'image') {
        updatedUserMessage.content.push(content);
      }
    }

    const messagesForUI = [...truncatedMessages, updatedUserMessage];
    acpChatSessionActions.setMessages(sessionId, messagesForUI);

    await submitMessage(sessionId, updatedUserMessage, options);
  } catch (error) {
    acpChatSessionActions.setChatState(sessionId, ChatState.Idle);
    throw error;
  }
}

async function setRecipeUserParams(
  sessionId: string,
  userRecipeValues: Record<string, string>,
  options: AcpSnapshotOptions
): Promise<void> {
  const currentSession =
    options.getCurrentSnapshot()?.session ?? acpChatSessionStore.getSnapshot(sessionId)?.session;

  if (currentSession) {
    await updateSessionUserRecipeValues({
      path: {
        session_id: sessionId,
      },
      body: {
        userRecipeValues,
      },
      throwOnError: true,
    });
    const updatedSession = {
      ...currentSession,
      user_recipe_values: userRecipeValues,
    };
    acpChatSessionActions.setSessionMetadata(sessionId, updatedSession);
  } else {
    acpChatSessionActions.setSessionLoadError(
      sessionId,
      "can't call setRecipeParams without a session"
    );
  }
}

export const acpChatSessionController: AcpChatSessionController = {
  createSession,
  loadSession,
  submitMessage,
  stop,
  updateMessage,
  setRecipeUserParams,
};
