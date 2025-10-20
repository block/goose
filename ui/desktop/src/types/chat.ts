import { Recipe } from '../recipe';
import { Message, Session } from '../api';

export interface CachedSession {
  session: Session;
  messages: Message[];
  cachedAt: number; // timestamp for cache invalidation if needed
}

export interface ChatType {
  sessionId: string;
  title: string;
  messageHistoryIndex: number;
  messages: Message[];
  recipe?: Recipe | null; // Add recipe configuration to chat state
  recipeParameters?: Record<string, string> | null; // Add recipe parameters to chat state
  // Note: sessionCache moved to SessionCacheContext
}
