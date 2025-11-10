import React, { useState, useCallback, useRef, useEffect } from 'react';
import { Plus, X, Globe, FileText, Edit } from 'lucide-react';
import { Button } from '../ui/button';
import SidecarTabs from '../SidecarTabs';
import { FileViewer } from '../FileViewer';
import DocumentEditor from '../DocumentEditor';

interface SidecarContainer {
  id: string;
  content: React.ReactNode;
  contentType: 'sidecar' | 'localhost' | 'file' | 'document-editor' | null;
  title?: string;
}

interface ContainerPopoverProps {
  onSelect: (type: 'sidecar' | 'localhost' | 'file' | 'document-editor') => void;
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

// ResizeHandle component for horizontal resizing between panels
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

// BentoBox component - contains all sidecars in a single flexible container
const BentoBox: React.FC<{
  containers: SidecarContainer[];
  onRemoveContainer: (containerId: string) => void;
  onAddContainer: (type: 'sidecar' | 'localhost' | 'file' | 'document-editor', filePath?: string) => void;
}> = ({ containers, onRemoveContainer, onAddContainer }) => {
  const [containerWidths, setContainerWidths] = useState<{ [containerId: string]: number }>({});
  const [isHovering, setIsHovering] = useState(false);
  const [showPopover, setShowPopover] = useState(false);
  const [popoverPosition, setPopoverPosition] = useState({ x: 0, y: 0 });
  const [isResizing, setIsResizing] = useState(false);
  const [resizingIndex, setResizingIndex] = useState<number | null>(null);

  // Calculate equal widths for all containers
  useEffect(() => {
    if (containers.length > 0) {
      const equalWidth = Math.floor(100 / containers.length); // Use percentages
      const widths = {};
      containers.forEach(container => {
        widths[container.id] = equalWidth;
      });
      setContainerWidths(widths);
    }
  }, [containers.length]);

  // Store parent width to avoid DOM queries during resize
  const parentWidthRef = useRef<number>(800);
  
  // Update parent width when containers change
  useEffect(() => {
    const updateParentWidth = () => {
      const bentoElement = document.querySelector('[data-bento-container="true"]');
      if (bentoElement) {
        parentWidthRef.current = bentoElement.clientWidth;
      }
    };
    
    updateParentWidth();
    window.addEventListener('resize', updateParentWidth);
    return () => window.removeEventListener('resize', updateParentWidth);
  }, [containers.length]);

  // Handle resizing between containers - FIXED VERSION
  const handleResize = useCallback((containerIndex: number, delta: number) => {
    if (containerIndex >= containers.length - 1) return; // Can't resize last container
    
    setContainerWidths(prev => {
      // Get current widths for all containers
      const currentWidths = containers.map(container => 
        prev[container.id] || 100 / containers.length
      );
      
      // Convert pixel delta to percentage using cached parent width
      const deltaPercent = (delta / parentWidthRef.current) * 100;
      
      // Apply delta to the two containers being resized
      const newWidths = [...currentWidths];
      const minWidth = 5; // Minimum 5% width per container
      const maxWidth = 80; // Maximum 80% width per container
      
      // Calculate proposed new widths
      const proposedLeftWidth = currentWidths[containerIndex] + deltaPercent;
      const proposedRightWidth = currentWidths[containerIndex + 1] - deltaPercent;
      
      // Apply constraints
      const constrainedLeftWidth = Math.max(minWidth, Math.min(maxWidth, proposedLeftWidth));
      const constrainedRightWidth = Math.max(minWidth, Math.min(maxWidth, proposedRightWidth));
      
      // Check if both constraints can be satisfied
      const totalConstrainedWidth = constrainedLeftWidth + constrainedRightWidth;
      const originalTotalWidth = currentWidths[containerIndex] + currentWidths[containerIndex + 1];
      
      // Only apply changes if constraints are satisfied and total width is preserved
      if (Math.abs(totalConstrainedWidth - originalTotalWidth) < 0.1) {
        newWidths[containerIndex] = constrainedLeftWidth;
        newWidths[containerIndex + 1] = constrainedRightWidth;
        
        // Convert back to object format
        const updatedWidths = {};
        containers.forEach((container, index) => {
          updatedWidths[container.id] = newWidths[index];
        });
        
        return updatedWidths;
      }
      
      return prev;
    });
  }, [containers]);

  // Resize handle component for between containers
  const ResizeHandleBento: React.FC<{ containerIndex: number }> = ({ containerIndex }) => {
    const handleMouseDown = useCallback((e: React.MouseEvent) => {
      e.preventDefault();
      setIsResizing(true);
      setResizingIndex(containerIndex);
      
      let startX = e.clientX;
      
      const handleMouseMove = (e: MouseEvent) => {
        const delta = e.clientX - startX;
        handleResize(containerIndex, delta);
        startX = e.clientX;
      };

      const handleMouseUp = () => {
        setIsResizing(false);
        setResizingIndex(null);
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }, [containerIndex]);

    return (
      <div 
        className={`w-1 cursor-col-resize hover:bg-blue-400 transition-colors group ${
          resizingIndex === containerIndex ? 'bg-blue-500' : 'bg-border-subtle'
        }`}
        onMouseDown={handleMouseDown}
        title="Drag to resize containers"
      >
        <div 
          className={`h-8 w-0.5 bg-border-subtle group-hover:bg-blue-400 rounded-full transition-colors my-auto ml-0.25 ${
            resizingIndex === containerIndex ? 'bg-blue-500' : ''
          }`} 
        />
      </div>
    );
  };

  const handleAddClick = (e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setPopoverPosition({
      x: rect.right,
      y: rect.top + rect.height / 2
    });
    setShowPopover(true);
  };

  const handlePopoverSelect = (type: 'sidecar' | 'localhost' | 'file') => {
    onAddContainer(type);
    setShowPopover(false);
  };

  return (
    <div className="flex-1 h-full bg-background-default rounded-xl overflow-hidden relative">
      {/* Container grid */}
      <div className="flex h-full w-full" data-bento-container="true">
        {containers.map((container, index) => (
          <React.Fragment key={container.id}>
            <div 
              className="h-full relative"
              style={{ width: `${containerWidths[container.id] || 100 / containers.length}%` }}
              data-container-id={container.id}
            >
              {/* Container content */}
              <div className="h-full w-full">
                {container.content || (
                  <div className="h-full w-full flex flex-col items-center justify-center p-4 space-y-3 bg-background-muted border border-border-subtle rounded-lg">
                    <p className="text-text-muted text-sm text-center">Empty container</p>
                  </div>
                )}
              </div>

              {/* X button for removing individual containers */}
              <button
                onClick={() => {
                  console.log('🔍 X BUTTON CLICKED for container:', container.id);
                  alert(`Removing container: ${container.id}`);
                  onRemoveContainer(container.id);
                }}
                onMouseEnter={() => console.log('🔍 X button mouse enter:', container.id)}
                onMouseLeave={() => console.log('🔍 X button mouse leave:', container.id)}
                className="absolute top-2 right-2 w-8 h-8 bg-red-500 hover:bg-red-600 text-white font-bold text-lg rounded-full flex items-center justify-center cursor-pointer shadow-xl border-2 border-white transition-all hover:scale-110"
                style={{ 
                  zIndex: 999999,
                  position: 'absolute',
                  top: '8px',
                  right: '8px'
                }}
                title="Remove container"
              >
                ×
              </button>
            </div>

            {/* Resize Handle between containers */}
            {index < containers.length - 1 && (
              <ResizeHandleBento containerIndex={index} />
            )}
          </React.Fragment>
        ))}
      </div>

      {/* Right edge hover zone for adding new container */}
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

      {/* Popover for content selection */}
      {showPopover && (
        <ContainerPopover
          onSelect={handlePopoverSelect}
          onClose={() => setShowPopover(false)}
          position={popoverPosition}
        />
      )}

      {/* X button to close entire bento box */}
      <button
        onClick={() => {
          // Remove all containers to close the bento box
          containers.forEach(container => onRemoveContainer(container.id));
        }}
        className="absolute top-2 left-2 z-[9999] w-6 h-6 rounded-full bg-background-default/80 hover:bg-background-default text-text-default shadow-lg border border-border-subtle flex items-center justify-center cursor-pointer pointer-events-auto transition-all"
      >
        <X className="w-3 h-3" />
      </button>
    </div>
  );
};

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  
  // Simplified state - just track if we have a bento box and what's in it
  const [hasBentoBox, setHasBentoBox] = useState(false);
  const [bentoBoxContainers, setBentoBoxContainers] = useState<SidecarContainer[]>([]);
  const [chatWidth, setChatWidth] = useState(600);

  // Create or show the bento box
  const createBentoBox = useCallback(() => {
    if (!hasBentoBox) {
      setHasBentoBox(true);
      // Start with one container
      const initialContainer: SidecarContainer = {
        id: `bento-${Date.now()}`,
        content: (
          <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
            <p>Sidecar content will go here</p>
          </div>
        ),
        contentType: 'sidecar',
        title: 'Sidecar'
      };
      setBentoBoxContainers([initialContainer]);
    }
  }, [hasBentoBox]);

  // Add content to bento box
  const addToBentoBox = useCallback((contentType: 'sidecar' | 'localhost' | 'file' | 'document-editor', filePath?: string) => {
    const newContainer: SidecarContainer = {
      id: `bento-${Date.now()}`,
      content: null,
      contentType: null
    };

    // Create content based on type
    if (contentType === 'sidecar') {
      newContainer.content = (
        <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
          <p>Sidecar content will go here</p>
        </div>
      );
      newContainer.contentType = 'sidecar';
      newContainer.title = 'Sidecar';
    } else if (contentType === 'localhost') {
      newContainer.content = <SidecarTabs initialUrl="http://localhost:3000" />;
      newContainer.contentType = 'localhost';
      newContainer.title = 'Localhost Viewer';
    } else if (contentType === 'file' && filePath) {
      newContainer.content = <FileViewer filePath={filePath} />;
      newContainer.contentType = 'file';
      newContainer.title = filePath?.split('/').pop() || 'File Viewer';
    } else if (contentType === 'document-editor') {
      const fileName = filePath ? filePath.split('/').pop() || filePath : 'Untitled Document';
      newContainer.content = <DocumentEditor filePath={filePath} placeholder="Start writing your document..." />;
      newContainer.contentType = 'document-editor';
      newContainer.title = fileName;
    }

    // If no bento box exists, create it first
    if (!hasBentoBox) {
      setHasBentoBox(true);
      setBentoBoxContainers([newContainer]);
    } else {
      // Add to existing bento box
      setBentoBoxContainers(prev => [...prev, newContainer]);
    }
  }, [hasBentoBox]);

  // Remove from bento box
  const removeFromBentoBox = useCallback((containerId: string) => {
    console.log('🔍 MainPanelLayout: removeFromBentoBox called with ID:', containerId);
    setBentoBoxContainers(prev => {
      console.log('🔍 MainPanelLayout: Current containers before removal:', prev.length);
      const updated = prev.filter(c => c.id !== containerId);
      console.log('🔍 MainPanelLayout: Containers after removal:', updated.length);
      
      // If no containers left, hide the bento box
      if (updated.length === 0) {
        console.log('🔍 MainPanelLayout: No containers left, hiding bento box');
        setHasBentoBox(false);
      }
      return updated;
    });
  }, []);

  // Handle chat panel resize
  const updateChatWidth = useCallback((delta: number) => {
    setChatWidth(prev => Math.max(300, Math.min(1000, prev + delta)));
  }, []);

  // Listen for add-container events from SidecarInvoker
  useEffect(() => {
    const handleAddContainer = (e: CustomEvent<{ type: 'sidecar' | 'localhost' | 'file' | 'document-editor'; filePath?: string }>) => {
      console.log('🔍 MainPanelLayout: Received add-container event:', e.detail.type, e.detail.filePath);
      addToBentoBox(e.detail.type, e.detail.filePath);
    };

    window.addEventListener('add-container', handleAddContainer as EventListener);
    return () => window.removeEventListener('add-container', handleAddContainer as EventListener);
  }, [addToBentoBox]);

  return (
    <div className="h-dvh">
      <div
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Chat Panel - Full width when no bento box, fixed width when bento box exists */}
        <div 
          className={hasBentoBox ? "flex flex-col flex-shrink-0" : "flex flex-col flex-1"}
          style={hasBentoBox ? { width: `${chatWidth}px` } : {}}
        >
          {children}
        </div>

        {/* Chat Resize Handle - only show when bento box exists */}
        {hasBentoBox && (
          <ResizeHandle
            onResize={updateChatWidth}
            isResizing={false}
          />
        )}

        {/* Bento Box - Single container that holds all sidecars */}
        {hasBentoBox && (
          <BentoBox
            containers={bentoBoxContainers}
            onRemoveContainer={removeFromBentoBox}
            onAddContainer={addToBentoBox}
          />
        )}
      </div>
    </div>
  );
};
