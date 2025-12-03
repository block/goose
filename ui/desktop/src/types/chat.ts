import { Message } from './message';
import { Recipe } from '../recipe';

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipeConfig?: Recipe | null; // Add recipe configuration to chat state
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
  // Matrix-specific properties
  matrixRoomId?: string | null; // The Matrix room ID (e.g., !roomId:server.com)
  matrixRecipientId?: string | null; // The recipient user ID for Matrix rooms
  isMatrixTab?: boolean; // Flag to identify Matrix tabs
}
