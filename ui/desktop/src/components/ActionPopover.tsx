import React, {
  useEffect,
  useRef,
  forwardRef,
  useImperativeHandle,
  useState,
} from 'react';
import { Zap, FileText, Code, Settings, Search, Play, Hash } from 'lucide-react';
import { CustomCommand } from '../types/customCommands';

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
}

const ActionPopover = forwardRef<
  { getDisplayActions: () => ActionItem[]; selectAction: (index: number) => void },
  ActionPopoverProps
>(({ isOpen, onClose, onSelect, position, selectedIndex, onSelectedIndexChange, query = '' }, ref) => {
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [customCommands, setCustomCommands] = useState<CustomCommand[]>([]);

  // Load custom commands on mount
  useEffect(() => {
    const loadCustomCommands = () => {
      try {
        const stored = localStorage.getItem('goose-custom-commands');
        if (stored) {
          const parsed = JSON.parse(stored);
          setCustomCommands(parsed.map((cmd: any) => ({
            ...cmd,
            createdAt: new Date(cmd.createdAt),
            updatedAt: new Date(cmd.updatedAt)
          })));
        }
      } catch (error) {
        console.error('Failed to load custom commands:', error);
      }
    };

    if (isOpen) {
      loadCustomCommands();
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

  // Convert custom commands to action items
  const customActions: ActionItem[] = customCommands.map(cmd => ({
    id: cmd.id,
    label: cmd.label,
    description: cmd.description,
    icon: getCustomCommandIcon(cmd.icon),
    isCustom: true,
    prompt: cmd.prompt,
    action: () => {
      console.log('Custom command action triggered:', cmd.name);
      // Increment usage count
      const updatedCommands = customCommands.map(c => 
        c.id === cmd.id ? { ...c, usageCount: c.usageCount + 1 } : c
      );
      localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
    },
  }));

  // Filter custom commands based on query
  const filteredActions = customActions.filter(action => {
    if (!query) return true;
    const searchTerm = query.toLowerCase();
    return (
      action.label.toLowerCase().includes(searchTerm) ||
      action.description.toLowerCase().includes(searchTerm) ||
      action.id.toLowerCase().includes(searchTerm)
    );
  });

  // Sort actions: favorites first, then by usage count, then alphabetically
  const sortedActions = filteredActions.sort((a, b) => {
    const cmdA = customCommands.find(c => c.id === a.id);
    const cmdB = customCommands.find(c => c.id === b.id);
    
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
      className="fixed z-50 bg-background-default border border-borderStandard rounded-lg shadow-lg min-w-80 max-w-md"
      style={{
        left: position.x,
        top: position.y - 10,
        transform: 'translateY(-100%)',
      }}
    >
      <div className="p-3">
        <div className="mb-2">
          <h3 className="text-sm font-medium text-textStandard">Custom Commands</h3>
          <p className="text-xs text-textSubtle">Your custom slash commands</p>
        </div>
        
        <div ref={listRef} className="space-y-1">
          {sortedActions.length > 0 ? (
            sortedActions.map((action, index) => (
              <div
                key={action.id}
                onClick={() => handleItemClick(index)}
                className={`flex items-center gap-3 p-3 rounded-md cursor-pointer transition-colors ${
                  index === selectedIndex
                    ? 'bg-bgProminent text-textProminentInverse'
                    : 'hover:bg-bgSubtle'
                }`}
              >
                <div className="flex-shrink-0 text-textSubtle">
                  {action.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <div className="text-sm font-medium text-textStandard">{action.label}</div>
                    <span className="text-xs px-1.5 py-0.5 bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 rounded-full font-medium">
                      Custom
                    </span>
                  </div>
                  <div className="text-xs text-textSubtle">{action.description}</div>
                </div>
              </div>
            ))
          ) : (
            <div className="p-3 text-center text-textSubtle">
              <div className="text-sm">No custom commands found</div>
              <div className="text-xs mt-1">Create custom commands in Settings → Chat</div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

ActionPopover.displayName = 'ActionPopover';

export default ActionPopover;
