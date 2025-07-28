import { useRef, useMemo } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';
import { formatMessageTimestamp } from '../utils/timeUtils';

interface UserMessageProps {
  message: Message;
}

export default function UserMessage({ message }: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Extract text content from the message
  const textContent = getTextContent(message);

  // Extract image paths from the message
  const imagePaths = extractImagePaths(textContent);

  // Remove image paths from text for display
  const displayText = removeImagePathsFromText(textContent, imagePaths);

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(displayText, []);

  return (
    <div className="message flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
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
            <div className="absolute right-0 pt-1">
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
    </div>
  );
}
