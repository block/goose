import React from 'react';
import LinkPreview from './LinkPreview';
import { extractUrls } from '../utils/urlUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import CopyButton from './ui/CopyButton';

interface UserMessageProps {
  message: Message;
}

export default function UserMessage({ message }: UserMessageProps) {
  // Extract text content from the message
  const textContent = getTextContent(message);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(textContent, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex bg-slate text-white rounded-xl rounded-br-none py-2 px-3 mr-4 relative group">
          <MarkdownContent content={textContent} className="text-white" />
          <CopyButton
            text={textContent}
            className="absolute -bottom-2 -right-2 p-1.5 rounded-full bg-white dark:bg-gray-800 shadow-md z-[1000] hover:bg-gray-100 dark:hover:bg-gray-700 opacity-0 group-hover:opacity-100 transition-opacity"
            iconClassName="h-4 w-4 text-gray-800 dark:text-white"
          />
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
