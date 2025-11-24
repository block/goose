import React from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import { cn } from '../utils';

interface CommentDrawerProps {
  messageId: string;
  comments: MessageComment[];
  selectedText?: string;
  isExpanded: boolean;
  onToggle: () => void;
  
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
 * Expandable drawer for displaying comments inline
 * Expands from the badge like an accordion
 */
export default function CommentDrawer({
  messageId,
  comments,
  selectedText,
  isExpanded,
  onToggle,
  activeSelection,
  isCreatingComment = false,
  onCreateComment,
  onUpdateComment,
  onDeleteComment,
  onReplyToComment,
  onResolveComment,
  onCancelComment,
  className,
}: CommentDrawerProps) {
  // Filter to top-level comments
  const topLevelComments = comments.filter(c => !c.parentId);
  const sortedComments = topLevelComments.sort((a, b) => b.timestamp - a.timestamp);

  const handleCreateComment = (content: string) => {
    if (activeSelection && onCreateComment) {
      onCreateComment(messageId, activeSelection, content);
    }
  };

  return (
    <div
      data-comment-ui
      className={cn(
        'border border-border-subtle rounded-lg overflow-hidden',
        'bg-background-default shadow-xl',
        'animate-in fade-in slide-in-from-top-2 duration-200',
        className
      )}
    >
      {/* Header - Always visible */}
      <button
        onClick={onToggle}
        className={cn(
          'w-full px-3 py-2 flex items-center justify-between',
          'hover:bg-background-medium transition-colors',
          'text-left'
        )}
      >
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-text-prominent">
            Comments
          </span>
          <span className="text-xs text-text-muted">
            ({comments.length})
          </span>
        </div>
        <div className="text-text-muted">
          {isExpanded ? (
            <ChevronUp className="w-4 h-4" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
        </div>
      </button>

      {/* Expandable content */}
      <div
        className={cn(
          'overflow-hidden transition-all duration-200 ease-in-out',
          isExpanded ? 'max-h-[600px] opacity-100' : 'max-h-0 opacity-0'
        )}
      >
        <div className="border-t border-border-subtle">
          {/* Selected text indicator */}
          {selectedText && (
            <div className="px-3 py-2 bg-background-subtle border-b border-border-subtle">
              <p className="text-xs text-text-muted mb-1">
                Selected text:
              </p>
              <p className="text-sm text-text-prominent italic line-clamp-2">
                "{selectedText}"
              </p>
            </div>
          )}

          {/* Comment list */}
          <div className="px-3 py-3 space-y-3 max-h-[500px] overflow-y-auto">
            {/* New comment input */}
            {isCreatingComment && activeSelection && (
              <div className="mb-3">
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
                <div className="text-center py-6 text-text-muted">
                  <p className="text-sm">No comments yet</p>
                  <p className="text-xs mt-1">Select text to add a comment</p>
                </div>
              )
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
