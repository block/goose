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
  Users,
  Hash,
  Clock,
  ExternalLink,
} from 'lucide-react';
import { resumeSession } from '../../sessions';
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
import { ContextManagerProvider } from '../context_management/ContextManager';
import { Message } from '../../types/message';
import BackButton from '../ui/BackButton';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
import { Session } from '../../api';
import { convertApiMessageToFrontendMessage } from '../context_management';
import { sessionMappingService, SessionMappingService } from '../../services/SessionMappingService';
import { matrixService } from '../../services/MatrixService';

// Helper function to determine if a message is a user message (same as useChatEngine)
const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  return !message.content.every((c) => c.type === 'toolConfirmationRequest');
};

const filterMessagesForDisplay = (messages: Message[]): Message[] => {
  return messages;
};

// Matrix Collaboration Info Component
const MatrixCollaborationInfo: React.FC<{ sessionId: string }> = ({ sessionId }) => {
  const [matrixRoomState, setMatrixRoomState] = useState<any>(null);
  const [matrixRoomId, setMatrixRoomId] = useState<string | null>(null);
  const [isRejoining, setIsRejoining] = useState(false);

  useEffect(() => {
    // Check if this session is associated with a Matrix room
    const checkMatrixAssociation = () => {
      // First check if this is a Matrix room ID directly
      if (SessionMappingService.isMatrixRoomId(sessionId)) {
        const roomState = sessionMappingService.getMatrixRoomState(sessionId);
        setMatrixRoomId(sessionId);
        setMatrixRoomState(roomState);
      } else {
        // Check if this Goose session ID maps to a Matrix room
        const roomId = sessionMappingService.getMatrixRoomId(sessionId);
        if (roomId) {
          const roomState = sessionMappingService.getMatrixRoomState(roomId);
          setMatrixRoomId(roomId);
          setMatrixRoomState(roomState);
        }
      }
    };

    checkMatrixAssociation();
  }, [sessionId]);

  const handleRejoinRoom = async () => {
    if (!matrixRoomId) return;

    setIsRejoining(true);
    try {
      await matrixService.joinRoom(matrixRoomId);
      toast.success('Successfully rejoined Matrix room!');
      
      // Refresh room state after joining
      const updatedRoomState = sessionMappingService.getMatrixRoomState(matrixRoomId);
      setMatrixRoomState(updatedRoomState);
    } catch (error) {
      console.error('Failed to rejoin Matrix room:', error);
      toast.error(`Failed to rejoin room: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setIsRejoining(false);
    }
  };

  const getCurrentMembership = () => {
    if (!matrixRoomId) return null;
    
    try {
      const room = (matrixService as any).client?.getRoom(matrixRoomId);
      return room?.getMyMembership() || 'unknown';
    } catch (error) {
      return 'unknown';
    }
  };

  const isCurrentlyJoined = getCurrentMembership() === 'join';

  if (!matrixRoomState || !matrixRoomId) {
    return null; // Not a Matrix collaborative session
  }

  const { metadata, participants, membershipHistory } = matrixRoomState;
  const participantList = Array.from(participants.values());
  const activeParticipants = participantList.filter(p => p.membership === 'join');

  return (
    <div className="mt-4 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-2">
          <Hash className="w-5 h-5 text-blue-600 dark:text-blue-400" />
          <h3 className="text-lg font-medium text-blue-900 dark:text-blue-100">
            Matrix Collaboration
          </h3>
        </div>
        {!isCurrentlyJoined && (
          <Button
            onClick={handleRejoinRoom}
            disabled={isRejoining}
            size="sm"
            variant="outline"
            className="border-blue-300 text-blue-700 hover:bg-blue-100 dark:border-blue-600 dark:text-blue-300 dark:hover:bg-blue-800"
          >
            {isRejoining ? (
              <>
                <LoaderCircle className="w-4 h-4 mr-2 animate-spin" />
                Rejoining...
              </>
            ) : (
              <>
                <ExternalLink className="w-4 h-4 mr-2" />
                Rejoin Room
              </>
            )}
          </Button>
        )}
      </div>

      <div className="space-y-3">
        {/* Room Info */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
          <div>
            <div className="flex items-center space-x-2 text-blue-800 dark:text-blue-200">
              <Hash className="w-4 h-4" />
              <span className="font-medium">Room:</span>
            </div>
            <p className="text-blue-700 dark:text-blue-300 ml-6 font-mono text-xs">
              {metadata.name || matrixRoomId.substring(1, 20) + '...'}
            </p>
            {metadata.topic && (
              <p className="text-blue-600 dark:text-blue-400 ml-6 text-xs mt-1">
                {metadata.topic}
              </p>
            )}
          </div>

          <div>
            <div className="flex items-center space-x-2 text-blue-800 dark:text-blue-200">
              <Users className="w-4 h-4" />
              <span className="font-medium">Participants:</span>
            </div>
            <p className="text-blue-700 dark:text-blue-300 ml-6">
              {activeParticipants.length} active ({participantList.length} total)
            </p>
          </div>
        </div>

        {/* Membership Status */}
        <div className="flex items-center space-x-2 text-sm">
          <div className={`w-2 h-2 rounded-full ${
            isCurrentlyJoined 
              ? 'bg-green-500' 
              : 'bg-yellow-500'
          }`} />
          <span className="text-blue-800 dark:text-blue-200">
            Status: {isCurrentlyJoined ? 'Joined' : 'Not joined'}
          </span>
        </div>

        {/* Participants List */}
        {activeParticipants.length > 0 && (
          <div>
            <div className="flex items-center space-x-2 text-sm text-blue-800 dark:text-blue-200 mb-2">
              <Users className="w-4 h-4" />
              <span className="font-medium">Active Participants:</span>
            </div>
            <div className="flex flex-wrap gap-2 ml-6">
              {activeParticipants.slice(0, 5).map((participant) => (
                <div
                  key={participant.userId}
                  className="inline-flex items-center px-2 py-1 bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-200 text-xs rounded-full"
                >
                  {participant.displayName || participant.userId.split(':')[0].substring(1)}
                </div>
              ))}
              {activeParticipants.length > 5 && (
                <div className="inline-flex items-center px-2 py-1 bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-200 text-xs rounded-full">
                  +{activeParticipants.length - 5} more
                </div>
              )}
            </div>
          </div>
        )}

        {/* Last Activity */}
        {metadata.lastActivity && (
          <div className="flex items-center space-x-2 text-sm text-blue-600 dark:text-blue-400">
            <Clock className="w-4 h-4" />
            <span>Last activity: {formatMessageTimestamp(new Date(metadata.lastActivity))}</span>
          </div>
        )}
      </div>
    </div>
  );
};

interface SessionHistoryViewProps {
  session: Session;
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

const SessionMessages: React.FC<{
  messages: Message[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}> = ({ messages, isLoading, error, onRetry }) => {
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
                      sessionId: 'session-preview',
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
  const [isShareModalOpen, setIsShareModalOpen] = useState(false);
  const [shareLink, setShareLink] = useState<string>('');
  const [isSharing, setIsSharing] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [canShare, setCanShare] = useState(false);

  const messages = (session.conversation || []).map(convertApiMessageToFrontendMessage);

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
        session.working_dir,
        messages,
        session.description || 'Shared Session',
        session.total_tokens || 0
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
    try {
      resumeSession(session);
    } catch (error) {
      toast.error(`Could not launch session: ${error instanceof Error ? error.message : error}`);
    }
  };

  // Define action buttons
  const actionButtons = showActionButtons ? (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
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
        </TooltipTrigger>
        {!canShare ? (
          <TooltipContent>
            <p>
              To enable session sharing, go to <b>Settings</b> {'>'} <b>Session</b> {'>'}{' '}
              <b>Session Sharing</b>.
            </p>
          </TooltipContent>
        ) : null}
      </Tooltip>
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
            title={session.description || 'Session Details'}
            actionButtons={!isLoading ? actionButtons : null}
          >
            <div className="flex flex-col">
              {!isLoading ? (
                <>
                  <div className="flex items-center text-text-muted text-sm space-x-5 font-mono">
                    <span className="flex items-center">
                      <Calendar className="w-4 h-4 mr-1" />
                      {formatMessageTimestamp(messages[0]?.created)}
                    </span>
                    <span className="flex items-center">
                      <MessageSquareText className="w-4 h-4 mr-1" />
                      {session.message_count}
                    </span>
                    {session.total_tokens !== null && (
                      <span className="flex items-center">
                        <Target className="w-4 h-4 mr-1" />
                        {(session.total_tokens || 0).toLocaleString()}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center text-text-muted text-sm mt-1 font-mono">
                    <span className="flex items-center">
                      <Folder className="w-4 h-4 mr-1" />
                      {session.working_dir}
                    </span>
                  </div>

                  {/* Matrix Collaboration Info */}
                  <MatrixCollaborationInfo sessionId={session.id} />
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
            messages={messages}
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
