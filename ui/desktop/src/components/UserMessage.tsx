import React, { useRef } from 'react';
import LinkPreview from './LinkPreview';
import { extractUrls } from '../utils/urlUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';

interface UserMessageProps {
  message: Message;
}

export default function UserMessage({ message }: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Extract text content from the message
  const textContent = getTextContent(message);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(textContent, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex flex-col group">
          <div className="flex bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
            <div ref={contentRef}>
              <MarkdownContent content={textContent} className="text-white" />
            </div>
          </div>
          <div className="flex justify-end">
            <MessageCopyLink text={textContent} contentRef={contentRef} />
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
