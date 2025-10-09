import React, { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { useSidecar, Sidecar } from '../SidecarLayout';
import { Plus, X, Globe, FileText } from 'lucide-react';
import { Button } from '../ui/button';

interface SidecarContainer {
  id: string;
  content: React.ReactNode;
  contentType: 'sidecar' | 'localhost' | 'file' | null;
  title?: string;
}

interface ContainerPopoverProps {
  onSelect: (type: 'sidecar' | 'localhost' | 'file') => void;
  onClose: () => void;
  position: { x: number; y: number };
}

const ContainerPopover: React.FC<ContainerPopoverProps> = ({ onSelect, onClose, position }) => {
  const popoverRef = useRef<HTMLDivElement>(null);

  // Close on click outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  const handleLocalhostClick = async () => {
    onSelect('localhost');
    onClose();
  };

  const handleFileViewerClick = async () => {
    try {
      const filePath = await window.electron.selectFileOrDirectory();
      if (filePath) {
        onSelect('file');
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }
    onClose();
  };

  const handleSidecarClick = () => {
    onSelect('sidecar');
    onClose();
  };

  return (
    <div
      ref={popoverRef}
      className="fixed z-50 bg-background-default border border-border-subtle rounded-lg shadow-xl p-2 min-w-[160px] animate-in fade-in slide-in-from-right-2 duration-200"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        transform: 'translate(-100%, -50%)'
      }}
    >
      <div className="space-y-1">
        <Button
          onClick={handleSidecarClick}
          className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
          variant="ghost"
          size="sm"
        >
          <Plus className="w-4 h-4 mr-2" />
          Sidecar View
        </Button>
        
        <Button
          onClick={handleLocalhostClick}
          className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
          variant="ghost"
          size="sm"
        >
          <Globe className="w-4 h-4 mr-2" />
          Localhost Viewer
        </Button>
        
        <Button
          onClick={handleFileViewerClick}
          className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
          variant="ghost"
          size="sm"
        >
          <FileText className="w-4 h-4 mr-2" />
          Open File
        </Button>
      </div>
    </div>
  );
};

// ResizeHandle component for horizontal resizing between containers
const ResizeHandle: React.FC<{
  onResize: (delta: number) => void;
  isResizing: boolean;
}> = ({ onResize, isResizing }) => {
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    let startX = e.clientX;
    
    const handleMouseMove = (e: MouseEvent) => {
      const delta = e.clientX - startX;
      onResize(delta);
      startX = e.clientX;
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [onResize]);

  return (
    <div 
      className={`w-1 cursor-col-resize hover:bg-borderSubtle transition-colors group ${
        isResizing ? 'bg-borderProminent' : ''
      }`}
      onMouseDown={handleMouseDown}
    >
      <div 
        className={`h-8 w-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors my-auto ml-0.5 ${
          isResizing ? 'bg-border-strong' : ''
        }`} 
      />
    </div>
  );
};

// Individual container component
const ContainerComponent: React.FC<{
  container: SidecarContainer;
  onRemove: () => void;
  width: number;
  isLast: boolean;
  onAddAfter: () => void;
}> = ({ container, onRemove, width, isLast, onAddAfter }) => {
  const [isHovering, setIsHovering] = useState(false);
  const [showPopover, setShowPopover] = useState(false);
  const [popoverPosition, setPopoverPosition] = useState({ x: 0, y: 0 });

  const handleAddClick = (e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setPopoverPosition({
      x: rect.right,
      y: rect.top + rect.height / 2
    });
    setShowPopover(true);
  };

  const handlePopoverSelect = (type: 'sidecar' | 'localhost' | 'file') => {
    onAddAfter();
    // The parent will handle creating the container with the selected type
  };

  return (
    <div 
      className="h-full relative group flex-shrink-0 overflow-hidden"
      style={{ width: `${width}px` }}
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      {/* Container header with remove button - only show on hover */}
      <div className="absolute top-2 right-2 z-30">
        <Button
          onClick={onRemove}
          variant="ghost"
          size="sm"
          className="w-6 h-6 rounded-full bg-background-default/90 hover:bg-background-default opacity-0 group-hover:opacity-100 transition-opacity shadow-lg border border-border-subtle"
        >
          <X className="w-3 h-3" />
        </Button>
      </div>

      {/* Container content - fills entire container */}
      <div className="h-full w-full relative">
        {container.content || (
          <div className="h-full w-full flex flex-col items-center justify-center p-4 space-y-3 bg-background-muted border border-border-subtle rounded-lg">
            <p className="text-text-muted text-sm text-center">Empty container</p>
          </div>
        )}
      </div>

      {/* Right edge hover zone for adding new container */}
      {isLast && (
        <div
          className="absolute top-0 right-0 w-4 h-full z-10 pointer-events-auto"
          onMouseEnter={() => setIsHovering(true)}
          onMouseLeave={() => setIsHovering(false)}
        >
          {isHovering && (
            <div className="absolute right-0 top-1/2 transform translate-x-1/2 -translate-y-1/2">
              <Button
                onClick={handleAddClick}
                className="w-8 h-8 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200 pointer-events-auto"
                variant="ghost"
                size="sm"
              >
                <Plus className="w-4 h-4" />
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Popover for content selection */}
      {showPopover && (
        <ContainerPopover
          onSelect={handlePopoverSelect}
          onClose={() => setShowPopover(false)}
          position={popoverPosition}
        />
      )}
    </div>
  );
};

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const sidecar = useSidecar();
  
  // State for sidecar containers
  const [containers, setContainers] = useState<SidecarContainer[]>([]);
  const [chatWidth, setChatWidth] = useState(600); // Fixed width for chat panel
  const [resizingIndex, setResizingIndex] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Check if main sidecar is visible
  const mainSidecarVisible = sidecar?.activeViews && sidecar?.activeViews.length > 0;

  // Calculate container widths (equal distribution)
  const containerWidth = useMemo(() => {
    if (containers.length === 0) return 0;
    
    const availableWidth = (containerRef.current?.clientWidth || window.innerWidth) - chatWidth - (containers.length * 4); // 4px per resize handle
    return Math.max(300, availableWidth / containers.length);
  }, [containers.length, chatWidth]);

  // Add a new container
  const addContainer = useCallback((contentType: 'sidecar' | 'localhost' | 'file' = 'sidecar', afterIndex?: number, filePath?: string) => {
    const newContainer: SidecarContainer = {
      id: `container-${Date.now()}`,
      content: null,
      contentType: null
    };

    // Create content based on type
    if (contentType === 'sidecar') {
      newContainer.content = <Sidecar className="h-full" />;
      newContainer.contentType = 'sidecar';
      newContainer.title = 'Sidecar';
    } else if (contentType === 'localhost') {
      const instanceId = `container-${newContainer.id}`;
      if (sidecar) {
        sidecar.showLocalhostViewer('http://localhost:3000', 'Localhost Viewer', instanceId);
        // Give the sidecar context time to create the view, then render it
        setTimeout(() => {
          setContainers(prev => prev.map(c => 
            c.id === newContainer.id 
              ? { ...c, content: <Sidecar className="h-full" viewId={`localhost-${instanceId}`} /> }
              : c
          ));
        }, 100);
      }
      newContainer.content = <div className="h-full flex items-center justify-center text-text-muted">Loading localhost viewer...</div>;
      newContainer.contentType = 'localhost';
      newContainer.title = 'Localhost Viewer';
    } else if (contentType === 'file' && filePath) {
      const instanceId = `container-${newContainer.id}`;
      if (sidecar) {
        sidecar.showFileViewer(filePath, instanceId);
        // Give the sidecar context time to create the view, then render it
        setTimeout(() => {
          setContainers(prev => prev.map(c => 
            c.id === newContainer.id 
              ? { ...c, content: <Sidecar className="h-full" viewId={`file-${instanceId}`} /> }
              : c
          ));
        }, 100);
      }
      newContainer.content = <div className="h-full flex items-center justify-center text-text-muted">Loading file viewer...</div>;
      newContainer.contentType = 'file';
      newContainer.title = filePath?.split('/').pop() || 'File Viewer';
    }

    setContainers(prev => {
      if (afterIndex !== undefined) {
        const newContainers = [...prev];
        newContainers.splice(afterIndex + 1, 0, newContainer);
        return newContainers;
      }
      return [...prev, newContainer];
    });
  }, [sidecar]);

  // Remove a container
  const removeContainer = useCallback((containerId: string) => {
    setContainers(prev => prev.filter(c => c.id !== containerId));
  }, []);

  // Handle chat panel resize
  const updateChatWidth = useCallback((delta: number) => {
    setChatWidth(prev => Math.max(300, Math.min(1000, prev + delta)));
  }, []);

  // Handle container resize
  const updateContainerWidth = useCallback((index: number, delta: number) => {
    // For now, we'll keep equal widths and just trigger a re-render
    // In a more advanced implementation, you could have individual container widths
    console.log(`Resize container ${index} by ${delta}px`);
  }, []);

  // Listen for add-container events from SidecarInvoker
  useEffect(() => {
    const handleAddContainer = (e: CustomEvent<{ type: 'sidecar' | 'localhost' | 'file'; filePath?: string }>) => {
      console.log('🔍 MainPanelLayout: Received add-container event:', e.detail.type, e.detail.filePath);
      addContainer(e.detail.type, undefined, e.detail.filePath);
    };

    window.addEventListener('add-container', handleAddContainer as EventListener);
    return () => window.removeEventListener('add-container', handleAddContainer as EventListener);
  }, [addContainer]);

  // Debug logging
  useEffect(() => {
    console.log('🔍 MainPanelLayout: containers:', containers.length);
    console.log('🔍 MainPanelLayout: mainSidecarVisible:', mainSidecarVisible);
  }, [containers.length, mainSidecarVisible]);

  const hasContainers = containers.length > 0;

  return (
    <div className={`h-dvh`} ref={containerRef}>
      <div
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Chat Panel - Full width when no containers, fixed width when containers exist */}
        <div 
          className={hasContainers ? "flex flex-col flex-shrink-0" : "flex flex-col flex-1"}
          style={hasContainers ? { width: `${chatWidth}px` } : {}}
        >
          {children}
        </div>

        {/* Chat Resize Handle - only show when containers exist */}
        {hasContainers && (
          <ResizeHandle
            onResize={updateChatWidth}
            isResizing={resizingIndex === -1}
          />
        )}

        {/* Container Layout - only show when containers exist */}
        {hasContainers && containers.map((container, index) => (
          <React.Fragment key={container.id}>
            <ContainerComponent
              container={container}
              onRemove={() => removeContainer(container.id)}
              width={containerWidth}
              isLast={index === containers.length - 1}
              onAddAfter={() => addContainer('sidecar', index)}
            />

            {/* Resize Handle between containers */}
            {index < containers.length - 1 && (
              <ResizeHandle
                onResize={(delta) => updateContainerWidth(index, delta)}
                isResizing={resizingIndex === index}
              />
            )}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
};
