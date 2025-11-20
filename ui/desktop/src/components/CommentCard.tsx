import React, { useState } from 'react';
import { MessageComment } from '../types/comment';
import { formatMessageTimestamp } from '../utils/timeUtils';
import { cn } from '../utils';

interface CommentCardProps {
  comment: MessageComment;
  onUpdate?: (commentId: string, content: string) => void;
  onDelete?: (commentId: string) => void;
  onReply?: (parentId: string, content: string) => void;
  onResolve?: (commentId: string, resolved: boolean) => void;
  isActive?: boolean;
  className?: string;
}

export default function CommentCard({
  comment,
  onUpdate,
  onDelete,
  onReply,
  onResolve,
  isActive = false,
  className,
}: CommentCardProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState(comment.content);
  const [isReplying, setIsReplying] = useState(false);
  const [replyContent, setReplyContent] = useState('');

  const handleSaveEdit = () => {
    if (editContent.trim() && onUpdate) {
      onUpdate(comment.id, editContent.trim());
      setIsEditing(false);
    }
  };

  const handleCancelEdit = () => {
    setEditContent(comment.content);
    setIsEditing(false);
  };

  const handleSubmitReply = () => {
    if (replyContent.trim() && onReply) {
      onReply(comment.id, replyContent.trim());
      setReplyContent('');
      setIsReplying(false);
    }
  };

  const handleCancelReply = () => {
    setReplyContent('');
    setIsReplying(false);
  };

  return (
    <div
      data-comment-ui
      data-comment-id={comment.id}
      className={cn(
        'bg-white dark:bg-gray-800 border rounded-lg shadow-sm transition-colors',
        isActive 
          ? 'border-blue-300 dark:border-blue-600 ring-2 ring-blue-100 dark:ring-blue-900' 
          : 'border-gray-200 dark:border-gray-700',
        comment.resolved && 'opacity-60',
        className
      )}
    >
      {/* Comment header */}
      <div className="px-3 py-2 border-b border-gray-100 dark:border-gray-700">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
              {comment.author}
            </span>
            <span className="text-xs text-gray-500 dark:text-gray-400">
              {formatMessageTimestamp(comment.timestamp)}
            </span>
          </div>
          <div className="flex items-center gap-1">
            {onResolve && (
              <button
                onClick={() => onResolve(comment.id, !comment.resolved)}
                className={cn(
                  'text-xs px-2 py-1 rounded transition-colors',
                  comment.resolved
                    ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                    : 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
                )}
              >
                {comment.resolved ? '✓ Resolved' : 'Resolve'}
              </button>
            )}
          </div>
        </div>


      </div>

      {/* Comment content */}
      <div className="px-3 py-2">
        {isEditing ? (
          <div className="space-y-2">
            <textarea
              value={editContent}
              onChange={(e) => setEditContent(e.target.value)}
              className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded px-2 py-1 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 resize-none"
              rows={3}
              autoFocus
            />
            <div className="flex gap-2">
              <button
                onClick={handleSaveEdit}
                className="text-xs px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
              >
                Save
              </button>
              <button
                onClick={handleCancelEdit}
                className="text-xs px-2 py-1 bg-gray-300 dark:bg-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-400 dark:hover:bg-gray-500 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-sm text-gray-900 dark:text-gray-100 whitespace-pre-wrap">
              {comment.content}
            </p>
            <div className="flex gap-2">
              {onUpdate && (
                <button
                  onClick={() => setIsEditing(true)}
                  className="text-xs text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
                >
                  Edit
                </button>
              )}
              {onReply && (
                <button
                  onClick={() => setIsReplying(true)}
                  className="text-xs text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
                >
                  Reply
                </button>
              )}
              {onDelete && (
                <button
                  onClick={() => onDelete(comment.id)}
                  className="text-xs text-red-500 hover:text-red-700 transition-colors"
                >
                  Delete
                </button>
              )}
            </div>
          </div>
        )}

        {/* Reply input */}
        {isReplying && (
          <div className="mt-3 pt-3 border-t border-gray-100 dark:border-gray-700 space-y-2">
            <textarea
              value={replyContent}
              onChange={(e) => setReplyContent(e.target.value)}
              placeholder="Write a reply..."
              className="w-full text-sm border border-gray-300 dark:border-gray-600 rounded px-2 py-1 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 resize-none"
              rows={2}
              autoFocus
            />
            <div className="flex gap-2">
              <button
                onClick={handleSubmitReply}
                className="text-xs px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
              >
                Reply
              </button>
              <button
                onClick={handleCancelReply}
                className="text-xs px-2 py-1 bg-gray-300 dark:bg-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-400 dark:hover:bg-gray-500 transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Replies */}
      {comment.replies.length > 0 && (
        <div className="border-t border-gray-100 dark:border-gray-700">
          {comment.replies.map((reply) => (
            <div key={reply.id} className="ml-4 border-l-2 border-gray-200 dark:border-gray-600 pl-3 py-2">
              <CommentCard
                comment={reply}
                onUpdate={onUpdate}
                onDelete={onDelete}
                onReply={onReply}
                onResolve={onResolve}
                className="shadow-none border-0 bg-gray-50 dark:bg-gray-700"
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
