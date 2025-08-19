import React from 'react';
import {
  Calendar,
  MessageSquareText,
  Folder,
  Sparkles,
  Target,
  LoaderCircle,
  AlertCircle,
} from 'lucide-react';
import { type SessionDetails } from '../../sessions';
import { Button } from '../ui/button';
import { toast } from 'react-toastify';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { ScrollArea } from '../ui/scroll-area';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import ProgressiveMessageList from '../ProgressiveMessageList';
import { SearchView } from '../conversation/SearchView';
import { ContextManagerProvider } from '../context_management/ContextManager';
import { Message } from '../../types/message';
import BackButton from '../ui/BackButton';

// Helper function to determine if a message is a user message (same as useChatEngine)
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  if (message.content.every((c) => c.type === 'toolConfirmationRequest')) {
    return false;
  }
  return true;
};

const filterMessagesForDisplay = (messages: Message[]): Message[] => {
  return messages.filter((message) => message.display ?? true);
};

interface SessionHistoryViewProps {
  session: SessionDetails;
  isLoading: boolean;
  error: string | null;
  onBack: () => void;
  onRetry: () => void;
  showActionButtons?: boolean;
}

// Custom SessionHeader component similar to SessionListView style
const SessionHeader: React.FC<{
  onBack: () => void;
  children: React.ReactNode;
  title: string;
  actionButtons?: React.ReactNode;
}> = ({ onBack, children, title, actionButtons }) => {
  return (
    <div className="flex flex-col pb-8 border-b">
      <div className="flex items-center pt-0 mb-1">
        <BackButton onClick={onBack} />
      </div>
      <h1 className="text-4xl font-light mb-4 pt-6">{title}</h1>
      <div className="flex items-center">{children}</div>
      {actionButtons && <div className="flex items-center space-x-3 mt-4">{actionButtons}</div>}
    </div>
  );
};

// Session messages component that uses the same rendering as BaseChat
const SessionMessages: React.FC<{
  messages: Message[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}> = ({ messages, isLoading, error, onRetry }) => {
  // Filter messages for display (same as BaseChat)
  const filteredMessages = filterMessagesForDisplay(messages);

  return (
    <ScrollArea className="h-full w-full">
      <div className="pb-24 pt-8">
        <div className="flex flex-col space-y-6">
          {isLoading ? (
            <div className="flex justify-center items-center py-12">
              <LoaderCircle className="animate-spin h-8 w-8 text-textStandard" />
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
          ) : filteredMessages?.length > 0 ? (
            <ContextManagerProvider>
              <div className="max-w-4xl mx-auto w-full">
                <SearchView>
                  <ProgressiveMessageList
                    messages={filteredMessages}
                    chat={{
                      id: 'session-preview',
                      messageHistoryIndex: filteredMessages.length,
                    }}
                    toolCallNotifications={new Map()}
                    append={() => {}} // Read-only for session history
                    appendMessage={(newMessage) => {
                      // Read-only - do nothing
                      console.log('appendMessage called in read-only session history:', newMessage);
                    }}
                    isUserMessage={isUserMessage} // Use the same function as BaseChat
                    batchSize={15} // Same as BaseChat default
                    batchDelay={30} // Same as BaseChat default
                    showLoadingThreshold={30} // Same as BaseChat default
                  />
                </SearchView>
              </div>
            </ContextManagerProvider>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
              <MessageSquareText className="w-12 h-12 mb-4" />
              <p className="text-lg mb-2">No messages found</p>
              <p className="text-sm">This session doesn't contain any messages</p>
            </div>
          )}
        </div>
      </div>
    </ScrollArea>
  );
};

const SessionHistoryView: React.FC<SessionHistoryViewProps> = ({
  session,
  isLoading,
  error,
  onBack,
  onRetry,
  showActionButtons = true,
}) => {
  const handleLaunchInNewWindow = () => {
    if (session) {
      console.log('Launching session in new window:', session.session_id);
      console.log('Session details:', session);

      // Get the working directory from the session metadata
      const workingDir = session.metadata?.working_dir;

      if (workingDir) {
        console.log(
          `Opening new window with session ID: ${session.session_id}, in working dir: ${workingDir}`
        );

        // Create a new chat window with the working directory and session ID
        window.electron.createChatWindow(
          undefined, // query
          workingDir, // dir
          undefined, // version
          session.session_id // resumeSessionId
        );

        console.log('createChatWindow called successfully');
      } else {
        console.error('No working directory found in session metadata');
        toast.error('Could not launch session: Missing working directory');
      }
    }
  };

  // Define action buttons
  const actionButtons = showActionButtons ? (
    <Button onClick={handleLaunchInNewWindow} size="sm" variant="outline">
      <Sparkles className="w-4 h-4" />
      Resume
    </Button>
  ) : null;

  return (
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0 px-8">
        <SessionHeader
          onBack={onBack}
          title={session.metadata.description || 'Session Details'}
          actionButtons={!isLoading ? actionButtons : null}
        >
          <div className="flex flex-col">
            {!isLoading && session.messages.length > 0 ? (
              <>
                <div className="flex items-center text-text-muted text-sm space-x-5 font-mono">
                  <span className="flex items-center">
                    <Calendar className="w-4 h-4 mr-1" />
                    {formatMessageTimestamp(session.messages[0]?.created)}
                  </span>
                  <span className="flex items-center">
                    <MessageSquareText className="w-4 h-4 mr-1" />
                    {session.metadata.message_count}
                  </span>
                  {session.metadata.total_tokens !== null && (
                    <span className="flex items-center">
                      <Target className="w-4 h-4 mr-1" />
                      {session.metadata.total_tokens.toLocaleString()}
                    </span>
                  )}
                </div>
                <div className="flex items-center text-text-muted text-sm mt-1 font-mono">
                  <span className="flex items-center">
                    <Folder className="w-4 h-4 mr-1" />
                    {session.metadata.working_dir}
                  </span>
                </div>
              </>
            ) : (
              <div className="flex items-center text-text-muted text-sm">
                <LoaderCircle className="w-4 h-4 mr-2 animate-spin" />
                <span>Loading session details...</span>
              </div>
            )}
          </div>
        </SessionHeader>

        <SessionMessages
          messages={session.messages}
          isLoading={isLoading}
          error={error}
          onRetry={onRetry}
        />
      </div>
    </MainPanelLayout>
  );
};

export default SessionHistoryView;
