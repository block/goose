/**
 * Message Grouping Utilities
 * 
 * Provides functionality to group consecutive messages from the same sender
 * to create a cleaner, more readable chat interface similar to Slack or Discord.
 */

import { Message } from '../types/message';

export interface MessageGroup {
  /** The sender information for this group */
  sender: {
    userId: string;
    displayName?: string;
    avatarUrl?: string | null;
  };
  /** Whether this group is from the current user */
  isFromSelf: boolean;
  /** Whether this group is from a user (vs assistant/system) */
  isUser: boolean;
  /** The messages in this group */
  messages: Message[];
  /** The timestamp of the first message in the group (for header display) */
  timestamp: number;
  /** Whether this is the first message in the group (shows avatar/header) */
  isFirstInGroup: boolean;
  /** Whether this is the last message in the group */
  isLastInGroup: boolean;
}

export interface GroupedMessage extends Message {
  /** Group information for this message */
  groupInfo: MessageGroup;
  /** Index within the group (0 = first message in group) */
  indexInGroup: number;
  /** Whether this message should show the avatar and header */
  showHeader: boolean;
  /** Whether this message should show reduced spacing */
  isGrouped: boolean;
}

/**
 * Determines if two messages should be grouped together
 */
export function shouldGroupMessages(
  prevMessage: Message | null,
  currentMessage: Message,
  isUserMessage: (message: Message) => boolean,
  maxGroupTimeGap: number = 5 * 60 * 1000 // 5 minutes in milliseconds
): boolean {
  if (!prevMessage) {
    return false;
  }

  // Don't group if messages are too far apart in time
  const timeDiff = (currentMessage.created || 0) - (prevMessage.created || 0);
  if (timeDiff > maxGroupTimeGap) {
    return false;
  }

  // Don't group different roles (user vs assistant)
  if (currentMessage.role !== prevMessage.role) {
    return false;
  }

  // For user messages, check if they're from the same sender
  if (isUserMessage(currentMessage) && isUserMessage(prevMessage)) {
    const currentSender = currentMessage.sender?.userId || 'current-user';
    const prevSender = prevMessage.sender?.userId || 'current-user';
    return currentSender === prevSender;
  }

  // For assistant messages, check if they're from the same assistant/collaborator
  if (!isUserMessage(currentMessage) && !isUserMessage(prevMessage)) {
    const currentSender = currentMessage.sender?.userId || 'goose';
    const prevSender = prevMessage.sender?.userId || 'goose';
    return currentSender === prevSender;
  }

  return false;
}

/**
 * Gets sender information from a message
 */
export function getSenderInfo(
  message: Message,
  isUserMessage: (message: Message) => boolean,
  currentUser?: { userId: string; displayName?: string; avatarUrl?: string | null }
): MessageGroup['sender'] {
  if (isUserMessage(message)) {
    // For user messages, use message sender or fall back to current user
    return message.sender || {
      userId: currentUser?.userId || 'current-user',
      displayName: currentUser?.displayName || 'You',
      avatarUrl: currentUser?.avatarUrl || null,
    };
  } else {
    // For assistant messages, use message sender or default to Goose
    return message.sender || {
      userId: 'goose',
      displayName: 'Goose',
      avatarUrl: null,
    };
  }
}

/**
 * Groups consecutive messages from the same sender
 */
export function groupMessages(
  messages: Message[],
  isUserMessage: (message: Message) => boolean,
  currentUser?: { userId: string; displayName?: string; avatarUrl?: string | null },
  maxGroupTimeGap: number = 5 * 60 * 1000 // 5 minutes
): GroupedMessage[] {
  if (messages.length === 0) {
    return [];
  }

  const groupedMessages: GroupedMessage[] = [];
  let currentGroup: Message[] = [];
  let currentGroupSender: MessageGroup['sender'] | null = null;
  let currentGroupIsUser = false;

  for (let i = 0; i < messages.length; i++) {
    const message = messages[i];
    const prevMessage = i > 0 ? messages[i - 1] : null;
    const isUser = isUserMessage(message);
    const senderInfo = getSenderInfo(message, isUserMessage, currentUser);
    
    const shouldGroup = shouldGroupMessages(prevMessage, message, isUserMessage, maxGroupTimeGap);

    if (!shouldGroup || currentGroup.length === 0) {
      // Start a new group
      if (currentGroup.length > 0) {
        // Finalize the previous group
        finalizeGroup(currentGroup, currentGroupSender!, currentGroupIsUser, groupedMessages, isUserMessage, currentUser);
      }

      // Start new group
      currentGroup = [message];
      currentGroupSender = senderInfo;
      currentGroupIsUser = isUser;
    } else {
      // Add to current group
      currentGroup.push(message);
    }
  }

  // Finalize the last group
  if (currentGroup.length > 0 && currentGroupSender) {
    finalizeGroup(currentGroup, currentGroupSender, currentGroupIsUser, groupedMessages, isUserMessage, currentUser);
  }

  return groupedMessages;
}

/**
 * Finalizes a group of messages and adds them to the result array
 */
function finalizeGroup(
  groupMessages: Message[],
  senderInfo: MessageGroup['sender'],
  isUser: boolean,
  result: GroupedMessage[],
  isUserMessage: (message: Message) => boolean,
  currentUser?: { userId: string; displayName?: string; avatarUrl?: string | null }
): void {
  const groupInfo: MessageGroup = {
    sender: senderInfo,
    isFromSelf: isUser && (senderInfo.userId === currentUser?.userId || senderInfo.userId === 'current-user'),
    isUser,
    messages: groupMessages,
    timestamp: groupMessages[0].created || 0,
    isFirstInGroup: true,
    isLastInGroup: true,
  };

  groupMessages.forEach((message, index) => {
    const groupedMessage: GroupedMessage = {
      ...message,
      groupInfo: {
        ...groupInfo,
        isFirstInGroup: index === 0,
        isLastInGroup: index === groupMessages.length - 1,
      },
      indexInGroup: index,
      showHeader: index === 0, // Only show header for first message in group
      isGrouped: groupMessages.length > 1 && index > 0, // Messages after the first are "grouped"
    };

    result.push(groupedMessage);
  });
}

/**
 * Checks if a message is the first in its group
 */
export function isFirstInGroup(message: GroupedMessage): boolean {
  return message.showHeader;
}

/**
 * Checks if a message is grouped (not the first in its group)
 */
export function isGroupedMessage(message: GroupedMessage): boolean {
  return message.isGrouped;
}

/**
 * Gets the spacing class for a message based on its group position
 */
export function getMessageSpacing(message: GroupedMessage): string {
  if (message.showHeader) {
    // First message in group gets normal spacing (16px)
    return 'mt-4';
  } else {
    // Grouped messages get very tight spacing - like a single line break (2px)
    return 'mt-0.5';
  }
}
