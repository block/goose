import React, { useState } from 'react';
import { X, Clock, Send, GripVertical, Zap, Sparkles, ChevronDown, ChevronUp } from 'lucide-react';
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
  onEditMessage?: (messageId: string, newContent: string) => void;
  onTriggerQueueProcessing?: () => void;
  editingMessageIdRef?: React.MutableRefObject<string | null>;
  onReorderMessages?: (reorderedMessages: QueuedMessage[]) => void;
  className?: string;
  isPaused?: boolean;
}

export const MessageQueue: React.FC<MessageQueueProps> = ({
  queuedMessages,
  onRemoveMessage,
  onClearQueue,
  onStopAndSend,
  onEditMessage,
  onTriggerQueueProcessing,
  editingMessageIdRef,
  onReorderMessages,
  className = '',
  isPaused = false,
}) => {
  const [isExpanded, setIsExpanded] = useState(true);
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);
  const [hoveredMessage, setHoveredMessage] = useState<string | null>(null);
  const [editingMessage, setEditingMessage] = useState<string | null>(null);
  const [editContent, setEditContent] = useState<string>('');

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

    const draggedIndex = queuedMessages.findIndex((msg) => msg.id === draggedItem);
    const targetIndex = queuedMessages.findIndex((msg) => msg.id === targetMessageId);

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
          className="flex items-center justify-between px-4 py-2.5 bg-background border-b border-border/20 cursor-pointer hover:bg-muted/30 transition-all duration-200"
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
                  : nextMessage.content}
              </p>
            </div>

            {/* Queue count */}
            {remainingCount > 0 && (
              <div className="flex items-center gap-1 text-xs text-muted-foreground bg-bgSubtle border border-borderSubtle px-2 py-1 rounded-full font-medium">
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
                className="h-7 px-2 text-xs text-info hover:text-info/80 hover:bg-info/10"
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

  // Expanded View
  return (
    <div className={`relative ${className}`}>
      {/* Expanded Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-background border-b border-border/30">
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
                <Sparkles className="w-4 h-4 text-info" />
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
            <span>
              Queue paused by interruption. Use "Send Now" or add a new message to resume.
            </span>
          </div>
        </div>
      )}

      {/* Message Bubbles */}
      <div className="p-4 space-y-3 bg-background max-h-80 overflow-y-auto">
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
            <div
              className={`relative flex items-center gap-3 rounded-xl px-4 py-3 border transition-all duration-300 ease-out ${
                draggedItem === message.id
                  ? 'bg-info/20 border-info opacity-60 scale-105 shadow-lg rotate-2'
                  : dragOverItem === message.id
                    ? 'bg-green-100/80 border-green-400 shadow-lg dark:bg-green-950/50 dark:border-green-600 scale-102'
                    : hoveredMessage === message.id
                      ? 'bg-muted/90 border-border shadow-md scale-101'
                      : 'bg-muted/60 hover:bg-muted/80 border-border/60 hover:border-border dark:border-border/60 dark:hover:border-border'
              } backdrop-blur-sm`}
            >
              {/* Priority indicator */}
              <div className="flex items-center gap-2">
                <div
                  className={`flex items-center justify-center w-6 h-6 rounded-full text-xs font-semibold transition-colors ${
                    index === 0
                      ? 'bg-blue-500 text-white shadow-md'
                      : 'bg-muted text-muted-foreground'
                  }`}
                >
                  {index + 1}
                </div>

                {/* Drag handle */}
                {onReorderMessages && (
                  <div
                    className={`opacity-0 group-hover:opacity-60 hover:opacity-100 transition-all duration-200 cursor-grab active:cursor-grabbing ${
                      hoveredMessage === message.id ? 'opacity-40' : ''
                    }`}
                  >
                    <GripVertical className="w-4 h-4 text-muted-foreground hover:text-foreground" />
                  </div>
                )}
              </div>

              {/* Message content */}
              <div className="flex-1 min-w-0">
                {editingMessage === message.id ? (
                  <div className="space-y-2">
                    <textarea
                      value={editContent}
                      onChange={(e) => setEditContent(e.target.value)}
                      className="w-full text-sm bg-background border border-border rounded-md px-2 py-1 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                      rows={Math.min(Math.ceil(editContent.length / 60), 4)}
                      autoFocus
                    />
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          if (onEditMessage) {
                            onEditMessage(message.id, editContent);
                          }
                          setEditingMessage(null);
                          if (editingMessageIdRef) editingMessageIdRef.current = null;
                          // Trigger queue processing if system is ready
                          if (onTriggerQueueProcessing) {
                            setTimeout(onTriggerQueueProcessing, 100);
                          }
                          setEditContent('');
                        }}
                        className="h-6 px-2 text-xs"
                      >
                        Save
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setEditingMessage(null);
                          if (editingMessageIdRef) editingMessageIdRef.current = null;
                          // Trigger queue processing if system is ready
                          if (onTriggerQueueProcessing) {
                            setTimeout(onTriggerQueueProcessing, 100);
                          }
                          setEditContent('');
                        }}
                        className="h-6 px-2 text-xs"
                      >
                        Cancel
                      </Button>
                    </div>
                  </div>
                ) : (
                  <p
                    className="text-sm text-foreground leading-relaxed cursor-pointer hover:bg-muted/30 rounded px-1 py-0.5 transition-colors"
                    title={`${message.content} (Click to edit)`}
                    onClick={() => {
                      setEditingMessage(message.id);
                      if (editingMessageIdRef) editingMessageIdRef.current = message.id;
                      setEditContent(message.content);
                    }}
                  >
                    {message.content.length > 80
                      ? `${message.content.substring(0, 80)}...`
                      : message.content}
                  </p>
                )}
              </div>

              {/* Right side actions */}
              <div className="flex items-center gap-2 flex-shrink-0">
                <span className="text-xs text-muted-foreground font-mono">
                  {formatTimestamp(message.timestamp)}
                </span>

                {/* Send Now button - inline */}
                {onStopAndSend && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onStopAndSend(message.id)}
                    disabled={editingMessage === message.id}
                    className={`h-7 w-7 p-0 rounded-full transition-all duration-200 ${
                      editingMessage === message.id
                        ? 'opacity-30 cursor-not-allowed'
                        : 'hover:bg-muted/50'
                    }`}
                    title={
                      editingMessage === message.id
                        ? 'Cannot send while editing'
                        : 'Stop current processing and send this message now'
                    }
                  >
                    <Send className="w-3 h-3" />
                  </Button>
                )}

                {/* Remove button */}
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onRemoveMessage(message.id)}
                  className="opacity-60 hover:opacity-100 transition-opacity h-6 w-6 p-0 hover:bg-destructive/20 hover:text-destructive rounded-full"
                  title="Remove this message from queue"
                >
                  <X className="w-3 h-3" />
                </Button>
              </div>
            </div>

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
