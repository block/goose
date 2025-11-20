import { useState, useCallback, useMemo, useEffect } from 'react';
import { MessageComment, TextSelection, CommentState, CommentActions } from '../types/comment';
import { CommentStorage } from '../utils/commentStorage';
import { useChatContext } from '../contexts/ChatContext';

export function useComments(sessionId?: string): CommentState & CommentActions {
  const chatContext = useChatContext();
  const effectiveSessionId = sessionId || chatContext?.chat?.sessionId || '';
  
  const [comments, setComments] = useState<Map<string, MessageComment[]>>(() => {
    if (effectiveSessionId) {
      const storedComments = CommentStorage.loadComments(effectiveSessionId);
      const commentsMap = new Map<string, MessageComment[]>();
      
      Object.entries(storedComments).forEach(([messageId, messageComments]) => {
        commentsMap.set(messageId, messageComments);
      });
      
      return commentsMap;
    }
    return new Map();
  });
  const [activeSelection, setActiveSelection] = useState<TextSelection | null>(null);
  const [activeCommentId, setActiveCommentId] = useState<string | null>(null);
  const [isCreatingComment, setIsCreatingComment] = useState(false);
  const [activePosition, setActivePosition] = useState<{ x: number; y: number } | null>(null);
  const [activeMessageId, setActiveMessageId] = useState<string | null>(null);

  // Load comments from storage when sessionId changes
  useEffect(() => {
    if (effectiveSessionId) {
      const storedComments = CommentStorage.loadComments(effectiveSessionId);
      const commentsMap = new Map<string, MessageComment[]>();
      
      Object.entries(storedComments).forEach(([messageId, messageComments]) => {
        commentsMap.set(messageId, messageComments);
      });
      
      setComments(commentsMap);
    } else {
      setComments(new Map());
    }
  }, [effectiveSessionId]);

  // Save comments to storage whenever comments change
  useEffect(() => {
    if (effectiveSessionId) {
      const commentsObject: Record<string, MessageComment[]> = {};
      comments.forEach((messageComments, messageId) => {
        if (messageComments.length > 0) {
          commentsObject[messageId] = messageComments;
        }
      });
      CommentStorage.saveComments(effectiveSessionId, commentsObject);
    }
  }, [effectiveSessionId, comments]);

  const createComment = useCallback((messageId: string, selection: TextSelection, content: string, position?: { x: number; y: number }) => {
    const newComment: MessageComment = {
      id: `comment-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      messageId,
      selectedText: selection.selectedText,
      startOffset: selection.startOffset,
      endOffset: selection.endOffset,
      position: selection.startOffset, // Use start offset as position for now
      content,
      author: 'You', // In the future, this would come from user context
      timestamp: Date.now(),
      replies: [],
      resolved: false,
    };

    setComments(prev => {
      const newMap = new Map(prev);
      const messageComments = newMap.get(messageId) || [];
      newMap.set(messageId, [...messageComments, newComment]);
      return newMap;
    });

    // Clear active selection and creation state
    setActiveSelection(null);
    setActivePosition(null);
    setActiveMessageId(null);
    setIsCreatingComment(false);
    
    // Clear the browser selection
    window.getSelection()?.removeAllRanges();
  }, []);

  const updateComment = useCallback((commentId: string, content: string) => {
    setComments(prev => {
      const newMap = new Map(prev);
      
      // Find and update the comment
      for (const [messageId, messageComments] of newMap.entries()) {
        const updatedComments = updateCommentInList(messageComments, commentId, { content });
        if (updatedComments !== messageComments) {
          newMap.set(messageId, updatedComments);
          break;
        }
      }
      
      return newMap;
    });
  }, []);

  const deleteComment = useCallback((commentId: string) => {
    setComments(prev => {
      const newMap = new Map(prev);
      
      // Find and remove the comment
      for (const [messageId, messageComments] of newMap.entries()) {
        const filteredComments = removeCommentFromList(messageComments, commentId);
        if (filteredComments.length !== messageComments.length) {
          newMap.set(messageId, filteredComments);
          break;
        }
      }
      
      return newMap;
    });
  }, []);

  const replyToComment = useCallback((parentId: string, content: string) => {
    const reply: MessageComment = {
      id: `reply-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      messageId: '', // Will be set when we find the parent
      selectedText: '', // Replies don't have their own selection
      startOffset: 0,
      endOffset: 0,
      position: 0,
      content,
      author: 'You',
      timestamp: Date.now(),
      parentId,
      replies: [],
      resolved: false,
    };

    setComments(prev => {
      const newMap = new Map(prev);
      
      // Find the parent comment and add the reply
      for (const [messageId, messageComments] of newMap.entries()) {
        const updatedComments = addReplyToComment(messageComments, parentId, { ...reply, messageId });
        if (updatedComments !== messageComments) {
          newMap.set(messageId, updatedComments);
          break;
        }
      }
      
      return newMap;
    });
  }, []);

  const resolveComment = useCallback((commentId: string, resolved: boolean) => {
    setComments(prev => {
      const newMap = new Map(prev);
      
      // Find and update the comment
      for (const [messageId, messageComments] of newMap.entries()) {
        const updatedComments = updateCommentInList(messageComments, commentId, { resolved });
        if (updatedComments !== messageComments) {
          newMap.set(messageId, updatedComments);
          break;
        }
      }
      
      return newMap;
    });
  }, []);

  const handleSetActiveSelection = useCallback((selection: TextSelection | null, position?: { x: number; y: number }, messageId?: string) => {
    setActiveSelection(selection);
    setActivePosition(position || null);
    setActiveMessageId(messageId || null);
    setIsCreatingComment(!!selection && !!messageId);
    
    // Only clear browser selection when explicitly cancelling (selection is null but we had an active selection)
    if (!selection && activeSelection) {
      window.getSelection()?.removeAllRanges();
    }
  }, [activeSelection]);

  const handleSetActiveComment = useCallback((commentId: string | null) => {
    setActiveCommentId(commentId);
  }, []);

  const handleFocusComment = useCallback((commentId: string) => {
    setActiveCommentId(commentId);
    // Scroll the comment into view if needed
    setTimeout(() => {
      const commentElement = document.querySelector(`[data-comment-id="${commentId}"]`);
      if (commentElement) {
        commentElement.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }, 100);
  }, []);

  return {
    comments,
    activeSelection,
    activeCommentId,
    isCreatingComment,
    activePosition,
    activeMessageId,
    createComment,
    updateComment,
    deleteComment,
    replyToComment,
    resolveComment,
    setActiveSelection: handleSetActiveSelection,
    setActiveComment: handleSetActiveComment,
    focusComment: handleFocusComment,
  };
}

// Helper functions for nested comment operations
function updateCommentInList(
  comments: MessageComment[], 
  commentId: string, 
  updates: Partial<MessageComment>
): MessageComment[] {
  return comments.map(comment => {
    if (comment.id === commentId) {
      return { ...comment, ...updates };
    }
    if (comment.replies.length > 0) {
      const updatedReplies = updateCommentInList(comment.replies, commentId, updates);
      if (updatedReplies !== comment.replies) {
        return { ...comment, replies: updatedReplies };
      }
    }
    return comment;
  });
}

function removeCommentFromList(comments: MessageComment[], commentId: string): MessageComment[] {
  return comments.filter(comment => {
    if (comment.id === commentId) {
      return false;
    }
    if (comment.replies.length > 0) {
      comment.replies = removeCommentFromList(comment.replies, commentId);
    }
    return true;
  });
}

function addReplyToComment(
  comments: MessageComment[], 
  parentId: string, 
  reply: MessageComment
): MessageComment[] {
  return comments.map(comment => {
    if (comment.id === parentId) {
      return { ...comment, replies: [...comment.replies, reply] };
    }
    if (comment.replies.length > 0) {
      const updatedReplies = addReplyToComment(comment.replies, parentId, reply);
      if (updatedReplies !== comment.replies) {
        return { ...comment, replies: updatedReplies };
      }
    }
    return comment;
  });
}
