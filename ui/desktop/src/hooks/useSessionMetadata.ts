import { useState, useEffect, useCallback } from 'react';
import { fetchSessionDetails } from '../sessions';

interface UseSessionMetadataReturn {
  sessionName: string | null;
  isSessionNameSet: boolean;
  refreshSessionName: () => Promise<void>;
  updateSessionName: (newName: string) => void;
}

/**
 * Custom hook to manage session metadata
 * Only shows session name if it's been set (not just the session ID)
 */
export function useSessionMetadata(sessionId: string): UseSessionMetadataReturn {
  const [sessionName, setSessionName] = useState<string | null>(null);
  const [isSessionNameSet, setIsSessionNameSet] = useState(false);

  const refreshSessionName = useCallback(async () => {
    if (!sessionId) return;

    try {
      const sessionDetails = await fetchSessionDetails(sessionId);
      const description = sessionDetails.metadata.description;

      // Only set the session name if it's different from the session ID
      // This indicates it's been auto-generated or user-set, not just the default ID
      if (description && description !== sessionId) {
        setSessionName(description);
        setIsSessionNameSet(true);
      } else {
        setSessionName(null);
        setIsSessionNameSet(false);
      }
    } catch (error) {
      console.error('Error fetching session metadata:', error);
      setSessionName(null);
      setIsSessionNameSet(false);
    }
  }, [sessionId]);

  const updateSessionName = (newName: string) => {
    setSessionName(newName);
    setIsSessionNameSet(true);
  };

  // Initial fetch
  useEffect(() => {
    refreshSessionName();
  }, [sessionId, refreshSessionName]);

  return {
    sessionName,
    isSessionNameSet,
    refreshSessionName,
    updateSessionName,
  };
}
