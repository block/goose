import React, { useEffect, useState } from "react"

const useCombinedSessions = () => {
  const [sessions, setSessions] = useState([]);

  useEffect(() => {
    async function loadSessions() {
      const sessions = await window.electron.listSessions();
      setSessions(sessions);
    }
    loadSessions();
  }, []);

  const getCombinedSessions = () => {
    if (sessions.length === 0) {
      return [];
    }

    const allSessions = [];
    const seenNames = new Set();

    // Process regular sessions
    for (const session of sessions) {
      if (!seenNames.has(session.name)) {
        allSessions.push({ ...session, isLatest: false });
        seenNames.add(session.name);
      }
    }

    // Sort sessions by name
    return allSessions.sort((a, b) => a.name.localeCompare(b.name));
  };

  return getCombinedSessions();
};

export default function SessionPills() {
  const allSessions = useCombinedSessions();

  if (allSessions.length === 0) {
    return (
      <div className="text-center text-splash-pills-text text-14 mt-4">
        No previous sessions found
      </div>
    );
  }

  const SessionPill = ({ session }) => (
    <div
      key={session.directory + session.name}
      className="w-[312px] px-16 py-4 mb-2 text-center text-splash-pills-text whitespace-nowrap cursor-pointer bg-prev-goose-gradient text-prev-goose-text rounded-[14px] inline-block hover:scale-[1.02] transition-all duration-150"
      onClick={async () => {
        window.electron.createChatWindow(undefined, session.directory, session.name);
      }}
      title={session.directory}
    >
      <div className="text-14">{session.name}</div>
      <div className="text-xs opacity-70 mt-1 truncate">{session.directory}</div>
    </div>
  );

  return (
    <div className="grid grid-cols-1 gap-2 mt-4 max-h-[80vh] overflow-y-auto px-4">
      <h3 className="text-11 text-splash-pills-text mb-2 text-center">Previous Sessions</h3>
      {allSessions.map((session) => (
        <SessionPill key={session.directory + session.name} session={session} />
      ))}
    </div>
  );
}
