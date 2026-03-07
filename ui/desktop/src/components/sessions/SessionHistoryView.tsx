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
import { resumeSession } from '../../sessions';
import { Button } from '../ui/button';
import { toast } from 'react-toastify';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { ScrollArea } from '../ui/scroll-area';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { createSharedSession } from '../../sharedSessions';
import { errorMessage } from '../../utils/conversionUtils';
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
import BackButton from '../ui/BackButton';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
import { Message, Session } from '../../api';
import { useNavigation } from '../../hooks/useNavigation';
import { useLocalization } from '../../contexts/LocalizationContext';

const isUserMessage = (message: Message): boolean => {
  if (message.role === 'assistant') {
    return false;
  }
  return !message.content.every(
    (c) => c.type === 'actionRequired' && c.data.actionType === 'toolConfirmation'
  );
};

const filterMessagesForDisplay = (messages: Message[]): Message[] => {
  return messages;
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
  const { t } = useLocalization();
  const filteredMessages = filterMessagesForDisplay(messages);

  return (
    <ScrollArea className="h-full w-full">
      <div className="pb-24 pt-8">
        <div className="flex flex-col space-y-6">
          {isLoading ? (
            <div className="flex justify-center items-center py-12">
              <LoaderCircle className="animate-spin h-8 w-8 text-text-primary" />
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-8 text-text-secondary">
              <div className="text-red-500 mb-4">
                <AlertCircle size={32} />
              </div>
              <p className="text-md mb-2">{t('sessions.history.errorTitle')}</p>
              <p className="text-sm text-center mb-4">{error}</p>
              <Button onClick={onRetry} variant="default">
                {t('common.actions.retry')}
              </Button>
            </div>
          ) : filteredMessages?.length > 0 ? (
            <div className="max-w-4xl mx-auto w-full">
              <SearchView placeholder={t('sessions.searchPlaceholder')}>
                <ProgressiveMessageList
                  messages={filteredMessages}
                  chat={{
                    sessionId: 'session-preview',
                  }}
                  toolCallNotifications={new Map()}
                  append={() => {}} // Read-only for session history
                  isUserMessage={isUserMessage} // Use the same function as BaseChat
                  batchSize={15} // Same as BaseChat default
                  batchDelay={30} // Same as BaseChat default
                  showLoadingThreshold={30} // Same as BaseChat default
                />
              </SearchView>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-text-secondary">
              <MessageSquareText className="w-12 h-12 mb-4" />
              <p className="text-lg mb-2">{t('sessions.history.noMessages')}</p>
              <p className="text-sm">{t('sessions.history.noMessagesDescription')}</p>
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
  const { t } = useLocalization();
  const [isShareModalOpen, setIsShareModalOpen] = useState(false);
  const [shareLink, setShareLink] = useState<string>('');
  const [isSharing, setIsSharing] = useState(false);
  const [isCopied, setIsCopied] = useState(false);
  const [canShare, setCanShare] = useState(false);

  const messages = session.conversation || [];

  const setView = useNavigation();

  useEffect(() => {
    window.electron.getSetting('sessionSharing').then((config) => {
      if (config.enabled && config.baseUrl) {
        setCanShare(true);
      }
    });
  }, []);

  const handleShare = async () => {
    setIsSharing(true);

    try {
      const config = await window.electron.getSetting('sessionSharing');
      if (!config.enabled || !config.baseUrl) {
        throw new Error('Session sharing is not enabled or base URL is not configured.');
      }

      const shareToken = await createSharedSession(
        config.baseUrl,
        session.working_dir,
        messages,
        session.name || t('sessions.sharedSession'),
        session.total_tokens || 0
      );

      const shareableLink = `goose://sessions/${shareToken}`;
      setShareLink(shareableLink);
      setIsShareModalOpen(true);
    } catch (error) {
      console.error('Error sharing session:', error);
      toast.error(
        t('sessions.history.shareFailed', {
          error: errorMessage(error, t('common.labels.unknown')),
        })
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
        toast.error(t('sessions.history.copyLinkFailed'));
      });
  };

  const handleResumeSession = () => {
    try {
      resumeSession(session, setView);
    } catch (error) {
      toast.error(`Could not launch session: ${errorMessage(error)}`);
    }
  };

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
                {t('common.labels.loading')}
              </>
            ) : (
              <>
                <Share2 className="w-4 h-4" />
                {t('sessions.history.share')}
              </>
            )}
          </Button>
        </TooltipTrigger>
        {!canShare ? (
          <TooltipContent>
            <p>{t('sessions.history.shareDisabledHint')}</p>
          </TooltipContent>
        ) : null}
      </Tooltip>
      <Button onClick={handleResumeSession} size="sm" variant="outline">
        <Sparkles className="w-4 h-4" />
        {t('sessions.history.resume')}
      </Button>
    </>
  ) : null;

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0 px-8">
          <SessionHeader
            onBack={onBack}
            title={session.name}
            actionButtons={!isLoading ? actionButtons : null}
          >
            <div className="flex flex-col">
              {!isLoading ? (
                <>
                  <div className="flex items-center text-text-secondary text-sm space-x-5 font-mono">
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
                  <div className="flex items-center text-text-secondary text-sm mt-1 font-mono">
                    <span className="flex items-center">
                      <Folder className="w-4 h-4 mr-1" />
                      {session.working_dir}
                    </span>
                  </div>
                </>
              ) : (
                <div className="flex items-center text-text-secondary text-sm">
                  <LoaderCircle className="w-4 h-4 mr-2 animate-spin" />
                  <span>{t('sessions.history.loadingDetails')}</span>
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
              <Share2 className="w-6 h-6 text-text-primary" />
              {t('sessions.history.shareBeta')}
            </DialogTitle>
            <DialogDescription>
              {t('sessions.history.shareDescription')}
            </DialogDescription>
          </DialogHeader>

          <div className="py-4">
            <div className="relative rounded-full border border-border-primary px-3 py-2 flex items-center bg-gray-100 dark:bg-gray-600">
              <code className="text-sm text-text-primary dark:text-text-inverse overflow-x-hidden break-all pr-8 w-full">
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
                <span className="sr-only">{t('common.actions.copy')}</span>
              </Button>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setIsShareModalOpen(false)}>
              {t('common.actions.cancel')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export default SessionHistoryView;
