import React from 'react';
import ToolInvocations from './ToolInvocations';
import LinkPreview from './LinkPreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';
import MarkdownContent from './MarkdownContent';

interface GooseMessageProps {
  message: any;
  messages: any[];
  metadata?: any;
  append: (value: any) => void;
}

export default function GooseMessage({ message, metadata, messages, append }: GooseMessageProps) {
  // Extract URLs under a few conditions
  // 1. The message is purely text
  // 2. The link wasn't also present in the previous message
  // 3. The message contains the explicit http:// or https:// protocol at the beginning
  const messageIndex = messages?.findIndex((msg) => msg.id === message.id);
  const previousMessage = messageIndex > 0 ? messages[messageIndex - 1] : null;
  const previousUrls = previousMessage ? extractUrls(previousMessage.content) : [];
  const urls = !message.toolInvocations ? extractUrls(message.content, previousUrls) : [];

  return (
    <div className="flex justify-start mb-[16px]">
      <div className="flex-col w-[90%]">
        <div className="flex flex-col bg-goose-bubble dark:bg-goose-bubble-dark rounded-2xl p-4">
          {message.content && <MarkdownContent content={message.content} />}
          {message.toolInvocations && (
            <div className="mt-1">
              <ToolInvocations toolInvocations={message.toolInvocations} />
            </div>
          )}
        </div>

        {urls.length > 0 && (
          <div className="flex flex-wrap mt-[16px]">
            {urls.map((url, index) => (
              <LinkPreview key={index} url={url} />
            ))}
          </div>
        )}

        {/* enable or disable prompts here */}
        {/* NOTE from alexhancock on 1/14/2025 - disabling again temporarily due to non-determinism in when the forms show up */}
        {false && metadata && (
          <div className="flex mt-[16px]">
            <GooseResponseForm message={message.content} metadata={metadata} append={append} />
          </div>
        )}
      </div>
    </div>
  );
}
