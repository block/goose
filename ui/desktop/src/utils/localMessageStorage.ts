interface StoredMessage {
  content: string;
  timestamp: number;
}

const STORAGE_KEY = 'goose-chat-history';
const MAX_MESSAGES = 500;
const EXPIRY_DAYS = 30;

export class LocalMessageStorage {
  private static getStoredMessages(): StoredMessage[] {
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
        this.setStoredMessages(validMessages);
      }

      return validMessages;
    } catch (error) {
      console.error('Error reading message history:', error);
      return [];
    }
  }

  private static setStoredMessages(messages: StoredMessage[]) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(messages));
    } catch (error) {
      console.error('Error saving message history:', error);
    }
  }

  static addMessage(content: string) {
    if (!content.trim()) return;

    const messages = this.getStoredMessages();
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

    this.setStoredMessages(validMessages);
  }

  static getRecentMessages(): string[] {
    return this.getStoredMessages()
      .map((msg) => msg.content)
      .reverse(); // Most recent first
  }

  static clearHistory() {
    localStorage.removeItem(STORAGE_KEY);
  }
}
