import React, {
  useEffect,
  useRef,
  forwardRef,
  useImperativeHandle,
  useState,
} from 'react';
import { Zap, FileText, Code, Settings, Search, Play, Hash, Plus } from 'lucide-react';
import { CustomCommand, BUILT_IN_COMMANDS } from '../types/customCommands';
import { Button } from './ui/button';

interface ActionItem {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  action: () => void;
  isCustom?: boolean;
  prompt?: string; // For custom commands
}

interface ActionPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (actionId: string) => void;
  position: { x: number; y: number };
  selectedIndex: number;
  onSelectedIndexChange: (index: number) => void;
  query?: string; // Filter actions based on query
  onCreateCommand?: () => void; // Callback to open command creation modal
}

const ActionPopover = forwardRef<
  { getDisplayActions: () => ActionItem[]; selectAction: (index: number) => void },
  ActionPopoverProps
>(({ isOpen, onClose, onSelect, position, selectedIndex, onSelectedIndexChange, query = '', onCreateCommand }, ref) => {
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [allCommands, setAllCommands] = useState<CustomCommand[]>([]);

  // Load both built-in and user commands on mount
  useEffect(() => {
    const loadAllCommands = () => {
      try {
        // Load user commands
        const userStored = localStorage.getItem('goose-custom-commands');
        let userCommands: CustomCommand[] = [];
        if (userStored) {
          const parsed = JSON.parse(userStored);
          userCommands = parsed
            .filter((cmd: any) => !cmd.isBuiltIn) // Only user commands
            .map((cmd: any) => ({
              ...cmd,
              createdAt: new Date(cmd.createdAt),
              updatedAt: new Date(cmd.updatedAt)
            }));
        }

        // Load built-in command favorites/usage
        const builtInStored = localStorage.getItem('goose-builtin-commands');
        let builtInCommands = [...BUILT_IN_COMMANDS];
        if (builtInStored) {
          const builtInData = JSON.parse(builtInStored);
          builtInCommands = BUILT_IN_COMMANDS.map(cmd => ({
            ...cmd,
            isFavorite: builtInData[cmd.id]?.isFavorite || false,
            usageCount: builtInData[cmd.id]?.usageCount || 0,
          }));
        }

        // Combine all commands
        setAllCommands([...builtInCommands, ...userCommands]);
      } catch (error) {
        console.error('Failed to load commands:', error);
      }
    };

    if (isOpen) {
      loadAllCommands();
    }
  }, [isOpen]);

  // Icon mapping for custom commands
  const getCustomCommandIcon = (iconName?: string) => {
    const iconMap: Record<string, React.ReactNode> = {
      'Zap': <Zap size={16} />,
      'Code': <Code size={16} />,
      'FileText': <FileText size={16} />,
      'Search': <Search size={16} />,
      'Play': <Play size={16} />,
      'Settings': <Settings size={16} />,
      'Hash': <Hash size={16} />,
    };
    return iconMap[iconName || 'Zap'] || <Zap size={16} />;
  };

  // Convert all commands to action items
  const allActions: ActionItem[] = allCommands.map(cmd => ({
    id: cmd.id,
    label: cmd.label,
    description: cmd.description,
    icon: getCustomCommandIcon(cmd.icon),
    isCustom: !cmd.isBuiltIn, // Built-in commands are not "custom"
    prompt: cmd.prompt,
    action: () => {
      console.log('Command action triggered:', cmd.name);
      // Increment usage count for both built-in and user commands
      if (cmd.isBuiltIn) {
        // Update built-in command usage
        const builtInStored = localStorage.getItem('goose-builtin-commands');
        let builtInData: Record<string, { isFavorite: boolean; usageCount: number }> = {};
        if (builtInStored) {
          builtInData = JSON.parse(builtInStored);
        }
        builtInData[cmd.id] = {
          isFavorite: builtInData[cmd.id]?.isFavorite || cmd.isFavorite,
          usageCount: (builtInData[cmd.id]?.usageCount || cmd.usageCount) + 1,
        };
        localStorage.setItem('goose-builtin-commands', JSON.stringify(builtInData));
      } else {
        // Update user command usage
        const userStored = localStorage.getItem('goose-custom-commands');
        if (userStored) {
          const userCommands = JSON.parse(userStored);
          const updatedCommands = userCommands.map((c: any) => 
            c.id === cmd.id ? { ...c, usageCount: c.usageCount + 1 } : c
          );
          localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
        }
      }
    },
  }));

  // Filter commands based on query
  const filteredActions = allActions.filter(action => {
    const cmd = allCommands.find(c => c.id === action.id);
    
    // If no query, show only starred commands
    if (!query) {
      return cmd?.isFavorite === true;
    }
    
    // If there's a query, search through all commands
    const searchTerm = query.toLowerCase();
    return (
      action.label.toLowerCase().includes(searchTerm) ||
      action.description.toLowerCase().includes(searchTerm) ||
      action.id.toLowerCase().includes(searchTerm)
    );
  });

  // Sort actions: favorites first, then by usage count, then alphabetically
  const sortedActions = filteredActions.sort((a, b) => {
    const cmdA = allCommands.find(c => c.id === a.id);
    const cmdB = allCommands.find(c => c.id === b.id);
    
    if (cmdA?.isFavorite && !cmdB?.isFavorite) return -1;
    if (!cmdA?.isFavorite && cmdB?.isFavorite) return 1;
    
    if (cmdA && cmdB) {
      if (cmdA.usageCount !== cmdB.usageCount) {
        return cmdB.usageCount - cmdA.usageCount;
      }
    }
    
    return a.label.localeCompare(b.label);
  });

  // Expose methods to parent component
  useImperativeHandle(
    ref,
    () => ({
      getDisplayActions: () => sortedActions,
      selectAction: (index: number) => {
        console.log('⌨️ ActionPopover: selectAction called via keyboard', { index, actionId: sortedActions[index]?.id });
        if (sortedActions[index]) {
          console.log('🔄 ActionPopover: Calling onSelect from selectAction:', sortedActions[index].id);
          onSelect(sortedActions[index].id);
          sortedActions[index].action();
          setTimeout(() => {
            onClose();
          }, 10);
        }
      },
    }),
    [sortedActions, onSelect, onClose]
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
    console.log('🎯 ActionPopover: handleItemClick called', { index, actionId: sortedActions[index].id });
    console.log('📋 ActionPopover: onSelect function:', onSelect);
    console.log('🔄 ActionPopover: About to call onSelect with:', sortedActions[index].id);
    
    onSelectedIndexChange(index);
    
    // Call onSelect first - this should trigger handleActionSelect in ChatInput
    onSelect(sortedActions[index].id);
    
    console.log('✅ ActionPopover: onSelect called successfully');
    
    // Call the local action (just for logging)
    sortedActions[index].action();
    
    // Close popover after a small delay to allow text replacement to complete
    console.log('🚪 ActionPopover: Closing popover after delay');
    setTimeout(() => {
      onClose();
    }, 10);
  };

  if (!isOpen) return null;

  return (
    <div
      ref={popoverRef}
      className="fixed z-50 bg-background-default border border-borderStandard rounded-2xl shadow-lg min-w-80 max-w-md"
      style={{
        left: position.x,
        top: position.y - 10,
        transform: 'translateY(-100%)',
      }}
    >
      <div className="p-3">
        <div className="mb-2">
          <h3 className="text-sm font-medium text-textStandard">
            {query ? 'Search Results' : 'Starred Commands'}
          </h3>
          <p className="text-xs text-textSubtle">
            {query ? `Commands matching "${query}"` : 'Your favorite slash commands'}
          </p>
        </div>
        
        <div ref={listRef} className="space-y-1">
          {sortedActions.length > 0 ? (
            sortedActions.map((action, index) => (
              <div
                key={action.id}
                onClick={() => handleItemClick(index)}
                className={`flex items-center gap-3 p-2 rounded-2xl cursor-pointer transition-all ${
                  index === selectedIndex
                    ? 'bg-gray-100 dark:bg-gray-700'
                    : 'hover:bg-gray-100 dark:hover:bg-gray-700'
                }`}
              >
                <div className="flex-shrink-0 text-textSubtle">
                  {action.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <div className="text-sm font-medium text-textStandard">
                      {action.label}
                    </div>
                  </div>
                  <div className="text-xs text-textSubtle">
                    {action.description}
                  </div>
                </div>
              </div>
            ))
          ) : (
            <div className="p-3 text-center text-textSubtle">
              <div className="text-sm mb-2">
                {query 
                  ? `No commands match "${query}"` 
                  : allCommands.length === 0 
                    ? 'No commands found'
                    : 'No starred commands found'
                }
              </div>
              {query && onCreateCommand ? (
                <Button
                  onClick={() => {
                    onCreateCommand();
                    onClose();
                  }}
                  size="sm"
                  className="flex items-center gap-2"
                >
                  <Plus size={14} />
                  Create Command
                </Button>
              ) : !query && allCommands.length === 0 ? (
                onCreateCommand ? (
                  <Button
                    onClick={() => {
                      onCreateCommand();
                      onClose();
                    }}
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <Plus size={14} />
                    Create Command
                  </Button>
                ) : (
                  <div className="text-xs">Create commands in Settings → Chat</div>
                )
              ) : (
                <div className="text-xs">Star commands to see them here when you type /</div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

ActionPopover.displayName = 'ActionPopover';

export default ActionPopover;
