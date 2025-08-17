export interface QueuedMessage {
  id: string;
  content: string;
  timestamp: number;
}

const QUEUE_STORAGE_KEY = 'goose-message-queue';
const MAX_QUEUE_SIZE = 50;
const QUEUE_EXPIRY_HOURS = 24;

export class QueueStorage {
  private static getStoredQueue(): QueuedMessage[] {
    try {
      const stored = localStorage.getItem(QUEUE_STORAGE_KEY);
      if (!stored) return [];

      const queue = JSON.parse(stored) as QueuedMessage[];
      const now = Date.now();
      const expiryTime = now - QUEUE_EXPIRY_HOURS * 60 * 60 * 1000;

      // Filter out expired messages and limit to max count
      const validMessages = queue
        .filter((msg) => msg.timestamp > expiryTime)
        .slice(0, MAX_QUEUE_SIZE);

      // If we filtered any messages, update storage
      if (validMessages.length !== queue.length) {
        this.setStoredQueue(validMessages);
      }

      return validMessages;
    } catch (error) {
      console.error('Error reading message queue:', error);
      return [];
    }
  }

  private static setStoredQueue(queue: QueuedMessage[]) {
    try {
      localStorage.setItem(QUEUE_STORAGE_KEY, JSON.stringify(queue));
    } catch (error) {
      console.error('Error saving message queue:', error);
    }
  }

  static saveQueue(queue: QueuedMessage[]) {
    // Only save non-empty queues, clear storage if empty
    if (queue.length === 0) {
      this.clearQueue();
      return;
    }

    // Limit queue size and remove expired messages
    const now = Date.now();
    const expiryTime = now - QUEUE_EXPIRY_HOURS * 60 * 60 * 1000;
    
    const validQueue = queue
      .filter((msg) => msg.timestamp > expiryTime)
      .slice(0, MAX_QUEUE_SIZE);

    this.setStoredQueue(validQueue);
  }

  static loadQueue(): QueuedMessage[] {
    return this.getStoredQueue();
  }

  static clearQueue() {
    localStorage.removeItem(QUEUE_STORAGE_KEY);
  }

  static addMessage(message: QueuedMessage) {
    const queue = this.getStoredQueue();
    queue.push(message);
    this.saveQueue(queue);
  }

  static removeMessage(messageId: string) {
    const queue = this.getStoredQueue();
    const updatedQueue = queue.filter(msg => msg.id !== messageId);
    this.saveQueue(updatedQueue);
  }

  static updateMessage(messageId: string, newContent: string) {
    const queue = this.getStoredQueue();
    const updatedQueue = queue.map(msg => 
      msg.id === messageId 
        ? { ...msg, content: newContent.trim(), timestamp: Date.now() }
        : msg
    );
    this.saveQueue(updatedQueue);
  }

  static reorderQueue(reorderedQueue: QueuedMessage[]) {
    this.saveQueue(reorderedQueue);
  }
}
