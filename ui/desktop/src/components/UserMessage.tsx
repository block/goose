import React from 'react'
import LinkPreview from './LinkPreview'
import { extractUrls } from '../utils/urlUtils'
import MarkdownContent from './MarkdownContent'

interface UserMessageProps {
  message: {
    content: string;
    image?: string;
  };
}

export default function UserMessage({ message }: UserMessageProps) {
  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(message.content, []);

  // Remove the image placeholder from displayed content if it exists
  const displayContent = message.image 
    ? message.content.replace(/\[Attached Image: .*?\]/, '').trim()
    : message.content;

  return (
    <div className="flex justify-end mb-[16px]">
      <div className="flex-col max-w-[90%]">
        {message.image && (
          <div className="flex justify-end mb-2">
            <div className="w-[120px] h-[120px] rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 ring-1 ring-black/5 dark:ring-white/5">
              <div className="w-full h-full relative">
                <img 
                  src={message.image} 
                  alt="User uploaded content"
                  className="absolute inset-0 w-full h-full object-cover"
                  style={{
                    imageRendering: 'auto',
                    transform: 'translate3d(0,0,0)',
                    backfaceVisibility: 'hidden',
                    WebkitFontSmoothing: 'antialiased',
                  }}
                />
              </div>
            </div>
          </div>
        )}
        {(displayContent || !message.image) && (
          <div className="flex flex-col bg-user-bubble dark:bg-user-bubble-dark text-goose-text-light dark:text-goose-text-light-dark rounded-2xl p-4">
            <MarkdownContent
              content={displayContent || message.content}
              className="text-white"
            />
          </div>
        )}
        {urls.length > 0 && (
          <div className="flex mt-2">
            {urls.map((url, index) => (
              <LinkPreview key={index} url={url} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
