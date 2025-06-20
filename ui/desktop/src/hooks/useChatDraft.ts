import { useState, useEffect, useCallback, useMemo } from 'react';
import { debounce } from 'lodash';
import { LocalMessageStorage } from '../utils/localMessageStorage';

/**
 * Custom hook for managing chat input draft state
 * Automatically saves and restores draft text when navigating away from chat
 */
export function useChatDraft(initialValue: string = '') {
  const [draftText, setDraftText] = useState<string>('');
  const [isInitialized, setIsInitialized] = useState(false);

  // Create debounced function using useMemo to avoid ESLint warning
  const debouncedSaveDraft = useMemo(
    () =>
      debounce((text: string) => {
        LocalMessageStorage.saveDraft(text);
      }, 500), // Save draft 500ms after user stops typing
    []
  );

  // Initialize draft from localStorage or initialValue
  useEffect(() => {
    if (isInitialized) return;

    const savedDraft = LocalMessageStorage.getDraft();

    // Priority: saved draft > initialValue > empty string
    const textToUse = savedDraft || initialValue || '';

    setDraftText(textToUse);
    setIsInitialized(true);
  }, [initialValue, isInitialized]);

  // Update draft text and save to localStorage
  const updateDraft = useCallback(
    (text: string) => {
      setDraftText(text);
      debouncedSaveDraft(text);
    },
    [debouncedSaveDraft]
  );

  // Clear draft from both state and localStorage
  const clearDraft = useCallback(() => {
    setDraftText('');
    LocalMessageStorage.clearDraft();
    debouncedSaveDraft.cancel(); // Cancel any pending saves
  }, [debouncedSaveDraft]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      debouncedSaveDraft.cancel();
    };
  }, [debouncedSaveDraft]);

  return {
    draftText,
    updateDraft,
    clearDraft,
    isInitialized,
  };
}
