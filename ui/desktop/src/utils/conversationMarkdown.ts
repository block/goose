import { Message, getTextAndImageContent, getElicitationContent } from '../types/message';

export function conversationToMarkdown(messages: Message[], title?: string): string {
  const parts: string[] = [];

  if (title && title.trim()) {
    parts.push(`# ${title.trim()}`);
  }

  for (const message of messages) {
    if (!message.metadata.userVisible) {
      continue; // mirror the chat display, which hides non-user-visible messages
    }

    const { textContent, imagePaths } = getTextAndImageContent(message);
    const elicitation = getElicitationContent(message);
    const elicitationText =
      elicitation?.data.actionType === 'elicitation' ? elicitation.data.message : '';

    const text = [textContent.trim(), elicitationText.trim()].filter(Boolean).join('\n\n');
    const images = imagePaths.map(() => '_[image]_');

    if (!text && images.length === 0) {
      continue; // skip tool-only / thinking-only turns with no visible content
    }

    const heading = message.role === 'user' ? '## You' : '## Goose';
    const body = [text, ...images].filter(Boolean).join('\n\n');
    parts.push(`${heading}\n\n${body}`);
  }

  return parts.join('\n\n');
}
