import React, { useState, useRef, useCallback, useEffect } from 'react';
import { DashboardWidget } from './DashboardWidget';
import { WidgetPosition, WidgetData } from '../../types/dashboard';

interface DashboardCanvasProps {
  widgets: WidgetData[];
  onWidgetMove: (id: string, position: WidgetPosition) => void;
  onWidgetResize: (id: string, size: { width: number; height: number }) => void;
}

export function DashboardCanvas({ widgets, onWidgetMove, onWidgetResize }: DashboardCanvasProps) {
  const canvasRef = useRef<HTMLDivElement>(null);
  const [draggedWidget, setDraggedWidget] = useState<string | null>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  const [dragPosition, setDragPosition] = useState<{ x: number; y: number } | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent, widgetId: string) => {
    e.preventDefault();
    if (!canvasRef.current) return;
    
    const rect = canvasRef.current.getBoundingClientRect();
    const widget = widgets.find(w => w.id === widgetId);
    if (!widget) return;

    const offsetX = e.clientX - rect.left - widget.position.x;
    const offsetY = e.clientY - rect.top - widget.position.y;

    setDraggedWidget(widgetId);
    setDragOffset({ x: offsetX, y: offsetY });
    setDragPosition({ x: widget.position.x, y: widget.position.y });

    // Add cursor style to body to show dragging state
    document.body.style.cursor = 'grabbing';
    document.body.style.userSelect = 'none';
  }, [widgets]);

  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!draggedWidget || !canvasRef.current) return;
    
    e.preventDefault();
    
    const rect = canvasRef.current.getBoundingClientRect();
    const widget = widgets.find(w => w.id === draggedWidget);
    if (!widget) return;

    const newX = Math.max(0, Math.min(rect.width - widget.size.width, e.clientX - rect.left - dragOffset.x));
    const newY = Math.max(0, Math.min(rect.height - widget.size.height, e.clientY - rect.top - dragOffset.y));
    
    // Use requestAnimationFrame for smooth updates
    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
    }
    
    animationFrameRef.current = requestAnimationFrame(() => {
      setDragPosition({ x: newX, y: newY });
    });
  }, [draggedWidget, dragOffset, widgets]);

  const handleMouseUp = useCallback(() => {
    if (draggedWidget && dragPosition) {
      // Commit the final position
      onWidgetMove(draggedWidget, dragPosition);
    }
    
    setDraggedWidget(null);
    setDragOffset({ x: 0, y: 0 });
    setDragPosition(null);
    
    // Reset cursor styles
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    
    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }
  }, [draggedWidget, dragPosition, onWidgetMove]);

  // Use document-level event listeners for smoother dragging
  useEffect(() => {
    if (draggedWidget) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [draggedWidget, handleMouseMove, handleMouseUp]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
  }, []);

  return (
    <div 
      ref={canvasRef}
      className="relative w-full h-full overflow-hidden"
    >
      {/* Enhanced monochromatic gradient with better light mode contrast */}
      <div 
        className="absolute inset-0 animate-gradient-slow"
        style={{
          background: `
            radial-gradient(circle at 20% 80%, rgba(100, 100, 110, 0.25) 0%, transparent 50%),
            radial-gradient(circle at 80% 20%, rgba(120, 120, 125, 0.22) 0%, transparent 50%),
            radial-gradient(circle at 40% 40%, rgba(90, 95, 100, 0.18) 0%, transparent 50%),
            linear-gradient(135deg, 
              rgba(0, 0, 0, 0.02) 0%, 
              rgba(0, 0, 0, 0.04) 25%, 
              rgba(0, 0, 0, 0.02) 50%, 
              rgba(0, 0, 0, 0.06) 75%, 
              rgba(0, 0, 0, 0.03) 100%
            )
          `,
          backgroundSize: '400% 400%',
        }}
      />
      
      {/* Dot pattern overlay */}
      <div 
        className="absolute inset-0 opacity-10 dark:opacity-20"
        style={{
          backgroundImage: `radial-gradient(circle, rgba(0, 0, 0, 0.4) 1px, transparent 1px)`,
          backgroundSize: '24px 24px',
          backgroundPosition: '12px 12px',
        }}
      />
      
      {/* Widgets */}
      {widgets.map((widget) => {
        const isDragging = draggedWidget === widget.id;
        const position = isDragging && dragPosition ? dragPosition : widget.position;
        
        return (
          <DashboardWidget
            key={widget.id}
            widget={{ ...widget, position }}
            onMouseDown={(e) => handleMouseDown(e, widget.id)}
            isDragging={isDragging}
          />
        );
      })}
    </div>
  );
}
