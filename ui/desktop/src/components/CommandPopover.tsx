const popoverStyles = `
@keyframes popoverFadeIn {
  from {
    opacity: 0;
    transform: translateY(-100%) scaleY(0.8);
  }
  to {
    opacity: 1;
    transform: translateY(-100%) scaleY(1);
  }
}
`;

// Inject styles
if (typeof document !== "undefined" && !document.getElementById("popover-styles")) {
  const style = document.createElement("style");
  style.id = "popover-styles";
  style.textContent = popoverStyles;
  document.head.appendChild(style);
}

import {
  useEffect,
  useRef,
  forwardRef,
  useImperativeHandle,
} from 'react';
import { SearchX } from 'lucide-react';
import { useCommands } from '../hooks/useCommands';
import type { Command } from '../hooks/useCommands';

interface CommandPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (actionId: string) => void;
  position: { x: number; y: number };
  selectedIndex: number;
  onSelectedIndexChange: (index: number) => void;
  query?: string;
}

const CommandPopover = forwardRef<
  { getDisplayCommands: () => Command[]; selectCommand: (index: number) => void },
  CommandPopoverProps
>(({ isOpen, onClose, onSelect, position, selectedIndex, onSelectedIndexChange, query = '' }, ref) => {
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const { commands } = useCommands();
  const filteredCommands = query
    ? commands.filter(command => {
        const searchTerm = query.toLowerCase();
        return command.description.toLowerCase().includes(searchTerm);
      })
    : commands;

  const sortedCommands = filteredCommands.sort((a, b) => 
    a.name.localeCompare(b.name)
  );

  // Expose methods to parent component
  useImperativeHandle(
    ref,
    () => ({
      getDisplayCommands: () => sortedCommands,
      selectCommand: (index: number) => {
        if (sortedCommands[index]) {
          onSelect(sortedCommands[index].id);
          sortedCommands[index].command();
          setTimeout(() => {
            onClose();
          }, 10);
        }
      },
    }),
    [sortedCommands, onSelect, onClose]
  );

  // Handle clicks outside the popover
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen, onClose]);

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selectedElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedElement) {
        selectedElement.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [selectedIndex]);

  const handleItemClick = (index: number) => {
    onSelectedIndexChange(index);
    onSelect(sortedCommands[index].id);
    sortedCommands[index].command();
    
    // Close popover after a small delay to allow text replacement to complete
    setTimeout(() => {
      onClose();
    }, 10);
  };

  if (!isOpen) return null;

  return (
    <div
      ref={popoverRef}
      className="fixed z-50 bg-background-default border border-borderStandard rounded-2xl min-w-80 max-w-md "
      style={{ boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.12), 0 0 0 1px rgba(0, 0, 0, 0.05)", transformOrigin: "bottom", animation: "popoverFadeIn 0.2s ease-out forwards", opacity: 0, transform: "translateY(-100%) scaleY(0.8)",
        left: position.x,
        top: position.y - 10,
        
      }}
    >
      <div className="p-3">
        <div className="mb-2">
          <h3 className="text-sm font-medium text-textStandard">
            {query ? 'Search Results' : 'Commands'}
          </h3>
          <p className="text-xs text-textSubtle">
            {query ? `Commands matching "${query}"` : 'Available commands'}
          </p>
        </div>
        <div ref={listRef} className="space-y-1">
          {sortedCommands.length > 0 ? (
            sortedCommands.map((command, index) => (
              <div
                key={command.id}
                onClick={() => handleItemClick(index)}
                className={`flex items-center gap-3 p-2 rounded-2xl cursor-pointer transition-all ${
                  index === selectedIndex
                    ? 'bg-gray-100 dark:bg-gray-700'
                    : 'hover:bg-gray-100 dark:hover:bg-gray-700'
                }`}
              >
                <div className="flex-shrink-0 text-textSubtle">
                  {command.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mx-auto">
                    <div className="text-sm font-medium text-textStandard">
                      {command.name}
                    </div>
                  </div>
                  <div className="text-xs text-textSubtle">
                    {command.description}
                  </div>
                </div>
              </div>
            ))
          ) : (
            <div className="p-3 text-center text-textSubtle">
              <div className="text-sm mb-2 text-textMuted">
                <SearchX size={24} className="text-textMuted mx-auto mb-1" />
                No commands found
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

CommandPopover.displayName = 'CommandPopover';

export default CommandPopover;
