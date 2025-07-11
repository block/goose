import React from 'react';
import { Session } from '../../sessions';
import { Card } from '../ui/card';
import { formatDate } from '../../utils/date';

interface SessionItemProps {
  session: Session;
  extraActions?: React.ReactNode;
}

const SessionItem: React.FC<SessionItemProps> = ({ session, extraActions }) => {
  return (
    <Card className="p-4 mb-2 hover:bg-accent/50 cursor-pointer flex justify-between items-center">
      <div>
        <div className="font-medium">{session.metadata.description || `Session ${session.id}`}</div>
        <div className="text-sm text-muted-foreground">
          {formatDate(session.modified)} • {session.metadata.message_count} messages
        </div>
        <div className="text-sm text-muted-foreground">{session.metadata.working_dir}</div>
      </div>
      {extraActions && <div>{extraActions}</div>}
    </Card>
  );
};

export default SessionItem;
