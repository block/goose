import React, { useState, useCallback } from 'react';
import { MessageComment, TextSelection } from '../types/comment';
import CommentCard from './CommentCard';
import CommentInput from './CommentInput';
import CommentBadge from './CommentBadge';
import CommentDrawer from './CommentDrawer';
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
  const [isDrawerExpanded, setIsDrawerExpanded] = useState(false);
  const [expandedBadgeType, setExpandedBadgeType] = useState<'existing' | 'new' | null>(null);
  
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

  // Condensed mode: show badge(s) - one for existing comments, one for new selection
  if (displayMode === 'condensed') {
    return (
      <>
        {/* Badge for existing comments - positioned at first comment */}
        {sortedComments.length > 0 && (
          <div
            className="absolute"
            style={{
              top: `${sortedComments[0].position * 0.1}px`,
            }}
          >
            <div className="relative">
              <CommentBadge
                comments={comments}
                position={{ x: 0, y: 0 }}
                onClick={() => {
                  setExpandedBadgeType('existing');
                  setIsDrawerExpanded(!isDrawerExpanded);
                }}
              />
              
              {/* Expandable drawer overlays below badge - shows all comments */}
              {isDrawerExpanded && expandedBadgeType === 'existing' && (
                <div className="absolute top-full right-0 mt-2 w-96 max-w-[90vw] z-50">
                  <CommentDrawer
                    messageId={messageId}
                    comments={comments}
                    selectedText={activeSelection?.selectedText}
                    isExpanded={isDrawerExpanded}
                    onToggle={() => {
                      setIsDrawerExpanded(!isDrawerExpanded);
                      setExpandedBadgeType(null);
                    }}
                    activeSelection={activeSelection}
                    isCreatingComment={isCreatingComment}
                    onCreateComment={onCreateComment}
                    onUpdateComment={onUpdateComment}
                    onDeleteComment={onDeleteComment}
                    onReplyToComment={onReplyToComment}
                    onResolveComment={onResolveComment}
                    onCancelComment={onCancelComment}
                  />
                </div>
              )}
            </div>
          </div>
        )}

        {/* Separate "Add comment" badge for new selection */}
        {isCreatingComment && activeSelection && (
          <div
            className="absolute animate-in fade-in zoom-in-95 duration-200"
            style={{
              top: activePosition ? `${activePosition.y}px` : '0px',
            }}
          >
            <div className="relative">
              <CommentBadge
                comments={[]}
                position={{ x: 0, y: 0 }}
                onClick={() => {
                  setExpandedBadgeType('new');
                  setIsDrawerExpanded(!isDrawerExpanded);
                }}
              />
              
              {/* Show only comment input for new comment - no existing comments */}
              {isDrawerExpanded && expandedBadgeType === 'new' && (
                <div className="absolute top-full right-0 mt-2 w-96 max-w-[90vw] z-50 animate-in fade-in slide-in-from-top-2 duration-200">
                  <div
                    data-comment-ui
                    className="border border-border-subtle rounded-lg overflow-hidden bg-background-default shadow-xl"
                  >
                    <div className="px-3 py-3">
                      <CommentInput
                        selection={activeSelection}
                        onSubmit={handleCreateComment}
                        onCancel={() => {
                          onCancelComment();
                          setIsDrawerExpanded(false);
                          setExpandedBadgeType(null);
                        }}
                        placeholder="Add your comment..."
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
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
