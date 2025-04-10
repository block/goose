import { useState, useEffect } from 'react';
import { getTools } from '../api';
import { ExclamationTriangleIcon } from '@radix-ui/react-icons';
import { Popover, PopoverContent, PopoverTrigger } from './ui/popover';

const SUGGESTED_MAX_TOOLS = 24;

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
    return <div>...</div>;
  }

  if (toolCount > SUGGESTED_MAX_TOOLS) {
    return (
      <div>
        <Popover>
          <PopoverTrigger asChild>
            <button className="flex items-center justify-center p-0 border-0 bg-transparent cursor-pointer">
              <ExclamationTriangleIcon color="orange" />
            </button>
          </PopoverTrigger>
          <PopoverContent className="p-3 bg-orangit ge-500 " side="top">
            <div className="space-y-2">
              <p className="text-xs text-gray-300">
                Too many tools can degrade goose's performance. Consider turning unused extensions
                off. Tool count: {toolCount} (recommend: {SUGGESTED_MAX_TOOLS})
              </p>
            </div>
          </PopoverContent>
        </Popover>
      </div>
    );
  } else {
    return <div></div>;
  }
}
