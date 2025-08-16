import React, { useState } from 'react';
import { X, Clock, Send, GripVertical, Zap, Sparkles, ChevronDown, ChevronUp, MoreHorizontal } from 'lucide-react';
import { Button } from './ui/button';

interface QueuedMessage {
  id: string;
  content: string;
  timestamp: number;
}

interface MessageQueueProps {
  queuedMessages: QueuedMessage[];
  onRemoveMessage: (id: string) => void;
  onClearQueue: () => void;
  onStopAndSend?: (messageId: string) => void;
  onReorderMessages?: (reorderedMessages: QueuedMessage[]) => void;
  className?: string;
  isPaused?: boolean;
}

export const MessageQueue: React.FC<MessageQueueProps> = ({
  queuedMessages,
  onRemoveMessage,
  onClearQueue,
  onStopAndSend,
  onReorderMessages,
  className = '',
  isPaused = false,
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [hoveredMessage, setHoveredMessage] = useState<string | null>(null);

  if (queuedMessages.length === 0) {
    return null;
  }

  const handleDragStart = (e: React.DragEvent, messageId: string) => {
    setDraggedItem(messageId);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/html', messageId);
  };

  const handleDragOver = (e: React.DragEvent, messageId: string) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDragOverItem(messageId);
  };

  const handleDragLeave = () => {
    setDragOverItem(null);
  };

  const handleDrop = (e: React.DragEvent, targetMessageId: string) => {
    e.preventDefault();
    
    if (!draggedItem || !onReorderMessages) return;
    
    const draggedIndex = queuedMessages.findIndex(msg => msg.id === draggedItem);
    const targetIndex = queuedMessages.findIndex(msg => msg.id === targetMessageId);
    
    if (draggedIndex === -1 || targetIndex === -1 || draggedIndex === targetIndex) {
      setDraggedItem(null);
      setDragOverItem(null);
      return;
    }

    const newMessages = [...queuedMessages];
    const [removed] = newMessages.splice(draggedIndex, 1);
    newMessages.splice(targetIndex, 0, removed);
    
    onReorderMessages(newMessages);
    setDraggedItem(null);
    setDragOverItem(null);
  };

  const handleDragEnd = () => {
    setDraggedItem(null);
    setDragOverItem(null);
  };

  const formatTimestamp = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp;
    if (diff < 60000) return 'now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
    return `${Math.floor(diff / 3600000)}h`;
  };

  const nextMessage = queuedMessages[0];
  const remainingCount = queuedMessages.length - 1;

  // Compact View
  if (!isExpanded) {
    return (
      <div className={`relative ${className}`}>
        {/* Compact Header */}
        <div 
          className="flex items-center justify-between px-4 py-2.5 bg-gradient-to-r from-slate-50/60 to-blue-50/60 dark:from-slate-900/60 dark:to-blue-900/20 border-b border-border/20 backdrop-blur-sm cursor-pointer hover:bg-gradient-to-r hover:from-slate-50/80 hover:to-blue-50/80 dark:hover:from-slate-900/80 dark:hover:to-blue-900/30 transition-all duration-200"
          onClick={() => setIsExpanded(true)}
        >
          <div className="flex items-center gap-3 flex-1 min-w-0">
            <div className="flex items-center gap-2">
              {isPaused ? (
                <div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
              ) : (
                <div className="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
              )}
              <span className="text-sm font-medium text-foreground">
                {isPaused ? 'Paused' : 'Next'}
              </span>
            </div>
            
            {/* Next message preview */}
            <div className="flex-1 min-w-0">
              <p className="text-sm text-muted-foreground truncate" title={nextMessage.content}>
                {nextMessage.content.length > 40 
                  ? `${nextMessage.content.substring(0, 40)}...` 
                  : nextMessage.content
                }
              </p>
            </div>
            
            {/* Queue count */}
            {remainingCount > 0 && (
              <div className="flex items-center gap-1 text-xs text-muted-foreground bg-white/50 dark:bg-black/20 px-2 py-1 rounded-full">
                <span>+{remainingCount}</span>
              </div>
            )}
          </div>
          
          <div className="flex items-center gap-2">
            {/* Quick Send Now button */}
            {onStopAndSend && (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onStopAndSend(nextMessage.id);
                }}
                className="h-7 px-2 text-xs text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:text-blue-400 dark:hover:text-blue-300 dark:hover:bg-blue-950/30"
                title="Send this message now"
              >
                <Send className="w-3 h-3" />
              </Button>
            )}
            
            {/* Expand button */}
            <Button
              variant="ghost"
              size="sm"
              className="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
              title="Expand queue"
            >
              <ChevronDown className="w-4 h-4" />
            </Button>
          </div>
        </div>

        {/* Paused state indicator */}
        {isPaused && (
          <div className="px-4 py-1.5 bg-amber-50/60 dark:bg-amber-900/20 border-b border-amber-200/30 dark:border-amber-800/30">
            <div className="flex items-center gap-2 text-xs text-amber-700 dark:text-amber-300">
              <Zap className="w-3 h-3" />
              <span>Queue paused - click "Send" or add new message to resume</span>
            </div>
          </div>
        )}
      </div>
    );
  }

  // Expanded View (your previous full implementation)
  return (
    <div className={`relative ${className}`}>
      {/* Expanded Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-gradient-to-r from-slate-50/80 to-blue-50/80 dark:from-slate-900/80 dark:to-blue-900/20 border-b border-border/30 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          <div className="relative">
            {isPaused ? (
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
                <Clock className="w-4 h-4 text-amber-600 dark:text-amber-400" />
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
                <Sparkles className="w-4 h-4 text-blue-600 dark:text-blue-400" />
              </div>
            )}
          </div>
          <div className="flex flex-col">
            <span className="text-sm font-medium text-foreground">
              {isPaused ? 'Queue Paused' : 'Message Queue'}
            </span>
            <span className="text-xs text-muted-foreground">
              {queuedMessages.length} message{queuedMessages.length !== 1 ? 's' : ''} 
              {isPaused ? ' waiting' : ' queued'}
            </span>
          </div>
        </div>
        
        <div className="flex items-center gap-2">
          {queuedMessages.length > 1 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onClearQueue}
              className="text-xs h-7 px-3 text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
            >
              Clear All
            </Button>
          )}
          
          {/* Collapse button */}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setIsExpanded(false)}
            className="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
            title="Collapse queue"
          >
            <ChevronUp className="w-4 h-4" />
          </Button>
        </div>
      </div>

      {/* Status Banner for Paused State */}
      {isPaused && (
        <div className="px-4 py-2 bg-amber-50/80 dark:bg-amber-900/20 border-b border-amber-200/50 dark:border-amber-800/50">
          <div className="flex items-center gap-2 text-sm text-amber-800 dark:text-amber-200">
            <Zap className="w-4 h-4" />
            <span>Queue paused by interruption. Use "Send Now" or add a new message to resume.</span>
          </div>
        </div>
      )}
      
      {/* Message Bubbles */}
      <div className="p-4 space-y-3 bg-gradient-to-b from-transparent to-slate-50/30 dark:to-slate-900/30 max-h-80 overflow-y-auto">
        {queuedMessages.map((message, index) => (
          <div
            key={message.id}
            className="group relative"
            draggable={onReorderMessages ? true : false}
            onDragStart={(e) => handleDragStart(e, message.id)}
            onDragOver={(e) => handleDragOver(e, message.id)}
            onDragLeave={handleDragLeave}
            onDrop={(e) => handleDrop(e, message.id)}
            onDragEnd={handleDragEnd}
            onMouseEnter={() => setHoveredMessage(message.id)}
            onMouseLeave={() => setHoveredMessage(null)}
          >
            {/* Main message bubble */}
            <div className={`relative flex items-center gap-3 rounded-xl px-4 py-3 border transition-all duration-300 ease-out ${
              draggedItem === message.id 
                ? 'bg-blue-100/80 border-blue-300 opacity-60 scale-105 shadow-lg dark:bg-blue-950/50 dark:border-blue-700 rotate-2' 
                : dragOverItem === message.id
                ? 'bg-green-100/80 border-green-400 shadow-lg dark:bg-green-950/50 dark:border-green-600 scale-102'
                : hoveredMessage === message.id
                ? 'bg-white/90 border-slate-300 shadow-md dark:bg-slate-800/90 dark:border-slate-600 scale-101'
                : 'bg-white/60 hover:bg-white/80 border-slate-200/60 hover:border-slate-300 dark:bg-slate-800/60 dark:hover:bg-slate-800/80 dark:border-slate-700/60 dark:hover:border-slate-600'
            } backdrop-blur-sm`}>
              
              {/* Priority indicator */}
              <div className="flex items-center gap-2">
                <div className={`flex items-center justify-center w-6 h-6 rounded-full text-xs font-semibold transition-colors ${
                  index === 0 
                    ? 'bg-blue-500 text-white shadow-md' 
                    : 'bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300'
                }`}>
                  {index + 1}
                </div>
                
                {/* Drag handle */}
                {onReorderMessages && (
                  <div className={`opacity-0 group-hover:opacity-60 hover:opacity-100 transition-all duration-200 cursor-grab active:cursor-grabbing ${
                    hoveredMessage === message.id ? 'opacity-40' : ''
                  }`}>
                    <GripVertical className="w-4 h-4 text-muted-foreground hover:text-foreground" />
                  </div>
                )}
              </div>
              
              {/* Message content */}
              <div className="flex-1 min-w-0">
                <div className="flex items-start justify-between gap-3">
                  <p className="text-sm text-foreground leading-relaxed" title={message.content}>
                    {message.content.length > 80 
                      ? `${message.content.substring(0, 80)}...` 
                      : message.content
                    }
                  </p>
                  <div className="flex items-center gap-2 flex-shrink-0">
                    <span className="text-xs text-muted-foreground font-mono">
                      {formatTimestamp(message.timestamp)}
                    </span>
                    
                    {/* Remove button */}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => onRemoveMessage(message.id)}
                      className="opacity-0 group-hover:opacity-100 transition-opacity h-6 w-6 p-0 hover:bg-destructive/20 hover:text-destructive rounded-full"
                      title="Remove this message from queue"
                    >
                      <X className="w-3 h-3" />
                    </Button>
                  </div>
                </div>
              </div>
            </div>
            
            {/* Send Now pill - appears below on hover with smooth animation */}
            {onStopAndSend && (
              <div className={`absolute top-full left-1/2 transform -translate-x-1/2 mt-2 transition-all duration-300 ease-out z-20 ${
                hoveredMessage === message.id 
                  ? 'opacity-100 translate-y-0 pointer-events-auto' 
                  : 'opacity-0 translate-y-[-8px] pointer-events-none'
              }`}>
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => onStopAndSend(message.id)}
                  className="h-8 px-4 text-xs bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white border-0 rounded-full shadow-lg hover:shadow-xl transition-all duration-200 whitespace-nowrap font-medium"
                  title="Stop current processing and send this message now"
                >
                  <Send className="w-3 h-3 mr-2" />
                  Send Now
                </Button>
              </div>
            )}
            
            {/* Drop indicator with enhanced visuals */}
            {dragOverItem === message.id && draggedItem !== message.id && (
              <div className="absolute inset-0 border-2 border-green-400 rounded-xl pointer-events-none animate-pulse bg-green-100/20 dark:bg-green-900/20" />
            )}
            
            {/* Next up indicator */}
            {index === 0 && !isPaused && (
              <div className="absolute -top-2 -right-2 bg-blue-500 text-white text-xs px-2 py-1 rounded-full font-medium shadow-md">
                Next
              </div>
            )}
          </div>
        ))}
      </div>
      
      {/* Drag instructions */}
      {onReorderMessages && queuedMessages.length > 1 && (
        <div className="px-4 pb-3 text-xs text-muted-foreground flex items-center gap-2 opacity-60 hover:opacity-100 transition-opacity">
          <GripVertical className="w-3 h-3" />
          <span>Drag messages to reorder priority</span>
        </div>
      )}
    </div>
  );
};

export default MessageQueue;
