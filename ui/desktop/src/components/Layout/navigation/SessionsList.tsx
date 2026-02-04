import React from 'react';
import { MessageSquare, ChefHat } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { SessionIndicators } from '../../SessionIndicators';
import { cn } from '../../../utils';
import { getSessionDisplayName, truncateMessage } from '../../../hooks/useNavigationSessions';
import type { Session } from '../../../api';
import type { SessionStatus } from '../../../hooks/useSidebarSessionStatus';

interface SessionsListProps {
  sessions: Session[];
  activeSessionId?: string;
  isExpanded: boolean;
  getSessionStatus: (sessionId: string) => SessionStatus | undefined;
  clearUnread: (sessionId: string) => void;
  onSessionClick: (sessionId: string) => void;
}

export const SessionsList: React.FC<SessionsListProps> = ({
  sessions,
  activeSessionId,
  isExpanded,
  getSessionStatus,
  clearUnread,
  onSessionClick,
}) => {
  return (
    <AnimatePresence>
      {isExpanded && (
        <motion.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: 'auto', opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{ duration: 0.2 }}
          className="overflow-hidden mt-[2px]"
        >
          <div className="bg-background-default rounded-lg py-1 flex flex-col gap-[2px]">
            {sessions.map((session) => {
              const status = getSessionStatus(session.id);
              const isStreaming = status?.streamState === 'streaming';
              const hasError = status?.streamState === 'error';
              const hasUnread = status?.hasUnreadActivity ?? false;
              const isActiveSession = session.id === activeSessionId;

              return (
                <button
                  key={session.id}
                  onClick={() => {
                    clearUnread(session.id);
                    onSessionClick(session.id);
                  }}
                  className={cn(
                    'w-full text-left py-1.5 px-2 text-xs rounded-md',
                    'hover:bg-background-medium transition-colors',
                    'flex items-center gap-2',
                    isActiveSession && 'bg-background-medium'
                  )}
                >
                  <div className="w-4 flex-shrink-0" />
                  {session.recipe ? (
                    <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                  ) : (
                    <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                  )}
                  <span className="truncate text-text-default flex-1">
                    {truncateMessage(getSessionDisplayName(session))}
                  </span>
                  <SessionIndicators
                    isStreaming={isStreaming}
                    hasUnread={hasUnread}
                    hasError={hasError}
                  />
                </button>
              );
            })}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
