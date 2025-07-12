import { useRef, useMemo, useState, useEffect } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';
import { formatMessageTimestamp } from '../utils/timeUtils';
import { Button } from './ui/button';
import { Edit } from './icons';

interface UserMessageProps {
  message: Message;
  isEditing?: boolean;
  onEdit?: () => void;
  onSave?: (newText: string) => void;
  onCancel?: () => void;
}

export default function UserMessage({
  message,
  isEditing = false,
  onEdit,
  onSave,
  onCancel,
}: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Extract text content from the message
  const textContent = getTextContent(message);

  const [editValue, setEditValue] = useState(textContent);

  // Reset edit value when toggling edit mode or message changes
  useEffect(() => {
    if (isEditing) {
      setEditValue(textContent);
    }
  }, [isEditing, textContent]);

  // Extract image paths from the message
  const imagePaths = extractImagePaths(textContent);

  // Remove image paths from text for display
  const displayText = removeImagePathsFromText(textContent, imagePaths);

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(displayText, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div
          className={`flex flex-col group ${isEditing ? 'border border-borderProminent rounded-xl p-2' : ''}`}
        >
          {isEditing ? (
            <>
              <textarea
                className="w-full resize-y rounded-md p-2 text-sm text-black"
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                rows={3}
              />
              <div className="flex justify-end gap-2 mt-2">
                <Button variant="outline" size="sm" onClick={onCancel}>
                  Cancel
                </Button>
                <Button size="sm" onClick={() => onSave && onSave(editValue)}>
                  Save
                </Button>
              </div>
            </>
          ) : (
            <>
              <div className="flex bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
                <div ref={contentRef}>
                  <MarkdownContent
                    content={displayText}
                    className="text-white prose-a:text-white prose-headings:text-white prose-strong:text-white prose-em:text-white user-message"
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

              <div className="relative h-[22px] flex justify-end">
                <div className="absolute right-0 text-xs text-textSubtle pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
                  {timestamp}
                </div>
                <div className="absolute right-0 pt-1 flex gap-2">
                  <MessageCopyLink text={displayText} contentRef={contentRef} />
                  {onEdit && (
                    <button
                      onClick={onEdit}
                      className="flex items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 opacity-0 group-hover:opacity-100"
                    >
                      <Edit className="h-3 w-3" />
                      <span>Edit</span>
                    </button>
                  )}
                </div>
              </div>
            </>
          )}
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
    </div>
  );
}
