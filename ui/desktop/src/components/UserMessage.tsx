import { useRef, useMemo } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import {
  Message,
  getTextContent,
  getSessionFilesFromMessage,
  ImageContent,
} from '../types/message';
import MessageCopyLink from './MessageCopyLink';
import { formatMessageTimestamp } from '../utils/timeUtils';
import { Document } from './icons';
import { FolderOpen } from 'lucide-react';

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

  // Extract session files from the message
  const sessionFiles = getSessionFilesFromMessage(message);

  // Extract images from ImageContent objects in the message
  const imageContents = message.content.filter(
    (content): content is ImageContent => content.type === 'image'
  );

  // Convert ImageContent objects to data URLs for display
  const imageDataUrls = imageContents.map(
    (imageContent) => `data:${imageContent.mimeType};base64,${imageContent.data}`
  );

  // Combine both image sources (new ImageContent and old image paths)
  const allImages = [...imageDataUrls, ...imagePaths];

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(displayText, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex flex-col group">
          {/* Only render the text message box if there is text to display */}
          {displayText && (
            <div className="flex bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
              <div ref={contentRef}>
                <MarkdownContent
                  content={displayText}
                  className="text-white prose-a:text-white user-message"
                />
              </div>
            </div>
          )}

          {/* Render session files if any */}
          {sessionFiles.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-2">
              {sessionFiles.map((sessionFile, index) => (
                <div
                  key={`${sessionFile.path}-${index}`}
                  className="flex items-center gap-1 px-2 py-1 bg-bgSubtle border border-borderSubtle rounded-full text-xs text-textStandard"
                >
                  {sessionFile.type === 'directory' ? (
                    <FolderOpen className="w-3 h-3 text-textSubtle" />
                  ) : (
                    <Document className="w-3 h-3 text-textSubtle" />
                  )}
                  <span className="max-w-[200px] truncate" title={sessionFile.path}>
                    {sessionFile.path.split('/').pop() || sessionFile.path}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* Render images if any */}
          {allImages.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-2">
              {allImages.map((imageSrc, index) => (
                <ImagePreview key={index} src={imageSrc} alt={`Pasted image ${index + 1}`} />
              ))}
            </div>
          )}

          <div className="relative h-[22px] flex justify-end">
            <div className="absolute right-0 text-xs text-textSubtle pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0">
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
