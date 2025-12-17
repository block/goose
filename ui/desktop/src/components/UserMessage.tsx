import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import ImagePreview from './ImagePreview';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import { getTextContent } from '../types/message';
import { Message } from '../api';
import MessageCopyLink from './MessageCopyLink';
import { formatMessageTimestamp } from '../utils/timeUtils';
import Edit from './icons/Edit';
import { Button } from './ui/button';

interface UserMessageProps {
  message: Message;
  onMessageUpdate?: (messageId: string, newContent: string, editType?: 'fork' | 'edit') => void;
}

export default function UserMessage({ message, onMessageUpdate }: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState('');
  const [hasBeenEdited, setHasBeenEdited] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Extract text content from the message
  const textContent = getTextContent(message);

  // Extract image paths from the message
  const imagePaths = extractImagePaths(textContent);

  // Remove image paths from text for display - memoized for performance
  const displayText = useMemo(
    () => removeImagePathsFromText(textContent, imagePaths),
    [textContent, imagePaths]
  );

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Effect to handle message content changes and ensure persistence
  useEffect(() => {
    // If we're not editing, update the edit content to match the current message
    if (!isEditing) {
      setEditContent(displayText);
    }
  }, [message.content, displayText, message.id, isEditing]);

  // Initialize edit mode with current message content
  const initializeEditMode = useCallback(() => {
    setEditContent(displayText);
    setError(null);
    window.electron.logInfo(`Entering edit mode with content: ${displayText}`);
  }, [displayText]);

  // Handle edit button click
  const handleEditClick = useCallback(() => {
    const newEditingState = !isEditing;
    setIsEditing(newEditingState);

    // Initialize edit content when entering edit mode
    if (newEditingState) {
      initializeEditMode();
      window.electron.logInfo(`Edit interface shown for message: ${message.id}`);

      // Focus the textarea after a brief delay to ensure it's rendered
      setTimeout(() => {
        if (textareaRef.current) {
          textareaRef.current.focus();
          textareaRef.current.setSelectionRange(
            textareaRef.current.value.length,
            textareaRef.current.value.length
          );
        }
      }, 50);
    }

    window.electron.logInfo(`Edit state toggled: ${newEditingState} for message: ${message.id}`);
  }, [isEditing, initializeEditMode, message.id]);

  // Handle content changes in edit mode
  const handleContentChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value;
    setEditContent(newContent);
    setError(null); // Clear any previous errors
    window.electron.logInfo(`Content changed: ${newContent}`);
  }, []);

  const handleSave = useCallback(
    (editType: 'fork' | 'edit' = 'fork') => {
      if (editContent.trim().length === 0) {
        setError('Message cannot be empty');
        return;
      }

      setIsEditing(false);

      if (editContent.trim() === displayText.trim()) {
        return;
      }

      if (onMessageUpdate && message.id) {
        onMessageUpdate(message.id, editContent, editType);
        setHasBeenEdited(true);
      }
    },
    [editContent, displayText, onMessageUpdate, message.id]
  );

  // Handle cancel action
  const handleCancel = useCallback(() => {
    window.electron.logInfo('Cancel clicked - reverting to original content');
    setIsEditing(false);
    setEditContent(displayText); // Reset to original content
    setError(null);
  }, [displayText]);

  // Handle keyboard events for accessibility
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      window.electron.logInfo(
        `Key pressed: ${e.key}, metaKey: ${e.metaKey}, ctrlKey: ${e.ctrlKey}`
      );

      if (e.key === 'Escape') {
        e.preventDefault();
        handleCancel();
      } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        window.electron.logInfo('Cmd+Enter detected, calling handleSave');
        handleSave();
      }
    },
    [handleCancel, handleSave]
  );

  // Auto-resize textarea based on content
  useEffect(() => {
    if (textareaRef.current && isEditing) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`;
    }
  }, [editContent, isEditing]);

  return (
    <div className="w-full mt-[16px] opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex flex-col group">
        {isEditing ? (
          // Truly wide, centered, in-place edit box replacing the bubble
          <div className="w-full max-w-4xl mx-auto bg-background-light dark:bg-background-dark text-text-prominent rounded-xl border border-border-subtle shadow-lg py-4 px-4 my-2 transition-all duration-200 ease-in-out">
            <textarea
              ref={textareaRef}
              value={editContent}
              onChange={handleContentChange}
              onKeyDown={handleKeyDown}
              className="w-full resize-none bg-transparent text-text-prominent placeholder:text-text-subtle border border-border-subtle rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400 focus:border-blue-400 transition-all duration-200 text-base leading-relaxed"
              style={{
                minHeight: '120px',
                maxHeight: '300px',
                padding: '16px',
                fontFamily: 'inherit',
                lineHeight: '1.6',
                wordBreak: 'break-word',
                overflowWrap: 'break-word',
              }}
              placeholder="Edit your message..."
              aria-label="Edit message content"
              aria-describedby={error ? `error-${message.id}` : undefined}
            />
            {/* Error message */}
            {error && (
              <div
                id={`error-${message.id}`}
                className="text-red-400 text-xs mt-2 mb-2"
                role="alert"
                aria-live="polite"
              >
                {error}
              </div>
            )}
            <div className="flex justify-between items-center mt-4">
              <div className="text-xs text-text-subtle">
                <span className="font-semibold">Edit in Place</span> updates this session •{' '}
                <span className="font-semibold">Fork Session</span> creates a new session
              </div>
              <div className="flex gap-3">
                <Button onClick={handleCancel} variant="ghost" aria-label="Cancel editing">
                  Cancel
                </Button>
                <Button
                  onClick={() => handleSave('edit')}
                  variant="secondary"
                  aria-label="Edit message in place"
                  title="Update the message in this session"
                >
                  Edit in Place
                </Button>
                <Button
                  onClick={() => handleSave('fork')}
                  aria-label="Fork session with edited message"
                  title="Create a new session with the edited message"
                >
                  Fork Session
                </Button>
              </div>
            </div>
          </div>
        ) : (
          // Normal message display
          <div className="message flex justify-end w-full">
            <div className="flex-col max-w-[85%] w-fit">
              <div className="flex flex-col group">
                <div className="flex bg-background-accent text-text-on-accent rounded-xl py-2.5 px-4">
                  <div ref={contentRef}>
                    <MarkdownContent
                      content={displayText}
                      className="text-text-on-accent prose-a:text-text-on-accent prose-headings:text-text-on-accent prose-strong:text-text-on-accent prose-em:text-text-on-accent user-message"
                    />
                  </div>
                </div>

                {/* Render images if any */}
                {imagePaths.length > 0 && (
                  <div className="flex flex-wrap gap-2 mt-2">
                    {imagePaths.map((imagePath, index) => (
                      <ImagePreview key={index} src={imagePath} alt={`Pasted image ${index + 1}`} />
                    ))}
                  </div>
                )}

                <div className="relative h-[22px] flex justify-end text-right">
                  <div className="absolute w-40 font-mono right-0 text-xs text-text-muted pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
                    {timestamp}
                  </div>
                  <div className="absolute right-0 pt-1 flex items-center gap-2">
                    <button
                      onClick={handleEditClick}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          e.preventDefault();
                          handleEditClick();
                        }
                      }}
                      className="flex items-center gap-1 text-xs text-text-subtle hover:cursor-pointer hover:text-text-prominent transition-all duration-200 opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-opacity-50 rounded"
                      aria-label={`Edit message: ${displayText.substring(0, 50)}${displayText.length > 50 ? '...' : ''}`}
                      aria-expanded={isEditing}
                      title="Edit message"
                    >
                      <Edit className="h-3 w-3" />
                      <span>Edit</span>
                    </button>
                    <MessageCopyLink text={displayText} contentRef={contentRef} />
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Edited indicator */}
        {hasBeenEdited && !isEditing && (
          <div className="text-xs text-text-subtle mt-1 text-right transition-opacity duration-200">
            Edited
          </div>
        )}
      </div>
    </div>
  );
}
