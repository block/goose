import React, { useState, useCallback, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  GripVertical, 
  GripHorizontal, 
  Grid2X2, 
  Columns2, 
  Rows2, 
  LayoutGrid,
  Maximize2,
  Minimize2,
  MoreVertical
} from 'lucide-react';
import { Button } from '../ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';

export type LayoutMode = 'single' | 'columns' | 'rows' | 'grid' | 'custom';

export interface PanelConfig {
  id: string;
  content: React.ReactNode;
  title?: string;
  minWidth?: number;
  minHeight?: number;
  position?: { row: number; col: number };
  size?: { width: number; height: number }; // Percentages
}

interface MultiPanelSplitterProps {
  leftContent: React.ReactNode;
  panels: PanelConfig[];
  layoutMode?: LayoutMode;
  onLayoutModeChange?: (mode: LayoutMode) => void;
  onPanelResize?: (panelId: string, size: { width: number; height: number }) => void;
  onPanelReorder?: (panels: PanelConfig[]) => void;
  className?: string;
  initialLeftWidth?: number; // Percentage for main content
}

interface ResizeHandle {
  type: 'vertical' | 'horizontal';
  position: number; // Percentage
  onResize: (delta: number) => void;
}

export const MultiPanelSplitter: React.FC<MultiPanelSplitterProps> = ({
  leftContent,
  panels,
  layoutMode = 'single',
  onLayoutModeChange,
  onPanelResize,
  onPanelReorder,
  className = '',
  initialLeftWidth = 60
}) => {
  const [leftWidth, setLeftWidth] = useState(initialLeftWidth);
  const [isDragging, setIsDragging] = useState(false);
  const [dragHandle, setDragHandle] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Panel layout state for different modes
  const [panelSizes, setPanelSizes] = useState<Record<string, { width: number; height: number }>>({});

  // Initialize panel sizes
  useEffect(() => {
    const initialSizes: Record<string, { width: number; height: number }> = {};
    panels.forEach(panel => {
      if (!panelSizes[panel.id]) {
        initialSizes[panel.id] = panel.size || { width: 50, height: 50 };
      }
    });
    if (Object.keys(initialSizes).length > 0) {
      setPanelSizes(prev => ({ ...prev, ...initialSizes }));
    }
  }, [panels, panelSizes]);

  const handleMainSplitterResize = useCallback((e: MouseEvent) => {
    if (!isDragging || !containerRef.current) return;

    const containerRect = containerRef.current.getBoundingClientRect();
    const containerWidth = containerRect.width;
    const mouseX = e.clientX - containerRect.left;
    
    const newLeftWidth = Math.min(Math.max((mouseX / containerWidth) * 100, 20), 80);
    setLeftWidth(newLeftWidth);
  }, [isDragging]);

  const handleMouseDown = useCallback((handleId: string) => {
    setIsDragging(true);
    setDragHandle(handleId);
  }, []);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    setDragHandle(null);
  }, []);

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMainSplitterResize);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      document.removeEventListener('mousemove', handleMainSplitterResize);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isDragging, handleMainSplitterResize, handleMouseUp]);

  const renderLayoutControls = () => (
    <div className="absolute top-3 right-3 z-50 flex items-center gap-2 bg-background-default/95 backdrop-blur-sm rounded-lg shadow-lg border border-border-subtle p-2">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant={layoutMode === 'columns' ? 'default' : 'ghost'}
            size="sm"
            onClick={() => onLayoutModeChange?.('columns')}
            className="h-8 w-8 p-0"
          >
            <Columns2 size={14} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Two Columns</TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant={layoutMode === 'rows' ? 'default' : 'ghost'}
            size="sm"
            onClick={() => onLayoutModeChange?.('rows')}
            className="h-8 w-8 p-0"
          >
            <Rows2 size={14} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Two Rows</TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant={layoutMode === 'grid' ? 'default' : 'ghost'}
            size="sm"
            onClick={() => onLayoutModeChange?.('grid')}
            className="h-8 w-8 p-0"
          >
            <Grid2X2 size={14} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>2x2 Grid</TooltipContent>
      </Tooltip>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
            <MoreVertical size={14} />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuItem onClick={() => onLayoutModeChange?.('single')}>
            Single Panel
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => onLayoutModeChange?.('custom')}>
            Custom Layout
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );

  const renderPanelContent = (panel: PanelConfig, style?: React.CSSProperties) => (
    <motion.div
      key={panel.id}
      layout
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.2 }}
      className="relative bg-background-default border border-border-subtle rounded-lg overflow-hidden"
      style={style}
    >
      {/* Panel Header */}
      {panel.title && (
        <div className="absolute top-0 left-0 right-0 z-10 bg-background-muted/95 backdrop-blur-sm px-3 py-2 border-b border-border-subtle">
          <span className="text-xs font-medium text-text-standard">{panel.title}</span>
        </div>
      )}
      
      {/* Panel Content */}
      <div className={`h-full w-full ${panel.title ? 'pt-10' : ''} overflow-hidden`}>
        {panel.content}
      </div>
    </motion.div>
  );

  const renderRightPanel = () => {
    if (panels.length === 0) {
      return (
        <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted/50 rounded-lg border border-border-subtle">
          <p>No panels active</p>
        </div>
      );
    }

    if (panels.length === 1) {
      return renderPanelContent(panels[0], { height: '100%' });
    }

    switch (layoutMode) {
      case 'columns':
        return (
          <div className="h-full flex gap-1">
            <AnimatePresence mode="popLayout">
              {panels.slice(0, 2).map((panel, index) => (
                renderPanelContent(panel, { 
                  flex: '1 1 0%', 
                  height: '100%',
                  minWidth: panel.minWidth || 200
                })
              ))}
            </AnimatePresence>
          </div>
        );

      case 'rows':
        return (
          <div className="h-full flex flex-col gap-1">
            <AnimatePresence mode="popLayout">
              {panels.slice(0, 2).map((panel, index) => (
                renderPanelContent(panel, { 
                  flex: '1 1 0%', 
                  width: '100%',
                  minHeight: panel.minHeight || 150
                })
              ))}
            </AnimatePresence>
          </div>
        );

      case 'grid':
        return (
          <div className="h-full grid grid-cols-2 grid-rows-2 gap-1">
            <AnimatePresence mode="popLayout">
              {panels.slice(0, 4).map((panel, index) => (
                renderPanelContent(panel, { 
                  minHeight: panel.minHeight || 150,
                  minWidth: panel.minWidth || 200
                })
              ))}
            </AnimatePresence>
          </div>
        );

      case 'custom':
        // For custom layout, use absolute positioning based on panel positions
        return (
          <div className="h-full relative">
            <AnimatePresence mode="popLayout">
              {panels.map((panel) => {
                const size = panelSizes[panel.id] || { width: 50, height: 50 };
                const position = panel.position || { row: 0, col: 0 };
                
                return renderPanelContent(panel, {
                  position: 'absolute',
                  left: `${position.col * 50}%`,
                  top: `${position.row * 50}%`,
                  width: `${size.width}%`,
                  height: `${size.height}%`,
                  minWidth: panel.minWidth || 200,
                  minHeight: panel.minHeight || 150
                });
              })}
            </AnimatePresence>
          </div>
        );

      default:
        return renderPanelContent(panels[0], { height: '100%' });
    }
  };

  const rightWidth = 100 - leftWidth;

  return (
    <div 
      ref={containerRef}
      className={`flex h-full w-full relative ${className}`}
    >
      {/* Left Panel (Main Content) */}
      <div 
        className="flex-shrink-0 overflow-hidden"
        style={{ width: `${leftWidth}%` }}
      >
        {leftContent}
      </div>

      {/* Main Splitter */}
      {panels.length > 0 && (
        <div
          className={`
            relative flex-shrink-0 w-1 bg-background-default cursor-col-resize 
            hover:bg-border-default transition-colors duration-150
            group
            ${isDragging && dragHandle === 'main' ? 'bg-border-default' : ''}
          `}
          onMouseDown={() => handleMouseDown('main')}
        >
          {/* Visual grip indicator */}
          <div className={`
            absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2
            opacity-0 group-hover:opacity-100 transition-opacity duration-150
            bg-background-default border border-border-subtle rounded p-1
            ${isDragging && dragHandle === 'main' ? 'opacity-100' : ''}
          `}>
            <GripVertical className="w-3 h-3 text-text-muted" />
          </div>
          
          {/* Invisible wider hit area for easier dragging */}
          <div className="absolute inset-y-0 -left-2 -right-2 cursor-col-resize" />
        </div>
      )}

      {/* Right Panel (Multi-Panel Area) */}
      {panels.length > 0 && (
        <div 
          className="flex-1 relative bg-background-default overflow-hidden"
          style={{ width: `${rightWidth}%` }}
        >
          <div className="absolute inset-0 p-4">
            <div className="h-full w-full rounded-2xl shadow-2xl drop-shadow-2xl border border-border-subtle overflow-hidden relative">
              {renderRightPanel()}
              {renderLayoutControls()}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default MultiPanelSplitter;
