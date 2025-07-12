import React from 'react';
import { MessageSquare, AlertCircle } from 'lucide-react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import MarkdownContent from '../MarkdownContent';
import ToolCallWithResponse from '../ToolCallWithResponse';
import ImagePreview from '../ImagePreview';
import {
  ToolRequestMessageContent,
  ToolResponseMessageContent,
  TextContent,
} from '../../types/message';
import { type Message } from '../../types/message';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { extractImagePaths, removeImagePathsFromText } from '../../utils/imageUtils';

/**
 * Get tool responses map from messages
 */
export const getToolResponsesMap = (
  messages: Message[],
  messageIndex: number,
  toolRequests: ToolRequestMessageContent[]
) => {
  const responseMap = new Map();

  if (messageIndex >= 0) {
    for (let i = messageIndex + 1; i < messages.length; i++) {
      const responses = messages[i].content
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

/**
 * Props for the SessionHeaderCard component
 */
export interface SessionHeaderCardProps {
  onBack: () => void;
  children: React.ReactNode;
}

/**
 * Common header card for session views
 */
export const SessionHeaderCard: React.FC<SessionHeaderCardProps> = ({ onBack, children }) => {
  return (
    <Card className="rounded-none px-8 pt-6 pb-4 bg-background-defaultInverse text-textProminentInverse flex items-center">
      <BackButton
        showText={false}
        onClick={onBack}
        size="lg"
        className="!text-textProminentInverse dark:!text-textStandardInverse"
      />
      {children}
    </Card>
  );
};

/**
 * Props for the SessionMessages component
 */
export interface SessionMessagesProps {
  messages: Message[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}

/**
 * Common component for displaying session messages
 */
export const SessionMessages: React.FC<SessionMessagesProps> = ({
  messages,
  isLoading,
  error,
  onRetry,
}) => {
  return (
    <ScrollArea className="h-full w-full">
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
            ) : messages?.length > 0 ? (
              messages
                .map((message, index) => {
                  // Extract text content from the message
                  let textContent = message.content
                    .filter((c): c is TextContent => c.type === 'text')
                    .map((c) => c.text)
                    .join('\n');

                  // Extract image paths from the message
                  const imagePaths = extractImagePaths(textContent);

                  // Remove image paths from text for display
                  const displayText =
                    imagePaths.length > 0
                      ? removeImagePathsFromText(textContent, imagePaths)
                      : textContent;

                  // Get tool requests from the message
                  const toolRequests = message.content
                    .filter((c) => c.type === 'toolRequest')
                    .map((c) => c as ToolRequestMessageContent);

                  // Get tool responses map using the helper function
                  const toolResponsesMap = getToolResponsesMap(messages, index, toolRequests);

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
                          {formatMessageTimestamp(message.created)}
                        </span>
                      </div>

                      <div className="flex flex-col w-full">
                        {/* Text content */}
                        {displayText && (
                          <div
                            className={`${toolRequests.length > 0 || imagePaths.length > 0 ? 'mb-4' : ''}`}
                          >
                            <MarkdownContent content={displayText} />
                          </div>
                        )}

                        {/* Render images if any */}
                        {imagePaths.length > 0 && (
                          <div className="flex flex-wrap gap-2 mt-2 mb-2">
                            {imagePaths.map((imagePath, imageIndex) => (
                              <ImagePreview
                                key={imageIndex}
                                src={imagePath}
                                alt={`Image ${imageIndex + 1}`}
                              />
                            ))}
                          </div>
                        )}

                        {/* Tool requests and responses */}
                        {toolRequests.length > 0 && (
                          <div className="goose-message-tool bg-background-default border border-borderSubtle dark:border-gray-700 rounded-b-2xl px-4 pt-4 pb-2 mt-1">
                            {toolRequests.map((toolRequest) => (
                              <ToolCallWithResponse
                                // In the session history page, if no tool response found for given request, it means the tool call
                                // is broken or cancelled.
                                isCancelledMessage={
                                  toolResponsesMap.get(toolRequest.id) == undefined
                                }
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
  );
};
