import React, { useCallback } from 'react';
import SessionListView from './SessionListView';
import { useNavigation } from '../../hooks/useNavigation';

const SessionsView: React.FC = () => {
  const setView = useNavigation();

  const handleSelectSession = useCallback(
    async (sessionId: string) => {
      setView('pair', {
        disableAnimation: true,
        resumeSessionId: sessionId,
      });
    },
    [setView]
  );

  return <SessionListView onSelectSession={handleSelectSession} />;
};

export default SessionsView;
