import React from 'react';
import ReactMarkdown from 'react-markdown';
import ToolInvocations from './ToolInvocations';
import LinkPreview from './LinkPreview';
import GooseResponseForm from './GooseResponseForm';
import { extractUrls } from '../utils/urlUtils';

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
  const messageIndex = messages?.findIndex(msg => msg.id === message.id);
  const previousMessage = messageIndex > 0 ? messages[messageIndex - 1] : null;
  const previousUrls = previousMessage ? extractUrls(previousMessage.content) : [];
  const urls = !message.toolInvocations ? extractUrls(message.content, previousUrls) : [];

  return (
    <div className="flex justify-start mb-[16px]">
      <div className="flex-col">
        {message.toolInvocations && (
          <ToolInvocations toolInvocations={message.toolInvocations} />
        )}

        {message.content && (
          <div className="flex bg-goose-bubble text-white rounded-2xl p-4 mb-[16px]">
            <ReactMarkdown className="prose prose-xs">{message.content}</ReactMarkdown>
          </div>
        )}

        {urls.length > 0 && (
          <div className="flex mb-[16px]">
            {urls.map((url, index) => (
              <LinkPreview key={index} url={url} />
            ))}
          </div>
        )}

        {/* append false && to turn this off */}
        {metadata && (
          <div className="flex mb-[16px]">
            <GooseResponseForm
              message={message.content}
              metadata={metadata}
              append={append}
            />
          </div>
        )}
      </div>
    </div>
  );
}