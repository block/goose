import React, { useState, useEffect } from 'react';
import { CLIChatView } from './CLIChatView';
import { useNavigate } from 'react-router-dom';
import { Button } from '../ui/button';
import { Badge } from '../ui/badge';
import { Terminal, Settings, History, MessageSquare, ArrowLeft, RefreshCw } from 'lucide-react';
import { generateSessionId } from '../../sessions';
import { type View, ViewOptions } from '../../App';
import { MainPanelLayout } from '../Layout/MainPanelLayout';

interface ChatMessage {
  role: string;
  content: string;
  id?: string;
  timestamp?: number;
  [key: string]: unknown;
}

interface ChatState {
  id: string;
  title: string;
  messageHistoryIndex: number;
  messages: ChatMessage[];
}

interface CLIHubProps {
  chat: ChatState;
  setChat: (chat: ChatState) => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

export const CLIHub: React.FC<CLIHubProps> = ({ chat, setChat, setView }) => {
  const navigate = useNavigate();
  const [sessionId, setSessionId] = useState(chat.id || generateSessionId());

  // Update chat when session changes
  useEffect(() => {
    setChat({
      ...chat,
      id: sessionId,
      title: `CLI Session - ${sessionId}`,
      messageHistoryIndex: 0,
      messages: [],
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId, setChat]);

  const handleNewSession = () => {
    const newSessionId = generateSessionId();
    setSessionId(newSessionId);
  };

  const handleRestartSession = () => {
    // Force restart by changing session ID
    const newSessionId = generateSessionId();
    setSessionId(newSessionId);
  };

  return (
    <MainPanelLayout>
      {/* Header */}
      <div className="h-12 flex items-center justify-between">
        <div className="flex items-center pr-4">
          <Button variant="ghost" size="sm" onClick={() => navigate('/')} className="mr-2">
            <ArrowLeft className="w-4 h-4" />
          </Button>

          <Terminal className="w-5 h-5 mr-2" />
          <div>
            <h1 className="text-lg font-semibold">Goose CLI Experience</h1>
            <p className="text-xs text-muted-foreground">
              Exact CLI behavior with GUI enhancements
            </p>
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <Badge variant="outline" className="text-xs">
            CLI Mode
          </Badge>

          <Button variant="outline" size="sm" onClick={handleNewSession}>
            <MessageSquare className="w-4 h-4 mr-2" />
            New Session
          </Button>

          <Button variant="outline" size="sm" onClick={handleRestartSession}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Restart
          </Button>

          <Button variant="outline" size="sm" onClick={() => setView('settings')}>
            <Settings className="w-4 h-4 mr-2" />
            Settings
          </Button>
        </div>
      </div>

      {/* CLI Chat View */}
      <div className="flex flex-col min-w-0 flex-1 overflow-y-auto relative pl-6 pr-4 pb-16 pt-2">
        <CLIChatView sessionId={sessionId} />
      </div>

      {/* Footer */}
      <div className="relative z-10 p-4 border-t bg-muted/30">
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <div className="flex items-center space-x-4">
            <span>Session: {sessionId}</span>
            <span>•</span>
            <span>CLI Mode Active</span>
          </div>

          <div className="flex items-center space-x-4">
            <Button variant="ghost" size="sm" onClick={() => setView('sessions')}>
              <History className="w-4 h-4 mr-2" />
              Sessions
            </Button>

            <Button variant="ghost" size="sm" onClick={() => setView('settings')}>
              <Settings className="w-4 h-4 mr-2" />
              Settings
            </Button>
          </div>
        </div>
      </div>
    </MainPanelLayout>
  );
};
