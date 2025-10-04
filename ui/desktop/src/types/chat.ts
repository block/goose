import { Message } from './message';
import { Recipe } from '../recipe';

export interface ChatType {
  sessionId: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipe?: Recipe | null; // Add recipe configuration to chat state
  recipeConfig?: Recipe | null; // Deprecated: Use recipe instead. Kept for backwards compatibility
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
}
