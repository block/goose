import { Message } from './message';
import { Recipe } from '../recipe';

export interface MatrixChatContext {
  roomId: string;
  recipientId?: string;
  recipientName?: string;
  recipientAvatar?: string;
  isMatrixMode: boolean;
}

export interface ChatType {
  sessionId: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipeConfig?: Recipe | null; // Add recipe configuration to chat state
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
  matrixContext?: MatrixChatContext | null; // Add Matrix chat context
  aiEnabled?: boolean; // Whether AI responses are enabled for this chat (default: true for regular chats, false for Matrix DMs)
}
