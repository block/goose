import { formatMessageTimestamp } from '../utils/timeUtils';
import { Message, getToolRequests } from '../types/message';
import { NotificationEvent } from '../hooks/useMessageStream';
import ToolCallWithResponse from './ToolCallWithResponse';

interface ToolCallChainProps {
  messages: Message[];
  chainIndices: number[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  toolResponsesMap: Map<string, import('../types/message').ToolResponseMessageContent>;
  messageHistoryIndex: number;
  isStreaming?: boolean;
}

export default function ToolCallChain({
  messages,
  chainIndices,
  toolCallNotifications,
  toolResponsesMap,
  messageHistoryIndex,
  isStreaming = false,
}: ToolCallChainProps) {
  const lastMessageIndex = chainIndices[chainIndices.length - 1];
  const lastMessage = messages[lastMessageIndex];
  const timestamp = lastMessage ? formatMessageTimestamp(lastMessage.created) : '';

  return (
    <div className="relative flex flex-col w-full">
      <div className="flex flex-col gap-3">
        {chainIndices.map((messageIndex) => {
          const message = messages[messageIndex];
          const toolRequests = getToolRequests(message);

          return toolRequests.map((toolRequest) => (
            <div key={toolRequest.id} className="goose-message-tool">
              <ToolCallWithResponse
                isCancelledMessage={
                  messageIndex < messageHistoryIndex &&
                  toolResponsesMap.get(toolRequest.id) == undefined
                }
                toolRequest={toolRequest}
                toolResponse={toolResponsesMap.get(toolRequest.id)}
                notifications={toolCallNotifications.get(toolRequest.id)}
                isStreamingMessage={isStreaming}
              />
            </div>
          ));
        })}
      </div>

      <div className="text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
        {!isStreaming && timestamp}
      </div>
    </div>
  );
}
