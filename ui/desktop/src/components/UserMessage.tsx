import React from 'react';
import MarkdownContent from './MarkdownContent';
import { Message } from '../ai-sdk-fork/useChat';

interface MessageWithImage extends Message {
  image?: {
    preview: string;
    compressed: string;
    path?: string;
  };
}

export default function UserMessage({ message }: { message: MessageWithImage }) {
  return (
    <div className="flex justify-end mb-[16px]">
      <div className="flex flex-col items-end max-w-[90%]">
        {message.image && (
          <div className="mb-2" style={{ width: '300px' }}>
            <div className="rounded-2xl overflow-hidden bg-gray-100 dark:bg-gray-700 ring-1 ring-black/5 dark:ring-white/5">
              <div className="w-full h-full relative">
                <img 
                  src={message.image.preview} 
                  alt="Attached"
                  className="w-full h-full object-cover"
                  style={{
                    maxHeight: '300px',
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
        {message.content && (
          <div className="inline-flex bg-user-bubble dark:bg-user-bubble-dark text-goose-text-light dark:text-goose-text-light-dark rounded-2xl p-4">
            <MarkdownContent
              content={message.content}
              className="text-white"
            />
          </div>
        )}
      </div>
    </div>
  );
}