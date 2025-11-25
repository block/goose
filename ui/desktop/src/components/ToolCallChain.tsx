import { formatMessageTimestamp } from '../utils/timeUtils';
import { Message, getToolRequests } from '../types/message';
import { NotificationEvent } from '../hooks/useMessageStream';
import CompactToolCall from './CompactToolCall';

interface ToolCallChainProps {
  messages: Message[];
  chainIndices: number[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  toolResponsesMap: Map<string, import('../types/message').ToolResponseMessageContent>;
  messageHistoryIndex: number;
  isStreaming?: boolean;
  tabId?: string;
}

export default function ToolCallChain({
  messages,
  chainIndices,
  toolCallNotifications,
  toolResponsesMap,
  messageHistoryIndex,
  isStreaming = false,
  tabId,
}: ToolCallChainProps) {
  const lastMessageIndex = chainIndices[chainIndices.length - 1];
  const lastMessage = messages[lastMessageIndex];
  const timestamp = lastMessage ? formatMessageTimestamp(lastMessage.created) : '';

  return (
    <div className="relative flex flex-col w-full">
      <div className="flex flex-wrap gap-2">
        {chainIndices.map((messageIndex) => {
          const message = messages[messageIndex];
          const toolRequests = getToolRequests(message);

          return toolRequests.map((toolRequest) => (
            <CompactToolCall
              key={toolRequest.id}
              tabId={tabId || 'default'}
              toolRequest={toolRequest}
              toolResponse={toolResponsesMap.get(toolRequest.id)}
              notifications={toolCallNotifications.get(toolRequest.id)}
              isStreamingMessage={isStreaming}
              isCancelledMessage={
                messageIndex < messageHistoryIndex &&
                toolResponsesMap.get(toolRequest.id) == undefined
              }
            />
          ));
        })}
      </div>

      <div className="text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
        {!isStreaming && timestamp}
      </div>
    </div>
  );
}
