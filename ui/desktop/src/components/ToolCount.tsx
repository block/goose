import React, { useState, useEffect } from 'react';
import { getTools } from '../api';
import { ExclamationTriangleIcon } from '@radix-ui/react-icons';
import { HammerIcon } from 'lucide-react';
import { Popover, PopoverContent, PopoverTrigger } from './ui/popover';

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
    return <div>...</div>;
  }

  if (toolCount < SUGGESTED_MAX_TOOLS) {
    return (
      <div>
        <Popover>
          <PopoverTrigger asChild>
            <button className="flex items-center justify-center p-0 border-0 bg-transparent cursor-pointer">
              <HammerIcon size={16} />
            </button>
          </PopoverTrigger>
          <PopoverContent className="p-3 w-auto" side="top">
            <div className="space-y-1">
              <p className="text-sm text-black dark:text-white">Tool count: {toolCount}</p>
            </div>
          </PopoverContent>
        </Popover>
      </div>
    );
  } else {
    return (
      <div>
        <Popover>
          <PopoverTrigger asChild>
            <button className="flex items-center justify-center p-0 border-0 bg-transparent cursor-pointer">
              <ExclamationTriangleIcon color="orange" />
            </button>
          </PopoverTrigger>
          <PopoverContent className="p-3" side="top">
            <div className="space-y-2">
              <h4 className="text-sm font-medium">Warning: High Tool Count</h4>
              <p className="text-xs text-black dark:text-white">
                Too many tools can degrade goose's performance. Consider turning a few extensions
                off.
              </p>
              <p className="text-xs font-medium">Tool count: {toolCount}</p>
            </div>
          </PopoverContent>
        </Popover>
      </div>
    );
  }
}
