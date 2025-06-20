interface StoredMessage {
  content: string;
  timestamp: number;
}

interface DraftData {
  text: string;
  timestamp: number;
}

const STORAGE_KEY = 'goose-chat-history';
const DRAFT_STORAGE_KEY = 'goose-chat-draft';
const MAX_MESSAGES = 500;
const EXPIRY_DAYS = 30;
const DRAFT_EXPIRY_HOURS = 24; // Drafts expire after 24 hours

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

  // Draft management methods
  static saveDraft(text: string) {
    if (!text.trim()) {
      this.clearDraft();
      return;
    }

    try {
      const draftData: DraftData = {
        text: text.trim(),
        timestamp: Date.now(),
      };
      localStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify(draftData));
    } catch (error) {
      console.error('Error saving draft:', error);
    }
  }

  static getDraft(): string | null {
    try {
      const stored = localStorage.getItem(DRAFT_STORAGE_KEY);
      if (!stored) return null;

      const draftData = JSON.parse(stored) as DraftData;
      const now = Date.now();
      const expiryTime = now - DRAFT_EXPIRY_HOURS * 60 * 60 * 1000;

      // Check if draft has expired
      if (draftData.timestamp < expiryTime) {
        this.clearDraft();
        return null;
      }

      return draftData.text;
    } catch (error) {
      console.error('Error reading draft:', error);
      return null;
    }
  }

  static clearDraft() {
    localStorage.removeItem(DRAFT_STORAGE_KEY);
  }
}
