import React, { useState } from 'react';
import { Popover, PopoverContent, PopoverTrigger } from './ui/popover';
import VertDots from './ui/VertDots';

interface MoreMenuProps {
  className?: string;
  onClearContext: () => void;
  onRestartGoose: () => void;
}

export default function MoreMenu({ onClearContext, onRestartGoose }: MoreMenuProps) {
  const [open, setOpen] = useState(false);

  const handleAction = (action: () => void) => {
    action();
    setOpen(false);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button className="z-[100] absolute top-[-4px] right-[10px] w-[20px] h-[20px] cursor-pointer no-drag">
          <VertDots size={18} />
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-48 rounded-md">
        <div className="flex flex-col bg-black text-white rounded-md">
          <button
            onClick={() => handleAction(onClearContext)}
            className="w-full text-left px-2 py-1.5 text-sm"
          >
            Clear context
          </button>
          <button
            onClick={() => handleAction(onRestartGoose)}
            className="w-full text-left px-2 py-1.5 text-sm"
          >
            Restart goose
          </button>
          <button
            onClick={() => window.electron.directoryChooser()}
            className="w-full text-left px-2 py-1.5 text-sm"
          >
            Open Dir (cmd+O)
          </button>          
          <button
            onClick={() => window.electron.createChatWindow()}
            className="w-full text-left px-2 py-1.5 text-sm"
          >
            New Session (cmd+N)
          </button>          

        </div>
      </PopoverContent>
    </Popover>
  );
}