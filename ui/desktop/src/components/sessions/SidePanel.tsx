import React, { useEffect, useState } from 'react';
import { Plus, ChevronLeft } from 'lucide-react';
import { fetchSessions, type Session } from '../../sessions';
import { ScrollArea } from '../ui/scroll-area';
import { Button } from '../ui/button';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';

interface SidePanelProps {
  onSelectSession: (session: Session) => void;
  onNewChat: () => void;
  onHide: () => void;
}

const SidePanel: React.FC<SidePanelProps> = ({ onSelectSession, onNewChat, onHide }) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    const load = async () => {
      try {
        const s = await fetchSessions();
        setSessions(s);
      } catch (err) {
        console.error('Failed to load sessions for side panel', err);
      }
    };
    load();
  }, []);

  const handleOpenSession = (session: Session) => {
    onSelectSession(session);
  };

  const filteredSessions = sessions.filter((session) => {
    const term = searchTerm.trim().toLowerCase();
    if (!term) return true;
    const description = session.metadata.description?.toLowerCase() ?? '';
    const id = session.id.toLowerCase();
    return description.includes(term) || id.includes(term);
  });

  return (
    <div className="h-full w-60 sm:w-64 lg:w-72 flex flex-col border-r border-borderSubtle bg-bgSecondary">
      {/* Header with hide & new chat buttons */}
      <div className="h-14 px-3 border-b border-borderSubtle flex items-center gap-2">
        {/* Spacer to clear macOS traffic-light window controls */}
        <div className="w-20 shrink-0" />
        <h2 className="flex-1 text-lg font-semibold text-textStandard">Chats</h2>
        <TooltipProvider>
          <div className="flex items-center gap-1 shrink-0">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="New chat"
                  onClick={onNewChat}
                  className="border border-borderSubtle hover:border-borderStandard rounded-lg p-2 hover:bg-bgSubtle"
                >
                  <Plus className="h-4 w-4 text-textStandard dark:text-textStandard" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>New Chat</p>
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="Hide sidebar"
                  onClick={onHide}
                  className="border border-borderSubtle hover:border-borderStandard rounded-lg p-2 hover:bg-bgSubtle"
                >
                  <ChevronLeft className="h-4 w-4 text-textStandard dark:text-textStandard" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>Hide Sidebar</p>
              </TooltipContent>
            </Tooltip>
          </div>
        </TooltipProvider>
      </div>

      {/* Search bar */}
      <div className="p-3 border-b border-borderSubtle">
        <input
          type="text"
          placeholder="Search chats…"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="w-full text-sm px-3 py-2 rounded-md bg-bgApp placeholder:text-textSubtle text-textStandard dark:text-textStandard border border-borderSubtle hover:border-borderStandard hover:bg-bgSubtle transition-colors focus:border-ring focus:outline-none"
        />
      </div>

      {/* Sessions list */}
      <ScrollArea className="flex-1">
        {filteredSessions.map((session) => (
          <div
            key={session.id}
            className="px-4 py-3 hover:bg-bgSubtle cursor-pointer border-b border-borderSubtle"
            onClick={() => handleOpenSession(session)}
          >
            <div className="text-sm font-medium truncate text-textStandard">
              {session.metadata.description || session.id}
            </div>
            <div className="text-xs text-textSubtle">
              {formatMessageTimestamp(Date.parse(session.modified) / 1000)}
            </div>
          </div>
        ))}
      </ScrollArea>
    </div>
  );
};

export default SidePanel;
