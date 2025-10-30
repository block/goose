import React, { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { Bug, FolderKey, ScrollText } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Button } from './ui/button';
import type { View } from '../utils/navigationUtils';
import Stop from './ui/Stop';
import { Attach, Send, Close, Microphone, Action } from './icons';
import { ChatState } from '../types/chatState';
import debounce from 'lodash/debounce';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { DirSwitcher } from './bottom_menu/DirSwitcher';
import ModelsBottomBar from './settings/models/bottom_bar/ModelsBottomBar';
import { BottomMenuModeSelection } from './bottom_menu/BottomMenuModeSelection';
import { AlertType, useAlerts } from './alerts';
import { useConfig } from './ConfigContext';
import { useModelAndProvider } from './ModelAndProviderContext';
import { useWhisper } from '../hooks/useWhisper';
import { WaveformVisualizer } from './WaveformVisualizer';
import { toastError } from '../toasts';
import MentionPopover, { FileItemWithMatch } from './MentionPopover';
import ActionPopover from './ActionPopover';

import { useDictationSettings } from '../hooks/useDictationSettings';
import { useChatContext } from '../contexts/ChatContext';
import { COST_TRACKING_ENABLED, VOICE_DICTATION_ELEVENLABS_ENABLED } from '../updates';
import { CostTracker } from './bottom_menu/CostTracker';
import { DroppedFile, useFileDrop } from '../hooks/useFileDrop';
import { RichChatInput, RichChatInputRef } from './RichChatInput';
import { Recipe } from '../recipe';
import MessageQueue from './MessageQueue';
import { detectInterruption } from '../utils/interruptionDetector';
import { DiagnosticsModal } from './ui/DownloadDiagnostics';
import { Message } from '../api';
import { useCustomCommands } from '../hooks/useCustomCommands';
import { AddCustomCommandModal } from './AddCustomCommandModal';
import { CustomCommand } from '../types/customCommands';

interface QueuedMessage {
  id: string;
  content: string;
  timestamp: number;
}

interface PastedImage {
  id: string;
  dataUrl: string; // For immediate preview
  filePath?: string; // Path on filesystem after saving
  isLoading: boolean;
  error?: string;
}

// Constants for image handling
const MAX_IMAGES_PER_MESSAGE = 5;
const MAX_IMAGE_SIZE_MB = 5;

// Constants for token and tool alerts
const TOKEN_LIMIT_DEFAULT = 128000; // fallback for custom models that the backend doesn't know about
const TOOLS_MAX_SUGGESTED = 60; // max number of tools before we show a warning

// Manual compact trigger message - must match backend constant
const MANUAL_COMPACT_TRIGGER = 'Please compact this conversation';

interface ModelLimit {
  pattern: string;
  context_limit: number;
}

interface ChatInputProps {
  sessionId: string | null;
  handleSubmit: (e: React.FormEvent) => void;
  chatState: ChatState;
  onStop?: () => void;
  commandHistory?: string[]; // Current chat's message history
  initialValue?: string;
  droppedFiles?: DroppedFile[];
  onFilesProcessed?: () => void; // Callback to clear dropped files after processing
  setView: (view: View) => void;
  numTokens?: number;
  inputTokens?: number;
  outputTokens?: number;
  messages?: Message[];
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  disableAnimation?: boolean;
  recipe?: Recipe | null;
  recipeId?: string | null;
  recipeAccepted?: boolean;
  initialPrompt?: string;
  toolCount: number;
  autoSubmit: boolean;
  append?: (message: Message) => void;
  isExtensionsLoading?: boolean;
}

export default function ChatInput({
  sessionId,
  handleSubmit,
  chatState = ChatState.Idle,
  onStop,
  commandHistory = [],
  initialValue = '',
  droppedFiles = [],
  onFilesProcessed,
  setView,
  numTokens,
  inputTokens,
  outputTokens,
  messages = [],
  disableAnimation = false,
  sessionCosts,
  setIsGoosehintsModalOpen,
  recipe,
  recipeId,
  recipeAccepted,
  initialPrompt,
  toolCount,
  autoSubmit = false,
  append: _append,
  isExtensionsLoading = false,
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback
  const [isFocused, setIsFocused] = useState(false);
  const [pastedImages, setPastedImages] = useState<PastedImage[]>([]);

  // Derived state - chatState != Idle means we're in some form of loading state
  const isLoading = chatState !== ChatState.Idle;
  const wasLoadingRef = useRef(isLoading);

  // Queue functionality - ephemeral, only exists in memory for this chat instance
  const [queuedMessages, setQueuedMessages] = useState<QueuedMessage[]>([]);
  const queuePausedRef = useRef(false);
  const editingMessageIdRef = useRef<string | null>(null);
  const [lastInterruption, setLastInterruption] = useState<string | null>(null);

  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef: React.RefObject<HTMLDivElement> = useRef<HTMLDivElement>(
    null
  ) as React.RefObject<HTMLDivElement>;
  const { getProviders, read } = useConfig();
  const { getCurrentModelAndProvider, currentModel, currentProvider } = useModelAndProvider();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);
  const [isTokenLimitLoaded, setIsTokenLimitLoaded] = useState(false);
  const [autoCompactThreshold, setAutoCompactThreshold] = useState<number>(0.8); // Default to 80%

  // Draft functionality - get chat context and global draft context
  // We need to handle the case where ChatInput is used without ChatProvider (e.g., in Hub)
  const chatContext = useChatContext(); // This should always be available now
  const agentIsReady = chatContext === null || chatContext.agentWaitingMessage === null;
  const draftLoadedRef = useRef(false);

  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);

  // Debug logging for draft context
  useEffect(() => {
    // Debug logging removed - draft functionality is working correctly
  }, [chatContext?.contextKey, chatContext?.draft, chatContext]);

  // Save queue state (paused/interrupted) to storage
  useEffect(() => {
    try {
      window.sessionStorage.setItem('goose-queue-paused', JSON.stringify(queuePausedRef.current));
    } catch (error) {
      console.error('Error saving queue pause state:', error);
    }
  }, [queuedMessages]); // Save when queue changes

  useEffect(() => {
    try {
      window.sessionStorage.setItem('goose-queue-interruption', JSON.stringify(lastInterruption));
    } catch (error) {
      console.error('Error saving queue interruption state:', error);
    }
  }, [lastInterruption]);

  // Cleanup effect - save final state on component unmount
  useEffect(() => {
    return () => {
      // Save final queue state when component unmounts
      try {
        window.sessionStorage.setItem('goose-queue-paused', JSON.stringify(queuePausedRef.current));
        window.sessionStorage.setItem('goose-queue-interruption', JSON.stringify(lastInterruption));
      } catch (error) {
        console.error('Error saving queue state on unmount:', error);
      }
    };
  }, [lastInterruption]); // Include lastInterruption in dependency array

  // Queue processing
  useEffect(() => {
    if (wasLoadingRef.current && !isLoading && queuedMessages.length > 0) {
      // After an interruption, we should process the interruption message immediately
      // The queue is only truly paused if there was an interruption AND we want to keep it paused
      const shouldProcessQueue = !queuePausedRef.current || lastInterruption;

      if (shouldProcessQueue) {
        const nextMessage = queuedMessages[0];
        LocalMessageStorage.addMessage(nextMessage.content);
        handleSubmit(
          new CustomEvent('submit', {
            detail: { value: nextMessage.content },
          }) as unknown as React.FormEvent
        );
        setQueuedMessages((prev) => {
          const newQueue = prev.slice(1);
          // If queue becomes empty after processing, clear the paused state
          if (newQueue.length === 0) {
            queuePausedRef.current = false;
            setLastInterruption(null);
          }
          return newQueue;
        });

        // Clear the interruption flag after processing the interruption message
        if (lastInterruption) {
          setLastInterruption(null);
          // Keep the queue paused after sending the interruption message
          // User can manually resume if they want to continue with queued messages
          queuePausedRef.current = true;
        }
      }
    }
    wasLoadingRef.current = isLoading;
  }, [isLoading, queuedMessages, handleSubmit, lastInterruption]);
  const [mentionPopover, setMentionPopover] = useState<{
    isOpen: boolean;
    position: { x: number; y: number };
    query: string;
    mentionStart: number;
    selectedIndex: number;
  }>({
    isOpen: false,
    position: { x: 0, y: 0 },
    query: '',
    mentionStart: -1,
    selectedIndex: 0,
  });
  const [actionPopover, setActionPopover] = useState<{
    isOpen: boolean;
    position: { x: number; y: number };
    selectedIndex: number;
    cursorPosition?: number;
    query?: string;
  }>({
    isOpen: false,
    position: { x: 0, y: 0 },
    selectedIndex: 0,
    cursorPosition: 0,
    query: '',
  });
  const actionPopoverRef = useRef<{
    getDisplayActions: () => any[];
    selectAction: (index: number) => void;
  }>(null);
  
  // Action pills for visual display
  const mentionPopoverRef = useRef<{
    getDisplayFiles: () => FileItemWithMatch[];
    selectFile: (index: number) => void;
  }>(null);

  // Whisper hook for voice dictation
  const {
    isRecording,
    isTranscribing,
    canUseDictation,
    audioContext,
    analyser,
    startRecording,
    stopRecording,
    recordingDuration,
    estimatedSize,
  } = useWhisper({
    onTranscription: (text) => {
      // Append transcribed text to the current input
      const newValue = displayValue.trim() ? `${displayValue.trim()} ${text}` : text;
      setDisplayValue(newValue);
      setValue(newValue);
      textAreaRef.current?.focus();
    },
    onError: (error) => {
      toastError({
        title: 'Dictation Error',
        msg: error.message,
      });
    },
    onSizeWarning: (sizeMB) => {
      toastError({
        title: 'Recording Size Warning',
        msg: `Recording is ${sizeMB.toFixed(1)}MB. Maximum size is 25MB.`,
      });
    },
  });

  // Get dictation settings to check configuration status
  const { settings: dictationSettings } = useDictationSettings();
  
  // Custom commands hook
  const { getCommand, expandCommandPrompt, incrementUsage } = useCustomCommands();

  // Add Custom Command Modal state
  const [isAddCommandModalOpen, setIsAddCommandModalOpen] = useState(false);
  const [customCommands, setCustomCommands] = useState<CustomCommand[]>([]);

  // Load custom commands on mount
  useEffect(() => {
    const loadCustomCommands = () => {
      try {
        const stored = localStorage.getItem('goose-custom-commands');
        if (stored) {
          const parsed = JSON.parse(stored);
          setCustomCommands(parsed.map((cmd: any) => ({
            ...cmd,
            createdAt: new Date(cmd.createdAt),
            updatedAt: new Date(cmd.updatedAt)
          })));
        }
      } catch (error) {
        console.error('Failed to load custom commands:', error);
      }
    };

    loadCustomCommands();
  }, []);

  // Handle modal save
  const handleModalSave = (command: CustomCommand) => {
    const now = new Date();
    const updatedCommands = [...customCommands, { ...command, createdAt: now, updatedAt: now }];
    
    try {
      localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
      setCustomCommands(updatedCommands);
    } catch (error) {
      console.error('Failed to save custom commands:', error);
    }
  };

  // Update internal value when initialValue changes
  useEffect(() => {
    setValue(initialValue);
    setDisplayValue(initialValue);

    // Reset draft loaded flag when initialValue changes
    draftLoadedRef.current = false;

    // Use a functional update to get the current pastedImages
    // and perform cleanup. This avoids needing pastedImages in the deps.
    setPastedImages((currentPastedImages) => {
      currentPastedImages.forEach((img) => {
        if (img.filePath) {
          window.electron.deleteTempFile(img.filePath);
        }
      });
      return []; // Return a new empty array
    });

    // Reset history index when input is cleared
    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
    setHasUserTyped(false);
  }, [initialValue]); // Keep only initialValue as a dependency

  // Handle recipe prompt updates
  useEffect(() => {
    // If recipe is accepted and we have an initial prompt, and no messages yet, and we haven't set it before
    if (recipeAccepted && initialPrompt && messages.length === 0) {
      setDisplayValue(initialPrompt);
      setValue(initialPrompt);
      setTimeout(() => {
        textAreaRef.current?.focus();
      }, 0);
    }
  }, [recipeAccepted, initialPrompt, messages.length]);

  // Draft functionality - load draft if no initial value or recipe
  useEffect(() => {
    // Reset draft loaded flag when context changes
    draftLoadedRef.current = false;
  }, [chatContext?.contextKey]);

  useEffect(() => {
    // Only load draft once and if conditions are met
    if (!initialValue && !recipe && !draftLoadedRef.current && chatContext) {
      const draftText = chatContext.draft || '';

      if (draftText) {
        setDisplayValue(draftText);
        setValue(draftText);
      }

      // Always mark as loaded after checking, regardless of whether we found a draft
      draftLoadedRef.current = true;
    }
  }, [chatContext, initialValue, recipe]);

  // Save draft when user types (debounced)
  const debouncedSaveDraft = useMemo(
    () =>
      debounce((value: string) => {
        if (chatContext && chatContext.setDraft) {
          chatContext.setDraft(value);
        }
      }, 500), // Save draft after 500ms of no typing
    [chatContext]
  );

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  const [hasUserTyped, setHasUserTyped] = useState(false);
  const textAreaRef = useRef<RichChatInputRef>(null);
  const timeoutRefsRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());
  const [didAutoSubmit, setDidAutoSubmit] = useState<boolean>(false);

  // Use shared file drop hook for ChatInput
  const {
    droppedFiles: localDroppedFiles,
    setDroppedFiles: setLocalDroppedFiles,
    handleDrop: handleLocalDrop,
    handleDragOver: handleLocalDragOver,
  } = useFileDrop();

  // Merge local dropped files with parent dropped files
  const allDroppedFiles = useMemo(
    () => [...droppedFiles, ...localDroppedFiles],
    [droppedFiles, localDroppedFiles]
  );

  const handleRemoveDroppedFile = (idToRemove: string) => {
    // Remove from local dropped files
    setLocalDroppedFiles((prev) => prev.filter((file) => file.id !== idToRemove));

    // If it's from parent, call the parent's callback
    if (onFilesProcessed && droppedFiles.some((file) => file.id === idToRemove)) {
      onFilesProcessed();
    }
  };

  const handleRemovePastedImage = (idToRemove: string) => {
    const imageToRemove = pastedImages.find((img) => img.id === idToRemove);
    if (imageToRemove?.filePath) {
      window.electron.deleteTempFile(imageToRemove.filePath);
    }
    setPastedImages((currentImages) => currentImages.filter((img) => img.id !== idToRemove));
  };

  const handleRetryImageSave = async (imageId: string) => {
    const imageToRetry = pastedImages.find((img) => img.id === imageId);
    if (!imageToRetry || !imageToRetry.dataUrl) return;

    // Set the image to loading state
    setPastedImages((prev) =>
      prev.map((img) => (img.id === imageId ? { ...img, isLoading: true, error: undefined } : img))
    );

    try {
      const result = await window.electron.saveDataUrlToTemp(imageToRetry.dataUrl, imageId);
      setPastedImages((prev) =>
        prev.map((img) =>
          img.id === result.id
            ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
            : img
        )
      );
    } catch (err) {
      console.error('Error retrying image save:', err);
      setPastedImages((prev) =>
        prev.map((img) =>
          img.id === imageId
            ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
            : img
        )
      );
    }
  };

  useEffect(() => {

  // Reset textarea height when displayValue is empty
  useEffect(() => {
    if (textAreaRef.current && displayValue === "") {
      // Cancel any pending debounced autosize calls
      debouncedAutosize.cancel?.();
      // Use the RichChatInput resetHeight method
      if (textAreaRef.current.resetHeight) {
        textAreaRef.current.resetHeight();
      }
    }
  }, [displayValue, debouncedAutosize]);

  //   const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
  //     const val = evt.target.value;
  //     const cursorPosition = evt.target.selectionStart;
  // 
  //     setDisplayValue(val); // Update display immediately
  //     updateValue(val); // Update actual value immediately for better responsiveness
  //     debouncedSaveDraft(val); // Save draft with debounce
  //     // Mark that the user has typed something
  //     setHasUserTyped(true);
  // 
  //     // Check for @ mention
  //     checkForMention(val, cursorPosition, evt.target);
  //   };

  const checkForMention = (text: string, cursorPosition: number, textArea: any) => {
    // Find the last @ and / before the cursor
    const beforeCursor = text.slice(0, cursorPosition);
    const lastAtIndex = beforeCursor.lastIndexOf('@');
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    // Determine which symbol is closer to cursor
    const isSlashTrigger = lastSlashIndex > lastAtIndex;
    const triggerIndex = isSlashTrigger ? lastSlashIndex : lastAtIndex;

    if (triggerIndex === -1) {
      // No trigger symbol found, close both popovers
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Check if there's a space between trigger symbol and cursor (which would end the trigger)
    const afterTrigger = beforeCursor.slice(triggerIndex + 1);
    if (afterTrigger.includes(' ') || afterTrigger.includes('\n')) {
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Calculate position for the popover - position it above the chat input
    const textAreaRect = textArea.getBoundingClientRect();

    if (isSlashTrigger) {
      // Open action popover for / trigger
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover({
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        selectedIndex: 0,
        cursorPosition: cursorPosition,
        query: afterTrigger,
      });
    } else {
      // Open mention popover for @ trigger (existing functionality)
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      setMentionPopover((prev) => ({
        ...prev,
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        query: afterTrigger,
        mentionStart: triggerIndex,
        selectedIndex: 0,
      }));
    }
  };

  const handlePaste = async (evt: React.ClipboardEvent<HTMLDivElement>) => {
    const files = Array.from(evt.clipboardData.files || []);
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));

    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    if (pastedImages.length + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setPastedImages((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          dataUrl: '',
          isLoading: false,
          error: `Cannot paste ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed. Currently have ${pastedImages.length}.`,
        },
      ]);

      // Remove the error message after 5 seconds with cleanup tracking
      const timeoutId = setTimeout(() => {
        setPastedImages((prev) => prev.filter((img) => !img.id.startsWith('error-')));
        timeoutRefsRef.current.delete(timeoutId);
      }, 5000);
      timeoutRefsRef.current.add(timeoutId);

      return;
    }

    evt.preventDefault();

    // Process each image file
    const newImages: PastedImage[] = [];

    for (const file of imageFiles) {
      // Check individual file size before processing
      if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
        const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
        newImages.push({
          id: errorId,
          dataUrl: '',
          isLoading: false,
          error: `Image too large (${Math.round(file.size / (1024 * 1024))}MB). Maximum ${MAX_IMAGE_SIZE_MB}MB allowed.`,
        });

        // Remove the error message after 5 seconds with cleanup tracking
        const timeoutId = setTimeout(() => {
          setPastedImages((prev) => prev.filter((img) => img.id !== errorId));
          timeoutRefsRef.current.delete(timeoutId);
        }, 5000);
        timeoutRefsRef.current.add(timeoutId);

        continue;
      }

      const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;

      // Add the image with loading state
      newImages.push({
        id: imageId,
        dataUrl: '',
        isLoading: true,
      });

      // Process the image asynchronously
      const reader = new FileReader();
      reader.onload = async (e) => {
        const dataUrl = e.target?.result as string;
        if (dataUrl) {
          // Update the image with the data URL
          setPastedImages((prev) =>
            prev.map((img) => (img.id === imageId ? { ...img, dataUrl, isLoading: true } : img))
          );

          try {
            const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
            setPastedImages((prev) =>
              prev.map((img) =>
                img.id === result.id
                  ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
                  : img
              )
            );
          } catch (err) {
            console.error('Error saving pasted image:', err);
            setPastedImages((prev) =>
              prev.map((img) =>
                img.id === imageId
                  ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
                  : img
              )
            );
          }
        }
      };
      reader.onerror = () => {
        console.error('Failed to read image file:', file.name);
        setPastedImages((prev) =>
          prev.map((img) =>
            img.id === imageId
              ? { ...img, error: 'Failed to read image file.', isLoading: false }
              : img
          )
        );
      };
      reader.readAsDataURL(file);
    }

    // Add all new images to the existing list
    setPastedImages((prev) => [...prev, ...newImages]);
  };

  // Cleanup debounced functions on unmount
  useEffect(() => {
    return () => {
      debouncedAutosize.cancel?.();
      debouncedSaveDraft.cancel?.();
    };
  }, [debouncedAutosize, debouncedSaveDraft]);

  // Handlers for composition events, which are crucial for proper IME behavior
  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLDivElement>) => {
    const isUp = evt.key === 'ArrowUp';
    const isDown = evt.key === 'ArrowDown';

    // Only handle up/down keys with Cmd/Ctrl modifier
    if ((!isUp && !isDown) || !(evt.metaKey || evt.ctrlKey) || evt.altKey || evt.shiftKey) {
      return;
    }

    // Only prevent history navigation if the user has actively typed something
    // This allows history navigation when text is populated from history or other sources
    // but prevents it when the user is actively editing text
    if (hasUserTyped && displayValue.trim() !== '') {
      return;
    }

    evt.preventDefault();

    // Get global history once to avoid multiple calls
    const globalHistory = LocalMessageStorage.getRecentMessages() || [];

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue || '');
      setIsInGlobalHistory(commandHistory.length === 0);
    }

    // Determine which history we're using
    const currentHistory = isInGlobalHistory ? globalHistory : commandHistory;
    let newIndex = historyIndex;
    let newValue = '';

    // Handle navigation
    if (isUp) {
      // Moving up through history
      if (newIndex < currentHistory.length - 1) {
        // Still have items in current history
        newIndex = historyIndex + 1;
        newValue = currentHistory[newIndex];
      } else if (!isInGlobalHistory && globalHistory.length > 0) {
        // Switch to global history
        setIsInGlobalHistory(true);
        newIndex = 0;
        newValue = globalHistory[newIndex];
      }
    } else {
      // Moving down through history
      if (newIndex > 0) {
        // Still have items in current history
        newIndex = historyIndex - 1;
        newValue = currentHistory[newIndex];
      } else if (isInGlobalHistory && commandHistory.length > 0) {
        // Switch to chat history
        setIsInGlobalHistory(false);
        newIndex = commandHistory.length - 1;
        newValue = commandHistory[newIndex];
      } else {
        // Return to original input
        newIndex = -1;
        newValue = savedInput;
      }
    }

    // Update display if we have a new value
    if (newIndex !== historyIndex) {
      setHistoryIndex(newIndex);
      if (newIndex === -1) {
        setDisplayValue(savedInput || '');
        setValue(savedInput || '');
      } else {
        setDisplayValue(newValue || '');
        setValue(newValue || '');
      }
      // Reset hasUserTyped when we populate from history
      setHasUserTyped(false);
    }
  };

  // Helper function to handle interruption and queue logic when loading
  const handleInterruptionAndQueue = () => {
    if (!isLoading || !displayValue.trim()) {
      return false; // Return false if no action was taken
    }

    const interruptionMatch = detectInterruption(displayValue.trim());

    if (interruptionMatch && interruptionMatch.shouldInterrupt) {
      setLastInterruption(interruptionMatch.matchedText);
      if (onStop) onStop();
      queuePausedRef.current = true;

      // For interruptions, we need to queue the message to be sent after the stop completes
      // rather than trying to send it immediately while the system is still loading
      const interruptionMessage = {
        id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
        content: displayValue.trim(),
        timestamp: Date.now(),
      };

      // Add the interruption message to the front of the queue so it gets sent first
      setQueuedMessages((prev) => [interruptionMessage, ...prev]);

      setDisplayValue('');
      setValue('');
      return true; // Return true if interruption was handled
    }

    const newMessage = {
      id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
      content: displayValue.trim(),
      timestamp: Date.now(),
    };
    setQueuedMessages((prev) => {
      const newQueue = [...prev, newMessage];
      // If adding to an empty queue, reset the paused state
      if (prev.length === 0) {
        queuePausedRef.current = false;
        setLastInterruption(null);
      }
      return newQueue;
    });
    setDisplayValue('');
    setValue('');
    return true; // Return true if message was queued
  };

  // Function to expand custom command pills to their full prompts
  const expandCustomCommandPills = useCallback((text: string): string => {
    // Find all action pills in the format [Action Label]
    const actionPillRegex = /\[([^\]]+)\]/g;
    let expandedText = text;
    
    // Replace each action pill with its corresponding prompt
    expandedText = expandedText.replace(actionPillRegex, (match, label) => {
      // First check if it's a custom command by looking for a command with this label
      const customCommand = getCommand ? (() => {
        try {
          // We need to find the command by label since that's what's stored in the pill
          // This is a bit inefficient but necessary given our current architecture
          const stored = localStorage.getItem('goose-custom-commands');
          if (stored) {
            const commands = JSON.parse(stored);
            return commands.find((cmd: any) => cmd.label === label);
          }
        } catch (error) {
          console.error('Error finding custom command:', error);
        }
        return null;
      })() : null;
      
      if (customCommand) {
        // Increment usage count for the custom command
        if (incrementUsage) {
          incrementUsage(customCommand.id);
        }
        
        // Expand the custom command prompt
        const context = {
          // Add any context variables here in the future
          // filename: currentFileName,
          // selection: selectedText,
          // directory: currentDirectory,
        };
        
        return expandCommandPrompt ? expandCommandPrompt(customCommand, context) : customCommand.prompt;
      }
      
      // If not a custom command, return the original pill (built-in actions)
      return match;
    });
    
    return expandedText;
  }, [getCommand, expandCommandPrompt, incrementUsage]);

  const canSubmit =
    !isLoading &&
    agentIsReady &&
    (displayValue.trim() ||
      pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
      allDroppedFiles.some((file) => !file.error && !file.isLoading));

  const performSubmit = useCallback(
    (text?: string) => {
      const validPastedImageFilesPaths = pastedImages
        .filter((img) => img.filePath && !img.error && !img.isLoading)
        .map((img) => img.filePath as string);
      // Get paths from all dropped files (both parent and local)
      const droppedFilePaths = allDroppedFiles
        .filter((file) => !file.error && !file.isLoading)
        .map((file) => file.path);

      let textToSend = text ?? displayValue.trim();

      // Combine pasted images and dropped files
      const allFilePaths = [...validPastedImageFilesPaths, ...droppedFilePaths];
      if (allFilePaths.length > 0) {
        const pathsString = allFilePaths.join(' ');
        textToSend = textToSend ? `${textToSend} ${pathsString}` : pathsString;
      }

      if (textToSend) {
        if (displayValue.trim()) {
          LocalMessageStorage.addMessage(displayValue);
        } else if (allFilePaths.length > 0) {
          LocalMessageStorage.addMessage(allFilePaths.join(' '));
        }

        handleSubmit(
          new CustomEvent('submit', { detail: { value: textToSend } }) as unknown as React.FormEvent
        );

        // Auto-resume queue after sending a NON-interruption message (if it was paused due to interruption)
        if (
          queuePausedRef.current &&
          lastInterruption &&
          textToSend &&
          !detectInterruption(textToSend)
        ) {
          queuePausedRef.current = false;
          setLastInterruption(null);
        }

        setDisplayValue('');
        setValue('');
        setPastedImages([]);
        setHistoryIndex(-1);
        setSavedInput('');
        setIsInGlobalHistory(false);
        setHasUserTyped(false);

        // Clear draft when message is sent
        if (chatContext && chatContext.clearDraft) {
          chatContext.clearDraft();
        }

        // Clear selected actions when message is sent
        // Actions cleared when message sent

        // Clear both parent and local dropped files after processing
        if (onFilesProcessed && droppedFiles.length > 0) {
          onFilesProcessed();
        }
        if (localDroppedFiles.length > 0) {
          setLocalDroppedFiles([]);
        }
      }
    },
    [
      allDroppedFiles,
      chatContext,
      displayValue,
      droppedFiles.length,
      expandCustomCommandPills,
      handleSubmit,
      lastInterruption,
      localDroppedFiles.length,
      onFilesProcessed,
      pastedImages,
      setLocalDroppedFiles,
    ]
  );

  useEffect(() => {
    if (!!autoSubmit && !didAutoSubmit) {
      setDidAutoSubmit(true);
      performSubmit(initialValue);
    }
  }, [autoSubmit, didAutoSubmit, initialValue, performSubmit]);

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLDivElement>) => {
    // If action popover is open, handle arrow keys and enter
    if (actionPopover.isOpen && actionPopoverRef.current) {
      if (evt.key === 'ArrowDown') {
        evt.preventDefault();
        const displayActions = actionPopoverRef.current.getDisplayActions();
        const maxIndex = Math.max(0, displayActions.length - 1);
        setActionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.min(prev.selectedIndex + 1, maxIndex),
        }));
        return;
      }
      if (evt.key === 'ArrowUp') {
        evt.preventDefault();
        setActionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.max(prev.selectedIndex - 1, 0),
        }));
        return;
      }
      if (evt.key === 'Enter') {
        evt.preventDefault();
        actionPopoverRef.current.selectAction(actionPopover.selectedIndex);
        return;
      }
      if (evt.key === 'Escape') {
        evt.preventDefault();
        setActionPopover((prev) => ({ ...prev, isOpen: false }));
        return;
      }
    }

    // If mention popover is open, handle arrow keys and enter
    if (mentionPopover.isOpen && mentionPopoverRef.current) {
      if (evt.key === 'ArrowDown') {
        evt.preventDefault();
        const displayFiles = mentionPopoverRef.current.getDisplayFiles();
        const maxIndex = Math.max(0, displayFiles.length - 1);
        setMentionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.min(prev.selectedIndex + 1, maxIndex),
        }));
        return;
      }
      if (evt.key === 'ArrowUp') {
        evt.preventDefault();
        setMentionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.max(prev.selectedIndex - 1, 0),
        }));
        return;
      }
      if (evt.key === 'Enter') {
        evt.preventDefault();
        mentionPopoverRef.current.selectFile(mentionPopover.selectedIndex);
        return;
      }
      if (evt.key === 'Escape') {
        evt.preventDefault();
        setMentionPopover((prev) => ({ ...prev, isOpen: false }));
        return;
      }
    }

    // Handle history navigation first
    handleHistoryNavigation(evt);

    if (evt.key === 'Enter') {
      // should not trigger submit on Enter if it's composing (IME input in progress) or shift/alt(option) is pressed
      if (evt.shiftKey || isComposing) {
        // Allow line break for Shift+Enter, or during IME composition
        return;
      }

      if (evt.altKey) {
        const newValue = displayValue + '\n';
        setDisplayValue(newValue);
        setValue(newValue);
        return;
      }

      evt.preventDefault();

      // Handle interruption and queue logic
      if (handleInterruptionAndQueue()) {
        return;
      }

      if (canSubmit) {
        performSubmit();
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const canSubmit =
      !isLoading &&
      agentIsReady &&
      (displayValue.trim() ||
        pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
        allDroppedFiles.some((file) => !file.error && !file.isLoading));
    if (canSubmit) {
      performSubmit();
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      const newValue = displayValue.trim() ? `${displayValue.trim()} ${path}` : path;
      setDisplayValue(newValue);
      setValue(newValue);
      textAreaRef.current?.focus();
    }
  };

  const handleMentionFileSelect = (filePath: string) => {
    console.log('📁 handleMentionFileSelect called with:', filePath);
    
    // Extract just the filename from the full path for the pill
    const fileName = filePath.split('/').pop() || filePath;
    console.log('📁 Extracted filename:', fileName);
    
    // Create @filename format for pill detection
    const mentionText = `@${fileName}`;
    console.log('📁 Creating mention text:', mentionText);
    
    // Replace the @ mention with @filename format
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      mentionPopover.mentionStart + 1 + mentionPopover.query.length
    );
    const newValue = `${beforeMention}${mentionText} ${afterMention}`;
    
    console.log('📁 New value will be:', newValue);

    setDisplayValue(newValue);
    setValue(newValue);
    setMentionPopover((prev) => ({ ...prev, isOpen: false }));
    textAreaRef.current?.focus();

    // Set cursor position after the inserted mention and space
    const newCursorPosition = beforeMention.length + mentionText.length + 1;
    setTimeout(() => {
