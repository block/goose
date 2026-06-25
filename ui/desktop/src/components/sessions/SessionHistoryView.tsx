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
import { defineMessages, useIntl } from '../../i18n';
import { resumeSession } from '../../sessions';
import { Button } from '../ui/button';
import { toast } from 'react-toastify';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { ScrollArea } from '../ui/scroll-area';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { errorMessage } from '../../utils/conversionUtils';
import ProgressiveMessageList from '../ProgressiveMessageList';
import { SearchView } from '../conversation/SearchView';
import BackButton from '../ui/BackButton';
import { Message, Session } from '../../api';
import { useNavigation } from '../../hooks/useNavigation';

const i18n = defineMessages({
  errorLoadingDetails: {
    id: 'sessionHistory.error.loading',
    defaultMessage: 'Error Loading Session Details',
  },
  tryAgain: {
    id: 'sessionHistory.error.tryAgain',
    defaultMessage: 'Try Again',
  },
  searchPlaceholder: {
    id: 'sessionHistory.searchPlaceholder',
    defaultMessage: 'Search history...',
  },
  noMessages: {
    id: 'sessionHistory.empty.title',
    defaultMessage: 'No messages found',
  },
  noMessagesDesc: {
    id: 'sessionHistory.empty.description',
    defaultMessage: "This session doesn't contain any messages",
  },
  loadingDetails: {
    id: 'sessionHistory.loading',
    defaultMessage: 'Loading session details...',
  },
  resume: {
    id: 'sessionHistory.resume',
    defaultMessage: 'Resume',
  },
  couldNotLaunch: {
    id: 'sessionHistory.toast.launchFailed',
    defaultMessage: 'Could not launch session: {error}',
  },
});

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
  const intl = useIntl();
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
              <p className="text-md mb-2">{intl.formatMessage(i18n.errorLoadingDetails)}</p>
              <p className="text-sm text-center mb-4">{error}</p>
              <Button onClick={onRetry} variant="default">
                {intl.formatMessage(i18n.tryAgain)}
              </Button>
            </div>
          ) : filteredMessages?.length > 0 ? (
            <div className="max-w-4xl mx-auto w-full">
              <SearchView placeholder={intl.formatMessage(i18n.searchPlaceholder)}>
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
              <p className="text-lg mb-2">{intl.formatMessage(i18n.noMessages)}</p>
              <p className="text-sm">{intl.formatMessage(i18n.noMessagesDesc)}</p>
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
  const intl = useIntl();
  const messages = session.conversation || [];

  const setView = useNavigation();

  const handleResumeSession = () => {
    try {
      resumeSession(session, setView);
    } catch (error) {
      toast.error(intl.formatMessage(i18n.couldNotLaunch, { error: errorMessage(error) }));
    }
  };

  const actionButtons = showActionButtons ? (
    <Button onClick={handleResumeSession} size="sm" variant="outline">
      <Sparkles className="w-4 h-4" />
      {intl.formatMessage(i18n.resume)}
    </Button>
  ) : null;

  return (
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
                  {session.usage?.total_tokens != null && (
                    <span className="flex items-center">
                      <Target className="w-4 h-4 mr-1" />
                      {session.usage.total_tokens.toLocaleString()}
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
                <span>{intl.formatMessage(i18n.loadingDetails)}</span>
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
  );
};

export default SessionHistoryView;
