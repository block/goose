import React, { createContext, useContext, useState, ReactNode } from 'react';

interface DraftContextType {
  getDraft: (contextKey: string) => string;
  setDraft: (contextKey: string, draft: string) => void;
  clearDraft: (contextKey: string) => void;
}

const DraftContext = createContext<DraftContextType | undefined>(undefined);

export const DraftProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  // Store all drafts by contextKey
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const getDraft = (contextKey: string): string => {
    return drafts[contextKey] || '';
  };

  const setDraft = (contextKey: string, draft: string) => {
    setDrafts((prev) => ({ ...prev, [contextKey]: draft }));
  };

  const clearDraft = (contextKey: string) => {
    setDrafts((prev) => {
      const newDrafts = { ...prev };
      delete newDrafts[contextKey];
      return newDrafts;
    });
  };

  return (
    <DraftContext.Provider value={{ getDraft, setDraft, clearDraft }}>
      {children}
    </DraftContext.Provider>
  );
};

export const useDraftContext = (): DraftContextType => {
  const context = useContext(DraftContext);
  if (context === undefined) {
    throw new Error('useDraftContext must be used within a DraftProvider');
  }
  return context;
};
