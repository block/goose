import { Message } from './message';
import { Recipe } from '../recipe';

export interface ChatType {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipeConfig?: Recipe | null; // Add recipe configuration to chat state
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
}
