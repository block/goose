import React, { useState, useCallback, useRef, useEffect } from 'react';
import { motion, AnimatePresence, Reorder, useDragControls } from 'framer-motion';
import { Plus, X, Globe, FileText, Grid3X3, LayoutGrid, Maximize2, Minimize2, GripVertical, Monitor, Edit } from 'lucide-react';
import { Button } from '../ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';
import { useNavigation } from './AppLayout';
import SidecarTabs from '../SidecarTabs';
import { FileViewer } from '../FileViewer';
import DocumentEditor from '../DocumentEditor';
import WebViewer from '../WebViewer';
import AppInstaller from '../AppInstaller';

export interface SidecarContainer {
  id: string;
  content: React.ReactNode;
  contentType: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer' | null;
  title?: string;
  size?: 'small' | 'medium' | 'large';
  position?: { row: number; col: number };
  // Store props for stable rendering
  contentProps?: {
    initialUrl?: string;
    allowAllSites?: boolean;
    filePath?: string;
    placeholder?: string;
  };
}

type LayoutType = 'horizontal' | 'vertical' | 'grid' | 'masonry';

interface ContainerPopoverProps {
  onSelect: (type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer') => void;
  onClose: () => void;
  position: { x: number; y: number };
}

const ContainerPopover: React.FC<ContainerPopoverProps> = ({ onSelect, onClose, position }) => {
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  const options = [
    { type: 'web-viewer' as const, icon: Monitor, label: 'Web Browser' },
    { type: 'localhost' as const, icon: Globe, label: 'Localhost Viewer' },
    { type: 'file' as const, icon: FileText, label: 'Open File' },
    { type: 'document-editor' as const, icon: Edit, label: 'Text Editor' },
    { type: 'sidecar' as const, icon: Plus, label: 'Sidecar View' },
  ];

  return (
    <motion.div
      ref={popoverRef}
      initial={{ opacity: 0, scale: 0.95, y: -10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95, y: -10 }}
      transition={{ duration: 0.15 }}
      className="fixed z-50 bg-background-default border border-border-subtle rounded-lg shadow-xl p-2 min-w-[160px]"
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        transform: 'translate(-100%, -50%)'
      }}
    >
      <div className="space-y-1">
        {options.map(({ type, icon: Icon, label }) => (
          <motion.div
            key={type}
            whileHover={{ x: 2 }}
            whileTap={{ scale: 0.98 }}
          >
            <Button
              onClick={() => {
                onSelect(type);
                onClose();
              }}
              className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
              variant="ghost"
              size="sm"
            >
              <Icon className="w-4 h-4 mr-2" />
              {label}
            </Button>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
};

// Completely isolated WebViewer component that only depends on containerId and initialUrl
const IsolatedWebViewer = React.memo<{ containerId: string; initialUrl: string }>(({ containerId, initialUrl }) => {
  console.log('🔍 IsolatedWebViewer: Rendering for container:', containerId, 'URL:', initialUrl);
  
  return (
    <WebViewer 
      initialUrl={initialUrl} 
      allowAllSites={true} 
    />
  );
}, (prevProps, nextProps) => {
  // Only re-render if containerId or initialUrl actually changes
  return prevProps.containerId === nextProps.containerId && prevProps.initialUrl === nextProps.initialUrl;
});

// Stable content renderer component - defined outside to avoid recreation
const ContainerContentRenderer = React.memo<{ container: SidecarContainer }>(({ container }) => {
  console.log('🔍 ContainerContentRenderer: Rendering for container:', container.id, container.contentType);
  
  // If we have legacy content (JSX), use it
  if (container.content) {
    return <>{container.content}</>;
  }

  // Otherwise, render based on contentType and contentProps
  switch (container.contentType) {
    case 'sidecar':
      return (
        <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
          <p>Sidecar content will go here</p>
        </div>
      );
    case 'localhost':
      return <SidecarTabs initialUrl="http://localhost:3000" />;
    case 'file':
      return container.contentProps?.filePath ? (
        <FileViewer filePath={container.contentProps.filePath} />
      ) : (
        <div className="h-full w-full flex items-center justify-center text-text-muted">
          <p>No file path specified</p>
        </div>
      );
    case 'document-editor':
      return (
        <DocumentEditor 
          filePath={container.contentProps?.filePath} 
          placeholder={container.contentProps?.placeholder || "Start writing your document..."} 
        />
      );
    case 'web-viewer':
      // Create a completely isolated WebViewer instance
      return <IsolatedWebViewer containerId={container.id} initialUrl={container.contentProps?.initialUrl || "https://google.com"} />;
    case 'app-installer':
      return <AppInstaller />;
    default:
      return (
        <div className="h-full w-full flex flex-col items-center justify-center p-4 space-y-3 bg-background-muted/50">
          <div className="w-8 h-8 rounded-lg bg-border-subtle animate-pulse" />
          <p className="text-text-muted text-sm text-center">Empty container</p>
        </div>
      );
  }
}, (prevProps, nextProps) => {
  // Custom comparison function to prevent unnecessary re-renders
  const prev = prevProps.container;
  const next = nextProps.container;
  
  // Compare all the important properties
  return (
    prev.id === next.id &&
    prev.contentType === next.contentType &&
    prev.title === next.title &&
    prev.size === next.size &&
    prev.contentProps?.initialUrl === next.contentProps?.initialUrl &&
    prev.contentProps?.allowAllSites === next.contentProps?.allowAllSites &&
    prev.contentProps?.filePath === next.contentProps?.filePath &&
    prev.contentProps?.placeholder === next.contentProps?.placeholder
  );
});

interface DraggableContainerProps {
  container: SidecarContainer;
  onRemove: (id: string) => void;
  onResize: (id: string, size: 'small' | 'medium' | 'large') => void;
  layout: LayoutType;
  style?: React.CSSProperties;
  className?: string;
  enableCustomDrag?: boolean; // For grid layout where we don't use Reorder.Item
}

const DraggableContainer = React.memo<DraggableContainerProps>(({ 
  container, 
  onRemove, 
  onResize, 
  layout,
  style,
  className,
  enableCustomDrag = false
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const controls = useDragControls();

  const getSizeClass = (size: string = 'medium') => {
    if (layout === 'grid') {
      switch (size) {
        case 'small': return 'col-span-1 row-span-1';
        case 'large': return 'col-span-2 row-span-2';
        default: return 'col-span-1 row-span-1';
      }
    }
    return '';
  };

  const handleSizeToggle = () => {
    const currentSize = container.size || 'medium';
    const nextSize = currentSize === 'small' ? 'medium' : currentSize === 'medium' ? 'large' : 'small';
    onResize(container.id, nextSize);
  };

  // For horizontal/vertical layouts, Reorder.Item handles dragging
  // For grid layout, we handle dragging ourselves
  const shouldEnableDrag = layout === 'grid' || layout === 'masonry';
  
  return (
    <motion.div
      layout={!isDragging} // Disable layout animation during drag
      initial={{ opacity: 0, scale: 0.8 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.8, transition: { duration: 0.2 } }}
      whileHover={!isDragging && shouldEnableDrag ? { y: -2 } : {}} // Only enable hover for grid/masonry
      {...(shouldEnableDrag && {
        whileDrag: { 
          scale: 1.02, 
          zIndex: 1000,
          boxShadow: "0 10px 30px rgba(0,0,0,0.3)"
        },
        onDragStart: () => setIsDragging(true),
        onDragEnd: () => setIsDragging(false),
        dragControls: controls
      })}
      transition={{ 
        layout: { duration: 0.3, ease: "easeInOut" },
        default: { duration: 0.2 }
      }}
      className={`relative bg-white overflow-hidden rounded-lg ${getSizeClass(container.size)} ${className || ''} ${isDragging ? 'cursor-grabbing' : ''}`}
      style={{
        ...style,
        zIndex: isDragging ? 1000 : 'auto'
      }}
    >
      {/* Container Header - Always Visible */}
      <div className="absolute top-0 left-0 right-0 z-10 bg-background-muted/95 backdrop-blur-sm p-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          {/* Only show grip handle for grid/masonry layouts */}
          {shouldEnableDrag && (
            <motion.div
              className="cursor-grab active:cursor-grabbing p-1 hover:bg-background-default rounded"
              onPointerDown={(e) => controls.start(e)}
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.9 }}
            >
              <GripVertical size={14} className="text-text-subtle" />
            </motion.div>
          )}
          <span className="text-xs font-medium text-text-standard truncate">
            {container.title || 'Container'}
          </span>
        </div>
        
        <div className="flex items-center gap-1">
          {layout === 'grid' && (
            <Tooltip>
              <TooltipTrigger asChild>
                <motion.button
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.9 }}
                  onClick={handleSizeToggle}
                  className="p-1 hover:bg-background-default rounded text-text-subtle hover:text-text-standard"
                >
                  {container.size === 'large' ? <Minimize2 size={12} /> : <Maximize2 size={12} />}
                </motion.button>
              </TooltipTrigger>
              <TooltipContent>Resize Container</TooltipContent>
            </Tooltip>
          )}
          
          <Tooltip>
            <TooltipTrigger asChild>
              <motion.button
                whileHover={{ scale: 1.1, backgroundColor: '#ef4444' }}
                whileTap={{ scale: 0.9 }}
                onClick={() => onRemove(container.id)}
                className="p-1 hover:bg-red-500 rounded text-text-subtle hover:text-white transition-colors"
              >
                <X size={12} />
              </motion.button>
            </TooltipTrigger>
            <TooltipContent>Remove Container</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* Container Content - Minimal top padding for header */}
      <div className="h-full w-full pt-8 overflow-hidden">
        <div className="h-full w-full">
          <ContainerContentRenderer container={container} />
        </div>
      </div>
    </motion.div>
  );
}, (prevProps, nextProps) => {
  // Custom comparison function to prevent unnecessary re-renders
  const prev = prevProps.container;
  const next = nextProps.container;
  
  // Compare all the important properties
  return (
    prev.id === next.id &&
    prev.contentType === next.contentType &&
    prev.title === next.title &&
    prev.size === next.size &&
    prev.contentProps?.initialUrl === next.contentProps?.initialUrl &&
    prev.contentProps?.allowAllSites === next.contentProps?.allowAllSites &&
    prev.contentProps?.filePath === next.contentProps?.filePath &&
    prev.contentProps?.placeholder === next.contentProps?.placeholder &&
    prevProps.layout === nextProps.layout &&
    prevProps.className === nextProps.className &&
    prevProps.enableCustomDrag === nextProps.enableCustomDrag
    // Note: We don't compare onRemove, onResize, or style as they may be recreated but functionally equivalent
  );
});

interface EnhancedBentoBoxProps {
  containers: SidecarContainer[];
  onRemoveContainer: (containerId: string) => void;
  onAddContainer: (type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer', filePath?: string, url?: string, title?: string) => void;
  onReorderContainers?: (containers: SidecarContainer[]) => void;
}

export const EnhancedBentoBox: React.FC<EnhancedBentoBoxProps> = ({ 
  containers, 
  onRemoveContainer, 
  onAddContainer,
  onReorderContainers 
}) => {
  // Enhanced container removal handler that destroys child windows
  const handleContainerRemoval = useCallback((containerId: string) => {
    console.log('🔍 EnhancedBentoBox: Removing container:', containerId);
    
    // Find the container being removed
    const containerToRemove = containers.find(c => c.id === containerId);
    
    if (containerToRemove && containerToRemove.contentType === 'web-viewer') {
      console.log('🔍 EnhancedBentoBox: Container is web-viewer, destroying child window');
      
      // Get the child window registry
      const childWindowRegistry = (window as any).childWindowRegistry;
      
      if (childWindowRegistry && containerToRemove.contentProps?.initialUrl) {
        // Determine if it's allowAllSites based on the URL or default to true
        const allowAllSites = containerToRemove.contentProps.allowAllSites !== false;
        const initialUrl = containerToRemove.contentProps.initialUrl;
        
        console.log('🔍 EnhancedBentoBox: Destroying child window for URL:', initialUrl, 'allowAllSites:', allowAllSites);
        
        // Explicitly destroy the child window
        childWindowRegistry.destroyWindowByUrl(initialUrl, allowAllSites);
      } else {
        console.warn('🔍 EnhancedBentoBox: Child window registry not available or no initial URL');
      }
    }
    
    // Call the original removal handler
    onRemoveContainer(containerId);
  }, [containers, onRemoveContainer]);
  const [layout, setLayout] = useState<LayoutType>('horizontal');
  const [showPopover, setShowPopover] = useState(false);
  const [popoverPosition, setPopoverPosition] = useState({ x: 0, y: 0 });
  const [isAddHovered, setIsAddHovered] = useState(false);
  const { isNavExpanded } = useNavigation();
  
  // Use ref to store current containers to avoid callback dependencies
  const containersRef = useRef(containers);
  containersRef.current = containers;
  
  // Debug containers changes
  useEffect(() => {
    console.log('🔍 EnhancedBentoBox: containers prop changed:', {
      length: containers.length,
      containers: containers.map(c => ({ 
        id: c.id, 
        type: c.contentType,
        title: c.title,
        hasContent: !!c.content,
        hasProps: !!c.contentProps,
        propsKeys: c.contentProps ? Object.keys(c.contentProps) : []
      }))
    });
  }, [containers]);
  
  const handleReorder = useCallback((newContainers: SidecarContainer[]) => {
    onReorderContainers?.(newContainers);
  }, [onReorderContainers]);

  const handleContainerResize = useCallback((id: string, size: 'small' | 'medium' | 'large') => {
    // Use ref to get current containers and avoid dependency
    const updatedContainers = containersRef.current.map(container =>
      container.id === id ? { ...container, size } : container
    );
    onReorderContainers?.(updatedContainers);
  }, [onReorderContainers]);

  const handleAddClick = (e: React.MouseEvent) => {
    const rect = e.currentTarget.getBoundingClientRect();
    setPopoverPosition({
      x: rect.left,
      y: rect.bottom + 8
    });
    setShowPopover(true);
  };

  const getLayoutClasses = () => {
    switch (layout) {
      case 'vertical':
        return 'flex flex-col gap-1 h-full'; // Ensure full height
      case 'grid':
        return 'grid grid-cols-3 auto-rows-fr gap-1 h-full overflow-auto';
      case 'masonry':
        return 'columns-2 gap-1 h-full overflow-auto';
      default: // horizontal
        return 'flex gap-1 h-full overflow-x-auto';
    }
  };

  const getContainerStyle = (index: number) => {
    if (layout === 'horizontal') {
      return { 
        minWidth: '300px', 
        flex: '1 1 0%', // Use flex instead of percentage for better distribution
        width: `${100 / Math.max(containers.length, 1)}%` 
      };
    }
    if (layout === 'vertical') {
      return { 
        minHeight: '200px', 
        flex: '1 1 0%', // Use flex instead of percentage for better distribution
        height: `${100 / Math.max(containers.length, 1)}%` 
      };
    }
    if (layout === 'masonry') {
      return { breakInside: 'avoid', marginBottom: '1rem' };
    }
    return {};
  };

  const layoutIcons = {
    horizontal: LayoutGrid,
    vertical: Grid3X3,
    grid: Grid3X3,
    masonry: LayoutGrid
  };

  return (
    <motion.div 
      layout
      className="flex-1 h-full bg-white rounded-xl overflow-hidden relative border border-border-subtle"
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.3 }}
    >
      {/* Header Controls */}
      <motion.div 
        className={`absolute top-3 left-3 z-[100] flex items-center justify-between bg-background-default/95 backdrop-blur-sm rounded-lg shadow-lg transition-all duration-300`}
        initial={{ opacity: 0, y: -20 }}
        animate={{ 
          opacity: 1, 
          y: 0,
          right: isNavExpanded ? '12px' : '170px' // Custom 170px offset when navigation is closed
        }}
        transition={{ delay: 0.1, duration: 0.3 }}
        style={{ zIndex: 100 }} // Lower z-index to stay below main navigation
      >
        <div className="flex items-center gap-4">
          {/* Layout Switcher - Moved to left side where counter was */}
          <div className="flex items-center bg-background-muted rounded-md border border-border-subtle overflow-hidden">
            {(['horizontal', 'vertical', 'grid'] as LayoutType[]).map((layoutType) => {
              const Icon = layoutIcons[layoutType];
              return (
                <Tooltip key={layoutType}>
                  <TooltipTrigger asChild>
                    <motion.button
                      whileHover={{ backgroundColor: 'var(--background-default)' }}
                      whileTap={{ scale: 0.95 }}
                      onClick={() => setLayout(layoutType)}
                      className={`p-2 transition-colors ${
                        layout === layoutType 
                          ? 'bg-background-default text-text-standard shadow-sm' 
                          : 'text-text-subtle hover:text-text-standard'
                      }`}
                    >
                      <Icon size={14} />
                    </motion.button>
                  </TooltipTrigger>
                  <TooltipContent>{layoutType.charAt(0).toUpperCase() + layoutType.slice(1)} Layout</TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </div>
        
        <div className="flex items-center gap-4">
          {/* Add Container Button */}
          <motion.div
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Button
              onClick={handleAddClick}
              onMouseEnter={() => setIsAddHovered(true)}
              onMouseLeave={() => setIsAddHovered(false)}
              className="h-8 w-8 p-0 bg-primary hover:bg-primary/90 text-primary-foreground shadow-md"
              size="sm"
            >
              <motion.div
                animate={{ rotate: isAddHovered ? 90 : 0 }}
                transition={{ duration: 0.2 }}
              >
                <Plus size={14} />
              </motion.div>
            </Button>
          </motion.div>

          {/* Close Button */}
          <motion.button
            whileHover={{ scale: 1.1, backgroundColor: '#ef4444' }}
            whileTap={{ scale: 0.9 }}
            onClick={() => containers.forEach(container => onRemoveContainer(container.id))}
            className="w-8 h-8 rounded-full bg-background-muted hover:bg-red-500 text-text-subtle hover:text-white transition-colors border border-border-subtle flex items-center justify-center"
          >
            <X size={12} />
          </motion.button>
        </div>
      </motion.div>

      {/* Container Area */}
      <div className="h-full p-1" style={{ paddingTop: '58px' }}> {/* 58px spacing from header */}
        {layout === 'horizontal' || layout === 'vertical' ? (
          <Reorder.Group
            axis={layout === 'horizontal' ? 'x' : 'y'}
            values={containers}
            onReorder={handleReorder}
            className={getLayoutClasses()}
          >
            <AnimatePresence mode="popLayout">
              {containers.map((container, index) => (
                <Reorder.Item
                  key={container.id}
                  value={container}
                  className="" // Remove flex-shrink-0 to allow proper flexing
                  style={getContainerStyle(index)}
                  whileDrag={{
                    zIndex: 1000,
                    scale: 1.02,
                    boxShadow: "0 10px 30px rgba(0,0,0,0.3)"
                  }}
                  dragTransition={{ 
                    bounceStiffness: 300, 
                    bounceDamping: 30 
                  }}
                >
                  <DraggableContainer
                    key={`draggable-${container.id}`}
                    container={container}
                    onRemove={handleContainerRemoval}
                    onResize={handleContainerResize}
                    layout={layout}
                    className="h-full"
                  />
                </Reorder.Item>
              ))}
            </AnimatePresence>
          </Reorder.Group>
        ) : (
          <div className={getLayoutClasses()}>
            <AnimatePresence mode="popLayout">
              {containers.map((container, index) => (
                <DraggableContainer
                  key={container.id}
                  container={container}
                  onRemove={handleContainerRemoval}
                  onResize={handleContainerResize}
                  layout={layout}
                  style={getContainerStyle(index)}
                  className={layout === 'grid' ? 'min-h-[200px]' : ''}
                />
              ))}
            </AnimatePresence>
          </div>
        )}

        {/* Empty State */}
        {containers.length === 0 && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="h-full flex flex-col items-center justify-center text-text-subtle"
          >
            <motion.div
              animate={{ 
                scale: [1, 1.1, 1],
                rotate: [0, 5, -5, 0]
              }}
              transition={{ 
                duration: 2,
                repeat: Infinity,
                repeatType: "reverse"
              }}
              className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center mb-4"
            >
              <LayoutGrid size={24} className="text-primary" />
            </motion.div>
            <p className="text-lg font-medium mb-2">Your workspace is empty</p>
            <p className="text-sm text-center max-w-md">
              Click the <Plus className="inline w-4 h-4 mx-1" /> button to add containers and start building your layout
            </p>
          </motion.div>
        )}
      </div>

      {/* Add Container Popover */}
      <AnimatePresence>
        {showPopover && (
          <ContainerPopover
            onSelect={(type) => {
              onAddContainer(type);
              setShowPopover(false);
            }}
            onClose={() => setShowPopover(false)}
            position={popoverPosition}
          />
        )}
      </AnimatePresence>
    </motion.div>
  );
};

export default EnhancedBentoBox;
