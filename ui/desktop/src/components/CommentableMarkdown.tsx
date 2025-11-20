import React, { useCallback, useRef, useState, useEffect } from 'react';
import MarkdownContent from './MarkdownContent';
import { TextSelection, MessageComment } from '../types/comment';
import { cn } from '../utils';

interface CommentableMarkdownProps {
  content: string;
  messageId: string;
  comments?: MessageComment[];
  onSelectionChange?: (selection: TextSelection | null, position?: { x: number; y: number }, messageId?: string) => void;
  onCreateComment?: (selection: TextSelection) => void;
  onFocusComment?: (commentId: string) => void;
  className?: string;
}

export default function CommentableMarkdown({
  content,
  messageId,
  comments = [],
  onSelectionChange,
  onCreateComment,
  onFocusComment,
  className,
}: CommentableMarkdownProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [currentSelection, setCurrentSelection] = useState<TextSelection | null>(null);
  const [showCommentButton, setShowCommentButton] = useState(false);
  const [buttonPosition, setButtonPosition] = useState({ x: 0, y: 0 });

  // Handle text selection
  const handleMouseUp = useCallback(() => {
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || !containerRef.current) {
      setCurrentSelection(null);
      setShowCommentButton(false);
      onSelectionChange?.(null);
      return;
    }

    // Check if selection is within our container
    const range = selection.getRangeAt(0);
    if (!containerRef.current.contains(range.commonAncestorContainer)) {
      return;
    }

    // Get the selected text and calculate offsets
    const selectedText = selection.toString().trim();
    if (!selectedText) {
      setCurrentSelection(null);
      setShowCommentButton(false);
      onSelectionChange?.(null);
      return;
    }

    // Calculate text offsets within the markdown content
    // This is a simplified approach - in production you'd want more robust offset calculation
    const containerText = containerRef.current.textContent || '';
    const startOffset = containerText.indexOf(selectedText);
    const endOffset = startOffset + selectedText.length;

    if (startOffset === -1) {
      console.warn('Could not find selected text in container');
      return;
    }

    const textSelection: TextSelection = {
      startOffset,
      endOffset,
      selectedText,
    };

    setCurrentSelection(textSelection);
    
    // Position the comment button to the right of the selection
    const rect = range.getBoundingClientRect();
    const containerRect = containerRef.current.getBoundingClientRect();
    
    const position = {
      x: containerRect.width + 16, // Position to the right of the container
      y: rect.top - containerRect.top,
    };
    
    setButtonPosition(position);
    onSelectionChange?.(textSelection, position, messageId);
    setShowCommentButton(true);
  }, [onSelectionChange, messageId]);

  // Handle clicking outside to clear selection
  const handleMouseDown = useCallback((e: MouseEvent) => {
    if (!containerRef.current?.contains(e.target as Node)) {
      // Only clear if we're not clicking on a comment button or comment input
      const target = e.target as Element;
      if (!target.closest('[data-comment-ui]')) {
        setCurrentSelection(null);
        setShowCommentButton(false);
        onSelectionChange?.(null);
      }
    }
  }, [onSelectionChange]);

  useEffect(() => {
    document.addEventListener('mousedown', handleMouseDown);
    return () => document.removeEventListener('mousedown', handleMouseDown);
  }, [handleMouseDown]);

  // Force re-render of highlights when comments change
  const [highlightKey, setHighlightKey] = useState(0);
  useEffect(() => {
    setHighlightKey(prev => prev + 1);
  }, [comments]);

  // Handle comment button click
  const handleCreateComment = useCallback(() => {
    if (currentSelection && onCreateComment) {
      // Trigger the comment creation flow
      onCreateComment(currentSelection);
      setShowCommentButton(false);
      
      // Don't clear the browser selection - keep it highlighted
      // window.getSelection()?.removeAllRanges();
    }
  }, [currentSelection, onCreateComment]);

  // Render highlights for existing comments
  const renderHighlights = useCallback(() => {
    if (!comments.length || !containerRef.current) return null;

    return comments.map((comment) => {
      try {
        // Use the stored offsets to recreate the selection
        const containerText = containerRef.current!.textContent || '';
        
        // Verify the comment text still exists at the expected location
        const expectedText = containerText.substring(comment.startOffset, comment.endOffset);
        if (expectedText !== comment.selectedText) {
          // Text might have changed, try to find it by content
          const foundIndex = containerText.indexOf(comment.selectedText);
          if (foundIndex === -1) return null;
        }

        // Create a temporary range to get the bounding rect
        const range = document.createRange();
        const walker = document.createTreeWalker(
          containerRef.current!,
          NodeFilter.SHOW_TEXT,
          null
        );

        let currentOffset = 0;
        let startNode = null;
        let endNode = null;
        let relativeStartOffset = 0;
        let relativeEndOffset = 0;

        // Walk through text nodes to find the right positions
        let textNode;
        while (textNode = walker.nextNode()) {
          const nodeLength = textNode.textContent?.length || 0;
          
          // Check if this node contains the start of our selection
          if (!startNode && currentOffset + nodeLength > comment.startOffset) {
            startNode = textNode;
            relativeStartOffset = comment.startOffset - currentOffset;
          }
          
          // Check if this node contains the end of our selection
          if (startNode && currentOffset + nodeLength >= comment.endOffset) {
            endNode = textNode;
            relativeEndOffset = comment.endOffset - currentOffset;
            break;
          }
          
          currentOffset += nodeLength;
        }

        if (!startNode || !endNode) return null;

        // Set the range and get its position
        range.setStart(startNode, Math.max(0, relativeStartOffset));
        range.setEnd(endNode, Math.min(endNode.textContent?.length || 0, relativeEndOffset));
        
        const rects = range.getClientRects();
        const containerRect = containerRef.current!.getBoundingClientRect();

        // Handle multi-line selections by creating multiple highlight boxes
        return Array.from(rects).map((rect, index) => (
          <div
            key={`${comment.id}-${index}`}
            className="absolute cursor-pointer bg-yellow-200/30 dark:bg-yellow-500/20 rounded-sm border border-yellow-300/50 dark:border-yellow-400/30 hover:bg-yellow-200/50 dark:hover:bg-yellow-500/30 transition-colors"
            style={{
              left: rect.left - containerRect.left,
              top: rect.top - containerRect.top,
              width: rect.width,
              height: rect.height,
            }}
            title={`Click to focus comment: ${comment.content.substring(0, 100)}${comment.content.length > 100 ? '...' : ''}`}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onFocusComment?.(comment.id);
            }}
          />
        ));
      } catch (error) {
        console.warn('Error rendering highlight for comment:', comment.id, error);
        return null;
      }
    }).flat().filter(Boolean);
  }, [comments, onFocusComment]);

  return (
    <div className={cn('relative', className)}>
      <div
        ref={containerRef}
        onMouseUp={handleMouseUp}
        className="relative select-text"
      >
        {renderHighlights()}
        <MarkdownContent content={content} />
      </div>

      {/* Comment button */}
      {showCommentButton && currentSelection && (
        <button
          data-comment-ui
          onClick={handleCreateComment}
          className="absolute z-10 px-2 py-1 text-xs bg-blue-500 text-white rounded shadow-lg hover:bg-blue-600 transition-colors"
          style={{
            left: buttonPosition.x,
            top: buttonPosition.y,
          }}
        >
          💬 Comment
        </button>
      )}
    </div>
  );
}
