import React, { useState } from 'react';
import { MessageCircle } from 'lucide-react';
import { MessageComment } from '../types/comment';
import { cn } from '../utils';

interface CommentBadgeProps {
  comments: MessageComment[];
  position: { x: number; y: number };
  onClick: () => void;
  className?: string;
}

/**
 * Compact badge showing comment count, displayed when in condensed mode
 */
export default function CommentBadge({
  comments,
  position,
  onClick,
  className,
}: CommentBadgeProps) {
  const [isHovered, setIsHovered] = useState(false);
  
  // Filter to top-level comments only
  const topLevelComments = comments.filter(c => !c.parentId);
  const totalComments = comments.length;
  const resolvedCount = comments.filter(c => c.resolved).length;
  
  // Get first comment for preview
  const firstComment = topLevelComments[0];
  const previewText = firstComment?.content.slice(0, 100) || '';

  return (
    <div
      className={cn(
        'absolute z-10 transition-all',
        className
      )}
      style={{
        left: position.x,
        top: position.y,
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Badge */}
      <button
        onClick={onClick}
        data-comment-ui
        className={cn(
          'flex items-center gap-1.5 px-2 py-1 rounded-full shadow-md transition-all',
          'bg-background-default border border-border-subtle',
          'hover:shadow-lg hover:scale-105 hover:bg-background-medium'
        )}
        title={`${totalComments} comment${totalComments !== 1 ? 's' : ''}`}
      >
        <MessageCircle className="w-3.5 h-3.5 text-text-prominent" />
        <span className="text-xs font-semibold text-text-prominent">
          {totalComments}
        </span>
        {resolvedCount > 0 && resolvedCount < totalComments && (
          <span className="text-[10px] text-text-muted">
            ({resolvedCount} ✓)
          </span>
        )}
        {resolvedCount === totalComments && (
          <span className="text-[10px] text-text-muted">
            ✓
          </span>
        )}
      </button>

      {/* Hover preview */}
      {isHovered && firstComment && (
        <div
          className="absolute left-0 top-full mt-2 w-64 p-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl z-20"
          data-comment-ui
        >
          <div className="flex items-start gap-2">
            <MessageCircle className="w-4 h-4 text-blue-500 flex-shrink-0 mt-0.5" />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-xs font-semibold text-gray-900 dark:text-gray-100">
                  {firstComment.author}
                </span>
                <span className="text-[10px] text-gray-500 dark:text-gray-400">
                  {new Date(firstComment.timestamp).toLocaleDateString()}
                </span>
              </div>
              <p className="text-xs text-gray-700 dark:text-gray-300 line-clamp-3">
                {previewText}
                {firstComment.content.length > 100 && '...'}
              </p>
              {topLevelComments.length > 1 && (
                <p className="text-[10px] text-gray-500 dark:text-gray-400 mt-2">
                  +{topLevelComments.length - 1} more comment{topLevelComments.length !== 2 ? 's' : ''}
                </p>
              )}
            </div>
          </div>
          <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
            <p className="text-[10px] text-gray-500 dark:text-gray-400 italic">
              Click to view all comments
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
