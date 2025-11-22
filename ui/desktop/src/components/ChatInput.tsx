import React, { useRef, useState, useEffect, useMemo, useCallback } from 'react';
import { FolderKey, ScrollText, Plus, MoreHorizontal, Mic, ArrowUp, Zap, FileText, Code, Settings, Search, Play, Hash } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Button } from './ui/button';
import type { View } from '../utils/navigationUtils';
import Stop from './ui/Stop';
import { Attach, Close, Action } from './icons';
import { ChatState } from '../types/chatState';
import debounce from 'lodash/debounce';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message } from '../types/message';
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
import { CustomCommand, BUILT_IN_COMMANDS } from '../types/customCommands';
import { useSessionSharing } from '../hooks/useSessionSharing';
import SessionSharing from './collaborative/SessionSharing';
import EnhancedMentionPopover from './EnhancedMentionPopover';
import { useMatrix } from '../contexts/MatrixContext';
import { sessionMappingService } from '../services/SessionMappingService';
import { useTabContext } from '../contexts/TabContext';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuLabel,
} from './ui/dropdown-menu';

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

  const plusButtonRef = useRef<HTMLButtonElement | null>(null);

  const readWorkingDirectory = useCallback(() => {
    try {
      return String(window.appConfig.get('GOOSE_WORKING_DIR') ?? '');
    } catch (error) {
      console.warn('Unable to read working directory from appConfig:', error);
      return '';
    }
  }, []);

  const [workingDirectory, setWorkingDirectory] = useState<string>(() => readWorkingDirectory());

  useEffect(() => {
    setWorkingDirectory(readWorkingDirectory());
  }, [readWorkingDirectory, sessionId]);

  useEffect(() => {
    const handler = (event: Event) => {
      const customEvent = event as CustomEvent<{ path?: string }>;
      if (customEvent.detail?.path) {
        setWorkingDirectory(customEvent.detail.path);
      } else {
        setWorkingDirectory(readWorkingDirectory());
      }
    };

    window.addEventListener('goose-working-dir-changed', handler as EventListener);
    return () => {
      window.removeEventListener('goose-working-dir-changed', handler as EventListener);
    };
  }, [readWorkingDirectory]);

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
  const [allCommands, setAllCommands] = useState<CustomCommand[]>([]);

  // Load commands (both built-in and custom)
  useEffect(() => {
    const loadAllCommands = () => {
      try {
        // Load user commands
        const userStored = localStorage.getItem('goose-custom-commands');
        let userCommands: CustomCommand[] = [];
        if (userStored) {
          const parsed = JSON.parse(userStored);
          userCommands = parsed
            .filter((cmd: any) => !cmd.isBuiltIn) // Only user commands
            .map((cmd: any) => ({
              ...cmd,
              createdAt: new Date(cmd.createdAt),
              updatedAt: new Date(cmd.updatedAt)
            }));
        }
        setCustomCommands(userCommands);

        // Load built-in command favorites/usage
        const builtInStored = localStorage.getItem('goose-builtin-commands');
        let builtInCommands = [...BUILT_IN_COMMANDS];
        if (builtInStored) {
          const builtInData = JSON.parse(builtInStored);
          builtInCommands = BUILT_IN_COMMANDS.map(cmd => ({
            ...cmd,
            isFavorite: builtInData[cmd.id]?.isFavorite || false,
            usageCount: builtInData[cmd.id]?.usageCount || 0,
          }));
        }

        // Combine all commands and sort
        const combined = [...builtInCommands, ...userCommands].sort((a, b) => {
          if (a.isFavorite && !b.isFavorite) return -1;
          if (!a.isFavorite && b.isFavorite) return 1;
          if (a.usageCount !== b.usageCount) return b.usageCount - a.usageCount;
          return a.label.localeCompare(b.label);
        });
        
        setAllCommands(combined);
      } catch (error) {
        console.error('Failed to load commands:', error);
      }
    };

    loadAllCommands();
    
    // Listen for storage events to sync updates
    const handleStorage = (e: StorageEvent) => {
      if (e.key === 'goose-custom-commands' || e.key === 'goose-builtin-commands') {
        loadAllCommands();
      }
    };
    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, []);

  // Icon mapping for custom commands
  const getCustomCommandIcon = (iconName?: string) => {
    const iconMap: Record<string, React.ReactNode> = {
      'Zap': <Zap size={14} />,
      'Code': <Code size={14} />,
      'FileText': <FileText size={14} />,
      'Search': <Search size={14} />,
      'Play': <Play size={14} />,
      'Settings': <Settings size={14} />,
      'Hash': <Hash size={14} />,
    };
    return iconMap[iconName || 'Zap'] || <Zap size={14} />;
  };

  // Handle command selection from menu
  const handleCommandSelect = (command: CustomCommand) => {
    const actionText = `[${command.label}]`;
    const newValue = displayValue.trim() ? `${displayValue.trim()} ${actionText}` : actionText;
    
    setDisplayValue(newValue);
    setValue(newValue);
    
    // Increment usage
    if (incrementUsage) {
      incrementUsage(command.id);
    }
    
    textAreaRef.current?.focus();
  };
  const tabContext = useTabContext();
  
  // Get Matrix room info from TabContext (primary source)
  let tabMatrixRoomId: string | undefined = undefined;
  let tabMatrixRecipientId: string | undefined = undefined;
  let isExplicitMatrixTab = false;
  
  if (tabContext) {
    try {
      const activeTabState = tabContext.getActiveTabState();
      if (activeTabState?.tab.type === 'matrix') {
        tabMatrixRoomId = activeTabState.tab.matrixRoomId || undefined;
        tabMatrixRecipientId = activeTabState.tab.matrixRecipientId || undefined;
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
  let actualMatrixRoomId: string | undefined = undefined;
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
      
      // Extract text content safely
      const firstContent = message.content && message.content.length > 0 ? message.content[0] : null;
      let contentPreview = 'N/A';
      
      if (firstContent) {
        if (firstContent.type === 'text') {
          contentPreview = firstContent.text.substring(0, 50) + '...';
        } else if (firstContent.type === 'image') {
          contentPreview = '[Image]';
        } else if (firstContent.type === 'toolRequest') {
          contentPreview = `[Tool: ${firstContent.toolCall.value?.name}]`;
        } else {
          contentPreview = `[${firstContent.type}]`;
        }
      }

      console.log('💬 ChatInput: Message details:', {
        id: message.id,
        role: message.role,
        content: contentPreview,
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
      if (lastMessage.id && sentToMatrixRef.current.has(lastMessage.id)) {
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
        if (lastMessage.id) {
          sentToMatrixRef.current.add(lastMessage.id);
        }
        
        // Format as goose-session-message so it can be properly parsed by other clients
        const sessionMessage = {
          sessionId: sessionId || actualMatrixRoomId,
          role: 'assistant',
          content: textContent,
          timestamp: Date.now(),
        };
        const formattedMessage = `goose-session-message:${JSON.stringify(sessionMessage)}`;
        
        // Only send if we have a room ID
        if (actualMatrixRoomId) {
          sendMessage(actualMatrixRoomId, formattedMessage).then(() => {
            console.log('✅ Successfully sent COMPLETE AI response to Matrix room (messageId:', lastMessage.id, ')');
          }).catch((error) => {
            console.error('❌ Failed to send AI response to Matrix room:', error);
            // Remove from sent set if it failed so we can retry
            if (lastMessage.id) {
              sentToMatrixRef.current.delete(lastMessage.id);
            }
          });
        }
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
    // Original loadCustomCommands logic removed as it's now handled by the main loading effect
  }, []);

  // Handle modal save
  const handleModalSave = (command: CustomCommand) => {
    const now = new Date();
    const updatedCommands = [...customCommands, { ...command, createdAt: now, updatedAt: now }];
    
    try {
      localStorage.setItem('goose-custom-commands', JSON.stringify(updatedCommands));
      // State updates will be handled by the storage event listener or re-render
      setCustomCommands(updatedCommands);
      // Manually trigger reload for current window
      window.dispatchEvent(new StorageEvent('storage', { key: 'goose-custom-commands' }));
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
      const customCommand = (() => {
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
      })();
      
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

  const openActionPopoverAt = useCallback(
    (anchorRect?: DOMRect | null) => {
      const referenceRect = anchorRect ?? plusButtonRef.current?.getBoundingClientRect();
      if (!referenceRect) {
        return;
      }

      const currentCursorPosition = textAreaRef.current?.getBoundingClientRect ? displayValue.length : 0;

      setActionPopover({
        isOpen: true,
        position: {
          x: referenceRect.left,
          y: referenceRect.top,
        },
        selectedIndex: 0,
        cursorPosition: currentCursorPosition,
      });
    },
    [displayValue]
  );




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
      } z-10 pt-4 pb-6 px-4 sm:px-6`}
      data-drop-zone="true"
      onDrop={handleLocalDrop}
      onDragOver={handleLocalDragOver}
    >
      <div id="mention-popover-zone" className="absolute -top-24 left-0 right-0 z-50 h-24 bg-transparent pointer-events-none" />

      <div className="max-w-4xl mx-auto w-full space-y-4">
        {!gooseEnabled && (
          <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-lg px-4 py-2 flex items-center gap-2">
            <span className="inline-flex w-2 h-2 bg-yellow-500 rounded-full animate-pulse" />
            <span className="text-yellow-600 dark:text-yellow-400 text-sm font-medium">
              Goose is OFF — type <code className="px-1.5 py-0.5 bg-yellow-500/20 rounded text-xs">@goose</code> to reactivate
            </span>
          </div>
        )}

        <div className="chat-composer-container rounded-3xl overflow-hidden transform-gpu">
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
              className="border-b border-white/50 dark:border-white/10"
            />
          )}

          <div className="flex flex-col gap-2 p-3 sm:p-3.5">
            <form onSubmit={onFormSubmit} className="space-y-1.5">
              <div className="relative flex-1 min-w-[200px] rounded-xl border border-transparent">
                <RichChatInput
                  data-testid="chat-input"
                  autoFocus
                  placeholder={isRecording ? '' : 'I want to...'}
                  value={displayValue}
                  onChange={(newValue, cursorPos) => {
                    console.log('🔄 ChatInput onChange called:', { newValue: newValue.substring(0, 20) + '...', cursorPos, hasCursorPos: cursorPos !== undefined });
                    setDisplayValue(newValue);
                    updateValue(newValue);
                    debouncedSaveDraft(newValue);
                    setHasUserTyped(true);

                    if (cursorPos !== undefined) {
                      const syntheticTarget = {
                        getBoundingClientRect: () => textAreaRef.current?.getBoundingClientRect?.() || new DOMRect(),
                        selectionStart: cursorPos,
                        selectionEnd: cursorPos,
                        value: newValue,
                      };
                      checkForMention(newValue, cursorPos, syntheticTarget as HTMLTextAreaElement);
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
                  className="w-full outline-none border-none focus:ring-0 bg-transparent px-0 py-0 text-base leading-6 resize-none text-[#050506] dark:text-white placeholder:text-[#8E8E93]"
                />
                {isRecording && (
                  <div className="absolute inset-0 flex items-center pr-4">
                    <WaveformVisualizer audioContext={audioContext} analyser={analyser} isRecording={isRecording} />
                  </div>
                )}
              </div>

              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <button
                        ref={plusButtonRef}
                        type="button"
                        className="w-8 h-8 rounded-full border border-zinc-200 dark:border-zinc-800 flex items-center justify-center bg-transparent text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors"
                      >
                        <Plus className="w-5 h-5" />
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      align="start"
                      sideOffset={12}
                      className="w-56 rounded-2xl border border-white/60 dark:border-zinc-800 bg-white/95 dark:bg-neutral-900/95 backdrop-blur-xl p-2 space-y-1"
                    >
                      <DropdownMenuItem
                        onSelect={(event) => {
                          event.preventDefault();
                          handleFileSelect();
                        }}
                        className="flex items-center gap-2 text-sm text-[#050506] dark:text-white"
                      >
                        <Attach className="w-4 h-4" />
                        Attach file or folder
                      </DropdownMenuItem>
                      
                      <DropdownMenuSeparator className="bg-white/50 dark:bg-white/10" />
                      
                      {allCommands.slice(0, 5).map((command) => (
                        <DropdownMenuItem
                          key={command.id}
                          onSelect={(event) => {
                            event.preventDefault();
                            handleCommandSelect(command);
                          }}
                          className="flex items-center gap-2 text-sm text-[#050506] dark:text-white"
                        >
                          <span className="text-[#3C3C43]/70 dark:text-zinc-400">
                            {getCustomCommandIcon(command.icon)}
                          </span>
                          {command.label}
                        </DropdownMenuItem>
                      ))}
                      
                      <DropdownMenuItem
                        onSelect={(event) => {
                          event.preventDefault();
                          openActionPopoverAt();
                        }}
                        className="flex items-center gap-2 text-sm text-[#050506] dark:text-white"
                      >
                        <Action className="w-4 h-4" />
                        View all actions...
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>

                  {(isRecording || isTranscribing) && (
                    <div className="text-[11px] text-[#3C3C43]/70 dark:text-zinc-400 flex items-center gap-1.5">
                      <span
                        className={`inline-block w-2 h-2 rounded-full animate-pulse ${
                          isTranscribing ? 'bg-blue-500' : 'bg-red-500'
                        }`}
                      />
                      {isTranscribing
                        ? 'Transcribing…'
                        : `${Math.floor(recordingDuration)}s • ~${estimatedSize.toFixed(1)}MB`}
                    </div>
                  )}
                </div>

                <div className="flex items-center gap-2 flex-wrap justify-end">
                  {/* Model Selector Pill */}
                  <div className="relative" ref={dropdownRef}>
                    <div className="hidden md:inline-flex h-8 px-2 rounded-2xl border border-black/10 dark:border-white/10 items-center justify-center bg-transparent transition-colors hover:bg-black/5 dark:hover:bg-white/5">
                       <ModelsBottomBar
                          sessionId={sessionId}
                          dropdownRef={dropdownRef}
                          setView={setView}
                          alerts={alerts}
                          recipeConfig={recipeConfig}
                          hasMessages={messages && Array.isArray(messages) && messages.length > 0}
                          shouldShowIconOnly={false}
                        />
                    </div>
                    {/* For mobile or icon-only mode, ModelsBottomBar handles its own internal logic or we can conditionally render. 
                        However, based on the user's request, they want the "Auto" pill style. 
                        ModelsBottomBar uses a DropdownMenu internally. 
                        We need to check if we need to customize the TRIGGER of ModelsBottomBar to match the pill style.
                        Wait, I can't easily customize the trigger inside ModelsBottomBar without editing that file.
                        But the user wants THIS specific styling.
                        
                        The ModelsBottomBar component renders a DropdownMenuTrigger with a specific style.
                        I should probably rely on the fact that I can pass children or styles to it? No, looking at the file, the trigger is hardcoded.
                        
                        Actually, looking at ModelsBottomBar.tsx content I read earlier:
                         <DropdownMenuTrigger className="flex items-center hover:cursor-pointer max-w-[180px] ...">
                           ... <Bot ... /> {displayModel} ...
                         </DropdownMenuTrigger>
                        
                        It doesn't perfectly match the user's "pill" design (outline, rounded-2xl, specific padding).
                        
                        Since I cannot edit ModelsBottomBar in this turn (I am editing ChatInput), 
                        I will wrap it or style it via CSS if possible, OR I should have edited ModelsBottomBar.
                        
                        Actually, I can just render it here. The user said "make the selection of models where you have the gpt.51 instead of from the more context menu".
                        Moving it here is the first step.
                        The styling inside ModelsBottomBar is "flex items-center...".
                        
                        To achieve the specific pill look: 
                        "w-14 h-6 px-1.5 rounded-2xl outline outline-1 ... outline-zinc-800 inline-flex flex-col justify-start items-start gap-2"
                        
                        I might need to wrap it in a div that enforces these styles, but the inner trigger might conflict.
                        Ideally, I would update ModelsBottomBar to accept a custom trigger or className.
                        
                        For now, I will wrap it in a div that approximates the look:
                        border, rounded-2xl, padding.
                    */}
                    <div className="hidden md:flex">
                        {/* We'll use a wrapper to apply the pill styles, but ModelsBottomBar's trigger has its own padding/styles.
                            Let's try to make it look decent with a wrapper.
                        */}
                    </div>
                  </div>

                  {dictationSettings?.enabled && (
                    !canUseDictation ? (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span className="inline-flex">
                            <Button
                              type="button"
                              size="sm"
                              variant="ghost"
                              disabled
                              className="w-8 h-8 rounded-full border border-white/60 dark:border-zinc-800 flex items-center justify-center bg-transparent text-[#3C3C43]/40 dark:text-white/30 cursor-not-allowed"
                            >
                              <Mic className="w-4 h-4" />
                            </Button>
                          </span>
                        </TooltipTrigger>
                        <TooltipContent>
                          {dictationSettings.provider === 'openai'
                            ? 'Configure an OpenAI API key in Settings → Models.'
                            : dictationSettings.provider === 'elevenlabs'
                              ? 'Configure an ElevenLabs API key in Settings → Chat → Voice Dictation.'
                              : 'Dictation provider is not fully configured.'}
                        </TooltipContent>
                      </Tooltip>
                    ) : (
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        disabled={isTranscribing}
                        onClick={() => (isRecording ? stopRecording() : startRecording())}
                        className={`w-8 h-8 rounded-full border flex items-center justify-center transition-all ${
                          isRecording
                            ? 'border-red-500 bg-red-500 text-white'
                            : 'border-zinc-200 dark:border-zinc-800 bg-transparent text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300'
                        }`}
                      >
                        {/* Using Mic icon instead of waveform to match the "simple icon" aesthetic of the snippet, although snippet had a waveform-ish icon */}
                         <div className="w-4 h-4 flex items-center justify-center">
                            {isRecording ? (
                                <div className="w-2 h-2 bg-white rounded-sm animate-pulse" />
                            ) : (
                                <Mic className="w-4 h-4" />
                            )}
                         </div>
                      </Button>
                    )
                  )}

                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <button
                        type="button"
                        className="w-8 h-8 rounded-full border border-zinc-200 dark:border-zinc-800 bg-transparent text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300 flex items-center justify-center transition-colors"
                      >
                        <MoreHorizontal className="w-4 h-4" />
                      </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      align="end"
                      sideOffset={12}
                      className="w-80 rounded-3xl border border-white/60 dark:border-zinc-800 bg-white/95 dark:bg-neutral-900/95 backdrop-blur-xl p-2 shadow-xl"
                    >
                      <div className="px-3 py-2 space-y-2">
                        <div className="flex flex-col gap-1">
                          <span className="text-[10px] uppercase tracking-wider text-zinc-500 dark:text-zinc-400 font-medium">Workspace</span>
                          <div className="flex flex-col gap-0.5">
                            <div 
                              className="text-sm text-zinc-900 dark:text-zinc-100 truncate font-medium" 
                              title={workingDirectory}
                            >
                              {workingDirectory ? workingDirectory.split('/').pop() : 'Not set'}
                            </div>
                            <p className="text-[10px] text-zinc-500 dark:text-zinc-400 truncate font-mono opacity-80">
                              {workingDirectory || ''}
                            </p>
                          </div>
                        </div>
                        <DirSwitcher className="w-full justify-between text-sm text-zinc-900 dark:text-zinc-100" />
                      </div>

                      <DropdownMenuSeparator className="bg-zinc-200 dark:bg-zinc-800 my-1" />

                      <div className="space-y-1 p-1">
                        <DropdownMenuItem
                          onSelect={(event) => {
                            event.preventDefault();
                            setIsGoosehintsModalOpen?.(true);
                          }}
                          className="flex items-center gap-3 px-3 py-2 text-sm cursor-pointer rounded-xl hover:bg-zinc-100 dark:hover:bg-zinc-800/50 outline-none transition-colors"
                        >
                          <FolderKey className="w-4 h-4 text-zinc-500 dark:text-zinc-400" />
                          <span className="font-medium text-zinc-900 dark:text-zinc-100">Goose hints</span>
                        </DropdownMenuItem>

                        {COST_TRACKING_ENABLED && (
                          <div className="px-1">
                            <CostTracker
                              inputTokens={inputTokens}
                              outputTokens={outputTokens}
                              sessionCosts={sessionCosts}
                              shouldShowIconOnly={false}
                            />
                          </div>
                        )}

                        <div className="px-1">
                          <BottomMenuModeSelection shouldShowIconOnly={false} />
                        </div>

                        {!isMatrixRoom && (
                          <div className="px-1">
                            <SessionSharing
                              sessionId={sessionId || ''}
                              sessionTitle={sessionId || 'Chat'}
                              messages={messages}
                              sessionSharing={sessionSharing}
                              shouldShowIconOnly={false}
                            />
                          </div>
                        )}
                      </div>
                    </DropdownMenuContent>
                  </DropdownMenu>

                  {isLoading ? (
                    <Button
                      type="button"
                      onClick={onStop}
                      size="sm"
                      variant="ghost"
                      className="rounded-full w-8 h-8 !p-0 bg-[#FF5F5F] text-white shadow-[0px_0px_25px_rgba(255,95,95,0.35)] flex items-center justify-center"
                    >
                      <Stop className="w-3 h-3" />
                    </Button>
                  ) : (
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <span>
                          <Button
                            type="submit"
                            size="sm"
                            variant="ghost"
                            disabled={isSubmitButtonDisabled}
                            className={`rounded-full w-8 h-8 !p-0 flex items-center justify-center shadow-lg transition-all ${
                              isSubmitButtonDisabled
                                ? 'bg-zinc-100 text-zinc-300 dark:bg-zinc-800 dark:text-zinc-600 cursor-not-allowed'
                                : 'bg-neutral-900 text-white dark:bg-white dark:text-black hover:opacity-90'
                            }`}
                          >
                            {/* Send arrow icon similar to design */}
                             <ArrowUp className="w-4 h-4" />
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
                </div>
              </div>

            </form>
          </div>

          {(pastedImages.length > 0 || allDroppedFiles.length > 0) && (
            <div className="px-4 sm:px-6 pb-4">
              <div className="flex flex-wrap gap-3 border-t border-white/50 dark:border-white/10 pt-4">
                {pastedImages.map((img) => (
                  <div key={img.id} className="relative group w-20 h-20">
                    {img.dataUrl && (
                      <img
                        src={img.dataUrl}
                        alt={`Pasted image ${img.id}`}
                        className={`w-full h-full object-cover rounded-xl border ${img.error ? 'border-red-500' : 'border-white/70 dark:border-white/10'}`}
                      />
                    )}
                    {img.isLoading && (
                      <div className="absolute inset-0 flex items-center justify-center bg-black/50 rounded-xl">
                        <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white" />
                      </div>
                    )}
                    {img.error && !img.isLoading && (
                      <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/70 rounded-xl p-2 text-center">
                        <p className="text-red-400 text-[10px] leading-tight mb-1">{img.error.substring(0, 50)}</p>
                        {img.dataUrl && (
                          <Button type="button" onClick={() => handleRetryImageSave(img.id)} title="Retry saving image" variant="outline" size="xs">
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

                {allDroppedFiles.map((file) => (
                  <div key={file.id} className="relative group">
                    {file.isImage ? (
                      <div className="w-20 h-20">
                        {file.dataUrl && (
                          <img
                            src={file.dataUrl}
                            alt={file.name}
                            className={`w-full h-full object-cover rounded-xl border ${file.error ? 'border-red-500' : 'border-white/70 dark:border-white/10'}`}
                          />
                        )}
                        {file.isLoading && (
                          <div className="absolute inset-0 flex items-center justify-center bg-black/50 rounded-xl">
                            <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white" />
                          </div>
                        )}
                        {file.error && !file.isLoading && (
                          <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/70 rounded-xl p-2 text-center">
                            <p className="text-red-400 text-[10px] leading-tight">{file.error.substring(0, 30)}</p>
                          </div>
                        )}
                      </div>
                    ) : (
                      <div className="flex items-center gap-2 px-3 py-2 bg-white/80 dark:bg-white/10 border border-white/70 dark:border-white/10 rounded-2xl min-w-[140px] max-w-[220px]">
                        <div className="flex-shrink-0 w-9 h-9 bg-white text-[#3C3C43] dark:text-[#E8E8FB] border border-white/70 dark:border-white/10 rounded-xl flex items-center justify-center text-xs font-semibold">
                          {file.name.split('.').pop()?.toUpperCase() || 'FILE'}
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm text-[#050506] dark:text-white truncate" title={file.name}>
                            {file.name}
                          </p>
                          <p className="text-xs text-[#3C3C43]/70 dark:text-white/70">{file.type || 'Unknown type'}</p>
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
            </div>
          )}
        </div>

        <div className="text-xs text-[#3C3C43]/70 dark:text-zinc-400 flex items-center gap-2 px-1">
          {/* Workspace removed as per request - moved to Tab tooltip */}
        </div>
      </div>

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

      <AddCustomCommandModal
        isOpen={isAddCommandModalOpen}
        onClose={() => setIsAddCommandModalOpen(false)}
        onSave={handleModalSave}
      />
    </div>
  );
}
