import React from 'react';
import { Calendar, MessageSquareText, Folder, Target } from 'lucide-react';
import { type SharedSessionDetails } from '../../sharedSessions';
import { SessionHeaderCard, SessionMessages } from './SessionViewComponents';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MainPanelLayout } from '../Layout/MainPanelLayout';

interface SharedSessionViewProps {
  session: SharedSessionDetails | null;
  isLoading: boolean;
  error: string | null;
  onBack: () => void;
  onRetry: () => void;
}

const SharedSessionView: React.FC<SharedSessionViewProps> = ({
  session,
  isLoading,
  error,
  onBack,
  onRetry,
}) => {
  return (
    <MainPanelLayout>
      <div className="flex flex-col h-full">
        <div className="relative flex items-center h-14 w-full"></div>

        {/* Top Row - back, info (fixed) */}
        <SessionHeaderCard onBack={onBack}>
          {/* Session info row */}
          <div className="ml-8">
            <h1 className="text-lg text-textStandardInverse">
              {session ? session.description : 'Shared Session'}
            </h1>
            <div className="flex items-center text-sm text-textSubtle mt-1 space-x-5">
              <span className="flex items-center">
                <Calendar className="w-4 h-4 mr-1" />
                {session ? formatMessageTimestamp(session.messages[0]?.created) : 'Unknown'}
              </span>
              <span className="flex items-center">
                <MessageSquareText className="w-4 h-4 mr-1" />
                {session ? session.message_count : 0}
              </span>
              {session && session.total_tokens !== null && (
                <span className="flex items-center">
                  <Target className="w-4 h-4 mr-1" />
                  {session.total_tokens.toLocaleString()}
                </span>
              )}
            </div>
            <div className="flex items-center text-sm text-textSubtle space-x-5">
              <span className="flex items-center">
                <Folder className="w-4 h-4 mr-1" />
                {session ? session.working_dir : 'Unknown'}
              </span>
            </div>
          </div>
        </SessionHeaderCard>

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
