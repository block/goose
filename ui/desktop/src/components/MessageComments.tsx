import React, { useState, useCallback, useEffect } from 'react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import CommentBadge from './CommentBadge';
import CommentPanel from './CommentPanel';
import { useCommentPanelOptional } from '../contexts/CommentPanelContext';
import { cn } from '../utils';

export type CommentDisplayMode = 'full' | 'condensed';

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
  displayMode?: CommentDisplayMode;
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
  displayMode = 'full',
  className,
}: MessageCommentsProps) {
  const [isLocalPanelOpen, setIsLocalPanelOpen] = useState(false);
  const panelContext = useCommentPanelOptional();
  
  // Use context if available, otherwise use local state
  const isPanelOpen = panelContext ? panelContext.isPanelOpen : isLocalPanelOpen;
  const openPanel = panelContext ? panelContext.openPanel : () => setIsLocalPanelOpen(true);
  const closePanel = panelContext ? panelContext.closePanel : () => setIsLocalPanelOpen(false);
  
  // Filter out reply comments (they're shown nested within their parents)
  const topLevelComments = comments.filter(comment => !comment.parentId);

  // Sort comments by position (top to bottom)
  const sortedComments = topLevelComments.sort((a, b) => a.position - b.position);

  const handleCreateComment = useCallback((content: string) => {
    if (activeSelection) {
      onCreateComment(messageId, activeSelection, content);
    }
  }, [messageId, activeSelection, onCreateComment]);

  // Calculate badge position - use first comment position or active selection position
  const badgePosition = activePosition || (sortedComments[0] ? { x: 0, y: sortedComments[0].position * 0.1 } : { x: 0, y: 0 });

  // If no comments and not creating, don't render anything
  if (sortedComments.length === 0 && !isCreatingComment) {
    return null;
  }

  // Condensed mode: show badge + slide-in panel
  if (displayMode === 'condensed') {
    return (
      <>
        <CommentBadge
          comments={comments}
          position={badgePosition}
          onClick={openPanel}
          className={className}
        />
        <CommentPanel
          messageId={messageId}
          comments={comments}
          selectedText={activeSelection?.selectedText}
          isOpen={isPanelOpen}
          onClose={closePanel}
          activeSelection={activeSelection}
          isCreatingComment={isCreatingComment}
          onCreateComment={onCreateComment}
          onUpdateComment={onUpdateComment}
          onDeleteComment={onDeleteComment}
          onReplyToComment={onReplyToComment}
          onResolveComment={onResolveComment}
          onCancelComment={onCancelComment}
        />
      </>
    );
  }

  // Full mode: show inline comments
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
