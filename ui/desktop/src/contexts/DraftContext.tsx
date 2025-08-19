import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';

export interface DraftItem {
  id: string;
  contextKey: string;
  content: string;
  timestamp: number;
  contextType: 'hub' | 'pair';
  title?: string;
}

interface DraftContextType {
  getDraft: (contextKey: string) => string;
  setDraft: (
    contextKey: string,
    draft: string,
    contextType?: 'hub' | 'pair',
    title?: string
  ) => void;
  clearDraft: (contextKey: string) => void;
  deleteDraft: (draftId: string) => void;
  deleteAllDrafts: () => void;
  getAllDrafts: () => Record<string, string>;
  getAllDraftItems: () => DraftItem[];
  refreshDrafts: () => void;
}

const DraftContext = createContext<DraftContextType | undefined>(undefined);

// Helper function to load all drafts from localStorage
const loadAllDraftsFromStorage = (): DraftItem[] => {
  const drafts: DraftItem[] = [];

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (key && key.startsWith('draft_')) {
      try {
        const draftData = localStorage.getItem(key);
        if (draftData) {
          const draft = JSON.parse(draftData) as DraftItem;
          drafts.push(draft);
        }
      } catch (error) {
        console.error('Error parsing draft:', error);
      }
    }
  }

  // Sort by timestamp (newest first)
  return drafts.sort((a, b) => b.timestamp - a.timestamp);
};

export const DraftProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  // Store all drafts by contextKey
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [draftItems, setDraftItems] = useState<DraftItem[]>([]);

  // Load drafts from localStorage
  const refreshDrafts = () => {
    console.log('[DraftContext] Loading drafts from localStorage...');
    const loadedDraftItems = loadAllDraftsFromStorage();
    const loadedDrafts: Record<string, string> = {};

    loadedDraftItems.forEach((draft) => {
      if (draft.content && draft.contextKey) {
        loadedDrafts[draft.contextKey] = draft.content;
      }
    });

    setDrafts(loadedDrafts);
    setDraftItems(loadedDraftItems);
  };

  // Load drafts from localStorage on mount
  useEffect(() => {
    refreshDrafts();
  }, []);

  const getDraft = (contextKey: string): string => {
    return drafts[contextKey] || '';
  };

  const setDraft = (
    contextKey: string,
    draft: string,
    contextType: 'hub' | 'pair' = 'hub',
    title?: string
  ) => {
    setDrafts((prev) => ({ ...prev, [contextKey]: draft }));

    // Persist to localStorage
    if (draft.trim()) {
      const draftData: DraftItem = {
        id: contextKey,
        contextKey,
        content: draft,
        timestamp: Date.now(),
        contextType,
        title,
      };
      localStorage.setItem(`draft_${contextKey}`, JSON.stringify(draftData));

      // Update draftItems
      setDraftItems((prev) => {
        const filtered = prev.filter((d) => d.contextKey !== contextKey);
        return [draftData, ...filtered].sort((a, b) => b.timestamp - a.timestamp);
      });
    } else {
      // Remove from localStorage if draft is empty
      localStorage.removeItem(`draft_${contextKey}`);
      // Remove from draftItems
      setDraftItems((prev) => prev.filter((d) => d.contextKey !== contextKey));
    }
  };

  const clearDraft = (contextKey: string) => {
    setDrafts((prev) => {
      const newDrafts = { ...prev };
      delete newDrafts[contextKey];
      return newDrafts;
    });

    // Remove from localStorage
    localStorage.removeItem(`draft_${contextKey}`);

    // Remove from draftItems
    setDraftItems((prev) => prev.filter((d) => d.contextKey !== contextKey));
  };

  const deleteDraft = (draftId: string) => {
    // Remove from localStorage
    localStorage.removeItem(`draft_${draftId}`);

    // Remove from state
    setDrafts((prev) => {
      const newDrafts = { ...prev };
      delete newDrafts[draftId];
      return newDrafts;
    });

    // Remove from draftItems
    setDraftItems((prev) => prev.filter((d) => d.id !== draftId));
  };

  const deleteAllDrafts = () => {
    // Remove all drafts from localStorage
    const keys: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key && key.startsWith('draft_')) {
        keys.push(key);
      }
    }
    keys.forEach((key) => localStorage.removeItem(key));

    // Clear state
    setDrafts({});
    setDraftItems([]);
  };

  const getAllDrafts = (): Record<string, string> => {
    return drafts;
  };

  const getAllDraftItems = (): DraftItem[] => {
    return draftItems;
  };

  return (
    <DraftContext.Provider
      value={{
        getDraft,
        setDraft,
        clearDraft,
        deleteDraft,
        deleteAllDrafts,
        getAllDrafts,
        getAllDraftItems,
        refreshDrafts,
      }}
    >
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

// Utility functions

/**
 * Group drafts by date
 */
export const groupDraftsByDate = (
  drafts: DraftItem[]
): { label: string; drafts: DraftItem[] }[] => {
  const groups: Record<string, DraftItem[]> = {};
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  drafts.forEach((draft) => {
    const draftDate = new Date(draft.timestamp);
    const draftDateOnly = new Date(
      draftDate.getFullYear(),
      draftDate.getMonth(),
      draftDate.getDate()
    );

    let label: string;
    if (draftDateOnly.getTime() === today.getTime()) {
      label = 'Today';
    } else if (draftDateOnly.getTime() === yesterday.getTime()) {
      label = 'Yesterday';
    } else {
      // Format as "Month Day, Year"
      label = draftDate.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    }

    if (!groups[label]) {
      groups[label] = [];
    }
    groups[label].push(draft);
  });

  // Convert to array and sort by date
  const sortedGroups = Object.entries(groups).map(([label, drafts]) => ({
    label,
    drafts,
  }));

  // Sort groups by date (Today first, then Yesterday, then by date)
  sortedGroups.sort((a, b) => {
    if (a.label === 'Today') return -1;
    if (b.label === 'Today') return 1;
    if (a.label === 'Yesterday') return -1;
    if (b.label === 'Yesterday') return 1;
    // For other dates, sort by the first draft's timestamp in each group
    return b.drafts[0].timestamp - a.drafts[0].timestamp;
  });

  return sortedGroups;
};

/**
 * Truncate text to a maximum length
 */
export const truncateText = (text: string, maxLength: number = 100): string => {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + '...';
};
