import React, { useState, useEffect } from 'react';
import { Clock, MessageSquare, Folder, Share, Copy, Check, LoaderCircle } from 'lucide-react';
import { type SessionDetails } from '../../sessions';
import { SessionHeaderCard, SessionMessages } from './SessionViewComponents';
import { createSharedSession } from '../../sharedSessions';
import { Modal, ModalContent, ModalHeader, ModalTitle, ModalFooter } from '../ui/modal';
import { Button } from '../ui/button';
import { toast } from 'react-toastify';

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
  const [isShareModalOpen, setIsShareModalOpen] = useState(false);
  const [shareLink, setShareLink] = useState<string>('');
  const [isSharing, setIsSharing] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [canShare, setCanShare] = useState(false);
  const [shareError, setShareError] = useState<string | null>(null);

  useEffect(() => {
    const savedSessionConfig = localStorage.getItem('session_sharing_config');
    if (savedSessionConfig) {
      try {
        const config = JSON.parse(savedSessionConfig);
        // If config.enabled is true and config.baseUrl is non-empty, we can share
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
    setShareError(null);

    try {
      // Get the session sharing configuration from localStorage
      const savedSessionConfig = localStorage.getItem('session_sharing_config');
      if (!savedSessionConfig) {
        throw new Error('Session sharing is not configured. Please configure it in settings.');
      }

      const config = JSON.parse(savedSessionConfig);
      if (!config.enabled || !config.baseUrl) {
        throw new Error('Session sharing is not enabled or base URL is not configured.');
      }

      // Create a shared session
      const shareToken = await createSharedSession(
        config.baseUrl,
        session.messages,
        session.metadata.description || 'Shared Session'
      );

      // Create the shareable link
      const shareableLink = `goose://sessions/${shareToken}`;
      setShareLink(shareableLink);
      setIsShareModalOpen(true);
    } catch (error) {
      console.error('Error sharing session:', error);
      setShareError(error instanceof Error ? error.message : 'Unknown error occurred');
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

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      {/* Top Row - back, info, reopen thread (fixed) */}
      <SessionHeaderCard onBack={onBack}>
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
              <Folder className="w-4 h-4 mr-1" />
              {session.metadata.working_dir}
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

        <div className="ml-auto flex items-center space-x-4">
          <button
            onClick={handleShare}
            disabled={!canShare || isSharing}
            className={`flex items-center text-textStandard px-3 py-1 border rounded-md ${
              canShare
                ? 'border-primary hover:text-primary hover:font-bold hover:scale-105 transition-all duration-150'
                : 'border-gray-300 cursor-not-allowed opacity-50'
            }`}
          >
            {isSharing ? (
              <>
                <LoaderCircle className="w-5 h-5 animate-spin mr-2" />
                <span>Sharing...</span>
              </>
            ) : (
              <>
                <Share className="w-5 h-5" />
              </>
            )}
          </button>

          <span
            onClick={onResume}
            className="text-md cursor-pointer text-textStandard hover:font-bold hover:scale-105 transition-all duration-150"
          >
            Resume Session
          </span>
        </div>
      </SessionHeaderCard>

      <SessionMessages
        messages={session.messages}
        isLoading={isLoading}
        error={error}
        onRetry={onRetry}
      />

      {/* Share Link Modal */}
      <Modal open={isShareModalOpen} onOpenChange={setIsShareModalOpen}>
        <ModalContent className="sm:max-w-md dark:bg-black">
          <ModalHeader>
            <ModalTitle className="text-textStandard">Share Session</ModalTitle>
          </ModalHeader>
          <div className="flex flex-col gap-2 mt-2">
            <div className="flex items-center gap-2">
              <div className="flex-1 p-2 rounded-md overflow-x-auto">
                <code className="text-sm text-textStandard">{shareLink}</code>
              </div>
              <Button
                size="sm"
                className="flex-shrink-0"
                onClick={handleCopyLink}
                disabled={isCopied}
              >
                {isCopied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
                <span className="sr-only">Copy</span>
              </Button>
            </div>
            <p className="text-sm text-textSubtle">
              Share this link with others to give them access to this session.
              <br />
              They will need to have Goose installed and session sharing configured.
            </p>
          </div>
          <ModalFooter className="sm:justify-start">
            <Button
              type="button"
              variant="ghost"
              onClick={() => setIsShareModalOpen(false)}
              className="hover:text-textStandard border border-borderSubtle text-textStandard hover:bg-bgSubtle"
            >
              Close
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
};

export default SessionHistoryView;
