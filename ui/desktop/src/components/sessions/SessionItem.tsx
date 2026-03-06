import React from 'react';
import { Card } from '../ui/card';
import { formatDate } from '../../utils/date';
import { Session } from '../../api';
import { shouldShowNewChatTitle } from '../../sessions';
import { useLocalization } from '../../contexts/LocalizationContext';

interface SessionItemProps {
  session: Session;
  extraActions?: React.ReactNode;
}

const SessionItem: React.FC<SessionItemProps> = ({ session, extraActions }) => {
  const { t } = useLocalization();
  const displayName = shouldShowNewChatTitle(session) ? t('chat.newChat') : session.name;

  return (
    <Card className="p-4 mb-2 hover:bg-background-inverse/50 cursor-pointer flex justify-between items-center">
      <div>
        <div className="font-medium">{displayName}</div>
        <div className="text-sm text-muted-foreground">
          {formatDate(session.updated_at)} •{' '}
          {t('sessions.messagesCount', { count: session.message_count })}
        </div>
        <div className="text-sm text-muted-foreground">{session.working_dir}</div>
      </div>
      {extraActions && <div>{extraActions}</div>}
    </Card>
  );
};

export default SessionItem;
