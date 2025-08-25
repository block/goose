import { Message, getToolRequests, getTextContent, getToolResponses } from '../types/message';

/**
 * Enhanced function to detect tool call chains including mixed content messages
 * @param messages - Array of all messages
 * @returns Array of message indices that should be chained together
 */
export function identifyConsecutiveToolCalls(messages: Message[]): number[][] {
  const chains: number[][] = [];
  let currentChain: number[] = [];

  for (let i = 0; i < messages.length; i++) {
    const message = messages[i];
    const toolRequests = getToolRequests(message);
    const toolResponses = getToolResponses(message);
    const textContent = getTextContent(message);
    const hasText = textContent.trim().length > 0;

    // Skip tool response messages - they don't break chains
    if (toolResponses.length > 0 && toolRequests.length === 0) {
      continue;
    }

    // This message has tool calls
    if (toolRequests.length > 0) {
      if (hasText) {
        // Message with text + tools - start a new chain or add to existing
        if (currentChain.length > 0) {
          // End current chain and start new one
          if (currentChain.length > 1) {
            chains.push([...currentChain]);
          }
        }
        // Start new chain with this mixed content message
        currentChain = [i];
      } else {
        // Pure tool call message - add to chain
        currentChain.push(i);
      }
    } else if (hasText) {
      // Pure text message - end current chain
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    } else {
      // Empty or other content - end current chain
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    }
  }

  // Don't forget the last chain
  if (currentChain.length > 1) {
    chains.push(currentChain);
  }

  return chains;
}

/**
 * Check if a message at given index should be hidden (part of chain but not first)
 * @param messageIndex - Index of the message to check
 * @param chains - Array of chains (arrays of message indices)
 * @returns True if message should be hidden
 */
export function shouldHideMessage(messageIndex: number, chains: number[][]): boolean {
  for (const chain of chains) {
    if (chain.includes(messageIndex)) {
      // Hide if it's in a chain but not the first message
      return chain[0] !== messageIndex;
    }
  }
  return false;
}

/**
 * Get the chain that contains the given message index
 * @param messageIndex - Index of the message
 * @param chains - Array of chains
 * @returns The chain containing this message, or null
 */
export function getChainForMessage(messageIndex: number, chains: number[][]): number[] | null {
  return chains.find(chain => chain.includes(messageIndex)) || null;
}
