import { marked } from "marked";

const MAX_DISCORD_LENGTH = 2000;

/**
 * Chunks markdown text intelligently, respecting markdown structure.
 * Avoids splitting code blocks, lists, and other block elements when possible.
 * Falls back to character-based splitting for oversized blocks.
 *
 * @param markdown - The markdown text to chunk
 * @param maxLength - Maximum length per chunk (default: 2000 for Discord)
 * @returns Array of markdown chunks
 */
export function chunkMarkdown(
  markdown: string,
  maxLength: number = MAX_DISCORD_LENGTH,
): string[] {
  // If text is short enough, return as-is
  if (markdown.length <= maxLength) {
    return [markdown];
  }

  const tokens = marked.lexer(markdown);
  const chunks: string[] = [];
  let currentChunk = "";

  for (const token of tokens) {
    const tokenText = token.raw;

    // If adding this token would exceed the limit
    if ((currentChunk + tokenText).length > maxLength) {
      // Save current chunk if it has content
      if (currentChunk) {
        chunks.push(currentChunk);
        currentChunk = "";
      }

      // If the token itself is too large, we have to split it
      if (tokenText.length > maxLength) {
        // Fall back to character-based splitting for this oversized block
        const splits = characterSplit(tokenText, maxLength);
        chunks.push(...splits.slice(0, -1));
        currentChunk = splits[splits.length - 1];
      } else {
        // Token fits in a new chunk
        currentChunk = tokenText;
      }
    } else {
      // Token fits in current chunk
      currentChunk += tokenText;
    }
  }

  // Add any remaining content
  if (currentChunk) {
    chunks.push(currentChunk);
  }

  return chunks;
}

/**
 * Character-based splitting with word boundary awareness.
 * Used as a fallback for oversized markdown blocks.
 *
 * @param text - The text to split
 * @param maxLength - Maximum length per chunk
 * @returns Array of text chunks
 */
function characterSplit(text: string, maxLength: number): string[] {
  if (text.length <= maxLength) {
    return [text];
  }

  const chunks: string[] = [];
  let remaining = text;

  while (remaining.length > maxLength) {
    let splitIndex = maxLength;
    const spaceIndex = remaining.lastIndexOf(" ", maxLength);

    // If there's a space in the last 20% of the chunk, split there
    if (spaceIndex > maxLength * 0.8) {
      splitIndex = spaceIndex;
    }

    chunks.push(remaining.slice(0, splitIndex));
    remaining = remaining.slice(splitIndex).trimStart();
  }

  if (remaining) {
    chunks.push(remaining);
  }

  return chunks;
}
