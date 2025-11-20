import { useState, useCallback, useEffect, useRef } from 'react';
import { PersistenceState, PersistenceActions, SaveOptions, SaveResult, AutoSaveConfig } from '../types/persistence';
import { DocumentPersistence } from '../utils/documentPersistence';

interface UsePersistenceOptions {
  filePath?: string;
  autoSave?: boolean;
  autoSaveInterval?: number;
  onSave?: (result: SaveResult) => void;
  onLoad?: (result: SaveResult & { content?: string }) => void;
  onError?: (error: string) => void;
}

interface UsePersistenceReturn extends PersistenceState, PersistenceActions {
  updateContent: (content: string) => void;
}

export function usePersistence(options: UsePersistenceOptions = {}): UsePersistenceReturn {
  const {
    filePath: initialFilePath,
    autoSave = true,
    autoSaveInterval = 30000, // 30 seconds
    onSave,
    onLoad,
    onError,
  } = options;

  // State
  const [state, setState] = useState<PersistenceState>({
    hasUnsavedChanges: false,
    isAutoSaving: false,
    isSaving: false,
  });

  const [filePath, setFilePath] = useState<string | undefined>(initialFilePath);
  const [lastContent, setLastContent] = useState<string>('');
  
  // Refs for auto-save
  const autoSaveTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const currentContentRef = useRef<string>('');
  const isInitializedRef = useRef(false);

  // Auto-save configuration
  const autoSaveConfig: AutoSaveConfig = {
    enabled: autoSave,
    intervalMs: autoSaveInterval,
    maxBackups: 5,
  };

  // Update current content reference
  const updateContent = useCallback((content: string) => {
    currentContentRef.current = content;
    
    // Check if content has changed
    const hasChanged = content !== lastContent;
    
    if (hasChanged && isInitializedRef.current) {
      setState(prev => ({ ...prev, hasUnsavedChanges: true }));
      
      // Schedule auto-save if enabled
      if (autoSaveConfig.enabled) {
        if (autoSaveTimeoutRef.current) {
          clearTimeout(autoSaveTimeoutRef.current);
        }
        
        autoSaveTimeoutRef.current = setTimeout(() => {
          performAutoSave(content);
        }, autoSaveConfig.intervalMs);
      }
    }
  }, [lastContent, autoSaveConfig.enabled, autoSaveConfig.intervalMs]);

  // Perform auto-save
  const performAutoSave = useCallback(async (content: string) => {
    if (!autoSaveConfig.enabled || state.isSaving) return;

    setState(prev => ({ ...prev, isAutoSaving: true }));

    try {
      await DocumentPersistence.autoSave(content, filePath);
      
      // Also create a backup periodically
      if (Math.random() < 0.1) { // 10% chance to create backup
        await DocumentPersistence.createBackup(content, filePath);
      }
    } catch (error) {
      console.error('Auto-save failed:', error);
      if (onError) {
        onError(error instanceof Error ? error.message : 'Auto-save failed');
      }
    } finally {
      setState(prev => ({ ...prev, isAutoSaving: false }));
    }
  }, [filePath, autoSaveConfig.enabled, state.isSaving, onError]);

  // Save function
  const save = useCallback(async (options: SaveOptions = {}): Promise<SaveResult> => {
    setState(prev => ({ ...prev, isSaving: true, saveError: undefined }));

    try {
      const content = currentContentRef.current;
      const saveOptions = { ...options, filePath: options.filePath || filePath };
      
      const result = await DocumentPersistence.save(content, saveOptions);

      if (result.success) {
        setLastContent(content);
        setFilePath(result.filePath);
        setState(prev => ({
          ...prev,
          hasUnsavedChanges: false,
          lastSaved: new Date(),
        }));

        if (onSave) {
          onSave(result);
        }
      } else {
        setState(prev => ({ ...prev, saveError: result.error }));
        if (onError && result.error) {
          onError(result.error);
        }
      }

      return result;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Save failed';
      setState(prev => ({ ...prev, saveError: errorMessage }));
      if (onError) {
        onError(errorMessage);
      }
      return { success: false, error: errorMessage };
    } finally {
      setState(prev => ({ ...prev, isSaving: false }));
    }
  }, [filePath, onSave, onError]);

  // Save As function
  const saveAs = useCallback(async (options = {}): Promise<SaveResult> => {
    setState(prev => ({ ...prev, isSaving: true, saveError: undefined }));

    try {
      const content = currentContentRef.current;
      const result = await DocumentPersistence.saveAs(content, options);

      if (result.success && !result.cancelled) {
        setLastContent(content);
        setFilePath(result.filePath);
        setState(prev => ({
          ...prev,
          hasUnsavedChanges: false,
          lastSaved: new Date(),
        }));

        if (onSave) {
          onSave(result);
        }
      } else if (result.error) {
        setState(prev => ({ ...prev, saveError: result.error }));
        if (onError) {
          onError(result.error);
        }
      }

      return result;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Save As failed';
      setState(prev => ({ ...prev, saveError: errorMessage }));
      if (onError) {
        onError(errorMessage);
      }
      return { success: false, error: errorMessage };
    } finally {
      setState(prev => ({ ...prev, isSaving: false }));
    }
  }, [onSave, onError]);

  // Load function
  const load = useCallback(async (targetPath?: string): Promise<SaveResult & { content?: string }> => {
    setState(prev => ({ ...prev, isSaving: true, saveError: undefined }));

    try {
      const result = await DocumentPersistence.load(targetPath);

      if (result.success && !result.cancelled && result.content !== undefined) {
        setLastContent(result.content);
        setFilePath(result.filePath);
        currentContentRef.current = result.content;
        setState(prev => ({
          ...prev,
          hasUnsavedChanges: false,
          lastSaved: new Date(),
        }));

        if (onLoad) {
          onLoad(result);
        }
      } else if (result.error) {
        setState(prev => ({ ...prev, saveError: result.error }));
        if (onError) {
          onError(result.error);
        }
      }

      return result;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Load failed';
      setState(prev => ({ ...prev, saveError: errorMessage }));
      if (onError) {
        onError(errorMessage);
      }
      return { success: false, error: errorMessage };
    } finally {
      setState(prev => ({ ...prev, isSaving: false }));
    }
  }, [onLoad, onError]);

  // Auto-save function (exposed for manual triggering)
  const autoSaveFunction = useCallback(async (content: string): Promise<void> => {
    currentContentRef.current = content;
    await performAutoSave(content);
  }, [performAutoSave]);

  // Create backup function
  const createBackup = useCallback(async (content: string): Promise<void> => {
    try {
      await DocumentPersistence.createBackup(content, filePath);
    } catch (error) {
      console.error('Backup creation failed:', error);
      if (onError) {
        onError(error instanceof Error ? error.message : 'Backup creation failed');
      }
    }
  }, [filePath, onError]);

  // Restore backup function
  const restoreBackup = useCallback(async (): Promise<string | null> => {
    try {
      return DocumentPersistence.restoreBackup(filePath);
    } catch (error) {
      console.error('Backup restoration failed:', error);
      if (onError) {
        onError(error instanceof Error ? error.message : 'Backup restoration failed');
      }
      return null;
    }
  }, [filePath, onError]);

  // Utility functions
  const hasUnsavedChanges = useCallback((): boolean => {
    return state.hasUnsavedChanges;
  }, [state.hasUnsavedChanges]);

  const markAsSaved = useCallback((): void => {
    setLastContent(currentContentRef.current);
    setState(prev => ({ ...prev, hasUnsavedChanges: false, lastSaved: new Date() }));
  }, []);

  const markAsChanged = useCallback((): void => {
    setState(prev => ({ ...prev, hasUnsavedChanges: true }));
  }, []);

  // Check for auto-saved content on initialization
  useEffect(() => {
    const checkAutoSavedContent = async () => {
      if (filePath) {
        const autoSaved = DocumentPersistence.getAutoSavedContent(filePath);
        if (autoSaved && autoSaved.content !== currentContentRef.current) {
          // There's auto-saved content that's different from current content
          // This could trigger a recovery dialog in the UI
          console.log('Auto-saved content found for', filePath);
        }
      }
      isInitializedRef.current = true;
    };

    checkAutoSavedContent();
  }, [filePath]);

  // Cleanup auto-save timeout on unmount
  useEffect(() => {
    return () => {
      if (autoSaveTimeoutRef.current) {
        clearTimeout(autoSaveTimeoutRef.current);
      }
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === 's') {
        event.preventDefault();
        if (event.shiftKey) {
          // Ctrl+Shift+S = Save As
          saveAs();
        } else {
          // Ctrl+S = Save
          save();
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [save, saveAs]);

  return {
    // State
    ...state,
    filePath,
    
    // Actions
    save,
    saveAs,
    load,
    autoSave: autoSaveFunction,
    createBackup,
    restoreBackup,
    hasUnsavedChanges,
    markAsSaved,
    markAsChanged,
    updateContent,
  };
}
