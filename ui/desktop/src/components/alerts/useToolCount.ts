import { useState, useEffect } from 'react';
import { getTools } from '../../api';

// TODO(Douwe): return this as part of the start agent request
export const useToolCount = (sessionId: string) => {
  const [toolCount, setToolCount] = useState<number | null>(null);

  useEffect(() => {
    const fetchTools = async () => {
      try {
        const response = await getTools({ query: { session_id: sessionId } });
        setToolCount(response.error || !response.data ? 0 : response.data.length);
      } catch (err) {
        console.error('Error fetching tools:', err);
        setToolCount(0);
      }
    };

    fetchTools();
  }, [sessionId]);

  return toolCount;
};
