import React, { useState } from 'react';
import { X, Clock, Send, GripVertical } from 'lucide-react';
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
}

export const MessageQueue: React.FC<MessageQueueProps> = ({
  queuedMessages,
  onRemoveMessage,
  onClearQueue,
  onStopAndSend,
  onReorderMessages,
  className = '',
}) => {
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);

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

  return (
    <div className={`flex flex-col gap-2 p-3 ${className}`}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Clock className="w-4 h-4" />
          <span>Queued Messages ({queuedMessages.length})</span>
        </div>
        {queuedMessages.length > 1 && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onClearQueue}
            className="text-xs h-6 px-2 text-muted-foreground hover:text-foreground"
          >
            Clear All
          </Button>
        )}
      </div>
      
      <div className="flex flex-wrap gap-2">
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
          >
            {/* Main message bubble */}
            <div className={`flex items-center gap-2 rounded-lg px-3 py-2 border transition-all duration-200 ${
              draggedItem === message.id 
                ? 'bg-blue-100 border-blue-300 opacity-50 scale-105 dark:bg-blue-950/50 dark:border-blue-700' 
                : dragOverItem === message.id
                ? 'bg-green-100 border-green-300 dark:bg-green-950/50 dark:border-green-700'
                : 'bg-secondary/50 hover:bg-secondary/70 border-border/50'
            }`}>
              {/* Drag handle */}
              {onReorderMessages && (
                <div className="opacity-0 group-hover:opacity-60 hover:opacity-100 transition-opacity cursor-grab active:cursor-grabbing">
                  <GripVertical className="w-3 h-3 text-muted-foreground" />
                </div>
              )}
              
              <div className="flex items-center gap-2 min-w-0">
                <span className="text-xs font-mono text-muted-foreground">
                  {index + 1}
                </span>
                <span className="text-sm truncate max-w-[200px]" title={message.content}>
                  {message.content.length > 50 
                    ? `${message.content.substring(0, 50)}...` 
                    : message.content
                  }
                </span>
              </div>
              
              {/* Remove button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onRemoveMessage(message.id)}
                className="opacity-0 group-hover:opacity-100 transition-opacity h-5 w-5 p-0 hover:bg-destructive/20 hover:text-destructive"
                title="Remove this message from queue"
              >
                <X className="w-3 h-3" />
              </Button>
            </div>
            
            {/* Send Now pill - appears below on hover */}
            {onStopAndSend && (
              <div className="absolute top-full left-1/2 transform -translate-x-1/2 mt-1 opacity-0 group-hover:opacity-100 transition-all duration-200 ease-in-out translate-y-[-4px] group-hover:translate-y-0 pointer-events-none group-hover:pointer-events-auto z-10">
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => onStopAndSend(message.id)}
                  className="h-7 px-3 text-xs bg-blue-600 hover:bg-blue-700 text-white border-0 rounded-full shadow-lg whitespace-nowrap"
                  title="Stop current processing and send this message now"
                >
                  <Send className="w-3 h-3 mr-1.5" />
                  Send Now
                </Button>
              </div>
            )}
            
            {/* Drop indicator */}
            {dragOverItem === message.id && draggedItem !== message.id && (
              <div className="absolute inset-0 border-2 border-green-400 rounded-lg pointer-events-none animate-pulse" />
            )}
          </div>
        ))}
      </div>
      
      {/* Drag instructions */}
      {onReorderMessages && queuedMessages.length > 1 && (
        <div className="text-xs text-muted-foreground mt-1 opacity-0 group-hover:opacity-100 transition-opacity">
          💡 Drag messages to reorder them
        </div>
      )}
    </div>
  );
};

export default MessageQueue;
