/**
 * Types for the margin comments system
 */

export interface TextSelection {
  startOffset: number;
  endOffset: number;
  selectedText: string;
}

export interface MessageComment {
  id: string;
  messageId: string;
  selectedText: string;
  startOffset: number;
  endOffset: number;
  position: number; // Y position for margin placement
  content: string;
  author: string;
  timestamp: number;
  parentId?: string; // For threading
  replies: MessageComment[];
  resolved: boolean;
}

export interface CommentState {
  comments: Map<string, MessageComment[]>; // messageId -> comments
  activeSelection: TextSelection | null;
  activeCommentId: string | null;
  isCreatingComment: boolean;
  activePosition: { x: number; y: number } | null; // Position of active selection
  activeMessageId: string | null; // Which message is being commented on
}

export interface CommentActions {
  createComment: (messageId: string, selection: TextSelection, content: string, position?: { x: number; y: number }) => void;
  updateComment: (commentId: string, content: string) => void;
  deleteComment: (commentId: string) => void;
  replyToComment: (parentId: string, content: string) => void;
  resolveComment: (commentId: string, resolved: boolean) => void;
  setActiveSelection: (selection: TextSelection | null, position?: { x: number; y: number }, messageId?: string) => void;
  setActiveComment: (commentId: string | null) => void;
  focusComment: (commentId: string) => void;
}
