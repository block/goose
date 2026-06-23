import type { Message, Session, TokenState } from '../api';
import type { ChatState } from '../types/chatState';
import type { NotificationEvent, UserInput } from '../types/message';

export interface UseChatSessionParams {
  sessionId: string;
  onStreamFinish: () => void;
  onSessionLoaded?: () => void;
}

export interface UseChatSessionResult {
  session?: Session;
  messages: Message[];
  chatState: ChatState;
  setChatState: (state: ChatState) => void;
  updateSession: (updater: (session: Session) => Session) => void;
  handleSubmit: (input: UserInput) => Promise<void>;
  onSteerQueuedMessage?: (input: UserInput) => Promise<boolean>;
  submitElicitationResponse: (
    elicitationId: string,
    userData: Record<string, unknown>
  ) => Promise<boolean>;
  setRecipeUserParams: (values: Record<string, string>) => Promise<void>;
  stopStreaming: () => void;
  sessionLoadError?: string;
  tokenState: TokenState;
  notifications: Map<string, NotificationEvent[]>;
  pauseQueueOnStop: boolean;
  queueProcessingBlocked: boolean;
  onMessageUpdate: (
    messageId: string,
    newContent: string,
    editType?: 'fork' | 'edit'
  ) => Promise<void>;
}

export type UseChatSessionHook = (params: UseChatSessionParams) => UseChatSessionResult;
