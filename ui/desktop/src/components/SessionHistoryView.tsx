import React, { useState } from 'react';
import { ViewConfig } from '../App';
import { ArrowLeft, Clock, MessageSquare } from 'lucide-react';
import { type SessionDetails } from '../api/sessions';
import { Card } from './ui/card';
import { Button } from './ui/button';

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
    <div className="flex flex-col h-screen bg-bgApp">
      {/* Header */}
      <div className="flex items-center p-4 border-b border-borderSubtle">
        <button
          onClick={onBack}
          className="flex items-center text-textPrimary hover:text-textSecondary transition-colors"
          aria-label="Back to sessions list"
        >
          <ArrowLeft className="w-5 h-5 mr-2" />
          Back to sessions
        </button>
        <h1 className="text-xl font-semibold text-textPrimary ml-4">Session Details</h1>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="flex flex-col space-y-4">
          <Card className="bg-bgSecondary p-4 border border-borderSubtle">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-lg font-medium text-textPrimary">
                {session.description || session.session_id}
              </h2>
              <Button onClick={onResume} variant="default" className="flex items-center">
                <MessageSquare className="w-4 h-4 mr-2" />
                Resume Chat
              </Button>
            </div>
            <div className="text-sm text-textSubtle mb-2">
              <span className="flex items-center">
                <Clock className="w-4 h-4 mr-1" />
                {new Date(session.messages[0]?.created * 1000).toLocaleString()}
              </span>
            </div>
            <div className="text-sm text-textSubtle">
              {getMessageCount(session)} messages in conversation
            </div>
          </Card>

          <div className="space-y-4">
            {isLoading ? (
              <div className="flex justify-center items-center py-12">
                <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textPrimary"></div>
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
                    <span className="font-medium text-textPrimary">
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
  );
};

export default SessionHistoryView;
