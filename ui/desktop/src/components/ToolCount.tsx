import React, { useState, useEffect } from 'react';
import { getTools } from '../api';
import { ExclamationTriangleIcon } from '@radix-ui/react-icons';

const SUGGESTED_MAX_TOOLS = 15;

export default function ToolCount() {
  const [toolCount, setToolCount] = useState(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    const fetchTools = async () => {
      try {
        const response = await getTools();
        if (response.error) {
          console.error('failed to get tool count');
          setError(true);
        } else {
          setToolCount(response.data.length);
        }
      } catch (err) {
        console.error('Error fetching tools:', err);
        setError(true);
      }
    };

    fetchTools();
  }, []);

  if (error) {
    return <div></div>;
  }

  if (toolCount === null) {
    return <div>Loading...</div>;
  }

  if (toolCount < SUGGESTED_MAX_TOOLS) {
    return <div></div>;
  } else {
    return (
      <div>
        <ExclamationTriangleIcon color={'orange'} />
      </div>
    );
  }
}
