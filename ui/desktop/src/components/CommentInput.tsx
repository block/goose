import React, { useState, useRef, useEffect } from 'react';
import { TextSelection } from '../types/comment';
import { cn } from '../utils';

interface CommentInputProps {
  selection: TextSelection;
  onSubmit: (content: string) => void;
  onCancel: () => void;
  placeholder?: string;
  className?: string;
}

export default function CommentInput({
  selection,
  onSubmit,
  onCancel,
  placeholder = "Add a comment...",
  className,
}: CommentInputProps) {
  const [content, setContent] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    // Auto-focus the textarea when component mounts
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
  }, []);

  const handleSubmit = () => {
    const trimmedContent = content.trim();
    if (trimmedContent) {
      onSubmit(trimmedContent);
      setContent('');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
  };

  const handleCancel = () => {
    setContent('');
    onCancel();
  };

  return (
    <div 
      data-comment-ui
      className={cn(
        'bg-white dark:bg-gray-800 rounded-lg',
        className
      )}>
      {/* Comment input - no selected text preview */}
      <div className="p-3">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          className="w-full text-sm rounded px-3 py-2 bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-gray-100 resize-none focus:outline-none focus:ring-1 focus:ring-gray-400 dark:focus:ring-gray-500 border-0"
          rows={3}
        />
        
        {/* Actions */}
        <div className="flex items-center justify-between mt-3">
          <div className="text-xs text-gray-500 dark:text-gray-400">
            Press Cmd/Ctrl+Enter to submit, Esc to cancel
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleCancel}
              className="text-xs px-3 py-1.5 bg-gray-200 dark:bg-gray-600 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-500 transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleSubmit}
              disabled={!content.trim()}
              className="text-xs px-3 py-1.5 bg-neutral-900 dark:bg-white text-white dark:text-black rounded hover:bg-neutral-800 dark:hover:bg-gray-100 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
            >
              Comment
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
