import type { Message } from '../api';
import { getTextAndImageContent, getToolRequests, getToolResponses } from '../types/message';

export function identifyConsecutiveToolCalls(messages: Message[]): number[][] {
  const chains: number[][] = [];
  let currentChain: number[] = [];

  for (let i = 0; i < messages.length; i++) {
    const message = messages[i];
    const toolRequests = getToolRequests(message);
    const toolResponses = getToolResponses(message);
    const { textContent } = getTextAndImageContent(message);
    const hasText = textContent.trim().length > 0;

    if (toolResponses.length > 0 && toolRequests.length === 0) {
      continue;
    }

    if (toolRequests.length > 0) {
      if (hasText) {
        if (currentChain.length > 0) {
          if (currentChain.length > 1) {
            chains.push([...currentChain]);
          }
        }
        currentChain = [i];
      } else {
        currentChain.push(i);
      }
    } else if (hasText) {
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    } else {
      if (currentChain.length > 1) {
        chains.push([...currentChain]);
      }
      currentChain = [];
    }
  }

  if (currentChain.length > 1) {
    chains.push(currentChain);
  }

  return chains;
}

export function shouldHideTimestamp(messageIndex: number, chains: number[][]): boolean {
  for (const chain of chains) {
    if (chain.includes(messageIndex)) {
      // Hide timestamp for all but the last message in the chain
      return chain[chain.length - 1] !== messageIndex;
    }
  }
  return false;
}

export function isInChain(messageIndex: number, chains: number[][]): boolean {
  return chains.some((chain) => chain.includes(messageIndex));
}
