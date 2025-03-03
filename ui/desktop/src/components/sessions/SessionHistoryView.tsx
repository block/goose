import React from 'react';
import { Clock, MessageSquare, ArrowLeft, AlertCircle } from 'lucide-react';
import { type SessionDetails } from '../../sessions';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import MarkdownContent from '../MarkdownContent';
import ToolCallWithResponse from '../ToolCallWithResponse';
import { ToolRequestMessageContent, ToolResponseMessageContent } from '../../types/message';

interface SessionHistoryViewProps {
  session: SessionDetails;
  isLoading: boolean;
  error: string | null;
  onBack: () => void;
  onResume: () => void;
  onRetry: () => void;
}

export const getToolResponsesMap = (
  session: SessionDetails,
  messageIndex: number,
  toolRequests: ToolRequestMessageContent[]
) => {
  const responseMap = new Map();

  if (messageIndex >= 0) {
    for (let i = messageIndex + 1; i < session.messages.length; i++) {
      const responses = session.messages[i].content
        .filter((c) => c.type === 'toolResponse')
        .map((c) => c as ToolResponseMessageContent);

      for (const response of responses) {
        const matchingRequest = toolRequests.find((req) => req.id === response.id);
        if (matchingRequest) {
          responseMap.set(response.id, response);
        }
      }
    }
  }

  return responseMap;
};

const SessionHistoryView: React.FC<SessionHistoryViewProps> = ({
  session,
  isLoading,
  error,
  onBack,
  onResume,
  onRetry,
}) => {
  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      {/* Top Row - back, info, reopen thread (fixed) */}
      <Card className="px-8 pt-6 pb-4 bg-bgSecondary flex items-center">
        <BackButton showText={false} onClick={onBack} className="text-textStandard" />

        {/* Session info row */}
        <div className="ml-8">
          <h1 className="text-lg font-bold text-textStandard">
            {session.metadata.description || session.session_id}
          </h1>
          <div className="flex items-center text-sm text-textSubtle mt-2 space-x-4">
            <span className="flex items-center">
              <Clock className="w-4 h-4 mr-1" />
              {new Date(session.messages[0]?.created * 1000).toLocaleString()}
            </span>
            <span className="flex items-center">
              <MessageSquare className="w-4 h-4 mr-1" />
              {session.metadata.message_count} messages
            </span>
            {session.metadata.total_tokens !== null && (
              <span className="flex items-center">
                {session.metadata.total_tokens.toLocaleString()} tokens
              </span>
            )}
          </div>
        </div>

        <span
          onClick={onResume}
          className="ml-auto text-md cursor-pointer text-textStandard hover:font-bold hover:scale-105 transition-all duration-150"
        >
          Resume Session
        </span>
      </Card>

      <ScrollArea className="h-[calc(100vh-120px)] w-full">
        {/* Content */}
        <div className="p-4">
          <div className="flex flex-col space-y-4">
            <div className="space-y-4 mb-6">
              {isLoading ? (
                <div className="flex justify-center items-center py-12">
                  <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
                </div>
              ) : error ? (
                <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
                  <div className="text-red-500 mb-4">
                    <AlertCircle size={32} />
                  </div>
                  <p className="text-md mb-2">Error Loading Session Details</p>
                  <p className="text-sm text-center mb-4">{error}</p>
                  <Button onClick={onRetry} variant="default">
                    Try Again
                  </Button>
                </div>
              ) : session?.messages?.length > 0 ? (
                session.messages
                  .map((message, index) => {
                    // Extract text content from the message
                    const textContent = message.content
                      .filter((c) => c.type === 'text')
                      .map((c) => c.text)
                      .join('\n');

                    // Get tool requests from the message
                    const toolRequests = message.content
                      .filter((c) => c.type === 'toolRequest')
                      .map((c) => c as ToolRequestMessageContent);

                    // Get tool responses map using the helper function
                    const toolResponsesMap = getToolResponsesMap(session, index, toolRequests);

                    // Skip pure tool response messages for cleaner display
                    const isOnlyToolResponse =
                      message.content.length > 0 &&
                      message.content.every((c) => c.type === 'toolResponse');

                    if (message.role === 'user' && isOnlyToolResponse) {
                      return null;
                    }

                    return (
                      <Card
                        key={index}
                        className={`p-4 ${
                          message.role === 'user'
                            ? 'bg-bgSecondary border border-borderSubtle'
                            : 'bg-bgSubtle'
                        }`}
                      >
                        <div className="flex justify-between items-center mb-2">
                          <span className="font-medium text-textStandard">
                            {message.role === 'user' ? 'You' : 'Goose'}
                          </span>
                          <span className="text-xs text-textSubtle">
                            {new Date(message.created * 1000).toLocaleTimeString()}
                          </span>
                        </div>

                        <div className="flex flex-col w-full">
                          {/* Text content */}
                          {textContent && (
                            <div className={`${toolRequests.length > 0 ? 'mb-4' : ''}`}>
                              <MarkdownContent content={textContent} />
                            </div>
                          )}

                          {/* Tool requests and responses */}
                          {toolRequests.length > 0 && (
                            <div className="goose-message-tool bg-bgApp border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 mt-1">
                              {toolRequests.map((toolRequest) => (
                                <ToolCallWithResponse
                                  key={toolRequest.id}
                                  toolRequest={toolRequest}
                                  toolResponse={toolResponsesMap.get(toolRequest.id)}
                                />
                              ))}
                            </div>
                          )}
                        </div>
                      </Card>
                    );
                  })
                  .filter(Boolean) // Filter out null entries
              ) : (
                <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
                  <MessageSquare className="w-12 h-12 mb-4" />
                  <p className="text-lg mb-2">No messages found</p>
                  <p className="text-sm">This session doesn't contain any messages</p>
                </div>
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
};

export default SessionHistoryView;
