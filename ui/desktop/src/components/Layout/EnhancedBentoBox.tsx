import React, { useState, useCallback, useRef, useEffect } from 'react';
import { motion, AnimatePresence, Reorder, useDragControls } from 'framer-motion';
import { Plus, X, Globe, FileText, Grid3X3, LayoutGrid, Maximize2, Minimize2, GripVertical } from 'lucide-react';
import { Button } from '../ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';

export interface SidecarContainer {
  id: string;
  content: React.ReactNode;
  contentType: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer' | null;
  title?: string;
  size?: 'small' | 'medium' | 'large';
  position?: { row: number; col: number };
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
    { type: 'sidecar' as const, icon: Plus, label: 'Sidecar View' },
    { type: 'localhost' as const, icon: Globe, label: 'Localhost Viewer' },
    { type: 'file' as const, icon: FileText, label: 'Open File' },
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

interface DraggableContainerProps {
  container: SidecarContainer;
  onRemove: (id: string) => void;
  onResize: (id: string, size: 'small' | 'medium' | 'large') => void;
  layout: LayoutType;
  style?: React.CSSProperties;
  className?: string;
}

const DraggableContainer: React.FC<DraggableContainerProps> = ({ 
  container, 
  onRemove, 
  onResize, 
  layout,
  style,
  className 
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);
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

  return (
    <motion.div
      layout
      initial={{ opacity: 0, scale: 0.8 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.8, transition: { duration: 0.2 } }}
      whileHover={{ y: -2 }}
      transition={{ 
        layout: { duration: 0.3, ease: "easeInOut" },
        default: { duration: 0.2 }
      }}
      className={`relative bg-background-default border border-border-subtle rounded-lg overflow-hidden shadow-sm hover:shadow-md transition-shadow ${getSizeClass(container.size)} ${className || ''}`}
      style={style}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      dragControls={controls}
    >
      {/* Container Header */}
      <AnimatePresence>
        {isHovered && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.15 }}
            className="absolute top-0 left-0 right-0 z-10 bg-background-muted/95 backdrop-blur-sm border-b border-border-subtle p-2 flex items-center justify-between"
          >
            <div className="flex items-center gap-2">
              <motion.div
                className="cursor-grab active:cursor-grabbing p-1 hover:bg-background-default rounded"
                onPointerDown={(e) => controls.start(e)}
                whileHover={{ scale: 1.1 }}
                whileTap={{ scale: 0.9 }}
              >
                <GripVertical size={14} className="text-text-subtle" />
              </motion.div>
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
          </motion.div>
        )}
      </AnimatePresence>

      {/* Container Content */}
      <div className={`h-full w-full ${isHovered ? 'pt-10' : ''} transition-all duration-150`}>
        {container.content || (
          <div className="h-full w-full flex flex-col items-center justify-center p-4 space-y-3 bg-background-muted/50">
            <div className="w-8 h-8 rounded-lg bg-border-subtle animate-pulse" />
            <p className="text-text-muted text-sm text-center">Empty container</p>
          </div>
        )}
      </div>
    </motion.div>
  );
};

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
  const [layout, setLayout] = useState<LayoutType>('horizontal');
  const [showPopover, setShowPopover] = useState(false);
  const [popoverPosition, setPopoverPosition] = useState({ x: 0, y: 0 });
  const [isAddHovered, setIsAddHovered] = useState(false);

  const handleReorder = useCallback((newContainers: SidecarContainer[]) => {
    onReorderContainers?.(newContainers);
  }, [onReorderContainers]);

  const handleContainerResize = useCallback((id: string, size: 'small' | 'medium' | 'large') => {
    const updatedContainers = containers.map(container =>
      container.id === id ? { ...container, size } : container
    );
    onReorderContainers?.(updatedContainers);
  }, [containers, onReorderContainers]);

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
        return 'flex flex-col gap-4 overflow-y-auto';
      case 'grid':
        return 'grid grid-cols-3 auto-rows-fr gap-4 overflow-auto';
      case 'masonry':
        return 'columns-2 gap-4 overflow-auto';
      default: // horizontal
        return 'flex gap-4 overflow-x-auto';
    }
  };

  const getContainerStyle = (index: number) => {
    if (layout === 'horizontal') {
      return { minWidth: '300px', width: `${100 / Math.max(containers.length, 1)}%` };
    }
    if (layout === 'vertical') {
      return { minHeight: '200px', height: `${100 / Math.max(containers.length, 1)}%` };
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
      className="flex-1 h-full bg-background-default rounded-xl overflow-hidden relative border border-border-subtle"
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.3 }}
    >
      {/* Header Controls */}
      <motion.div 
        className="absolute top-3 left-3 right-3 z-20 flex items-center justify-between"
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.1 }}
      >
        <div className="flex items-center gap-2">
          <motion.div 
            className="text-xs font-medium text-text-standard bg-background-muted px-2 py-1 rounded-md border border-border-subtle"
            whileHover={{ scale: 1.05 }}
          >
            {containers.length} container{containers.length !== 1 ? 's' : ''}
          </motion.div>
        </div>
        
        <div className="flex items-center gap-2">
          {/* Layout Switcher */}
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
            className="w-6 h-6 rounded-full bg-background-muted hover:bg-red-500 text-text-subtle hover:text-white transition-colors border border-border-subtle flex items-center justify-center"
          >
            <X size={12} />
          </motion.button>
        </div>
      </motion.div>

      {/* Container Area */}
      <div className="h-full pt-16 p-4">
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
                  dragControls={false}
                  className="flex-shrink-0"
                  style={getContainerStyle(index)}
                >
                  <DraggableContainer
                    container={container}
                    onRemove={onRemoveContainer}
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
                  onRemove={onRemoveContainer}
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
