import { USE_ACP_CHAT } from '../acpChatFeatureFlag';
import { useAcpChatSession } from './useAcpChatSession';
import { useChatStream } from './useChatStream';
import type { UseChatSessionHook } from './useChatSessionTypes';

export const useChatSession: UseChatSessionHook = USE_ACP_CHAT
  ? useAcpChatSession
  : useChatStream;
