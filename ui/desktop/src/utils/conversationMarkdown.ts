import { Message, getTextAndImageContent } from '../types/message';

export function conversationToMarkdown(messages: Message[], title?: string): string {
  const parts: string[] = [];

  if (title && title.trim()) {
    parts.push(`# ${title.trim()}`);
  }

  for (const message of messages) {
    const { textContent } = getTextAndImageContent(message);
    if (!textContent || !textContent.trim()) {
      continue; // skip tool-only / thinking-only turns
    }
    const heading = message.role === 'user' ? '## You' : '## Goose';
    parts.push(`${heading}\n\n${textContent.trim()}`);
  }

  return parts.join('\n\n');
}
