import React, { useState, useEffect } from 'react';
import { CardContent, CardDescription } from '../ui/card';
import { WidgetData, WidgetType } from '../../types/dashboard';
import { Button } from '../ui/button';
import { ChatSmart } from '../icons/';
import { useNavigate } from 'react-router-dom';

interface DashboardWidgetProps {
  widget: WidgetData;
  onMouseDown: (e: React.MouseEvent) => void;
  isDragging: boolean;
  onReset?: () => void; // Optional reset callback
}

export function DashboardWidget({ widget, onMouseDown, isDragging, onReset }: DashboardWidgetProps) {
  const [showSavedIndicator, setShowSavedIndicator] = useState(false);
  const navigate = useNavigate();

  // Show saved indicator when position changes (but not during dragging)
  useEffect(() => {
    if (!isDragging) {
      setShowSavedIndicator(true);
      const timer = setTimeout(() => {
        setShowSavedIndicator(false);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [widget.position.x, widget.position.y, isDragging]);

  const handleSessionClick = (sessionId: string) => {
    // Navigate to sessions view with the selected session
    navigate('/sessions', { state: { selectedSessionId: sessionId } });
  };

  const handleSeeAllClick = () => {
    // Navigate to sessions view (history space)
    navigate('/sessions');
  };

  const renderWidgetContent = () => {
    switch (widget.type) {
      case WidgetType.TOTAL_SESSIONS:
        return (
          <CardContent className="flex flex-col justify-end h-full p-0">
            <div className="flex flex-col justify-end">
              <p className="text-3xl font-mono font-light flex items-end text-text-default">
                {widget.data?.totalSessions ?? 0}
              </p>
              <span className="text-xs text-text-muted">Total sessions</span>
            </div>
          </CardContent>
        );

      case WidgetType.TOTAL_TOKENS:
        return (
          <CardContent className="flex flex-col justify-end h-full p-0">
            <div className="flex flex-col justify-end">
              <p className="text-3xl font-mono font-light flex items-end text-text-default">
                {widget.data?.totalTokens && widget.data.totalTokens > 0
                  ? `${(widget.data.totalTokens / 1000000).toFixed(2)}M`
                  : '0.00M'}
              </p>
              <span className="text-xs text-text-muted">Total tokens</span>
            </div>
          </CardContent>
        );

      case WidgetType.RECENT_CHATS:
        return (
          <CardContent className="p-0 h-full flex flex-col">
            <div className="flex justify-between items-center mb-2 flex-shrink-0">
              <CardDescription className="mb-0">
                <span className="text-sm text-text-default">Recent chats</span>
              </CardDescription>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleSeeAllClick}
                className="text-xs text-text-muted flex items-center gap-1 !px-0 hover:bg-transparent hover:underline hover:text-text-default pointer-events-auto"
              >
                See all
              </Button>
            </div>
            <div className="space-y-1 flex-1 overflow-y-auto min-h-0">
              {widget.data?.recentSessions?.slice(0, 5).map((session: any) => (
                <div
                  key={session.id}
                  onClick={() => handleSessionClick(session.id)}
                  className="flex items-center justify-between text-xs py-1 px-1 rounded-md hover:bg-background-muted/50 cursor-pointer transition-colors pointer-events-auto"
                >
                  <div className="flex items-center space-x-2 min-w-0">
                    <ChatSmart className="h-3 w-3 text-text-muted flex-shrink-0" />
                    <span className="truncate text-text-default">
                      {session.metadata?.description || session.id}
                    </span>
                  </div>
                  <span className="text-text-muted font-mono font-light text-xs flex-shrink-0 ml-2">
                    {new Date(session.modified).toLocaleDateString('en-US', { 
                      month: '2-digit', 
                      day: '2-digit' 
                    })}
                  </span>
                </div>
              )) || (
                <div className="text-text-muted text-xs py-2 text-center">No recent chats</div>
              )}
            </div>
          </CardContent>
        );

      case WidgetType.GREETING:
        return (
          <CardContent className="flex flex-col justify-center h-full p-0">
            <div className="text-center">
              <h2 className="text-lg font-medium text-text-default mb-2">
                {widget.data?.greeting || 'Welcome back!'}
              </h2>
              <p className="text-sm text-text-muted">
                {widget.data?.subtitle || 'Ready to start a new conversation?'}
              </p>
            </div>
          </CardContent>
        );

      default:
        return (
          <CardContent className="flex items-center justify-center h-full p-0">
            <div className="text-center">
              <p className="text-sm text-text-muted">Unknown widget type</p>
            </div>
          </CardContent>
        );
    }
  };

  return (
    <div
      className={`absolute cursor-grab select-none ${
        isDragging ? 'z-50 cursor-grabbing' : 'z-10'
      }`}
      style={{
        left: widget.position.x,
        top: widget.position.y,
        width: widget.size.width,
        height: widget.size.height,
        transform: isDragging ? 'scale(1.05)' : 'scale(1)',
        transition: isDragging ? 'none' : 'transform 0.2s ease-out',
        willChange: isDragging ? 'transform' : 'auto',
      }}
      onMouseDown={onMouseDown}
    >
      {/* Glass effect background */}
      <div
        className={`absolute inset-0 rounded-xl border transition-all duration-200 ${
          isDragging 
            ? 'border-border/60 shadow-3xl' 
            : 'border-border/30 shadow-2xl hover:border-border/50 hover:shadow-3xl'
        }`}
        style={{
          background: `
            linear-gradient(135deg, 
              rgba(255, 255, 255, ${isDragging ? '0.15' : '0.1'}) 0%, 
              rgba(255, 255, 255, ${isDragging ? '0.08' : '0.05'}) 100%
            )
          `,
          backdropFilter: `blur(20px) saturate(${isDragging ? '200%' : '180%'})`,
          WebkitBackdropFilter: `blur(20px) saturate(${isDragging ? '200%' : '180%'})`,
        }}
      />
      
      {/* Saved indicator */}
      {showSavedIndicator && !isDragging && (
        <div className="absolute top-2 right-2 z-30 pointer-events-none">
          <div className="w-1 h-1 bg-green-500 rounded-full animate-in fade-in duration-200"></div>
        </div>
      )}
      
      {/* Content */}
      <div className="relative z-10 p-4 h-full pointer-events-none">
        {renderWidgetContent()}
      </div>
      
      {/* Resize handle */}
      <div className="absolute bottom-0 right-0 w-3 h-3 cursor-se-resize opacity-0 hover:opacity-100 transition-opacity z-20 pointer-events-auto">
        <div className="w-full h-full bg-text-muted/30 rounded-tl-sm" />
      </div>
    </div>
  );
}
