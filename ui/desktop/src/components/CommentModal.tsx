import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import { cn } from '../utils';

interface CommentModalProps {
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
 * Modal/popover for displaying comments in condensed mode
 */
export default function CommentModal({
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
}: CommentModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);
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

  // Handle click outside
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as Element;
      
      // Don't close if clicking on comment UI elements
      if (target.closest('[data-comment-ui]')) {
        return;
      }
      
      if (modalRef.current && !modalRef.current.contains(target)) {
        onClose();
      }
    };

    // Delay adding listener to avoid immediate close
    const timeout = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
    }, 100);

    return () => {
      clearTimeout(timeout);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen, onClose]);

  // Focus management
  useEffect(() => {
    if (isOpen) {
      previousFocusRef.current = document.activeElement as HTMLElement;
      modalRef.current?.focus();
    } else {
      previousFocusRef.current?.focus();
    }
  }, [isOpen]);

  const handleCreateComment = (content: string) => {
    if (activeSelection && onCreateComment) {
      onCreateComment(messageId, activeSelection, content);
    }
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div 
        className="fixed inset-0 bg-black/20 dark:bg-black/40 z-40 backdrop-blur-sm"
        onClick={onClose}
      />
      
      {/* Modal */}
      <div
        ref={modalRef}
        data-comment-ui
        tabIndex={-1}
        className={cn(
          'fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2',
          'w-[90vw] max-w-2xl max-h-[80vh]',
          'bg-white dark:bg-gray-900 rounded-lg shadow-2xl',
          'border border-gray-200 dark:border-gray-700',
          'flex flex-col',
          'z-50',
          className
        )}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-2">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Comments
            </h2>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              ({comments.length})
            </span>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            aria-label="Close"
          >
            <X className="w-5 h-5 text-gray-500 dark:text-gray-400" />
          </button>
        </div>

        {/* Selected text indicator */}
        {selectedText && (
          <div className="px-4 py-2 bg-yellow-50 dark:bg-yellow-900/20 border-b border-yellow-200 dark:border-yellow-800">
            <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
              Selected text:
            </p>
            <p className="text-sm text-gray-900 dark:text-gray-100 italic">
              "{selectedText}"
            </p>
          </div>
        )}

        {/* Comment list */}
        <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
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
              <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                <p className="text-sm">No comments yet</p>
                <p className="text-xs mt-1">Select text to add a comment</p>
              </div>
            )
          )}
        </div>

        {/* Footer */}
        <div className="px-4 py-2 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Press <kbd className="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-[10px]">Esc</kbd> to close
          </p>
        </div>
      </div>
    </>
  );
}
