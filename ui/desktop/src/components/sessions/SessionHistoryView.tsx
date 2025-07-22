import React, { useState, useEffect } from 'react';
import {
  Calendar,
  MessageSquareText,
  Folder,
  Share2,
  Sparkles,
  Copy,
  Check,
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
import { createSharedSession } from '../../sharedSessions';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import ProgressiveMessageList from '../ProgressiveMessageList';
import { SearchView } from '../conversation/SearchView';
import { ChatContextManagerProvider } from '../context_management/ChatContextManager';
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
            <ChatContextManagerProvider>
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
            </ChatContextManagerProvider>
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
  const [isShareModalOpen, setIsShareModalOpen] = useState(false);
  const [shareLink, setShareLink] = useState<string>('');
  const [isSharing, setIsSharing] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [canShare, setCanShare] = useState(false);

  useEffect(() => {
    const savedSessionConfig = localStorage.getItem('session_sharing_config');
    if (savedSessionConfig) {
      try {
        const config = JSON.parse(savedSessionConfig);
        if (config.enabled && config.baseUrl) {
          setCanShare(true);
        }
      } catch (error) {
        console.error('Error parsing session sharing config:', error);
      }
    }
  }, []);

  const handleShare = async () => {
    setIsSharing(true);

    try {
      const savedSessionConfig = localStorage.getItem('session_sharing_config');
      if (!savedSessionConfig) {
        throw new Error('Session sharing is not configured. Please configure it in settings.');
      }

      const config = JSON.parse(savedSessionConfig);
      if (!config.enabled || !config.baseUrl) {
        throw new Error('Session sharing is not enabled or base URL is not configured.');
      }

      const shareToken = await createSharedSession(
        config.baseUrl,
        session.metadata.working_dir,
        session.messages,
        session.metadata.description || 'Shared Session',
        session.metadata.total_tokens
      );

      const shareableLink = `goose://sessions/${shareToken}`;
      setShareLink(shareableLink);
      setIsShareModalOpen(true);
    } catch (error) {
      console.error('Error sharing session:', error);
      toast.error(
        `Failed to share session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsSharing(false);
    }
  };

  const handleCopyLink = () => {
    navigator.clipboard
      .writeText(shareLink)
      .then(() => {
        setIsCopied(true);
        setTimeout(() => setIsCopied(false), 2000);
      })
      .catch((err) => {
        console.error('Failed to copy link:', err);
        toast.error('Failed to copy link to clipboard');
      });
  };

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
    <>
      <Button
        onClick={handleShare}
        disabled={!canShare || isSharing}
        size="sm"
        variant="outline"
        className={canShare ? '' : 'cursor-not-allowed opacity-50'}
      >
        {isSharing ? (
          <>
            <LoaderCircle className="w-4 h-4 mr-2 animate-spin" />
            Sharing...
          </>
        ) : (
          <>
            <Share2 className="w-4 h-4" />
            Share
          </>
        )}
      </Button>
      <Button onClick={handleLaunchInNewWindow} size="sm" variant="outline">
        <Sparkles className="w-4 h-4" />
        Resume
      </Button>
    </>
  ) : null;

  return (
    <>
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

      <Dialog open={isShareModalOpen} onOpenChange={setIsShareModalOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex justify-center items-center gap-2">
              <Share2 className="w-6 h-6 text-textStandard" />
              Share Session (beta)
            </DialogTitle>
            <DialogDescription>
              Share this session link to give others a read only view of your goose chat.
            </DialogDescription>
          </DialogHeader>

          <div className="py-4">
            <div className="relative rounded-full border border-borderSubtle px-3 py-2 flex items-center bg-gray-100 dark:bg-gray-600">
              <code className="text-sm text-textStandard dark:text-textStandardInverse overflow-x-hidden break-all pr-8 w-full">
                {shareLink}
              </code>
              <Button
                shape="pill"
                variant="ghost"
                className="absolute right-2 top-1/2 -translate-y-1/2"
                onClick={handleCopyLink}
                disabled={isCopied}
              >
                {isCopied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                <span className="sr-only">Copy</span>
              </Button>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsShareModalOpen(false)}>
              Cancel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export default SessionHistoryView;
