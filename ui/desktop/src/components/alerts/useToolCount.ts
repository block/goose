import { useState, useEffect } from 'react';
import { getTools } from '../../api';

const { clearTimeout } = window;

export const useToolCount = (sessionId: string) => {
  const [toolCount, setToolCount] = useState<number | null>(null);

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout>;

    const fetchTools = async () => {
      try {
        const response = await getTools({ query: { session_id: sessionId } });
        if (!response.error && response.data) {
          setToolCount(response.data.length);
        } else {
          setToolCount(0);
        }
      } catch (err) {
        console.error('Error fetching tools:', err);
        setToolCount(0);
      }
    };

    // Add initial 1s delay before first fetch
    timeoutId = setTimeout(fetchTools, 1000);

    // Cleanup timeouts on unmount
    return () => {
      clearTimeout(timeoutId);
    };
  }, [sessionId]);

  return toolCount;
};
