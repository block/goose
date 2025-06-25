import { useRef, useMemo, useState, useEffect, useCallback } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';
import { formatMessageTimestamp } from '../utils/timeUtils';
import Edit from './icons/Edit';

interface UserMessageProps {
  message: Message;
  onMessageUpdate?: (messageId: string, newContent: string) => void;
  onTriggerAIResponse?: (messageId: string, newContent: string) => Promise<void>;
}

export default function UserMessage({
  message,
  onMessageUpdate,
  onTriggerAIResponse,
}: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState('');
  const [hasBeenEdited, setHasBeenEdited] = useState(false);
  const [isLoadingResponse, setIsLoadingResponse] = useState(false);
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

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = useMemo(() => extractUrls(displayText, []), [displayText]);

  // Effect to handle message content changes and ensure persistence
  useEffect(() => {
    // Log content display for debugging
    window.electron.logInfo(
      `Displaying content for message: ${message.id} content: ${displayText}`
    );

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

  // Handle save action with improved error handling
  const handleSave = useCallback(async () => {
    window.electron.logInfo(`Save clicked - new content: ${editContent}`);

    // Exit edit mode immediately to prevent UI issues
    setIsEditing(false);
    setError(null);

    // Check if content has actually changed
    if (editContent !== displayText) {
      // Validate content
      if (editContent.trim().length === 0) {
        setError('Message cannot be empty');
        window.electron.logInfo('Save failed: Message is empty');
        return;
      }

      // Update the message content through the callback
      if (onMessageUpdate && message.id) {
        try {
          onMessageUpdate(message.id, editContent);
          setHasBeenEdited(true);
          window.electron.logInfo(`Content updated successfully: ${editContent}`);

          // Trigger AI re-response
          if (onTriggerAIResponse && message.id) {
            try {
              setIsLoadingResponse(true);
              window.electron.logInfo(
                `Triggering AI re-response for edited message: ${message.id}`
              );

              await onTriggerAIResponse(message.id, editContent);
              window.electron.logInfo(
                `AI re-response triggered successfully for message: ${message.id}`
              );
            } catch (error) {
              const errorMessage = `AI re-response failed for message ${message.id}: ${error}`;
              window.electron.logInfo(errorMessage);
              setError('Failed to generate new response. Please try again.');
            } finally {
              setIsLoadingResponse(false);
            }
          }
        } catch (error) {
          const errorMessage = `Failed to save message ${message.id}: ${error}`;
          window.electron.logInfo(errorMessage);
          setError('Failed to save message. Please try again.');
        }
      }
    } else {
      window.electron.logInfo('No content changes detected, skipping update');
    }
  }, [editContent, displayText, onMessageUpdate, onTriggerAIResponse, message.id]);

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
          <div className="w-full max-w-4xl mx-auto bg-[#222] dark:bg-[#1a1a1a] text-white rounded-xl border border-[#444] shadow-lg py-4 px-4 my-2 transition-all duration-200 ease-in-out">
            <textarea
              ref={textareaRef}
              value={editContent}
              onChange={handleContentChange}
              onKeyDown={handleKeyDown}
              className="w-full resize-none bg-transparent text-white placeholder:text-white/50 border border-[#555] rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-400 focus:border-blue-400 transition-all duration-200 text-base leading-relaxed"
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
                className="text-red-300 text-xs mt-2 mb-2"
                role="alert"
                aria-live="polite"
              >
                {error}
              </div>
            )}
            <div className="flex justify-end gap-3 mt-4">
              <button
                onClick={handleCancel}
                className="px-4 py-2 text-sm text-textSubtle hover:text-textProminent transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-opacity-50 rounded"
                aria-label="Cancel editing"
              >
                Cancel
              </button>
              <button
                onClick={handleSave}
                disabled={isLoadingResponse}
                className="px-4 py-2 text-sm bg-white text-slate rounded hover:bg-gray-100 transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-opacity-50 disabled:opacity-50 disabled:cursor-not-allowed"
                aria-label="Save changes"
              >
                {isLoadingResponse ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>
        ) : (
          // Normal message display (bubble hugs content)
          <div className="flex justify-end w-full">
            <div className="inline-block bg-slate text-white rounded-xl rounded-br-none py-2 px-3 max-w-[75%] align-bottom">
              <div ref={contentRef}>
                <MarkdownContent
                  content={displayText}
                  className="text-white prose-a:text-white user-message"
                />
              </div>
            </div>
          </div>
        )}

        {/* Loading indicator for AI re-response */}
        {isLoadingResponse && (
          <div className="text-xs text-textSubtle mt-1 text-right transition-opacity duration-200">
            Generating new response...
          </div>
        )}

        {/* Edited indicator */}
        {hasBeenEdited && !isEditing && !isLoadingResponse && (
          <div className="text-xs text-textSubtle mt-1 text-right transition-opacity duration-200">
            Edited
          </div>
        )}

        {/* Render images if any */}
        {imagePaths.length > 0 && (
          <div className="flex flex-wrap gap-2 mt-2">
            {imagePaths.map((imagePath, index) => (
              <ImagePreview key={index} src={imagePath} alt={`Pasted image ${index + 1}`} />
            ))}
          </div>
        )}

        <div className="relative h-[22px] flex justify-end">
          <div className="absolute right-0 text-xs text-textSubtle pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
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
              className="flex items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0 focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-opacity-50 rounded"
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

      {/* TODO(alexhancock): Re-enable link previews once styled well again */}
      {false && urls.length > 0 && (
        <div className="flex flex-wrap mt-2">
          {urls.map((url, index) => (
            <LinkPreview key={index} url={url} />
          ))}
        </div>
      )}
    </div>
  );
}
