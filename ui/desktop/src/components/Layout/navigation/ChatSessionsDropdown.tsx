import React from 'react';
import { MessageSquare, History, Plus, ChefHat } from 'lucide-react';
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from '../../ui/dropdown-menu';
import { SessionIndicators } from '../../SessionIndicators';
import { cn } from '../../../utils';
import { getSessionDisplayName, truncateMessage } from '../../../hooks/useNavigationSessions';
import type { Session } from '../../../api';
import type { SessionStatus } from './types';
import { useLocalization } from '../../../contexts/LocalizationContext';

interface ChatSessionsDropdownProps {
  sessions: Session[];
  activeSessionId?: string;
  side?: 'top' | 'bottom' | 'left' | 'right';
  zIndex?: number;
  getSessionStatus: (sessionId: string) => SessionStatus | undefined;
  clearUnread: (sessionId: string) => void;
  onNewChat: () => void;
  onSessionClick: (sessionId: string) => void;
  onShowAll: () => void;
}

export const ChatSessionsDropdown: React.FC<ChatSessionsDropdownProps> = ({
  sessions,
  activeSessionId,
  side = 'right',
  zIndex,
  getSessionStatus,
  clearUnread,
  onNewChat,
  onSessionClick,
  onShowAll,
}) => {
  const { t } = useLocalization();

  return (
    <DropdownMenuContent
      className="w-64 p-1 bg-background-primary border-border-secondary rounded-lg shadow-lg"
      side={side}
      align="start"
      sideOffset={8}
      style={zIndex ? { zIndex } : undefined}
    >
      <DropdownMenuItem
        onClick={onNewChat}
        className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
      >
        <Plus className="w-4 h-4 flex-shrink-0" />
        <span>{t('chat.newChat')}</span>
      </DropdownMenuItem>

      {sessions.length > 0 && <DropdownMenuSeparator className="my-1" />}

      {sessions.map((session) => {
        const status = getSessionStatus(session.id);
        const isStreaming = status?.streamState === 'streaming';
        const hasError = status?.streamState === 'error';
        const hasUnread = status?.hasUnreadActivity ?? false;
        const isActiveSession = session.id === activeSessionId;

        return (
          <DropdownMenuItem
            key={session.id}
            onClick={() => {
              clearUnread(session.id);
              onSessionClick(session.id);
            }}
            className={cn(
              'flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer',
              isActiveSession && 'bg-background-tertiary'
            )}
          >
            {session.recipe ? (
              <ChefHat className="w-4 h-4 flex-shrink-0 text-text-secondary" />
            ) : (
              <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-secondary" />
            )}
            <span className="truncate flex-1">
              {truncateMessage(getSessionDisplayName(session, t('chat.newChat')), 30)}
            </span>
            <SessionIndicators
              isStreaming={isStreaming}
              hasUnread={hasUnread}
              hasError={hasError}
            />
          </DropdownMenuItem>
        );
      })}

      {sessions.length > 0 && (
        <>
          <DropdownMenuSeparator className="my-1" />
          <DropdownMenuItem
            onClick={onShowAll}
            className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-secondary"
          >
            <History className="w-4 h-4 flex-shrink-0" />
            <span>{t('chat.showAll')}</span>
          </DropdownMenuItem>
        </>
      )}
    </DropdownMenuContent>
  );
};
