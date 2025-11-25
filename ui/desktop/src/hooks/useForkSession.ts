import { useState } from 'react';
import { toast } from 'react-toastify';
import { forkSession } from '../api';

export function useForkSession() {
  const [isForking, setIsForking] = useState(false);

  const forkAndOpenWindow = async (sessionId: string) => {
    if (isForking) return;
    setIsForking(true);
    try {
      const response = await forkSession({ path: { session_id: sessionId }, throwOnError: true });
      const forkedSession = response.data;
      if (!forkedSession) {
        throw new Error('Fork response missing session data');
      }
      toast.success(`Forked session: ${forkedSession.name}`);
      window.electron.createChatWindow(
        undefined,
        forkedSession.working_dir,
        undefined,
        forkedSession.id
      );
      return forkedSession;
    } catch (error) {
      toast.error(
        `Failed to fork session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
      return undefined;
    } finally {
      setIsForking(false);
    }
  };

  return { forkAndOpenWindow, isForking };
}
