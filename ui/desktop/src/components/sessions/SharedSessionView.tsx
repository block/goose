import React from 'react';
import { Calendar, MessageSquareText, Folder, Target } from 'lucide-react';
import { type SharedSessionDetails } from '../../sharedSessions';
import { SessionHeaderCard, SessionMessages } from './SessionViewComponents';
import { formatMessageTimestamp } from '../../utils/timeUtils';

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
    <div className="h-screen w-full flex flex-col">
      <div className="relative flex items-center h-[36px] w-full"></div>

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
          <div className="flex items-center text-sm text-textSubtle space-x-5">
            <span className="flex items-center">
              <Folder className="w-4 h-4 mr-1" />
              {session.working_dir}
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
  );
};

export default SharedSessionView;
