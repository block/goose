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

  // Process content array instead of using getTextContent
  // Combine all text parts for copy functionality
  const combinedTextForCopy = message.content
    .filter((part): part is TextContent => part.type === 'text')
    .map((part) => part.text)
    .join('\n');

  // Extract URLs from combined text for previews (if re-enabled)
  const urls = extractUrls(combinedTextForCopy, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex flex-col group">
          {/* Render message parts */}
          <div className="flex flex-col bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
            {message.content.map((part, index) => {
              if (part.type === 'text') {
                return (
                  <div key={index} ref={index === 0 ? contentRef : null} className="user-message">
                    {' '}
                    {/* Added user-message class */}
                    <MarkdownContent
                      content={part.text}
                      className="text-white prose-a:text-white"
                    />
                  </div>
                );
              } else if (part.type === 'image') {
                // Render image using standard img tag
                return (
                  <img
                    key={index}
                    src={part.data} // Assumes data is base64 data URI
                    alt="User uploaded image"
                    className="max-w-full max-h-64 my-2 rounded object-contain" // Added styling
                  />
                );
              }
              return null; // Handle other potential types if needed
            })}
          </div>
          <div className="flex justify-end">
            {/* Use combined text for copy link */}
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
