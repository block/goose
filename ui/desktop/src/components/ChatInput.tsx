import React, { useRef, useState, useEffect, useMemo } from 'react';
import { FolderKey } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send, Close, Microphone } from './icons';
import { ChatState } from '../types/chatState';
import { debounce } from 'lodash';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';
import { DirSwitcher } from './bottom_menu/DirSwitcher';
import ModelsBottomBar from './settings/models/bottom_bar/ModelsBottomBar';
import { BottomMenuModeSelection } from './bottom_menu/BottomMenuModeSelection';
import { ManualCompactButton } from './context_management/ManualCompactButton';
import { AlertType, useAlerts } from './alerts';
import { useToolCount } from './alerts/useToolCount';
import { useConfig } from './ConfigContext';
import { useModelAndProvider } from './ModelAndProviderContext';
import { useWhisper } from '../hooks/useWhisper';
import { WaveformVisualizer } from './WaveformVisualizer';
import { toastError } from '../toasts';
import MentionPopover, { FileItemWithMatch } from './MentionPopover';
import { useDictationSettings } from '../hooks/useDictationSettings';
import { useChatContextManager } from './context_management/ChatContextManager';
import { useChatContext } from '../contexts/ChatContext';
import { COST_TRACKING_ENABLED } from '../updates';
import { CostTracker } from './bottom_menu/CostTracker';
import { DroppedFile, useFileDrop } from '../hooks/useFileDrop';
import { Recipe } from '../recipe';

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
const TOKEN_WARNING_THRESHOLD = 0.8; // warning shows at 80% of the token limit
const TOOLS_MAX_SUGGESTED = 60; // max number of tools before we show a warning

interface ModelLimit {
  pattern: string;
  context_limit: number;
}

interface ChatInputProps {
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
  setMessages: (messages: Message[]) => void;
  sessionCosts?: {
    [key: string]: {
      inputTokens: number;
      outputTokens: number;
      totalCost: number;
    };
  };
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  disableAnimation?: boolean;
  recipeConfig?: Recipe | null;
  recipeAccepted?: boolean;
  initialPrompt?: string;
}

export default function ChatInput({
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
  setMessages,
  disableAnimation = false,
  sessionCosts,
  setIsGoosehintsModalOpen,
  recipeConfig,
  recipeAccepted,
  initialPrompt,
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback
  const [isFocused, setIsFocused] = useState(false);
  const [pastedImages, setPastedImages] = useState<PastedImage[]>([]);

  // Derived state - chatState != Idle means we're in some form of loading state
  const isLoading = chatState !== ChatState.Idle;
  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef = useRef<HTMLDivElement>(null);
  const toolCount = useToolCount();
  const { isLoadingCompaction } = useChatContextManager();
  const { getProviders, read } = useConfig();
  const { getCurrentModelAndProvider, currentModel, currentProvider } = useModelAndProvider();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);
  const [isTokenLimitLoaded, setIsTokenLimitLoaded] = useState(false);

  // Draft functionality - get chat context and global draft context
  // We need to handle the case where ChatInput is used without ChatProvider (e.g., in Hub)
  const chatContext = useChatContext(); // This should always be available now
  const draftLoadedRef = useRef(false);

  // Debug logging for draft context
  useEffect(() => {
    // Debug logging removed - draft functionality is working correctly
  }, [chatContext?.contextKey, chatContext?.draft, chatContext]);
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
    // If recipe is accepted and we have an initial prompt, and no messages yet, set the prompt
    if (recipeAccepted && initialPrompt && messages.length === 0 && !displayValue.trim()) {
      setDisplayValue(initialPrompt);
      setValue(initialPrompt);
      setTimeout(() => {
        textAreaRef.current?.focus();
      }, 0);
    }
  }, [recipeAccepted, initialPrompt, messages.length, displayValue]);

  // Draft functionality - load draft if no initial value or recipe
  useEffect(() => {
    // Reset draft loaded flag when context changes
    draftLoadedRef.current = false;
  }, [chatContext?.contextKey]);

  useEffect(() => {
    // Only load draft once and if conditions are met
    if (!initialValue && !recipeConfig && !draftLoadedRef.current && chatContext) {
      const draftText = chatContext.draft || '';

      if (draftText) {
        setDisplayValue(draftText);
        setValue(draftText);
      }

      // Always mark as loaded after checking, regardless of whether we found a draft
      draftLoadedRef.current = true;
    }
  }, [chatContext, initialValue, recipeConfig]);

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
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const timeoutRefsRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  // Use shared file drop hook for ChatInput
  const {
    droppedFiles: localDroppedFiles,
    setDroppedFiles: setLocalDroppedFiles,
    handleDrop: handleLocalDrop,
    handleDragOver: handleLocalDragOver,
  } = useFileDrop();

  // Merge local dropped files with parent dropped files
  const allDroppedFiles = [...droppedFiles, ...localDroppedFiles];

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
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  // Load model limits from the API
  const getModelLimits = async () => {
    try {
      const response = await read('model-limits', false);
      if (response) {
        // The response is already parsed, no need for JSON.parse
        return response as ModelLimit[];
      }
    } catch (err) {
      console.error('Error fetching model limits:', err);
    }
    return [];
  };

  // Helper function to find model limit using pattern matching
  const findModelLimit = (modelName: string, modelLimits: ModelLimit[]): number | null => {
    if (!modelName) return null;
    const matchingLimit = modelLimits.find((limit) =>
      modelName.toLowerCase().includes(limit.pattern.toLowerCase())
    );
    return matchingLimit ? matchingLimit.context_limit : null;
  };

  // Load providers and get current model's token limit
  const loadProviderDetails = async () => {
    try {
      // Reset token limit loaded state
      setIsTokenLimitLoaded(false);

      // Get current model and provider first to avoid unnecessary provider fetches
      const { model, provider } = await getCurrentModelAndProvider();
      if (!model || !provider) {
        console.log('No model or provider found');
        setIsTokenLimitLoaded(true);
        return;
      }

      const providers = await getProviders(true);

      // Find the provider details for the current provider
      const currentProvider = providers.find((p) => p.name === provider);
      if (currentProvider?.metadata?.known_models) {
        // Find the model's token limit from the backend response
        const modelConfig = currentProvider.metadata.known_models.find((m) => m.name === model);
        if (modelConfig?.context_limit) {
          setTokenLimit(modelConfig.context_limit);
          setIsTokenLimitLoaded(true);
          return;
        }
      }

      // Fallback: Use pattern matching logic if no exact model match was found
      const modelLimit = await getModelLimits();
      const fallbackLimit = findModelLimit(model as string, modelLimit);
      if (fallbackLimit !== null) {
        setTokenLimit(fallbackLimit);
        setIsTokenLimitLoaded(true);
        return;
      }

      // If no match found, use the default model limit
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
      setIsTokenLimitLoaded(true);
    } catch (err) {
      console.error('Error loading providers or token limit:', err);
      // Set default limit on error
      setTokenLimit(TOKEN_LIMIT_DEFAULT);
      setIsTokenLimitLoaded(true);
    }
  };

  // Initial load and refresh when model changes
  useEffect(() => {
    loadProviderDetails();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentModel, currentProvider]);

  // Handle tool count alerts and token usage
  useEffect(() => {
    clearAlerts();

    // Always show token alerts if we have loaded the real token limit and have tokens
    if (isTokenLimitLoaded && tokenLimit && numTokens && numTokens > 0) {
      if (numTokens >= tokenLimit) {
        // Only show error alert when limit reached
        addAlert({
          type: AlertType.Error,
          message: `Token limit reached (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You've reached the model's conversation limit. The session will be saved — copy anything important and start a new one to continue.`,
          autoShow: true, // Auto-show token limit errors
        });
      } else if (numTokens >= tokenLimit * TOKEN_WARNING_THRESHOLD) {
        // Only show warning alert when approaching limit
        addAlert({
          type: AlertType.Warning,
          message: `Approaching token limit (${numTokens.toLocaleString()}/${tokenLimit.toLocaleString()}) \n You're reaching the model's conversation limit. Consider compacting the conversation to continue.`,
          autoShow: true, // Auto-show token limit warnings
        });
      } else {
        // Show info alert only when not in warning/error state
        addAlert({
          type: AlertType.Info,
          message: 'Context window',
          progress: {
            current: numTokens,
            total: tokenLimit,
          },
        });
      }
    } else if (isTokenLimitLoaded && tokenLimit) {
      // Always show context window info even when no tokens are present (start of conversation)
      addAlert({
        type: AlertType.Info,
        message: 'Context window',
        progress: {
          current: 0,
          total: tokenLimit,
        },
      });
    }

    // Add tool count alert if we have the data
    if (toolCount !== null && toolCount > TOOLS_MAX_SUGGESTED) {
      addAlert({
        type: AlertType.Warning,
        message: `Too many tools can degrade performance.\nTool count: ${toolCount} (recommend: ${TOOLS_MAX_SUGGESTED})`,
        action: {
          text: 'View extensions',
          onClick: () => setView('extensions'),
        },
        autoShow: false, // Don't auto-show tool count warnings
      });
    }
    // We intentionally omit setView as it shouldn't trigger a re-render of alerts
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [numTokens, toolCount, tokenLimit, isTokenLimitLoaded, addAlert, clearAlerts]);

  // Cleanup effect for component unmount - prevent memory leaks
  useEffect(() => {
    return () => {
      // Clear any pending timeouts from image processing
      setPastedImages((currentImages) => {
        currentImages.forEach((img) => {
          if (img.filePath) {
            try {
              window.electron.deleteTempFile(img.filePath);
            } catch (error) {
              console.error('Error deleting temp file:', error);
            }
          }
        });
        return [];
      });

      // Clear all tracked timeouts
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const timeouts = timeoutRefsRef.current;
      timeouts.forEach((timeoutId) => {
        window.clearTimeout(timeoutId);
      });
      timeouts.clear();

      // Clear alerts to prevent memory leaks
      clearAlerts();
    };
  }, [clearAlerts]);

  const maxHeight = 10 * 24;

  // Immediate function to update actual value - no debounce for better responsiveness
  const updateValue = React.useCallback((value: string) => {
    setValue(value);
  }, []);

  const debouncedAutosize = useMemo(
    () =>
      debounce((element: HTMLTextAreaElement) => {
        element.style.height = '0px'; // Reset height
        const scrollHeight = element.scrollHeight;
        element.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }, 50),
    [maxHeight]
  );

  useEffect(() => {
    if (textAreaRef.current) {
      debouncedAutosize(textAreaRef.current);
    }
  }, [debouncedAutosize, displayValue]);

  // Reset textarea height when displayValue is empty
  useEffect(() => {
    if (textAreaRef.current && displayValue === '') {
      textAreaRef.current.style.height = 'auto';
    }
  }, [displayValue]);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    const cursorPosition = evt.target.selectionStart;

    setDisplayValue(val); // Update display immediately
    updateValue(val); // Update actual value immediately for better responsiveness
    debouncedSaveDraft(val); // Save draft with debounce
    // Mark that the user has typed something
    setHasUserTyped(true);

    // Check for @ mention
    checkForMention(val, cursorPosition, evt.target);
  };

  const checkForMention = (text: string, cursorPosition: number, textArea: HTMLTextAreaElement) => {
    // Find the last @ before the cursor
    const beforeCursor = text.slice(0, cursorPosition);
    const lastAtIndex = beforeCursor.lastIndexOf('@');

    if (lastAtIndex === -1) {
      // No @ found, close mention popover
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Check if there's a space between @ and cursor (which would end the mention)
    const afterAt = beforeCursor.slice(lastAtIndex + 1);
    if (afterAt.includes(' ') || afterAt.includes('\n')) {
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Calculate position for the popover - position it above the chat input
    const textAreaRect = textArea.getBoundingClientRect();

    setMentionPopover((prev) => ({
      ...prev,
      isOpen: true,
      position: {
        x: textAreaRect.left,
        y: textAreaRect.top, // Position at the top of the textarea
      },
      query: afterAt,
      mentionStart: lastAtIndex,
      selectedIndex: 0, // Reset selection when query changes
      // filteredFiles will be populated by the MentionPopover component
    }));
  };

  const handlePaste = async (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
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

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
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

  const performSubmit = () => {
    const validPastedImageFilesPaths = pastedImages
      .filter((img) => img.filePath && !img.error && !img.isLoading)
      .map((img) => img.filePath as string);

    // Get paths from all dropped files (both parent and local)
    const droppedFilePaths = allDroppedFiles
      .filter((file) => !file.error && !file.isLoading)
      .map((file) => file.path);

    let textToSend = displayValue.trim();

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

      // Clear both parent and local dropped files after processing
      if (onFilesProcessed && droppedFiles.length > 0) {
        onFilesProcessed();
      }
      if (localDroppedFiles.length > 0) {
        setLocalDroppedFiles([]);
      }
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
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
      const canSubmit =
        !isLoading &&
        !isLoadingCompaction &&
        (displayValue.trim() ||
          pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
          allDroppedFiles.some((file) => !file.error && !file.isLoading));
      if (canSubmit) {
        performSubmit();
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const canSubmit =
      !isLoading &&
      !isLoadingCompaction &&
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
    // Replace the @ mention with the file path
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      mentionPopover.mentionStart + 1 + mentionPopover.query.length
    );
    const newValue = `${beforeMention}${filePath}${afterMention}`;

    setDisplayValue(newValue);
    setValue(newValue);
    setMentionPopover((prev) => ({ ...prev, isOpen: false }));
    textAreaRef.current?.focus();

    // Set cursor position after the inserted file path
    setTimeout(() => {
      if (textAreaRef.current) {
        const newCursorPosition = beforeMention.length + filePath.length;
        textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
      }
    }, 0);
  };

  const hasSubmittableContent =
    displayValue.trim() ||
    pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
    allDroppedFiles.some((file) => !file.error && !file.isLoading);
  const isAnyImageLoading = pastedImages.some((img) => img.isLoading);
  const isAnyDroppedFileLoading = allDroppedFiles.some((file) => file.isLoading);

  return (
    <div
      className={`flex flex-col relative h-auto p-4 transition-colors ${
        disableAnimation ? '' : 'page-transition'
      } ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-background-default z-10 rounded-t-2xl`}
      data-drop-zone="true"
      onDrop={handleLocalDrop}
      onDragOver={handleLocalDragOver}
    >
      {/* Input row with inline action buttons wrapped in form */}
      <form onSubmit={onFormSubmit} className="relative flex items-end">
        <div className="relative flex-1">
          <textarea
            data-testid="chat-input"
            autoFocus
            id="dynamic-textarea"
            placeholder={isRecording ? '' : '⌘↑/⌘↓ to navigate messages'}
            value={displayValue}
            onChange={handleChange}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
            ref={textAreaRef}
            rows={1}
            style={{
              maxHeight: `${maxHeight}px`,
              overflowY: 'auto',
              opacity: isRecording ? 0 : 1,
            }}
            className="w-full outline-none border-none focus:ring-0 bg-transparent px-3 pt-3 pb-1.5 pr-20 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
          />
          {isRecording && (
            <div className="absolute inset-0 flex items-center pl-4 pr-20 pt-3 pb-1.5">
              <WaveformVisualizer
                audioContext={audioContext}
                analyser={analyser}
                isRecording={isRecording}
              />
            </div>
          )}
        </div>

        {/* Inline action buttons on the right */}
        <div className="flex items-center gap-1 px-2 relative">
          {/* Microphone button - show if dictation is enabled, disable if not configured */}
          {dictationSettings?.enabled && (
            <>
              {!canUseDictation ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className="inline-flex">
                      <Button
                        type="button"
                        size="sm"
                        shape="round"
                        variant="outline"
                        onClick={() => {}}
                        disabled={true}
                        className="bg-slate-600 text-white cursor-not-allowed opacity-50 border-slate-600 rounded-full px-6 py-2"
                      >
                        <Microphone />
                      </Button>
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {dictationSettings.provider === 'openai'
                      ? 'OpenAI API key is not configured. Set it up in Settings > Models.'
                      : dictationSettings.provider === 'elevenlabs'
                        ? 'ElevenLabs API key is not configured. Set it up in Settings > Chat > Voice Dictation.'
                        : 'Dictation provider is not properly configured.'}
                  </TooltipContent>
                </Tooltip>
              ) : (
                <Button
                  type="button"
                  size="sm"
                  shape="round"
                  variant="outline"
                  onClick={() => {
                    if (isRecording) {
                      stopRecording();
                    } else {
                      startRecording();
                    }
                  }}
                  disabled={isTranscribing}
                  className={`rounded-full px-6 py-2 ${
                    isRecording
                      ? 'bg-red-500 text-white hover:bg-red-600 border-red-500'
                      : isTranscribing
                        ? 'bg-slate-600 text-white cursor-not-allowed animate-pulse border-slate-600'
                        : 'bg-slate-600 text-white hover:bg-slate-700 border-slate-600'
                  }`}
                >
                  <Microphone />
                </Button>
              )}
            </>
          )}

          {/* Send/Stop button */}
          {isLoading ? (
            <Button
              type="button"
              onClick={onStop}
              size="sm"
              shape="round"
              variant="outline"
              className="bg-slate-600 text-white hover:bg-slate-700 border-slate-600 rounded-full px-6 py-2"
            >
              <Stop />
            </Button>
          ) : (
            <Button
              type="submit"
              size="sm"
              shape="round"
              variant="outline"
              disabled={
                !hasSubmittableContent ||
                isAnyImageLoading ||
                isAnyDroppedFileLoading ||
                isRecording ||
                isTranscribing ||
                isLoadingCompaction
              }
              className={`rounded-full px-10 py-2 flex items-center gap-2 ${
                !hasSubmittableContent ||
                isAnyImageLoading ||
                isAnyDroppedFileLoading ||
                isRecording ||
                isTranscribing ||
                isLoadingCompaction
                  ? 'bg-slate-600 text-white cursor-not-allowed opacity-50 border-slate-600'
                  : 'bg-slate-600 text-white hover:bg-slate-700 border-slate-600 hover:cursor-pointer'
              }`}
              title={
                isLoadingCompaction
                  ? 'Summarizing conversation...'
                  : isAnyImageLoading
                    ? 'Waiting for images to save...'
                    : isAnyDroppedFileLoading
                      ? 'Processing dropped files...'
                      : isRecording
                        ? 'Recording...'
                        : isTranscribing
                          ? 'Transcribing...'
                          : 'Send'
              }
            >
              <Send className="w-4 h-4" />
              <span className="text-sm">Send</span>
            </Button>
          )}

          {/* Recording/transcribing status indicator - positioned above the button row */}
          {(isRecording || isTranscribing) && (
            <div className="absolute right-0 -top-8 bg-background-default px-2 py-1 rounded text-xs whitespace-nowrap shadow-md border border-borderSubtle">
              {isTranscribing ? (
                <span className="text-blue-500 flex items-center gap-1">
                  <span className="inline-block w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                  Transcribing...
                </span>
              ) : (
                <span
                  className={`flex items-center gap-2 ${estimatedSize > 20 ? 'text-orange-500' : 'text-textSubtle'}`}
                >
                  <span className="inline-block w-2 h-2 bg-red-500 rounded-full animate-pulse" />
                  {Math.floor(recordingDuration)}s • ~{estimatedSize.toFixed(1)}MB
                  {estimatedSize > 20 && <span className="text-xs">(near 25MB limit)</span>}
                </span>
              )}
            </div>
          )}
        </div>
      </form>

      {/* Combined files and images preview */}
      {(pastedImages.length > 0 || allDroppedFiles.length > 0) && (
        <div className="flex flex-wrap gap-2 p-2 border-t border-borderSubtle">
          {/* Render pasted images first */}
          {pastedImages.map((img) => (
            <div key={img.id} className="relative group w-20 h-20">
              {img.dataUrl && (
                <img
                  src={img.dataUrl}
                  alt={`Pasted image ${img.id}`}
                  className={`w-full h-full object-cover rounded border ${img.error ? 'border-red-500' : 'border-borderStandard'}`}
                />
              )}
              {img.isLoading && (
                <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 rounded">
                  <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white"></div>
                </div>
              )}
              {img.error && !img.isLoading && (
                <div className="absolute inset-0 flex flex-col items-center justify-center bg-black bg-opacity-75 rounded p-1 text-center">
                  <p className="text-red-400 text-[10px] leading-tight break-all mb-1">
                    {img.error.substring(0, 50)}
                  </p>
                  {img.dataUrl && (
                    <Button
                      type="button"
                      onClick={() => handleRetryImageSave(img.id)}
                      title="Retry saving image"
                      variant="outline"
                      size="xs"
                    >
                      Retry
                    </Button>
                  )}
                </div>
              )}
              {!img.isLoading && (
                <Button
                  type="button"
                  shape="round"
                  onClick={() => handleRemovePastedImage(img.id)}
                  className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity z-10"
                  aria-label="Remove image"
                  variant="outline"
                  size="xs"
                >
                  <Close />
                </Button>
              )}
            </div>
          ))}

          {/* Render dropped files after pasted images */}
          {allDroppedFiles.map((file) => (
            <div key={file.id} className="relative group">
              {file.isImage ? (
                // Image preview
                <div className="w-20 h-20">
                  {file.dataUrl && (
                    <img
                      src={file.dataUrl}
                      alt={file.name}
                      className={`w-full h-full object-cover rounded border ${file.error ? 'border-red-500' : 'border-borderStandard'}`}
                    />
                  )}
                  {file.isLoading && (
                    <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 rounded">
                      <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white"></div>
                    </div>
                  )}
                  {file.error && !file.isLoading && (
                    <div className="absolute inset-0 flex flex-col items-center justify-center bg-black bg-opacity-75 rounded p-1 text-center">
                      <p className="text-red-400 text-[10px] leading-tight break-all">
                        {file.error.substring(0, 30)}
                      </p>
                    </div>
                  )}
                </div>
              ) : (
                // File box preview
                <div className="flex items-center gap-2 px-3 py-2 bg-bgSubtle border border-borderStandard rounded-lg min-w-[120px] max-w-[200px]">
                  <div className="flex-shrink-0 w-8 h-8 bg-background-default border border-borderSubtle rounded flex items-center justify-center text-xs font-mono text-textSubtle">
                    {file.name.split('.').pop()?.toUpperCase() || 'FILE'}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-textStandard truncate" title={file.name}>
                      {file.name}
                    </p>
                    <p className="text-xs text-textSubtle">{file.type || 'Unknown type'}</p>
                  </div>
                </div>
              )}
              {!file.isLoading && (
                <Button
                  type="button"
                  shape="round"
                  onClick={() => handleRemoveDroppedFile(file.id)}
                  className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity z-10"
                  aria-label="Remove file"
                  variant="outline"
                  size="xs"
                >
                  <Close />
                </Button>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Secondary actions and controls row below input */}
      <div className="flex flex-row items-center gap-1 p-2 relative">
        {/* Directory path */}
        <DirSwitcher className="mr-0" />
        <div className="w-px h-4 bg-border-default mx-2" />

        {/* Attach button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              onClick={handleFileSelect}
              variant="ghost"
              size="sm"
              className="flex items-center justify-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
            >
              <Attach className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Attach file or directory</TooltipContent>
        </Tooltip>
        <div className="w-px h-4 bg-border-default mx-2" />

        {/* Model selector, mode selector, alerts, summarize button */}
        <div className="flex flex-row items-center">
          {/* Cost Tracker */}
          {COST_TRACKING_ENABLED && (
            <>
              <div className="flex items-center h-full ml-1 mr-1">
                <CostTracker
                  inputTokens={inputTokens}
                  outputTokens={outputTokens}
                  sessionCosts={sessionCosts}
                />
              </div>
            </>
          )}
          <Tooltip>
            <div>
              <ModelsBottomBar
                dropdownRef={dropdownRef}
                setView={setView}
                alerts={alerts}
                recipeConfig={recipeConfig}
                hasMessages={messages.length > 0}
              />
            </div>
          </Tooltip>
          <div className="w-px h-4 bg-border-default mx-2" />
          <BottomMenuModeSelection />
          {messages.length > 0 && (
            <ManualCompactButton
              messages={messages}
              isLoading={isLoading}
              setMessages={setMessages}
            />
          )}
          <div className="w-px h-4 bg-border-default mx-2" />
          <div className="flex items-center h-full">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  onClick={() => setIsGoosehintsModalOpen?.(true)}
                  variant="ghost"
                  size="sm"
                  className="flex items-center justify-center text-text-default/70 hover:text-text-default text-xs cursor-pointer"
                >
                  <FolderKey size={16} />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Configure goosehints</TooltipContent>
            </Tooltip>
          </div>
        </div>

        <MentionPopover
          ref={mentionPopoverRef}
          isOpen={mentionPopover.isOpen}
          onClose={() => setMentionPopover((prev) => ({ ...prev, isOpen: false }))}
          onSelect={handleMentionFileSelect}
          position={mentionPopover.position}
          query={mentionPopover.query}
          selectedIndex={mentionPopover.selectedIndex}
          onSelectedIndexChange={(index) =>
            setMentionPopover((prev) => ({ ...prev, selectedIndex: index }))
          }
        />
      </div>
    </div>
  );
}
