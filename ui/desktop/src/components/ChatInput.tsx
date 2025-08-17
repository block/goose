import React, { useRef, useState, useEffect, useMemo } from 'react';
import { FolderKey, ScrollText } from 'lucide-react';
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
import MessageQueue from './MessageQueue';
import { QueueStorage, QueuedMessage as StoredQueuedMessage } from '../utils/queueStorage';

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
  const [queuedMessages, setQueuedMessages] = useState<Array<{id: string, content: string, timestamp: number}>>(() => {
    // Load saved queue on component mount
    return QueueStorage.loadQueue();
  });
  const [isComposing, setIsComposing] = useState(false);

  // Save queue to localStorage whenever it changes

  // Reset processing state and check queue on mount
  useEffect(() => {
    // Reset refs to ensure clean state after navigation
    queuePausedRef.current = false;
    editingMessageIdRef.current = null;
    wasLoadingRef.current = isLoading;
    
    // If we have queued messages and not loading, trigger processing
    if (queuedMessages.length > 0 && !isLoading) {
      // Small delay to ensure component is fully mounted
      const timer = setTimeout(() => {
        if (!isLoading && queuedMessages.length > 0 && !queuePausedRef.current) {
          const nextMessage = queuedMessages[0];
          LocalMessageStorage.addMessage(nextMessage.content);
          handleSubmit(new CustomEvent("submit", { detail: { value: nextMessage.content } }) as unknown as React.FormEvent);
          setQueuedMessages(prev => prev.slice(1));
        }
      }, 500);
      return () => clearTimeout(timer);
    }
  }, []); // Only run on mount
  useEffect(() => {
    QueueStorage.saveQueue(queuedMessages);
  }, [queuedMessages]);

  // File drop functionality
  const { droppedFiles: localDroppedFiles, handleDrop: handleLocalDrop, handleDragOver: handleLocalDragOver } = useFileDrop();
  

  const [mentionPopover, setMentionPopover] = useState<{
    isOpen: boolean;
    position: { x: number; y: number };
    query: string;
    mentionStart: number;
    selectedIndex: number;
  }>({
    isOpen: false,
    position: { x: 0, y: 0 },
    query: "",
    mentionStart: -1,
    selectedIndex: 0,
  });
  const [hasUserTyped, setHasUserTyped] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState("");
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  // Combine parent dropped files with local ones
  // Voice recording functionality
  const {
    isRecording,
    isTranscribing,
    startRecording,
    stopRecording,
    audioContext,
    analyser
  } = useWhisper();
  const allDroppedFiles = [...(droppedFiles || []), ...localDroppedFiles];
  // Derived state - chatState != Idle means we're in some form of loading state
  const isLoading = chatState !== ChatState.Idle;
  const { alerts, addAlert, clearAlerts } = useAlerts();
  const dropdownRef = useRef<HTMLDivElement>(null);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const toolCount = useToolCount();
  const mentionPopoverRef = useRef<{
    getDisplayFiles: () => FileItemWithMatch[];
    selectFile: (index: number) => void;
  }>(null);
  const { isLoadingCompaction, handleManualCompaction } = useChatContextManager();
  const { getProviders, read } = useConfig();
  const { getCurrentModelAndProvider, currentModel, currentProvider } = useModelAndProvider();
  const [tokenLimit, setTokenLimit] = useState<number>(TOKEN_LIMIT_DEFAULT);
  const [isTokenLimitLoaded, setIsTokenLimitLoaded] = useState(false);

  // Draft functionality - get chat context and global draft context
  // We need to handle the case where ChatInput is used without ChatProvider (e.g., in Hub)
  const chatContext = useChatContext(); // This should always be available now
  const draftLoadedRef = useRef(false);

  const maxHeight = 10 * 24;
  const dictationSettings = useDictationSettings();


  const hasSubmittableContent =
    displayValue.trim() ||
    pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
    allDroppedFiles.some((file) => !file.error && !file.isLoading);
  const isAnyImageLoading = pastedImages.some((img) => img.isLoading);
  const isAnyDroppedFileLoading = allDroppedFiles.some((file) => file.isLoading);
  const wasLoadingRef = useRef(isLoading);
  const sendNowTriggeredRef = useRef(false);
  const queuePausedRef = useRef(false);
  const editingMessageIdRef = useRef<string | null>(null);
  const isEditingRef = useRef(false);
  // Debug logging for draft context
  useEffect(() => {
    if (wasLoadingRef.current && !isLoading && queuedMessages.length > 0) {
      // Skip automatic processing if queue is paused by stop command

      // Skip automatic processing only if editing the first message in queue
      if (editingMessageIdRef.current && queuedMessages.length > 0) {
        // Only pause if editing the first message (next to be processed)
        if (editingMessageIdRef.current === queuedMessages[0].id) {
          wasLoadingRef.current = isLoading;
          return;
        }
      }

      if (queuePausedRef.current) {
        wasLoadingRef.current = isLoading;
        return;
      }

      // Skip automatic processing if Send Now was triggered
      if (sendNowTriggeredRef.current) {
        sendNowTriggeredRef.current = false;
        wasLoadingRef.current = isLoading;
        return;
      }
      
      // submit the queued message directly when ready
      const nextMessage = queuedMessages[0];
      LocalMessageStorage.addMessage(nextMessage.content);
      handleSubmit(
        new CustomEvent("submit", {
          detail: { value: nextMessage.content },
        }) as unknown as React.FormEvent
      );

      // Remove the processed message from queue
      setQueuedMessages(prev => prev.slice(1));
    }
    wasLoadingRef.current = isLoading;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLoading]);  // Queue management functions
  const handleRemoveQueuedMessage = (id: string) => {
    // Update both state and storage
    QueueStorage.removeMessage(id);
    setQueuedMessages(prev => prev.filter(msg => msg.id !== id));
  };

  const handleClearQueue = () => {
    // Clear both state and storage
    QueueStorage.clearQueue();
    setQueuedMessages([]);
  };

  const handleStopAndSend = (messageId: string) => {
    // Resume queue processing when using Send Now

    // After Send Now, if there are more messages and not loading, trigger processing
    if (queuedMessages.length > 1 && !isLoading) {
      // Use a simple timeout to trigger next message processing
      setTimeout(() => {
        if (!isLoading && queuedMessages.length > 0 && !queuePausedRef.current) {
          // Manually trigger the next message processing
          const remainingAfterRemoval = queuedMessages.filter(msg => msg.id !== messageId);
          if (remainingAfterRemoval.length > 0) {
            const nextMessage = remainingAfterRemoval[0];
            LocalMessageStorage.addMessage(nextMessage.content);
            handleSubmit(new CustomEvent("submit", { detail: { value: nextMessage.content } }) as unknown as React.FormEvent);
            setQueuedMessages(prev => prev.filter(msg => msg.id !== nextMessage.id));
          }
        }
      }, 1500);
    }

    queuePausedRef.current = false;

    const messageToSend = queuedMessages.find(msg => msg.id === messageId);
    if (!messageToSend) return;
    
    // Set flag to prevent automatic queue processing
    sendNowTriggeredRef.current = true;
    
    if (onStop) onStop();
    setQueuedMessages(prev => prev.filter(msg => msg.id !== messageId));
    LocalMessageStorage.addMessage(messageToSend.content);
    handleSubmit(new CustomEvent("submit", { detail: { value: messageToSend.content } }) as unknown as React.FormEvent);
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
      const pathsString = allFilePaths.join(" ");
      textToSend = textToSend ? `${textToSend} ${pathsString}` : pathsString;
    }

    if (textToSend) {
      if (displayValue.trim()) {
        LocalMessageStorage.addMessage(displayValue);
      } else if (allFilePaths.length > 0) {
        LocalMessageStorage.addMessage(allFilePaths.join(" "));
      }

      handleSubmit(
        new CustomEvent("submit", { detail: { value: textToSend } }) as unknown as React.FormEvent
      );

      setDisplayValue("");
      setValue("");
      setPastedImages([]);
      setHistoryIndex(-1);
      setSavedInput("");
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
        // Note: We need to add setLocalDroppedFiles function
      }
    }
  };
  const handleReorderMessages = (reorderedMessages: QueuedMessage[]) => {

    // Update both state and storage
    QueueStorage.reorderQueue(reorderedMessages);
    setQueuedMessages(reorderedMessages);
  };

  const handleEditMessage = (messageId: string, newContent: string) => {
    // Update both state and storage
    QueueStorage.updateMessage(messageId, newContent);
    setQueuedMessages(prev => 
      prev.map(msg => 
        msg.id === messageId 
          ? { ...msg, content: newContent.trim(), timestamp: Date.now() }
          : msg
      )
    );
  };

  const handleTriggerQueueProcessing = () => {
    // Manually trigger queue processing if not loading and messages exist
    if (!isLoading && queuedMessages.length > 0 && !queuePausedRef.current && !editingMessageIdRef.current) {
      const nextMessage = queuedMessages[0];
      LocalMessageStorage.addMessage(nextMessage.content);
      handleSubmit(new CustomEvent("submit", { detail: { value: nextMessage.content } }) as unknown as React.FormEvent);
      setQueuedMessages(prev => prev.slice(1));
    }
  };

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    const cursorPosition = evt.target.selectionStart;

    setDisplayValue(val); // Update display immediately
    setValue(val); // Update actual value immediately for better responsiveness
    // Mark that the user has typed something
    setHasUserTyped(true);

    // Check for @ mention
    checkForMention(val, cursorPosition, evt.target);
  };

  const checkForMention = (text: string, cursorPosition: number, textArea: HTMLTextAreaElement) => {
    // Find the last @ before the cursor
    const textBeforeCursor = text.substring(0, cursorPosition);
    const atIndex = textBeforeCursor.lastIndexOf("@");
    
    if (atIndex !== -1) {
      const query = textBeforeCursor.substring(atIndex + 1);
      const rect = textArea.getBoundingClientRect();
      setMentionPopover({
        isOpen: true,
        query,
        selectedIndex: 0,
        position: { x: rect.left, y: rect.top - 200 },
        mentionStart: atIndex,
      });
    } else {
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
    }
  };
  // Handlers for composition events, which are crucial for proper IME behavior
  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };
  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const isUp = evt.key === "ArrowUp";
    const isDown = evt.key === "ArrowDown";

    // Only handle up/down keys with Cmd/Ctrl modifier
    if ((!isUp && !isDown) || !(evt.metaKey || evt.ctrlKey) || evt.altKey || evt.shiftKey) {
      return;
    }

    // Only prevent history navigation if the user has actively typed something
    if (hasUserTyped && displayValue.trim() !== "") {
      return;
    }

    evt.preventDefault();

    // Get global history once to avoid multiple calls
    const globalHistory = LocalMessageStorage.getRecentMessages() || [];

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue || "");
      setIsInGlobalHistory(commandHistory.length === 0);
    }

    // Determine which history we're using
    const currentHistory = isInGlobalHistory ? globalHistory : commandHistory;
    let newIndex = historyIndex;
    let newValue = "";

    if (isUp) {
      // Go backwards in history (older messages)
      if (newIndex < currentHistory.length - 1) {
        newIndex++;
        newValue = currentHistory[currentHistory.length - 1 - newIndex];
      }
    } else if (isDown) {
      // Go forwards in history (newer messages)
      if (newIndex > 0) {
        newIndex--;
        newValue = currentHistory[currentHistory.length - 1 - newIndex];
      } else if (newIndex === 0) {
        // Return to saved input
        newIndex = -1;
        newValue = savedInput;
      }
    }

    setHistoryIndex(newIndex);
    setDisplayValue(newValue);
    setValue(newValue);
    // Reset hasUserTyped when we populate from history
    setHasUserTyped(false);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // If mention popover is open, handle arrow keys and enter
    if (mentionPopover.isOpen && mentionPopoverRef.current) {
      if (evt.key === "ArrowDown") {
        evt.preventDefault();
        const displayFiles = mentionPopoverRef.current.getDisplayFiles();
        const maxIndex = Math.max(0, displayFiles.length - 1);
        setMentionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.min(prev.selectedIndex + 1, maxIndex),
        }));
        return;
      }
      if (evt.key === "ArrowUp") {
        evt.preventDefault();
        setMentionPopover((prev) => ({
          ...prev,
          selectedIndex: Math.max(prev.selectedIndex - 1, 0),
        }));
        return;
      }
      if (evt.key === "Enter") {
        evt.preventDefault();
        mentionPopoverRef.current.selectFile(mentionPopover.selectedIndex);
        return;
      }
      if (evt.key === "Escape") {
        evt.preventDefault();
        setMentionPopover((prev) => ({ ...prev, isOpen: false }));
        return;
      }
    }

    // Handle history navigation first
    handleHistoryNavigation(evt);

    if (evt.key === "Enter") {
      // should not trigger submit on Enter if it's composing (IME input in progress) or shift/alt(option) is pressed
      if (evt.shiftKey || isComposing) {
        // Allow line break for Shift+Enter, or during IME composition
        return;
      }

      if (evt.altKey) {
        const newValue = displayValue + "\n";
        setDisplayValue(newValue);
        setValue(newValue);
        return;
      }

      evt.preventDefault();

      // Check for stop commands and interrupt immediately
      if (isLoading && displayValue.trim()) {
        const input = displayValue.trim().toLowerCase();
        if (input === "stop" || input === "wait" || input.startsWith("stop ") || input.startsWith("wait ")) {
          if (onStop) onStop(); // Stop immediately

          // Pause the queue to prevent auto-processing next message
          queuePausedRef.current = true;

          LocalMessageStorage.addMessage(displayValue.trim());
          handleSubmit(new CustomEvent("submit", { detail: { value: displayValue.trim() } }) as unknown as React.FormEvent);
          setDisplayValue("");
          setValue("");
          return;
        }
        
        // If not an interruption, add to queue
        const newMessage = {
          id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
          content: displayValue.trim(),
          timestamp: Date.now()
        };
        setQueuedMessages(prev => [...prev, newMessage]);
        setDisplayValue("");
        setValue("");
        return;
      }

      const canSubmit =
        !isLoading &&
        !isLoadingCompaction &&
        (displayValue.trim() ||
          pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
          allDroppedFiles.some((file) => !file.error && !file.isLoading));
      // Resume queue processing when sending a new message
      queuePausedRef.current = false;

      if (canSubmit) {
        performSubmit();
      }
    }
  };
  const handlePaste = async (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(evt.clipboardData.files || []);
    const imageFiles = files.filter((file) => file.type.startsWith("image/"));

    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    if (pastedImages.length + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setPastedImages((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          dataUrl: "",
          isLoading: false,
          error: `Cannot paste ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed. Currently have ${pastedImages.length}.`,
        },
      ]);

      // Remove the error message after 5 seconds with cleanup tracking
      const timeoutId = setTimeout(() => {
        setPastedImages((prev) => prev.filter((img) => !img.id.startsWith("error-")));
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
          dataUrl: "",
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
        dataUrl: "",
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
            console.error("Error saving pasted image:", err);
            setPastedImages((prev) =>
              prev.map((img) =>
                img.id === imageId
                  ? { ...img, error: "Failed to save image via Electron.", isLoading: false }
                  : img
              )
            );
          }
        }
      };
      reader.onerror = () => {
        console.error("Failed to read image file:", file.name);
        setPastedImages((prev) =>
          prev.map((img) =>
            img.id === imageId
              ? { ...img, error: "Failed to read image file.", isLoading: false }
              : img
          )
        );
      };
      reader.readAsDataURL(file);
    }

    // Add all new images to the existing list
    setPastedImages((prev) => [...prev, ...newImages]);
  };
  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const canSubmit =
      !isLoading &&
      !isLoadingCompaction &&
      (displayValue.trim() ||
        pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
        allDroppedFiles.some((file) => !file.error && !file.isLoading));
      // Resume queue processing when sending a new message
      queuePausedRef.current = false;

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

  return (
    <div      className={`flex flex-col relative h-auto px-4 pb-4 transition-colors ${
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
      {/* Message Queue Display */}
      <MessageQueue
        isPaused={queuePausedRef.current}
        queuedMessages={queuedMessages}
        onRemoveMessage={handleRemoveQueuedMessage}
        onClearQueue={handleClearQueue}
        onStopAndSend={handleStopAndSend}
        onReorderMessages={handleReorderMessages}
        onEditMessage={handleEditMessage}
        onTriggerQueueProcessing={handleTriggerQueueProcessing}
        editingMessageIdRef={editingMessageIdRef}
        className="border-b border-border/30"
      />
      {/* Input row with inline action buttons wrapped in form */}
      <form onSubmit={onFormSubmit} className={`relative flex items-end ${queuedMessages.length > 0 ? "" : "pt-4"}`}>
        <div className="relative flex-1">
          <textarea
            data-testid="chat-input"
            autoFocus
            id="dynamic-textarea"
            placeholder={
              isRecording
                ? ''
                : queuedMessages.length > 0
                  ? `${queuedMessages.length} message${queuedMessages.length > 1 ? 's' : ''} queued`
                  : '⌘↑/⌘↓ to navigate messages'
            }
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
                        : 'bg-slate-600 text-white hover:bg-slate-700 border-slate-600 hover:cursor-pointer'
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
