import React, { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { FolderKey, ScrollText } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Button } from './ui/button';
import type { View } from '../utils/navigationUtils';
import Stop from './ui/Stop';
import { Attach, Send, Close, Microphone, Action } from './icons';
import { ChatState } from '../types/chatState';
import debounce from 'lodash/debounce';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';
import { DirSwitcher } from './bottom_menu/DirSwitcher';
import ModelsBottomBar from './settings/models/bottom_bar/ModelsBottomBar';
import { BottomMenuModeSelection } from './bottom_menu/BottomMenuModeSelection';
import { AlertType, useAlerts } from './alerts';
import { ChatSettingsPopover } from './ChatSettingsPopover';
import { ChatActionsPopover } from './ChatActionsPopover';
import { useConfig } from './ConfigContext';
import { useModelAndProvider } from './ModelAndProviderContext';
import { useWhisper } from '../hooks/useWhisper';
import { WaveformVisualizer } from './WaveformVisualizer';
import { toastError } from '../toasts';
import MentionPopover, { FileItemWithMatch } from './MentionPopover';
import ActionPopover from './ActionPopover';

import { useDictationSettings } from '../hooks/useDictationSettings';
import { useContextManager } from './context_management/ContextManager';
import { useChatContext } from '../contexts/ChatContext';
import { COST_TRACKING_ENABLED } from '../updates';
import { CostTracker } from './bottom_menu/CostTracker';
import { DroppedFile, useFileDrop } from '../hooks/useFileDrop';
import { RichChatInput, RichChatInputRef } from './RichChatInput';
import { Recipe } from '../recipe';
import MessageQueue from './MessageQueue';
import { detectInterruption } from '../utils/interruptionDetector';
import { getApiUrl } from '../config';
import { useCustomCommands } from '../hooks/useCustomCommands';
import { AddCustomCommandModal } from './AddCustomCommandModal';
import { CustomCommand } from '../types/customCommands';
import { useSessionSharing } from '../hooks/useSessionSharing';
import SessionSharing from './collaborative/SessionSharing';
import { CollaborativeButton } from './collaborative';
import EnhancedMentionPopover from './EnhancedMentionPopover';
import { useMatrix } from '../contexts/MatrixContext';
import { sessionMappingService } from '../services/SessionMappingService';
import { useTabContext } from '../contexts/TabContext';

// Force rebuild timestamp: 2025-01-15T01:00:00Z - All .length errors fixed

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
  toolCount: number;
  autoSubmit: boolean;
  append?: (message: Message) => void;
  isExtensionsLoading?: boolean;
  gooseEnabled?: boolean;
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
  setMessages,
  disableAnimation = false,
  sessionCosts,
  setIsGoosehintsModalOpen,
  recipeConfig,
  recipeAccepted,
  initialPrompt,
  toolCount,
  autoSubmit = false,
  append,
  isExtensionsLoading = false,
  gooseEnabled = true,
}: ChatInputProps) {
  // Track the available width for responsive layout
  const [availableWidth, setAvailableWidth] = useState(window.innerWidth);
  const chatInputRef = useRef<HTMLDivElement>(null);

  // Update available width based on container size
  useEffect(() => {
    const updateAvailableWidth = () => {
      if (chatInputRef.current) {
        const containerWidth = chatInputRef.current.offsetWidth;
        setAvailableWidth(containerWidth);
      } else {
        setAvailableWidth(window.innerWidth);
      }
    };

    // Initial measurement
    updateAvailableWidth();

    // Listen for window resize
    const handleResize = () => {
      updateAvailableWidth();
    };

    window.addEventListener('resize', handleResize);
    
    // Use ResizeObserver to detect container size changes (when sidecars are added/removed)
    let resizeObserver: ResizeObserver | null = null;
    if (chatInputRef.current) {
      resizeObserver = new ResizeObserver(updateAvailableWidth);
      resizeObserver.observe(chatInputRef.current);
    }

    return () => {
      window.removeEventListener('resize', handleResize);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };
  }, []);

  // Calculate responsive breakpoint based on available width instead of window width
  const shouldShowIconOnly = availableWidth < 750; // Adjusted from 1050px to 750px for container width
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
  const { isCompacting, handleManualCompaction } = useContextManager();
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
  
  // Enhanced mention popover ref
  const enhancedMentionPopoverRef = useRef<{
    getDisplayItems: () => any[];
    selectItem: (index: number) => void;
  }>(null);

  // Ref for the bottom controls area (where app icons are)
  const bottomControlsRef = useRef<HTMLDivElement>(null);

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

  // BACKEND-CENTRIC APPROACH: Matrix detection based on tab properties AND backend state
  const tabContext = useTabContext();
  
  // Get Matrix room info from TabContext (primary source)
  let tabMatrixRoomId = null;
  let tabMatrixRecipientId = null;
  let isExplicitMatrixTab = false;
  
  if (tabContext) {
    try {
      const activeTabState = tabContext.getActiveTabState();
      if (activeTabState?.tab.type === 'matrix') {
        tabMatrixRoomId = activeTabState.tab.matrixRoomId || null;
        tabMatrixRecipientId = activeTabState.tab.matrixRecipientId || null;
        isExplicitMatrixTab = true;
      }
    } catch (error) {
      console.debug('TabContext not available for Matrix detection');
    }
  }
  
  // STRICT TAB-CENTRIC Matrix detection: ONLY rely on tab context, not backend mapping
  const isNewSession = sessionId?.startsWith('new_') || false;
  const hasTabMatrixRoom = !!(tabMatrixRoomId && tabMatrixRoomId.startsWith('!'));
  
  // Matrix room detection: ONLY explicit Matrix tab properties (no backend fallback)
  const isMatrixRoom = isExplicitMatrixTab && hasTabMatrixRoom;
  
  // Get the actual Matrix room ID for useSessionSharing - ONLY for explicit Matrix tabs
  let actualMatrixRoomId = null;
  if (isExplicitMatrixTab && tabMatrixRoomId) {
    // CRITICAL: Only set Matrix room ID if this is explicitly a Matrix tab
    // This prevents solo tabs from accidentally getting Matrix room IDs
    actualMatrixRoomId = tabMatrixRoomId;
  }
  
  console.log('🔍 ChatInput Matrix room detection (STRICT TAB-CENTRIC):', {
    sessionId,
    isNewSession,
    tabMatrixRoomId,
    tabMatrixRecipientId,
    isExplicitMatrixTab,
    hasTabMatrixRoom,
    isMatrixRoom,
    actualMatrixRoomId,
    tabContextAvailable: !!tabContext,
    detectionMethod: isMatrixRoom ? 'TabContext-Explicit-Only' : 'None',
    // Additional debugging for useSessionSharing
    willPassToUseSessionSharing: {
      sessionId: sessionId, // Always use actual backend session ID
      initialRoomId: actualMatrixRoomId, // Matrix room ID for Matrix operations (only from tab)
      isMatrixMode: isMatrixRoom
    }
  });
  
  // CRITICAL DEBUG: Log what we're actually passing to useSessionSharing
  console.log('🚨 ChatInput: About to call useSessionSharing with:', {
    sessionId: sessionId,
    initialRoomId: actualMatrixRoomId,
    isMatrixRoom: isMatrixRoom,
    shouldSetupMatrixListeners: !!actualMatrixRoomId,
    timestamp: new Date().toISOString()
  });
  
  // Get Matrix context for current user information and sending functionality
  const { currentUser, sendMessage } = useMatrix();
  
  // Session sharing hook - HYBRID: always use backend session ID, pass Matrix room ID separately
  const sessionSharing = useSessionSharing({
    sessionId: sessionId, // Always use actual backend session ID for API calls
    sessionTitle: isMatrixRoom && actualMatrixRoomId ? `Matrix Room ${actualMatrixRoomId.substring(0, 8)}` : `Chat Session ${sessionId?.substring(0, 8) || 'default'}`,
    messages: messages, // Always sync messages
    // CRITICAL FIX: Only provide onMessageSync callback for Matrix tabs
    // This prevents non-Matrix tabs from receiving Matrix messages through the append function
    onMessageSync: isMatrixRoom && actualMatrixRoomId ? (message) => {
      console.log('💬 ChatInput: *** RECEIVED MESSAGE FROM useSessionSharing (MATRIX TAB ONLY) ***', message);
      console.log('💬 ChatInput: Message details:', {
        id: message.id,
        role: message.role,
        content: Array.isArray(message.content) ? message.content[0]?.text?.substring(0, 50) + '...' : 'N/A',
        sender: message.sender?.displayName || message.sender?.userId || 'unknown',
        hasAppendFunction: !!append,
        appendFunctionType: typeof append,
        sessionId: sessionId,
        isMatrixRoom: isMatrixRoom,
        actualMatrixRoomId: actualMatrixRoomId,
        timestamp: new Date().toISOString()
      });
      
      // Only Matrix tabs should receive Matrix messages through onMessageSync
      if (append) {
        console.log('💬 ChatInput: *** CALLING APPEND FUNCTION WITH MESSAGE (MATRIX TAB) ***');
        console.log('💬 ChatInput: *** MESSAGE BEING SENT TO APPEND ***:', JSON.stringify(message, null, 2));
        try {
          const result = append(message);
          console.log('💬 ChatInput: *** APPEND FUNCTION RETURNED ***:', result);
          console.log('💬 ChatInput: *** APPEND SUCCESSFUL - MESSAGE SHOULD APPEAR IN MATRIX TAB ***');
          
          // Also dispatch a custom event to verify message was processed
          window.dispatchEvent(new CustomEvent('matrix-message-received', {
            detail: { message, timestamp: new Date().toISOString() }
          }));
          
        } catch (error) {
          console.error('💬 ChatInput: *** APPEND FUNCTION FAILED ***:', error);
        }
      } else {
        console.warn('⚠️ ChatInput: *** APPEND FUNCTION IS NOT AVAILABLE! ***');
      }
    } : undefined, // Non-Matrix tabs get undefined, so they won't receive Matrix messages
    initialRoomId: actualMatrixRoomId, // FIXED: Always pass Matrix room ID if available, regardless of isMatrixRoom flag
    onParticipantJoin: (participant) => {
      console.log('👥 Participant joined session:', participant);
    },
    onParticipantLeave: (userId) => {
      console.log('👋 Participant left session:', userId);
    },
  });

  // Track which messages have been sent to Matrix to prevent duplicates
  const sentToMatrixRef = useRef<Set<string>>(new Set());

  // Listen for AI responses to sync to Matrix or collaborative sessions
  // FIXED: Robust null checking to prevent "Cannot read properties of undefined (reading 'length')" error
  // Updated: Fixed all commandHistory.length accesses with safeCommandHistory
  // Final fix: All .length accesses now properly null-checked
  // CRITICAL FIX: Only send to Matrix when streaming is complete (chatState is Idle)
  // CRITICAL FIX 2: Track sent messages to prevent duplicate sends
  useEffect(() => {
    if (!messages || !Array.isArray(messages) || messages.length === 0) return;

    const lastMessage = messages[messages.length - 1];
    
    // Check if the last message is an AI response (assistant role) and not already synced
    // Also check if it's not from Matrix (to prevent sync loops)
    if (lastMessage && 
        lastMessage.role === 'assistant' && 
        !lastMessage.id?.startsWith('shared-') && 
        !lastMessage.id?.startsWith('matrix-') &&
        !lastMessage.sender && // Messages from Matrix have sender info, local AI responses don't
        !lastMessage.metadata?.isFromMatrix && // Additional check for Matrix-originated messages
        !lastMessage.metadata?.isFromCollaborator) { // Additional check for collaborator messages
      
      // CRITICAL: Only send to Matrix when streaming is complete (chatState is Idle)
      // This prevents sending partial messages during streaming
      if (chatState !== ChatState.Idle) {
        console.log('🚫 Skipping Matrix sync - streaming in progress (chatState:', chatState, ')');
        return;
      }
      
      // CRITICAL: Check if we've already sent this message to Matrix
      if (sentToMatrixRef.current.has(lastMessage.id)) {
        console.log('🚫 Skipping Matrix sync - message already sent:', lastMessage.id);
        return;
      }
      
      // Extract text content from the message - with robust null checking
      let textContent = '';
      if (lastMessage.content && Array.isArray(lastMessage.content)) {
        textContent = lastMessage.content
          .filter(c => c && c.type === 'text')
          .map(c => c.text || '')
          .join('');
      } else if (typeof lastMessage.content === 'string') {
        textContent = lastMessage.content;
      }
      
      if (!textContent.trim()) return;
      
      // Handle Matrix rooms: send AI response directly to Matrix with goose-session-message format
      if (isMatrixRoom && actualMatrixRoomId && sendMessage) {
        console.log('🤖 Sending COMPLETE AI response to Matrix room:', actualMatrixRoomId, '(chatState:', chatState, ', messageId:', lastMessage.id, ')');
        
        // Mark as sent BEFORE sending to prevent race conditions
        sentToMatrixRef.current.add(lastMessage.id);
        
        // Format as goose-session-message so it can be properly parsed by other clients
        const sessionMessage = {
          sessionId: sessionId || actualMatrixRoomId,
          role: 'assistant',
          content: textContent,
          timestamp: Date.now(),
        };
        const formattedMessage = `goose-session-message:${JSON.stringify(sessionMessage)}`;
        
        sendMessage(actualMatrixRoomId, formattedMessage).then(() => {
          console.log('✅ Successfully sent COMPLETE AI response to Matrix room (messageId:', lastMessage.id, ')');
        }).catch((error) => {
          console.error('❌ Failed to send AI response to Matrix room:', error);
          // Remove from sent set if it failed so we can retry
          sentToMatrixRef.current.delete(lastMessage.id);
        });
      }
      // Handle non-Matrix collaborative sessions: sync through sessionSharing
      else if (sessionSharing.isSessionActive && !isMatrixRoom) {
        console.log('🤖 Syncing COMPLETE AI response to collaborative session (non-Matrix):', lastMessage);
        sessionSharing.syncMessage({
          id: lastMessage.id || `ai-${Date.now()}`,
          role: 'assistant',
          content: textContent,
          timestamp: new Date().toISOString(),
        });
      }
    }
  }, [messages, sessionSharing, isMatrixRoom, actualMatrixRoomId, sendMessage, chatState]);



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
    if (recipeAccepted && initialPrompt && messages && Array.isArray(messages) && messages.length === 0) {
      setDisplayValue(initialPrompt);
      setValue(initialPrompt);
      setTimeout(() => {
        textAreaRef.current?.focus();
      }, 0);
    }
  }, [recipeAccepted, initialPrompt, messages]);

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

  // Load auto-compact threshold
  const loadAutoCompactThreshold = useCallback(async () => {
    try {
      const secretKey = await window.electron.getSecretKey();
      const response = await fetch(getApiUrl('/config/read'), {
        method: 'POST',
        headers: {
          'X-Secret-Key': secretKey,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          key: 'GOOSE_AUTO_COMPACT_THRESHOLD',
          is_secret: false,
        }),
      });
      if (response.ok) {
        const data = await response.json();
        console.log('Loaded auto-compact threshold from config:', data);
        if (data !== undefined && data !== null) {
          setAutoCompactThreshold(data);
          console.log('Set auto-compact threshold to:', data);
        }
      } else {
        console.error('Failed to fetch auto-compact threshold, status:', response.status);
      }
    } catch (err) {
      console.error('Error fetching auto-compact threshold:', err);
    }
  }, []);

  useEffect(() => {
    loadAutoCompactThreshold();
  }, [loadAutoCompactThreshold]);

  // Listen for threshold change events from AlertBox
  useEffect(() => {
    const handleThresholdChange = (event: CustomEvent<{ threshold: number }>) => {
      setAutoCompactThreshold(event.detail.threshold);
    };

    // Type assertion to handle the mismatch between CustomEvent and EventListener
    const eventListener = handleThresholdChange as (event: globalThis.Event) => void;
    window.addEventListener('autoCompactThresholdChanged', eventListener);

    return () => {
      window.removeEventListener('autoCompactThresholdChanged', eventListener);
    };
  }, []);

  // Handle tool count alerts and token usage
  useEffect(() => {
    clearAlerts();

    // Show alert when either there is registered token usage, or we know the limit
    if ((numTokens && numTokens > 0) || (isTokenLimitLoaded && tokenLimit)) {
      // in these conditions we want it to be present but disabled
      const compactButtonDisabled = !numTokens || isCompacting;

      addAlert({
        type: AlertType.Info,
        message: 'Context window',
        progress: {
          current: numTokens || 0,
          total: tokenLimit,
        },
        showCompactButton: true,
        compactButtonDisabled,
        onCompact: () => {
          // Hide the alert popup by dispatching a custom event that the popover can listen to
          // Importantly, this leaves the alert so the dot still shows up, but hides the popover
          window.dispatchEvent(new CustomEvent('hide-alert-popover'));
          handleManualCompaction(messages, setMessages, append);
        },
        compactIcon: <ScrollText size={12} />,
        autoCompactThreshold: autoCompactThreshold,
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
  }, [
    numTokens,
    toolCount,
    tokenLimit,
    isTokenLimitLoaded,
    addAlert,
    isCompacting,
    clearAlerts,
    autoCompactThreshold,
  ]);

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
      debounce((element: HTMLElement) => {
        element.style.height = '0px'; // Reset height
        const scrollHeight = element.scrollHeight;
        element.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }, 50),
    [maxHeight]
  );

  useEffect(() => {
    if (textAreaRef.current) {
      const element = (textAreaRef.current as any).contentRef?.current; if (element) { debouncedAutosize(element); }
    }
  }, [debouncedAutosize, displayValue]);

  // Reset textarea height when displayValue is empty
  useEffect(() => {
    if (textAreaRef.current && displayValue === '') {
      const element = (textAreaRef.current as any)?.contentRef?.current; if (element && element.style) { element.style.height = 'auto'; }
    }
  }, [displayValue]);

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
    console.log('🔍 checkForMention called:', { text: text.substring(0, 50) + '...', cursorPosition, textAreaType: typeof textArea });
    
    // Find the last @ and / before the cursor
    const beforeCursor = text.slice(0, cursorPosition);
    const lastAtIndex = beforeCursor.lastIndexOf('@');
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    console.log('🔍 Found triggers:', { lastAtIndex, lastSlashIndex, beforeCursor: beforeCursor.substring(Math.max(0, beforeCursor.length - 20)) });
    
    // Determine which symbol is closer to cursor
    const isSlashTrigger = lastSlashIndex > lastAtIndex;
    const triggerIndex = isSlashTrigger ? lastSlashIndex : lastAtIndex;

    if (triggerIndex === -1) {
      // No trigger symbol found, close both popovers
      console.log('🔍 No trigger found, closing popovers');
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Check if there's a space between trigger symbol and cursor (which would end the trigger)
    const afterTrigger = beforeCursor.slice(triggerIndex + 1);
    if (afterTrigger.includes(' ') || afterTrigger.includes('\n')) {
      console.log('🔍 Space found after trigger, closing popovers');
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }

    // Calculate position for the popover - position it above the chat input
    let chatInputRect;
    try {
      // Get the chat input container's bounding rect to position the popover above it
      chatInputRect = chatInputRef.current?.getBoundingClientRect?.() || new DOMRect();
      console.log('🔍 Got chatInputRect:', { x: chatInputRect.left, y: chatInputRect.top, width: chatInputRect.width, height: chatInputRect.height });
    } catch (error) {
      console.error('🔍 Error getting chat input bounding rect:', error);
      chatInputRect = new DOMRect();
    }

    if (isSlashTrigger) {
      // Open action popover for / trigger - position above the chat input
      console.log('🔍 Opening action popover for /', { query: afterTrigger, position: { x: chatInputRect.left, y: chatInputRect.top } });
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setActionPopover({
        isOpen: true,
        position: {
          x: chatInputRect.left,
          y: chatInputRect.top,
        },
        selectedIndex: 0,
        cursorPosition: cursorPosition,
        query: afterTrigger,
      });
    } else {
      // Open mention popover for @ trigger - position above the chat input
      console.log('🔍 Opening mention popover for @', { 
        query: afterTrigger, 
        mentionStart: triggerIndex, 
        position: { x: chatInputRect.left, y: chatInputRect.top },
        chatInputRect: chatInputRect 
      });
      setActionPopover((prev) => ({ ...prev, isOpen: false }));
      const newMentionPopover = {
        ...mentionPopover,
        isOpen: true,
        position: {
          x: chatInputRect.left,
          y: chatInputRect.top,
        },
        query: afterTrigger,
        mentionStart: triggerIndex,
        selectedIndex: 0,
      };
      console.log('🔍 Setting mention popover state:', newMentionPopover);
      setMentionPopover(newMentionPopover);
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
      // Determine which history we're using - ensure commandHistory is always an array
      const safeCommandHistory = commandHistory || [];
      setIsInGlobalHistory(safeCommandHistory.length === 0);
    }

    // Determine which history we're using - ensure commandHistory is always an array
    const safeCommandHistory = commandHistory || [];
    const currentHistory = isInGlobalHistory ? globalHistory : safeCommandHistory;
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
      } else if (isInGlobalHistory && safeCommandHistory.length > 0) {
        // Switch to chat history
        setIsInGlobalHistory(false);
        newIndex = safeCommandHistory.length - 1;
        newValue = safeCommandHistory[newIndex];
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
    !isCompacting &&
    agentIsReady &&
    (displayValue.trim() ||
      pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
      allDroppedFiles.some((file) => !file.error && !file.isLoading));

  const performSubmit = useCallback(
    async (text?: string) => {
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

        // CRITICAL: Handle message sending based on room type
        if (sessionSharing.isSessionActive && !isMatrixRoom) {
          // Non-Matrix collaborative sessions: sync through sessionSharing
          console.log('🔄 Syncing user message to collaborative session (non-Matrix):', textToSend);
          sessionSharing.syncMessage({
            id: Date.now().toString(),
            role: 'user',
            content: textToSend,
            timestamp: new Date().toISOString(),
          });
        } else if (isMatrixRoom && actualMatrixRoomId && sendMessage) {
          // Matrix rooms: send directly to Matrix
          console.log('📤 Sending message to Matrix room:', actualMatrixRoomId);
          try {
            await sendMessage(actualMatrixRoomId, textToSend);
            console.log('✅ Successfully sent message to Matrix room');
          } catch (error) {
            console.error('❌ Failed to send message to Matrix room:', error);
            // Still proceed with the normal handleSubmit to show the message locally
          }
        } else if (isMatrixRoom) {
          console.log('⚠️ Matrix room detected but missing actualMatrixRoomId or sendMessage function');
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
      sessionSharing,
      setLocalDroppedFiles,
      isMatrixRoom,
      actualMatrixRoomId,
      sendMessage,
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
    if (mentionPopover.isOpen && enhancedMentionPopoverRef.current) {
      if (evt.key === 'ArrowDown') {
        evt.preventDefault();
        const displayItems = enhancedMentionPopoverRef.current.getDisplayItems();
        const maxIndex = Math.max(0, displayItems.length - 1);
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
        enhancedMentionPopoverRef.current.selectItem(mentionPopover.selectedIndex);
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
      !isCompacting &&
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
      if (textAreaRef.current) {
        textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
        textAreaRef.current.focus();
      }
    }, 0);
  };

  const handleFriendInvite = async (friendUserId: string) => {
    console.log('👥 handleFriendInvite called with:', friendUserId);
    
    // Handle special cases for goose commands - these should NOT trigger Matrix invitations
    if (friendUserId.startsWith('goose')) {
      console.log('🦆 Handling @goose command:', friendUserId);
      
      // Replace the @ mention with the full goose command
      const mentionText = `@${friendUserId}`;
      const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
      const afterMention = displayValue.slice(
        mentionPopover.mentionStart + 1 + mentionPopover.query.length
      );
      const newValue = `${beforeMention}${mentionText} ${afterMention}`;

      setDisplayValue(newValue);
      setValue(newValue);
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      textAreaRef.current?.focus();

      // Set cursor position after the inserted mention and space
      const newCursorPosition = beforeMention.length + mentionText.length + 1;
      setTimeout(() => {
        if (textAreaRef.current) {
          textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
          textAreaRef.current.focus();
        }
      }, 0);
      
      console.log('✅ Successfully added @goose command:', friendUserId);
      return;
    }
    
    try {
      // Only attempt Matrix invitation for actual user IDs (not goose commands)
      await sessionSharing.inviteToSession(friendUserId);
      
      // Replace the @ mention with a friend mention format
      const friendName = friendUserId.split(':')[0].substring(1); // Extract username from Matrix ID
      const mentionText = `@${friendName}`;
      
      const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
      const afterMention = displayValue.slice(
        mentionPopover.mentionStart + 1 + mentionPopover.query.length
      );
      const newValue = `${beforeMention}${mentionText} ${afterMention}`;

      setDisplayValue(newValue);
      setValue(newValue);
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      textAreaRef.current?.focus();

      // Set cursor position after the inserted mention and space
      const newCursorPosition = beforeMention.length + mentionText.length + 1;
      setTimeout(() => {
        if (textAreaRef.current) {
          textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
          textAreaRef.current.focus();
        }
      }, 0);
      
      console.log('✅ Successfully invited friend and updated UI');
    } catch (error) {
      console.error('❌ Failed to invite friend:', error);
      // Keep the popover open so user can try again
      // You might want to show a toast notification here
      toastError({
        title: 'Invitation Failed',
        msg: error instanceof Error ? error.message : 'Failed to invite friend to session',
      });
    }
  };

  const handleActionButtonClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    const buttonRect = event.currentTarget.getBoundingClientRect();
    
    // Get the current cursor position from the RichChatInput
    const currentCursorPosition = textAreaRef.current?.getBoundingClientRect ? displayValue.length : 0;
    
    setActionPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
      cursorPosition: currentCursorPosition,
    });
  };



  const handleActionSelect = (actionId: string) => {
    // Get the action info from the ActionPopover's display actions
    const displayActions = actionPopoverRef.current?.getDisplayActions() || [];
    const selectedAction = displayActions.find(action => action.id === actionId);
    
    if (!selectedAction) {
      console.error('Selected action not found:', actionId);
      setActionPopover(prev => ({ ...prev, isOpen: false }));
      return;
    }
    
    // Get current cursor position from the RichChatInput
    const currentValue = displayValue;
    const cursorPosition = actionPopover.cursorPosition || 0;
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const afterCursor = currentValue.slice(cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      // Check if we're still in the same "word" after the slash
      if (!afterSlash.includes(' ') && !afterSlash.includes('\n')) {
        // Replace the /query with [Action Label] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        const actionText = `[${selectedAction.label}]`;
        const newValue = beforeSlash + actionText + " " + afterCursor;
        
        setDisplayValue(newValue);
        setValue(newValue);
        
        // Set cursor position after the action text and space
        const newCursorPosition = lastSlashIndex + actionText.length + 1;
        setTimeout(() => {
          if (textAreaRef.current) {
            textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
            textAreaRef.current.focus();
          }
        }, 0);
      }
    }
    
    console.log('Action selected:', actionId, 'label:', selectedAction.label);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };


  const hasSubmittableContent =
    displayValue.trim() ||
    pastedImages.some((img) => img.filePath && !img.error && !img.isLoading) ||
    allDroppedFiles.some((file) => !file.error && !file.isLoading);
  const isAnyImageLoading = pastedImages.some((img) => img.isLoading);
  const isAnyDroppedFileLoading = allDroppedFiles.some((file) => file.isLoading);

  const isSubmitButtonDisabled =
    !hasSubmittableContent ||
    isAnyImageLoading ||
    isAnyDroppedFileLoading ||
    isRecording ||
    isTranscribing ||
    isCompacting ||
    !agentIsReady ||
    isExtensionsLoading;

  const isUserInputDisabled =
    isAnyImageLoading ||
    isAnyDroppedFileLoading ||
    isRecording ||
    isTranscribing ||
    isCompacting ||
    !agentIsReady ||
    isExtensionsLoading;

  // Queue management functions - no storage persistence, only in-memory
  const handleRemoveQueuedMessage = (messageId: string) => {
    setQueuedMessages((prev) => prev.filter((msg) => msg.id !== messageId));
  };

  const handleClearQueue = () => {
    setQueuedMessages([]);
    queuePausedRef.current = false;
    setLastInterruption(null);
  };

  const handleReorderMessages = (reorderedMessages: QueuedMessage[]) => {
    setQueuedMessages(reorderedMessages);
  };

  const handleEditMessage = (messageId: string, newContent: string) => {
    setQueuedMessages((prev) =>
      prev.map((msg) => (msg.id === messageId ? { ...msg, content: newContent } : msg))
    );
  };

  const handleStopAndSend = (messageId: string) => {
    const messageToSend = queuedMessages.find((msg) => msg.id === messageId);
    if (!messageToSend) return;

    // Stop current processing and temporarily pause queue to prevent double-send
    if (onStop) onStop();
    const wasPaused = queuePausedRef.current;
    queuePausedRef.current = true;

    // Remove the message from queue and send it immediately
    setQueuedMessages((prev) => prev.filter((msg) => msg.id !== messageId));
    LocalMessageStorage.addMessage(messageToSend.content);
    handleSubmit(
      new CustomEvent('submit', {
        detail: { value: messageToSend.content },
      }) as unknown as React.FormEvent
    );

    // Restore previous pause state after a brief delay to prevent race condition
    setTimeout(() => {
      queuePausedRef.current = wasPaused;
    }, 100);
  };

  const handleResumeQueue = () => {
    queuePausedRef.current = false;
    setLastInterruption(null);
    if (!isLoading && queuedMessages.length > 0) {
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
    }
  };

  return (
    <div
      ref={chatInputRef}
      className={`flex flex-col relative h-auto transition-colors ${
        disableAnimation ? '' : 'page-transition'
      } z-10 pt-6 px-6 pb-4`}
      data-drop-zone="true"
      onDrop={handleLocalDrop}
      onDragOver={handleLocalDragOver}
    >
      {/* Popover Zone - Absolute positioned above chat input, doesn't affect layout */}
      <div id="mention-popover-zone" className="absolute -top-24 left-0 right-0 z-50 h-24 bg-transparent pointer-events-none">
        {/* This space is reserved for mention popovers to render above the chat input */}
      </div>

      {/* Goose Off Indicator */}
      {!gooseEnabled && (
        <div className="max-w-4xl mx-auto w-full mb-2">
          <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-lg px-4 py-2 flex items-center gap-2">
            <div className="flex-shrink-0 w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
            <span className="text-yellow-600 dark:text-yellow-400 text-sm font-medium">
              Goose is OFF - Type <code className="px-1.5 py-0.5 bg-yellow-500/20 rounded text-xs">@goose</code> to reactivate
            </span>
          </div>
        </div>
      )}

      {/* Chat input container with max width - floating card */}
      <div className="max-w-4xl mx-auto w-full shadow-2xl drop-shadow-2xl">
        <div className="bg-background-default rounded-2xl pt-2 px-2">
      {/* Message Queue Display */}
      {queuedMessages.length > 0 && (
        <MessageQueue
          queuedMessages={queuedMessages}
          onRemoveMessage={handleRemoveQueuedMessage}
          onClearQueue={handleClearQueue}
          onStopAndSend={handleStopAndSend}
          onReorderMessages={handleReorderMessages}
          onEditMessage={handleEditMessage}
          onTriggerQueueProcessing={handleResumeQueue}
          editingMessageIdRef={editingMessageIdRef}
          isPaused={queuePausedRef.current}
          className="border-b border-borderSubtle"
        />
      )}



      {/* Input row with inline action buttons wrapped in form */}
      <form onSubmit={onFormSubmit} className="relative flex items-end">
        <div className="relative flex-1">
                  

        <RichChatInput
            data-testid="chat-input"
            autoFocus
            placeholder={isRecording ? '' : '⌘↑/⌘↓ to navigate messages'}
            value={displayValue}
            onChange={(newValue, cursorPos) => {
              console.log('🔄 ChatInput onChange called:', { newValue: newValue.substring(0, 20) + '...', cursorPos, hasCursorPos: cursorPos !== undefined });
              setDisplayValue(newValue);
              updateValue(newValue);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              
              // Check for @ mention and / action triggers
              if (cursorPos !== undefined) {
                console.log('🔄 ChatInput calling checkForMention with cursorPos:', cursorPos);
                const syntheticTarget = {
                  getBoundingClientRect: () => textAreaRef.current?.getBoundingClientRect?.() || new DOMRect(),
                  selectionStart: cursorPos,
                  selectionEnd: cursorPos,
                  value: newValue,
                };
                checkForMention(newValue, cursorPos, syntheticTarget as HTMLTextAreaElement);
              } else {
                console.log('🔄 ChatInput skipping checkForMention - cursorPos is undefined');
              }
            }}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
            ref={textAreaRef}
            rows={1}
            disabled={isUserInputDisabled}
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
        <div className="flex items-center gap-1 px-2 relative self-center">
          {/* Microphone button - show only if dictation is enabled */}
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
                        className="bg-text-default text-background-default cursor-not-allowed opacity-50 border-text-default rounded-full px-6 py-2"
                      >
                        <Microphone />
                      </Button>
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {dictationSettings.provider === 'openai' ? (
                      <p>
                        OpenAI API key is not configured. Set it up in <b>Settings</b> {'>'}{' '}
                        <b>Models.</b>
                      </p>
                    ) : dictationSettings.provider === 'elevenlabs' ? (
                      <p>
                        ElevenLabs API key is not configured. Set it up in <b>Settings</b> {'>'}{' '}
                        <b>Chat</b> {'>'} <b>Voice Dictation.</b>
                      </p>
                    ) : dictationSettings.provider === null ? (
                      <p>
                        Dictation is not configured. Configure it in <b>Settings</b> {'>'}{' '}
                        <b>Chat</b> {'>'} <b>Voice Dictation.</b>
                      </p>
                    ) : (
                      <p>Dictation provider is not properly configured.</p>
                    )}
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
                        ? 'bg-text-default text-background-default cursor-not-allowed animate-pulse border-text-default'
                        : 'bg-text-default text-background-default hover:bg-text-muted border-text-default'
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
              className="bg-text-default text-background-default hover:bg-text-muted border-text-default rounded-full px-6 py-2"
            >
              <Stop />
            </Button>
          ) : (
            <Tooltip>
              <TooltipTrigger asChild>
                <span>
                  <Button
                    type="submit"
                    size="sm"
                    shape="round"
                    variant="outline"
                    disabled={isSubmitButtonDisabled}
                    className={`rounded-full px-10 py-2 flex items-center gap-2 ${
                      isSubmitButtonDisabled
                        ? 'bg-text-default text-background-default cursor-not-allowed opacity-50 border-text-default'
                        : 'bg-text-default text-background-default hover:bg-text-muted border-text-default hover:cursor-pointer'
                    }`}
                  >
                    <Send className="w-4 h-4" />
                    <span className="text-sm">Send</span>
                  </Button>
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p>
                  {isExtensionsLoading
                    ? 'Loading extensions...'
                    : isCompacting
                      ? 'Compacting conversation...'
                      : isAnyImageLoading
                        ? 'Waiting for images to save...'
                        : isAnyDroppedFileLoading
                          ? 'Processing dropped files...'
                          : isRecording
                            ? 'Recording...'
                            : isTranscribing
                              ? 'Transcribing...'
                              : (chatContext?.agentWaitingMessage ?? 'Send')}
                </p>
              </TooltipContent>
            </Tooltip>
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
        <div className="flex flex-wrap gap-2 p-4 mt-2 border-t border-borderSubtle">
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
      <div ref={bottomControlsRef} className="flex flex-row items-center gap-1 p-2 relative">
        {/* Chat Actions Popover - consolidated tools */}
        <ChatActionsPopover
          shouldShowIconOnly={shouldShowIconOnly}
          onActionButtonClick={handleActionButtonClick}
          onAttachClick={handleFileSelect}
        />
        <div className="w-px h-4 bg-border-default mx-2" />

        {/* Session Sharing Component - disabled for Matrix rooms */}
        {!isMatrixRoom && (
          <SessionSharing
            sessionSharing={sessionSharing}
            shouldShowIconOnly={shouldShowIconOnly}
          />
        )}
        <div className="w-px h-4 bg-border-default mx-2" />

        {/* Chat Settings Popover - consolidated settings */}
        <ChatSettingsPopover
          sessionId={sessionId}
          setView={setView}
          alerts={alerts}
          recipeConfig={recipeConfig}
          hasMessages={messages && Array.isArray(messages) && messages.length > 0}
          shouldShowIconOnly={shouldShowIconOnly}
          inputTokens={inputTokens}
          outputTokens={outputTokens}
          sessionCosts={sessionCosts}
          setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
        />

        <EnhancedMentionPopover
          ref={enhancedMentionPopoverRef}
          isOpen={mentionPopover.isOpen}
          onClose={() => setMentionPopover((prev) => ({ ...prev, isOpen: false }))}
          onSelectFile={handleMentionFileSelect}
          onInviteFriend={handleFriendInvite}
          position={mentionPopover.position}
          query={mentionPopover.query}
          selectedIndex={mentionPopover.selectedIndex}
          onSelectedIndexChange={(index) =>
            setMentionPopover((prev) => ({ ...prev, selectedIndex: index }))
          }
        />

        <ActionPopover
          ref={actionPopoverRef}
          isOpen={actionPopover.isOpen}
          onClose={() => setActionPopover((prev) => ({ ...prev, isOpen: false }))}
          onSelect={handleActionSelect}
          position={actionPopover.position}
          selectedIndex={actionPopover.selectedIndex}
          onSelectedIndexChange={(index) =>
            setActionPopover((prev) => ({ ...prev, selectedIndex: index }))
          }
          query={actionPopover.query}
          onCreateCommand={() => {
            setIsAddCommandModalOpen(true);
            setActionPopover((prev) => ({ ...prev, isOpen: false }));
          }}
        />
      </div>

      {/* Add Custom Command Modal */}
      <AddCustomCommandModal
        isOpen={isAddCommandModalOpen}
        onClose={() => setIsAddCommandModalOpen(false)}
        onSave={handleModalSave}
      />
        </div>
      </div>
    </div>
  );
}
