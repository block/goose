import { formatMessageTimestamp } from '../utils/timeUtils';
import { Message, getToolRequests } from '../types/message';
import { NotificationEvent } from '../hooks/useMessageStream';
import ToolCallWithResponse from './ToolCallWithResponse';

interface ToolCallChainProps {
  messages: Message[];
  chainIndices: number[];
  toolCallNotifications: Map<string, NotificationEvent[]>;
  toolResponsesMap: Map<string, any>;
  messageHistoryIndex: number;
  isStreaming?: boolean;
}

/**
 * Component that renders a chain of consecutive tool call messages with a single timestamp
 */
export default function ToolCallChain({
  messages,
  chainIndices,
  toolCallNotifications,
  toolResponsesMap,
  messageHistoryIndex,
  isStreaming = false
}: ToolCallChainProps) {
  // Get the timestamp from the last message in the chain
  const lastMessageIndex = chainIndices[chainIndices.length - 1];
  const lastMessage = messages[lastMessageIndex];
  const timestamp = lastMessage ? formatMessageTimestamp(lastMessage.created) : '';

  return (
    <div className="relative flex flex-col w-full">
      {/* Render each message's tool calls in the chain */}
      {chainIndices.map((messageIndex) => {
        const message = messages[messageIndex];
        const toolRequests = getToolRequests(message);

        return toolRequests.map((toolRequest) => (
          <div 
            key={toolRequest.id} 
            className="goose-message-tool pb-2"
          >
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
      
      {/* Single timestamp for the entire chain */}
      <div className="text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
        {!isStreaming && timestamp}
      </div>
    </div>
  );
}
