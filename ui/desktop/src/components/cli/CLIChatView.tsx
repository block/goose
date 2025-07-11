import React from 'react';

interface CLIChatViewProps {
  sessionId: string;
  // onSessionExit: () => void;
}

export const CLIChatView: React.FC<CLIChatViewProps> = ({ sessionId }) => {
  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 p-4">
        <p className="text-muted-foreground">CLI Chat View for session: {sessionId}</p>
        <p className="text-sm text-muted-foreground mt-2">This component is under development.</p>
      </div>
    </div>
  );
};
