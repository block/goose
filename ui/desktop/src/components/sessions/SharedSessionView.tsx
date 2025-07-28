import React from 'react';
import { Calendar, MessageSquareText, Folder, Target, LoaderCircle, Share2 } from 'lucide-react';
import { type SharedSessionDetails } from '../../sharedSessions';
import { SessionMessages } from './SessionViewComponents';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MainPanelLayout } from '../Layout/MainPanelLayout';

interface SharedSessionViewProps {
  session: SharedSessionDetails | null;
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}

// Custom SessionHeader component matching SessionHistoryView style
const SessionHeader: React.FC<{
  children: React.ReactNode;
  title: string;
}> = ({ children, title }) => {
  return (
    <div className="flex flex-col pb-8 border-b">
      <h1 className="text-4xl font-light mb-4 pt-6">{title}</h1>
      <div className="flex items-center">{children}</div>
    </div>
  );
};

const SharedSessionView: React.FC<SharedSessionViewProps> = ({
  session,
  isLoading,
  error,
  onRetry,
}) => {
  return (
    <MainPanelLayout>
      <div className="flex-1 flex flex-col min-h-0 px-8">
        <div className="flex items-center py-4 border-b border-border-subtle mb-6">
          <div className="flex items-center text-text-muted">
            <Share2 className="w-5 h-5 mr-2" />
            <span className="text-sm font-medium">Shared Session</span>
          </div>
        </div>

        <SessionHeader title={session ? session.description : 'Shared Session'}>
          <div className="flex flex-col">
            {!isLoading && session && session.messages.length > 0 ? (
              <>
                <div className="flex items-center text-text-muted text-sm space-x-5 font-mono">
                  <span className="flex items-center">
                    <Calendar className="w-4 h-4 mr-1" />
                    {formatMessageTimestamp(session.messages[0]?.created)}
                  </span>
                  <span className="flex items-center">
                    <MessageSquareText className="w-4 h-4 mr-1" />
                    {session.message_count}
                  </span>
                  {session.total_tokens !== null && (
                    <span className="flex items-center">
                      <Target className="w-4 h-4 mr-1" />
                      {session.total_tokens.toLocaleString()}
                    </span>
                  )}
                </div>
                <div className="flex items-center text-text-muted text-sm mt-1 font-mono">
                  <span className="flex items-center">
                    <Folder className="w-4 h-4 mr-1" />
                    {session.working_dir}
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
          messages={session?.messages || []}
          isLoading={isLoading}
          error={error}
          onRetry={onRetry}
        />
      </div>
    </MainPanelLayout>
  );
};

export default SharedSessionView;
