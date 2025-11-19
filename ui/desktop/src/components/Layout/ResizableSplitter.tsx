import React, { useState, useCallback, useRef, useEffect } from 'react';
import { GripVertical } from 'lucide-react';

interface ResizableSplitterProps {
  leftContent: React.ReactNode;
  rightContent: React.ReactNode;
  initialLeftWidth?: number; // Percentage (0-100)
  minLeftWidth?: number; // Percentage (0-100)
  maxLeftWidth?: number; // Percentage (0-100)
  onResize?: (leftWidth: number) => void;
  className?: string;
  floatingRight?: boolean; // Whether right panel should float above background
}

export const ResizableSplitter: React.FC<ResizableSplitterProps> = ({
  leftContent,
  rightContent,
  initialLeftWidth = 60,
  minLeftWidth = 20,
  maxLeftWidth = 80,
  onResize,
  className = '',
  floatingRight = false
}) => {
  const [leftWidth, setLeftWidth] = useState(initialLeftWidth);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const splitterRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging || !containerRef.current) return;

    const containerRect = containerRef.current.getBoundingClientRect();
    const containerWidth = containerRect.width;
    const mouseX = e.clientX - containerRect.left;
    
    // Calculate new left width as percentage
    const newLeftWidth = Math.min(
      Math.max((mouseX / containerWidth) * 100, minLeftWidth),
      maxLeftWidth
    );

    setLeftWidth(newLeftWidth);
    onResize?.(newLeftWidth);
  }, [isDragging, minLeftWidth, maxLeftWidth, onResize]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, [isDragging, handleMouseMove, handleMouseUp]);

  const rightWidth = 100 - leftWidth;

  return (
    <div 
      ref={containerRef}
      className={`flex h-full w-full ${className}`}
    >
      {/* Left Panel */}
      <div 
        className="flex-shrink-0 overflow-hidden"
        style={{ width: `${leftWidth}%` }}
      >
        {leftContent}
      </div>

      {/* Resizable Splitter */}
      <div
        ref={splitterRef}
        className={`
          relative flex-shrink-0 w-1 bg-border-subtle cursor-col-resize 
          hover:bg-border-default transition-colors duration-150
          group
          ${isDragging ? 'bg-border-default' : ''}
        `}
        onMouseDown={handleMouseDown}
      >
        {/* Visual grip indicator */}
        <div className={`
          absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2
          opacity-0 group-hover:opacity-100 transition-opacity duration-150
          bg-background-default border border-border-subtle rounded p-1
          ${isDragging ? 'opacity-100' : ''}
        `}>
          <GripVertical className="w-3 h-3 text-text-muted" />
        </div>
        
        {/* Invisible wider hit area for easier dragging */}
        <div className="absolute inset-y-0 -left-2 -right-2 cursor-col-resize" />
      </div>

      {/* Right Panel */}
      <div 
        className={`flex-1 overflow-hidden ${floatingRight ? 'relative' : ''}`}
        style={{ width: `${rightWidth}%` }}
      >
        {floatingRight ? (
          <div className="absolute inset-0 p-4">
            <div className="h-full w-full rounded-lg shadow-2xl drop-shadow-2xl border border-border-subtle overflow-hidden">
              {rightContent}
            </div>
          </div>
        ) : (
          rightContent
        )}
      </div>
    </div>
  );
};

export default ResizableSplitter;
