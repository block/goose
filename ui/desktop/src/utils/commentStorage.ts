import { MessageComment } from '../types/comment';

interface StoredCommentData {
  comments: Record<string, MessageComment[]>; // messageId -> comments
  timestamp: number;
}

const STORAGE_KEY_PREFIX = 'goose-comments-';
const EXPIRY_DAYS = 90; // Keep comments longer than messages

export class CommentStorage {
  private static getStorageKey(sessionId: string): string {
    return `${STORAGE_KEY_PREFIX}${sessionId}`;
  }

  private static getStoredComments(sessionId: string): Record<string, MessageComment[]> {
    try {
      const storageKey = this.getStorageKey(sessionId);
      const stored = localStorage.getItem(storageKey);
      if (!stored) return {};

      const data = JSON.parse(stored) as StoredCommentData;
      const now = Date.now();
      const expiryTime = now - EXPIRY_DAYS * 24 * 60 * 60 * 1000;

      // Check if data is expired
      if (data.timestamp < expiryTime) {
        localStorage.removeItem(storageKey);
        return {};
      }

      return data.comments || {};
    } catch (error) {
      console.error('Error reading comment storage:', error);
      return {};
    }
  }

  private static setStoredComments(sessionId: string, comments: Record<string, MessageComment[]>) {
    try {
      const storageKey = this.getStorageKey(sessionId);
      const data: StoredCommentData = {
        comments,
        timestamp: Date.now(),
      };
      localStorage.setItem(storageKey, JSON.stringify(data));
    } catch (error) {
      console.error('Error saving comment storage:', error);
    }
  }

  static loadComments(sessionId: string): Record<string, MessageComment[]> {
    if (!sessionId) return {};
    return this.getStoredComments(sessionId);
  }

  static saveComments(sessionId: string, comments: Record<string, MessageComment[]>) {
    if (!sessionId) return;
    this.setStoredComments(sessionId, comments);
  }

  static addComment(sessionId: string, messageId: string, comment: MessageComment) {
    if (!sessionId || !messageId) return;

    const allComments = this.getStoredComments(sessionId);
    if (!allComments[messageId]) {
      allComments[messageId] = [];
    }
    
    allComments[messageId].push(comment);
    this.setStoredComments(sessionId, allComments);
  }

  static updateComment(sessionId: string, messageId: string, commentId: string, updatedComment: MessageComment) {
    if (!sessionId || !messageId || !commentId) return;

    const allComments = this.getStoredComments(sessionId);
    if (!allComments[messageId]) return;

    const commentIndex = allComments[messageId].findIndex(c => c.id === commentId);
    if (commentIndex !== -1) {
      allComments[messageId][commentIndex] = updatedComment;
      this.setStoredComments(sessionId, allComments);
    }
  }

  static deleteComment(sessionId: string, messageId: string, commentId: string) {
    if (!sessionId || !messageId || !commentId) return;

    const allComments = this.getStoredComments(sessionId);
    if (!allComments[messageId]) return;

    // Remove the comment and any replies
    const filterComments = (comments: MessageComment[]): MessageComment[] => {
      return comments.filter(comment => {
        if (comment.id === commentId) return false;
        if (comment.parentId === commentId) return false;
        // Recursively remove replies to deleted replies
        if (comment.parentId && !comments.find(c => c.id === comment.parentId)) return false;
        return true;
      });
    };

    allComments[messageId] = filterComments(allComments[messageId]);
    
    // Remove empty message entries
    if (allComments[messageId].length === 0) {
      delete allComments[messageId];
    }

    this.setStoredComments(sessionId, allComments);
  }

  static clearComments(sessionId: string) {
    if (!sessionId) return;
    const storageKey = this.getStorageKey(sessionId);
    localStorage.removeItem(storageKey);
  }

  static clearAllComments() {
    try {
      const keys = Object.keys(localStorage).filter(key => key.startsWith(STORAGE_KEY_PREFIX));
      keys.forEach(key => localStorage.removeItem(key));
    } catch (error) {
      console.error('Error clearing all comments:', error);
    }
  }
}
