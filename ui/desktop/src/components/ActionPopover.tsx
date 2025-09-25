import {
  
  useEffect,
  useRef,
  forwardRef,
  useImperativeHandle,
} from 'react';
import { Zap, FileText, Code, Settings, Search, Play } from 'lucide-react';

interface ActionItem {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  action: () => void;
}

interface ActionPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (actionId: string) => void;
  position: { x: number; y: number };
  selectedIndex: number;
  onSelectedIndexChange: (index: number) => void;
}

const ActionPopover = forwardRef<
  { getDisplayActions: () => ActionItem[]; selectAction: (index: number) => void },
  ActionPopoverProps
>(({ isOpen, onClose, onSelect, position, selectedIndex, onSelectedIndexChange }, ref) => {
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Define available actions
  const actions: ActionItem[] = [
    {
      id: 'quick-task',
      label: 'Quick Task',
      description: 'Create a quick task or reminder',
      icon: <Zap size={16} />,
      action: () => {
        // TODO: Implement quick task creation
        console.log('Quick task action triggered');
      },
    },
    {
      id: 'generate-code',
      label: 'Generate Code',
      description: 'Generate code snippet or template',
      icon: <Code size={16} />,
      action: () => {
        // TODO: Implement code generation
        console.log('Generate code action triggered');
      },
    },
    {
      id: 'create-document',
      label: 'Create Document',
      description: 'Create a new document or file',
      icon: <FileText size={16} />,
      action: () => {
        // TODO: Implement document creation
        console.log('Create document action triggered');
      },
    },
    {
      id: 'search-files',
      label: 'Search Files',
      description: 'Search through project files',
      icon: <Search size={16} />,
      action: () => {
        // TODO: Implement file search
        console.log('Search files action triggered');
      },
    },
    {
      id: 'run-command',
      label: 'Run Command',
      description: 'Execute a shell command',
      icon: <Play size={16} />,
      action: () => {
        // TODO: Implement command execution
        console.log('Run command action triggered');
      },
    },
    {
      id: 'settings',
      label: 'Settings',
      description: 'Open settings and preferences',
      icon: <Settings size={16} />,
      action: () => {
        // TODO: Implement settings navigation
        console.log('Settings action triggered');
      },
    },
  ];

  // Expose methods to parent component
  useImperativeHandle(
    ref,
    () => ({
      getDisplayActions: () => actions,
      selectAction: (index: number) => {
        console.log('⌨️ ActionPopover: selectAction called via keyboard', { index, actionId: actions[index]?.id });
        if (actions[index]) {
          console.log('🔄 ActionPopover: Calling onSelect from selectAction:', actions[index].id);
          onSelect(actions[index].id);
          actions[index].action();
          setTimeout(() => {
            onClose();
          }, 10);
        }
      },
    }),
    [actions, onSelect, onClose]
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
    console.log('🎯 ActionPopover: handleItemClick called', { index, actionId: actions[index].id });
    console.log('📋 ActionPopover: onSelect function:', onSelect);
    console.log('🔄 ActionPopover: About to call onSelect with:', actions[index].id);
    
    onSelectedIndexChange(index);
    
    // Call onSelect first - this should trigger handleActionSelect in ChatInput
    onSelect(actions[index].id);
    
    console.log('✅ ActionPopover: onSelect called successfully');
    
    // Call the local action (just for logging)
    actions[index].action();
    
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
          <h3 className="text-sm font-medium text-textStandard">Quick Actions</h3>
          <p className="text-xs text-textSubtle">Choose an action to perform</p>
        </div>
        
        <div ref={listRef} className="space-y-1">
          {actions.map((action, index) => (
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
                <div className="text-sm font-medium text-textStandard">{action.label}</div>
                <div className="text-xs text-textSubtle">{action.description}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
});

ActionPopover.displayName = 'ActionPopover';

export default ActionPopover;
