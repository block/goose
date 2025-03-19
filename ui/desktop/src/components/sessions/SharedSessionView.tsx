import React from 'react';
import { Clock, Globe } from 'lucide-react';
import { type SharedSessionDetails } from '../../sharedSessions';
import { SessionHeaderCard, SessionMessages } from './SessionViewComponents';

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
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      {/* Top Row - back, info (fixed) */}
      <SessionHeaderCard onBack={onBack}>
        {/* Session info row */}
        <div className="ml-8">
          <h1 className="text-lg font-bold text-textStandard">
            {session ? session.description : 'Shared Session'}
          </h1>
          {session && (
            <div className="flex items-center text-sm text-textSubtle mt-2 space-x-4">
              <span className="flex items-center">
                <Clock className="w-4 h-4 mr-1" />
                {new Date(session.messages[0]?.created * 1000).toLocaleString()}
              </span>
              <span className="flex items-center">
                <Globe className="w-4 h-4 mr-1" />
                {session.base_url}
              </span>
            </div>
          )}
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
