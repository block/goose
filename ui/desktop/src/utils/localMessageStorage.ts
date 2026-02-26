interface StoredMessage {
  content: string;
  timestamp: number;
}

const STORAGE_KEY = 'goose-chat-history';
const MAX_MESSAGES = 500;
const EXPIRY_DAYS = 30;

function getStoredMessages(): StoredMessage[] {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (!stored) return [];

      const messages = JSON.parse(stored) as StoredMessage[];
      const now = Date.now();
      const expiryTime = now - EXPIRY_DAYS * 24 * 60 * 60 * 1000;

      // Filter out expired messages and limit to max count
      const validMessages = messages
        .filter((msg) => msg.timestamp > expiryTime)
        .slice(-MAX_MESSAGES);

      // If we filtered any messages, update storage
      if (validMessages.length !== messages.length) {
        setStoredMessages(validMessages);
      }

      return validMessages;
    } catch (error) {
      console.error('Error reading message history:', error);
      return [];
    }
}

function setStoredMessages(messages: StoredMessage[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(messages));
  } catch (error) {
    console.error('Error saving message history:', error);
  }
}

function addMessage(content: string) {
    if (!content.trim()) return;

    const messages = getStoredMessages();
    const now = Date.now();

    // Don't add duplicate of last message
    if (messages.length > 0 && messages[messages.length - 1].content === content) {
      return;
    }

    messages.push({
      content,
      timestamp: now,
    });

    // Keep only the most recent MAX_MESSAGES
    const validMessages = messages.slice(-MAX_MESSAGES);

    setStoredMessages(validMessages);
}

function getRecentMessages(): string[] {
    return getStoredMessages()
      .map((msg) => msg.content)
      .reverse(); // Most recent first
}

function clearHistory() {
    localStorage.removeItem(STORAGE_KEY);
}

export const LocalMessageStorage = {
  addMessage,
  getRecentMessages,
  clearHistory,
} as const;
