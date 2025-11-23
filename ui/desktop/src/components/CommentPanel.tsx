import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import { cn } from '../utils';

interface CommentPanelProps {
  messageId: string;
  comments: MessageComment[];
  selectedText?: string;
  isOpen: boolean;
  onClose: () => void;
  
  // Comment actions
  activeSelection?: TextSelection | null;
  isCreatingComment?: boolean;
  onCreateComment?: (messageId: string, selection: TextSelection, content: string) => void;
  onUpdateComment?: (commentId: string, content: string) => void;
  onDeleteComment?: (commentId: string) => void;
  onReplyToComment?: (parentId: string, content: string) => void;
  onResolveComment?: (commentId: string, resolved: boolean) => void;
  onCancelComment?: () => void;
  
  className?: string;
}

/**
 * Slide-in panel for displaying comments in condensed mode
 * Acts like a mini-sidecar that pushes the chat content to the left
 */
export default function CommentPanel({
  messageId,
  comments,
  selectedText,
  isOpen,
  onClose,
  activeSelection,
  isCreatingComment = false,
  onCreateComment,
  onUpdateComment,
  onDeleteComment,
  onReplyToComment,
  onResolveComment,
  onCancelComment,
  className,
}: CommentPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // Filter to top-level comments
  const topLevelComments = comments.filter(c => !c.parentId);
  const sortedComments = topLevelComments.sort((a, b) => b.timestamp - a.timestamp);

  // Handle escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isOpen, onClose]);

  // Focus management
  useEffect(() => {
    if (isOpen) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      panelRef.current?.focus();
    } else {
      previousFocusRef.current?.focus();
    }
  }, [isOpen]);

  const handleCreateComment = (content: string) => {
    if (activeSelection && onCreateComment) {
      onCreateComment(messageId, activeSelection, content);
    }
  };

  return (
    <div
      ref={panelRef}
      data-comment-ui
      tabIndex={-1}
      className={cn(
        'fixed top-0 right-0 h-full w-96 z-40',
        'bg-background-default border-l border-border-subtle',
        'flex flex-col shadow-2xl',
        'transform transition-transform duration-300 ease-in-out',
        isOpen ? 'translate-x-0' : 'translate-x-full',
        className
      )}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border-subtle bg-background-medium">
        <div className="flex items-center gap-2">
          <h2 className="text-base font-semibold text-text-prominent">
            Comments
          </h2>
          <span className="text-sm text-text-muted">
            ({comments.length})
          </span>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 rounded hover:bg-background-subtle transition-colors"
          aria-label="Close comments"
        >
          <X className="w-4 h-4 text-text-muted" />
        </button>
      </div>

      {/* Selected text indicator */}
      {selectedText && (
        <div className="px-4 py-3 bg-background-subtle border-b border-border-subtle">
          <p className="text-xs text-text-muted mb-1">
            Selected text:
          </p>
          <p className="text-sm text-text-prominent italic line-clamp-2">
            "{selectedText}"
          </p>
        </div>
      )}

      {/* Comment list */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-3">
        {/* New comment input */}
        {isCreatingComment && activeSelection && (
          <div className="mb-4">
            <CommentInput
              selection={activeSelection}
              onSubmit={handleCreateComment}
              onCancel={onCancelComment || (() => {})}
              placeholder="Add your comment..."
            />
          </div>
        )}

        {/* Existing comments */}
        {sortedComments.length > 0 ? (
          sortedComments.map((comment) => (
            <CommentCard
              key={comment.id}
              comment={comment}
              onUpdate={onUpdateComment || (() => {})}
              onDelete={onDeleteComment || (() => {})}
              onReply={onReplyToComment || (() => {})}
              onResolve={onResolveComment || (() => {})}
            />
          ))
        ) : (
          !isCreatingComment && (
            <div className="text-center py-12 text-text-muted">
              <p className="text-sm">No comments yet</p>
              <p className="text-xs mt-1">Select text to add a comment</p>
            </div>
          )
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-2 border-t border-border-subtle bg-background-medium">
        <p className="text-xs text-text-muted">
          Press <kbd className="px-1.5 py-0.5 bg-background-subtle border border-border-subtle rounded text-[10px] font-mono">Esc</kbd> to close
        </p>
      </div>
    </div>
  );
}
