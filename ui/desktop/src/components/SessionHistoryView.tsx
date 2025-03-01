import React, { useState } from 'react';
import { ViewConfig } from '../App';
import { ArrowLeft, Clock, MessageSquare } from 'lucide-react';
import { type SessionDetails } from '../api/sessions';
import { Card } from './ui/card';
import { Button } from './ui/button';
import BackButton from './ui/BackButton';
import ResumeButton from './ui/ResumeButton';
import { ScrollArea } from './ui/scroll-area';

interface SessionHistoryViewProps {
  session: SessionDetails;
  isLoading: boolean;
  error: string | null;
  onBack: () => void;
  onResume: () => void;
  onRetry: () => void;
}

const SessionHistoryView: React.FC<SessionHistoryViewProps> = ({
  session,
  isLoading,
  error,
  onBack,
  onResume,
  onRetry,
}) => {
  // Get the message count
  const getMessageCount = (session: SessionDetails): number => {
    return session.messages.length;
  };

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>
      <ScrollArea className="h-full w-full">
        <div className="flex flex-col h-screen bg-bgApp">
          {/* Header */}
          <div className="px-8 pt-6 pb-4">
            {/* Navigation row */}
            <div className="flex items-center justify-between mb-4">
              <BackButton onClick={onBack} />
              <ResumeButton onClick={onResume} />
            </div>

            {/* Session info row */}
            <div>
              <h1 className="text-xl font-medium text-textStandard">
                {session.description || session.session_id}
              </h1>
              <div className="flex items-center text-sm text-textSubtle mt-2 space-x-4">
                <span className="flex items-center">
                  <Clock className="w-4 h-4 mr-1" />
                  {new Date(session.messages[0]?.created * 1000).toLocaleString()}
                </span>
                <span className="flex items-center">
                  <MessageSquare className="w-4 h-4 mr-1" />
                  {getMessageCount(session)} messages
                </span>
              </div>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto p-4">
            <div className="flex flex-col space-y-4">
              <div className="space-y-4">
                {isLoading ? (
                  <div className="flex justify-center items-center py-12">
                    <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
                  </div>
                ) : error ? (
                  <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
                    <div className="text-red-500 mb-4">
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="32"
                        height="32"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      >
                        <circle cx="12" cy="12" r="10"></circle>
                        <line x1="12" y1="8" x2="12" y2="12"></line>
                        <line x1="12" y1="16" x2="12.01" y2="16"></line>
                      </svg>
                    </div>
                    <p className="text-md mb-2">Error Loading Session Details</p>
                    <p className="text-sm text-center mb-4">{error}</p>
                    <Button onClick={onRetry} variant="default">
                      Try Again
                    </Button>
                  </div>
                ) : session?.messages?.length > 0 ? (
                  session.messages.map((message, index) => (
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
                      <div className="prose dark:prose-invert max-w-none">
                        {message.content.map((content, i) => (
                          <div key={i}>
                            {content.type === 'text' && (
                              <div className="whitespace-pre-wrap">{content.text}</div>
                            )}
                            {content.type === 'toolRequest' && (
                              <div className="mt-2 p-2 bg-gray-100 dark:bg-gray-800 rounded-md">
                                <div className="font-medium text-sm text-blue-600 dark:text-blue-400">
                                  Tool Request: {content.toolCall.value?.name}
                                </div>
                                {content.toolCall.value?.arguments && (
                                  <pre className="text-xs mt-1 overflow-x-auto">
                                    {JSON.stringify(content.toolCall.value.arguments, null, 2)}
                                  </pre>
                                )}
                              </div>
                            )}
                            {content.type === 'toolResponse' && (
                              <div className="mt-2 p-2 bg-gray-100 dark:bg-gray-800 rounded-md">
                                <div className="font-medium text-sm text-green-600 dark:text-green-400">
                                  Tool Response
                                </div>
                                {content.toolResult.status === 'error' ? (
                                  <div className="text-red-500 text-sm">
                                    {content.toolResult.error}
                                  </div>
                                ) : (
                                  content.toolResult.value?.map((item, idx) => (
                                    <div key={idx} className="text-xs mt-1">
                                      {item.type === 'text' && (
                                        <div className="whitespace-pre-wrap">{item.text}</div>
                                      )}
                                      {item.type === 'resource' && item.resource && (
                                        <div className="mt-1">
                                          <div className="text-xs text-blue-500">
                                            Resource: {item.resource.uri}
                                          </div>
                                          {item.resource.text && (
                                            <pre className="text-xs mt-1 max-h-40 overflow-y-auto p-2 bg-gray-50 dark:bg-gray-900 rounded">
                                              {item.resource.text.length > 500
                                                ? `${item.resource.text.substring(0, 500)}... (truncated)`
                                                : item.resource.text}
                                            </pre>
                                          )}
                                        </div>
                                      )}
                                    </div>
                                  ))
                                )}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    </Card>
                  ))
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
        </div>
      </ScrollArea>
    </div>
  );
};

export default SessionHistoryView;
