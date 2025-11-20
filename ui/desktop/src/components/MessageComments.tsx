import React, { useState, useCallback } from 'react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import { cn } from '../utils';

interface MessageCommentsProps {
  messageId: string;
  comments: MessageComment[];
  activeSelection: TextSelection | null;
  activePosition?: { x: number; y: number } | null;
  isCreatingComment: boolean;
  onCreateComment: (messageId: string, selection: TextSelection, content: string) => void;
  onUpdateComment: (commentId: string, content: string) => void;
  onDeleteComment: (commentId: string) => void;
  onReplyToComment: (parentId: string, content: string) => void;
  onResolveComment: (commentId: string, resolved: boolean) => void;
  onCancelComment: () => void;
  className?: string;
}

export default function MessageComments({
  messageId,
  comments,
  activeSelection,
  activePosition,
  isCreatingComment,
  onCreateComment,
  onUpdateComment,
  onDeleteComment,
  onReplyToComment,
  onResolveComment,
  onCancelComment,
  className,
}: MessageCommentsProps) {
  // Filter out reply comments (they're shown nested within their parents)
  const topLevelComments = comments.filter(comment => !comment.parentId);

  // Sort comments by position (top to bottom)
  const sortedComments = topLevelComments.sort((a, b) => a.position - b.position);

  const handleCreateComment = useCallback((content: string) => {
    if (activeSelection) {
      onCreateComment(messageId, activeSelection, content);
    }
  }, [messageId, activeSelection, onCreateComment]);

  // If no comments and not creating, don't render anything
  if (sortedComments.length === 0 && !isCreatingComment) {
    return null;
  }

  return (
    <div className={cn('relative', className)} data-comment-ui>
      {/* Existing comments - positioned absolutely based on their position */}
      {sortedComments.map((comment) => (
        <div
          key={comment.id}
          className="absolute"
          style={{
            top: `${comment.position * 0.1}px`, // Simple positioning - would need refinement
          }}
          data-comment-ui
        >
          <CommentCard
            comment={comment}
            onUpdate={onUpdateComment}
            onDelete={onDeleteComment}
            onReply={onReplyToComment}
            onResolve={onResolveComment}
          />
        </div>
      ))}

      {/* New comment input - positioned based on selection */}
      {isCreatingComment && activeSelection && (
        <div
          className="absolute"
          style={{
            top: activePosition ? `${activePosition.y}px` : `${activeSelection.startOffset * 0.1}px`,
          }}
          data-comment-ui
        >
          <CommentInput
            selection={activeSelection}
            onSubmit={handleCreateComment}
            onCancel={onCancelComment}
          />
        </div>
      )}
    </div>
  );
}
