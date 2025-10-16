import { Recipe } from '../recipe';
import { Message } from '../api';

export interface ChatType {
  sessionId: string;
  name: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipe?: Recipe | null; // Add recipe configuration to chat state
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
}
