import React, { createContext, useContext, useState, useCallback } from 'react';

interface CommentPanelContextType {
  isPanelOpen: boolean;
  openPanel: () => void;
  closePanel: () => void;
  togglePanel: () => void;
}

const CommentPanelContext = createContext<CommentPanelContextType | null>(null);

export const useCommentPanel = () => {
  const context = useContext(CommentPanelContext);
  if (!context) {
    throw new Error('useCommentPanel must be used within CommentPanelProvider');
  }
  return context;
};

export const useCommentPanelOptional = () => {
  return useContext(CommentPanelContext);
};

interface CommentPanelProviderProps {
  children: React.ReactNode;
}

export const CommentPanelProvider: React.FC<CommentPanelProviderProps> = ({ children }) => {
  const [isPanelOpen, setIsPanelOpen] = useState(false);

  const openPanel = useCallback(() => {
    setIsPanelOpen(true);
  }, []);

  const closePanel = useCallback(() => {
    setIsPanelOpen(false);
  }, []);

  const togglePanel = useCallback(() => {
    setIsPanelOpen(prev => !prev);
  }, []);

  return (
    <CommentPanelContext.Provider value={{ isPanelOpen, openPanel, closePanel, togglePanel }}>
      {children}
    </CommentPanelContext.Provider>
  );
};
