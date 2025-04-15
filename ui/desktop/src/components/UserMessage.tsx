import React, { useRef } from 'react';
import LinkPreview from './LinkPreview';
import { extractUrls } from '../utils/urlUtils';
import MarkdownContent from './MarkdownContent';
import { Message, TextContent, ImageContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';

interface UserMessageProps {
  message: Message;
}

export default function UserMessage({ message }: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Separate content parts by type
  const imageParts = message.content.filter((part): part is ImageContent => part.type === 'image');
  const textParts = message.content.filter((part): part is TextContent => part.type === 'text');

  // Combine text for copy functionality (remains the same)
  const combinedTextForCopy = textParts.map((part) => part.text).join('\n');

  // Extract URLs (remains the same)
  const urls = extractUrls(combinedTextForCopy, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex flex-col group">
          <div className="flex flex-col bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
            {/* Render image parts first in a flex row container */}
            {imageParts.length > 0 && (
              <div className="flex flex-row flex-wrap gap-2 my-2">
                {imageParts.map((part, index) => (
                  <img
                    key={`img-${index}`}
                    src={part.data}
                    alt="User uploaded image"
                    className="max-w-full max-h-24 rounded object-contain"
                  />
                ))}
              </div>
            )}
            {/* Render text parts second */}
            {textParts.map((part, index) => (
              <div
                key={`txt-${index}`}
                ref={index === 0 ? contentRef : null}
                className="user-message"
              >
                <MarkdownContent content={part.text} className="text-white prose-a:text-white" />
              </div>
            ))}
          </div>
          <div className="flex justify-end">
            <MessageCopyLink text={combinedTextForCopy} contentRef={contentRef} />
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
