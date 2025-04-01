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
} from 'lucide-react';
import { type SessionDetails } from '../../sessions';
import { SessionHeaderCard, SessionMessages, formatDate } from './SessionViewComponents';
import { createSharedSession } from '../../sharedSessions';
import { Modal, ModalContent } from '../ui/modal';
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
        session.metadata.working_dir,
        session.messages,
        session.metadata.description || 'Shared Session',
        session.metadata.total_tokens
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
    <div className="h-screen w-full flex flex-col">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      {/* Top Row - back, info, reopen thread (fixed) */}
      <SessionHeaderCard onBack={onBack}>
        {/* Session info row */}
        <div className="ml-8">
          <h1 className="text-lg text-textStandardInverse">
            {session.metadata.description || session.session_id}
          </h1>
          <div className="flex items-center text-sm text-textSubtle mt-1 space-x-5">
            <span className="flex items-center">
              <Calendar className="w-4 h-4 mr-1" />
              {formatDate(session.messages[0]?.created)}
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
          <div className="flex items-center text-sm text-textSubtle space-x-5">
            <span className="flex items-center">
              <Folder className="w-4 h-4 mr-1" />
              {session.metadata.working_dir}
            </span>
          </div>
        </div>

        <div className="ml-auto flex items-center space-x-4">
          <button
            onClick={handleShare}
            disabled={!canShare || isSharing}
            className={`flex items-center text-textStandardInverse px-2 py-1 ${
              canShare
                ? 'hover:font-bold hover:scale-110 transition-all duration-150'
                : 'cursor-not-allowed opacity-50'
            }`}
          >
            {isSharing ? (
              <>
                <LoaderCircle className="w-7 h-7 animate-spin mr-2" />
                <span>Sharing...</span>
              </>
            ) : (
              <>
                <Share2 className="w-7 h-7" />
              </>
            )}
          </button>

          <button
            onClick={onResume}
            className="flex items-center text-textStandardInverse px-2 py-1 hover:font-bold hover:scale-110 transition-all duration-150"
          >
            <Sparkles className="w-7 h-7" />
          </button>
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
        <ModalContent className="sm:max-w-md p-0 bg-bgApp dark:bg-bgApp dark:border-borderSubtle">
          {/* Share Icon */}
          <div className="flex justify-center mt-4">
            <Share2 className="w-6 h-6 text-textStandard" />
          </div>

          {/* Centered Title */}
          <div className="mt-2 px-6 text-center">
            <h2 className="text-lg font-semibold text-textStandard">Share Session (beta)</h2>
          </div>

          {/* Description & Link */}
          <div className="px-6 flex flex-col gap-4 mt-2">
            <p className="text-sm text-center text-textSubtle">
              Share this session link to give others a read only view of your goose chat.
            </p>

            <div className="relative rounded-lg border border-borderSubtle px-3 py-2 flex items-center bg-gray-100 dark:bg-gray-600">
              <code className="text-sm text-textStandard dark:text-textStandardInverse overflow-x-hidden break-all pr-8 w-full">
                {shareLink}
              </code>
              <Button
                size="icon"
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

          {/* Footer */}
          <div>
            <Button
              type="button"
              variant="ghost"
              onClick={() => setIsShareModalOpen(false)}
              className="w-full h-[60px] border-t rounded-b-lg dark:border-gray-600 text-lg text-textStandard hover:bg-gray-100 hover:dark:bg-gray-600"
            >
              Cancel
            </Button>
          </div>
        </ModalContent>
      </Modal>
    </div>
  );
};

export default SessionHistoryView;
