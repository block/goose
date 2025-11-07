import React, {
  useRef,
  useState,
  useEffect,
  useMemo,
  useCallback,
  forwardRef,
  useImperativeHandle,
} from 'react';
import { Bug, FolderKey, ScrollText, Command as CommandIcon } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { Button } from './ui/button';
import type { View } from '../utils/navigationUtils';
import Stop from './ui/Stop';
import { Attach, Send, Close, Microphone } from './icons';
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
import CommandPopover from './CommandPopover';
import { useDictationSettings } from '../hooks/useDictationSettings';
import { useChatContext } from '../contexts/ChatContext';
import { COST_TRACKING_ENABLED, VOICE_DICTATION_ELEVENLABS_ENABLED } from '../updates';
import { CostTracker } from './bottom_menu/CostTracker';
import { DroppedFile, useFileDrop } from '../hooks/useFileDrop';
import { Recipe } from '../recipe';
import MessageQueue from './MessageQueue';
import { detectInterruption } from '../utils/interruptionDetector';
import { DiagnosticsModal } from './ui/DownloadDiagnostics';
import { Message } from '../api';
import { useCommands } from '../hooks/useCommands';
import type { Command } from '../hooks/useCommands';
import { CommandPill } from './CommandPill';
import MentionPill from './MentionPill';
import { Zap } from 'lucide-react';

interface RichTextAreaProps {
  value: string;
  onChange: (value: string, cursorPos?: number) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onPaste?: (e: React.ClipboardEvent<HTMLDivElement>) => void;
  onFocus?: () => void;
  onBlur?: () => void;
  onCompositionStart?: () => void;
  onCompositionEnd?: () => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
  autoFocus?: boolean;
  'data-testid'?: string;
  rows?: number;
}

export interface RichTextAreaRef {
  focus: () => void;
  blur: () => void;
  setSelectionRange: (start: number, end: number) => void;
  // eslint-disable-next-line no-undef
  getBoundingClientRect: () => DOMRect | undefined;
  contentRef: React.RefObject<HTMLTextAreaElement | null>;
  resetHeight: () => void;
}

const RichTextArea = forwardRef<RichTextAreaRef, RichTextAreaProps>(function RichTextArea(
  {
    value,
    onChange,
    onKeyDown,
    onPaste,
    onFocus,
    onBlur,
    onCompositionStart,
    onCompositionEnd,
    placeholder,
    disabled,
    className,
    style,
    autoFocus,
    'data-testid': testId,
    rows = 1,
  },
  ref
) {
  const hiddenTextareaRef = useRef<HTMLTextAreaElement>(null);
  const displayRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [containerHeight, setContainerHeight] = useState<number>(0);
  const { commands } = useCommands();

  const handleTextareaScroll = useCallback(() => {
    if (hiddenTextareaRef.current && displayRef.current) {
      requestAnimationFrame(() => {
        displayRef.current!.scrollTop = hiddenTextareaRef.current!.scrollTop;
        displayRef.current!.scrollLeft = hiddenTextareaRef.current!.scrollLeft;
      });
    }
  }, []);

  // Comprehensive height and scroll synchronization
  const syncDisplayHeight = useCallback(() => {
    if (hiddenTextareaRef.current && displayRef.current) {
      const textarea = hiddenTextareaRef.current;
      const display = displayRef.current;
      const computedStyle = window.getComputedStyle(textarea);
      const fontSize = parseFloat(computedStyle.fontSize);
      const lineHeightValue = computedStyle.lineHeight;
      const lineHeight = Math.round(
        lineHeightValue === 'normal' ? fontSize * 1.2 : parseFloat(lineHeightValue)
      );
      const minHeight = rows * lineHeight;
      const maxHeight = style?.maxHeight ? parseInt(style.maxHeight.toString()) : 300;

      textarea.style.height = '0px';
      // Force reflow to reset height calculation
      void textarea.offsetHeight;
      textarea.style.height = 'auto';

      const textareaScrollHeight = textarea.scrollHeight;

      const desiredHeight = Math.min(textareaScrollHeight, maxHeight);
      const finalHeight = Math.max(desiredHeight, minHeight);

      textarea.style.height = `${finalHeight}px`;
      textarea.style.lineHeight = `${lineHeight}px`;
      textarea.style.minHeight = `${minHeight}px`;
      textarea.style.maxHeight = `${maxHeight}px`;

      display.style.height = `${finalHeight}px`;
      display.style.lineHeight = `${lineHeight}px`;
      display.style.minHeight = `${minHeight}px`;
      display.style.maxHeight = `${maxHeight}px`;

      // Update container height so it takes up space in the layout
      setContainerHeight(finalHeight);

      // Handle display content height for scrolling
      const displayContent = display.firstElementChild as HTMLElement;
      if (displayContent) {
        if (textareaScrollHeight > finalHeight) {
          displayContent.style.minHeight = `${textareaScrollHeight}px`;
        } else {
          displayContent.style.minHeight = 'auto';
        }
      }

      // Sync scroll positions first
      display.scrollTop = textarea.scrollTop;
      display.scrollLeft = textarea.scrollLeft;

      // Then ensure cursor is visible and update both layers together
      if (textareaScrollHeight > finalHeight) {
        const cursorPosition = textarea.selectionStart;
        const textBeforeCursor = textarea.value.substring(0, cursorPosition);
        const linesBeforeCursor = textBeforeCursor.split('\n').length;
        const cursorLineTop = (linesBeforeCursor - 1) * 21; // lineHeight = 21
        const cursorLineBottom = cursorLineTop + 21;

        const currentScrollTop = textarea.scrollTop;
        const visibleTop = currentScrollTop;
        const visibleBottom = currentScrollTop + finalHeight;

        let newScrollTop = currentScrollTop;

        // Calculate new scroll position if cursor is not visible
        if (cursorLineBottom > visibleBottom) {
          // Cursor is below visible area - scroll down to show it
          newScrollTop = Math.max(0, cursorLineBottom - finalHeight + 21);
        } else if (cursorLineTop < visibleTop) {
          // Cursor is above visible area - scroll up to show it
          newScrollTop = Math.max(0, cursorLineTop - 21);
        }

        // Update both layers together if scroll position changed
        if (newScrollTop !== currentScrollTop) {
          textarea.scrollTop = newScrollTop;
          display.scrollTop = newScrollTop;
        }
      }
    }
  }, [rows, style]);

  // Ensure line height consistency between layers
  // Comprehensive style synchronization between layers
  const ensureStyleConsistency = useCallback(() => {
    if (hiddenTextareaRef.current && displayRef.current) {
      const textarea = hiddenTextareaRef.current;
      const display = displayRef.current;

      // Get all computed styles that affect text layout
      const computedStyle = window.getComputedStyle(textarea);

      // Apply critical layout styles to ensure perfect alignment
      // Force exact line height match with higher specificity
      const exactLineHeight = computedStyle.lineHeight;
      display.style.setProperty('line-height', exactLineHeight, 'important');

      // Also set minHeight using the computed line height for consistency
      const computedLineHeight = parseFloat(exactLineHeight);
      const calculatedMinHeight = rows * computedLineHeight;
      display.style.setProperty('min-height', `${calculatedMinHeight}px`, 'important');

      // Ensure all child elements inherit the exact line height
      const allChildren = display.querySelectorAll('*');
      allChildren.forEach((child) => {
        (child as HTMLElement).style.setProperty('line-height', 'inherit', 'important');
      });
      display.style.fontSize = computedStyle.fontSize;
      display.style.fontFamily = computedStyle.fontFamily;
      display.style.letterSpacing = computedStyle.letterSpacing;
      display.style.wordSpacing = computedStyle.wordSpacing;
      // Ensure identical padding (critical for alignment)
      display.style.paddingTop = computedStyle.paddingTop;
      display.style.paddingRight = computedStyle.paddingRight;
      display.style.paddingBottom = computedStyle.paddingBottom;
      display.style.paddingLeft = computedStyle.paddingLeft;

      // Ensure identical text rendering to prevent subpixel drift
      display.style.textRendering = computedStyle.textRendering;
    }
  }, [rows]);

  // Monitor textarea for any changes that might affect height
  const monitorTextareaChanges = useCallback(() => {
    if (hiddenTextareaRef.current) {
      const textarea = hiddenTextareaRef.current;

      // Use ResizeObserver to detect when textarea dimensions change
      const resizeObserver = new ResizeObserver(() => {
        syncDisplayHeight();
        ensureStyleConsistency();
      });

      resizeObserver.observe(textarea);

      // Also monitor scroll height changes
      let lastScrollHeight = textarea.scrollHeight;
      const checkScrollHeight = () => {
        if (textarea.scrollHeight !== lastScrollHeight) {
          lastScrollHeight = textarea.scrollHeight;
          syncDisplayHeight();
          ensureStyleConsistency();
        }
        requestAnimationFrame(checkScrollHeight);
      };

      checkScrollHeight();

      return () => {
        resizeObserver.disconnect();
      };
    }
    return undefined;
  }, [syncDisplayHeight, ensureStyleConsistency]);

  const handleRemoveMention = useCallback(
    (fileName: string) => {
      const mentionText = `@${fileName}`;
      const newValue = value.replace(mentionText, '');
      onChange(newValue);
    },
    [value, onChange]
  );

  const handleRemoveCommand = useCallback(
    (commandLabel: string) => {
      const commandText = `[${commandLabel}]`;
      const newValue = value.replace(commandText, '');
      onChange(newValue);
    },
    [value, onChange]
  );

  useImperativeHandle(
    ref,
    () => ({
      focus: () => hiddenTextareaRef.current?.focus(),
      blur: () => hiddenTextareaRef.current?.blur(),
      setSelectionRange: (start: number, end: number) => {
        hiddenTextareaRef.current?.setSelectionRange(start, end);
        setCursorPosition(start);
      },
      getBoundingClientRect: () => {
        // eslint-disable-next-line no-undef
        return displayRef.current?.getBoundingClientRect() || new DOMRect();
      },
      contentRef: hiddenTextareaRef,
      resetHeight: () => {
        if (hiddenTextareaRef.current && displayRef.current) {
          const textarea = hiddenTextareaRef.current;
          const display = displayRef.current;
          // Calculate actual line height from the textarea
          const computedStyle = window.getComputedStyle(textarea);
          const fontSize = parseFloat(computedStyle.fontSize);
          const lineHeightValue = computedStyle.lineHeight;
          const lineHeight = Math.round(
            lineHeightValue === 'normal' ? fontSize * 1.2 : parseFloat(lineHeightValue)
          );
          const minHeight = rows * lineHeight;

          // Reset to auto first, then set to minHeight - DON'T call syncDisplayHeight after!
          textarea.style.height = 'auto';
          textarea.style.height = `${minHeight}px`;
          display.style.height = `${minHeight}px`;
          setContainerHeight(minHeight);
        }
      },
    }),
    [rows]
  );

  // Update cursor position when selection changes
  const updateCursorPosition = useCallback(() => {
    if (hiddenTextareaRef.current) {
      setCursorPosition(hiddenTextareaRef.current.selectionStart);
    }
  }, []);

  const renderContent = useCallback(() => {
    if (!value || (value.trim() === '' && !value.includes('\n'))) {
      return (
        <div className="whitespace-pre-wrap relative">
          {/* Placeholder text positioned absolutely to prevent movement */}
          <span className="text-text-muted pointer-events-none select-none absolute inset-0">
            {placeholder}
          </span>
          {/* Cursor positioned absolutely at the start */}
          {isFocused && cursorPosition === 0 && (
            <span
              className="border-l border-text-default inline-block absolute"
              style={{
                animation: 'blink 1s step-end infinite',
                height: '1.3em', // Taller than text to extend below baseline
                width: '1px',
                left: '0px',
                top: '0.1em', // Start slightly below text top
                position: 'absolute',
              }}
            />
          )}
        </div>
      );
    }

    const parts: React.ReactNode[] = [];
    const actionRegex = /\[([^\]]+)\]/g;
    const mentionRegex = /@([^\s]+)/g;
    let lastIndex = 0;
    let keyCounter = 0;
    let currentPos = 0;

    // Find all actions and mentions then sort by position
    const allMatches = [];

    // Find all action matches
    let actionMatch;
    actionRegex.lastIndex = 0;
    while ((actionMatch = actionRegex.exec(value)) !== null) {
      allMatches.push({
        type: 'action',
        match: actionMatch,
        index: actionMatch.index,
        length: actionMatch[0].length,
        content: actionMatch[1],
      });
    }

    let mentionMatch;
    mentionRegex.lastIndex = 0;
    while ((mentionMatch = mentionRegex.exec(value)) !== null) {
      allMatches.push({
        type: 'mention',
        match: mentionMatch,
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1],
      });
    }
    allMatches.sort((a, b) => a.index - b.index);

    // Process all matches in order, handling overlaps
    const processedMatches = [];
    let lastProcessedEnd = 0;

    for (const matchData of allMatches) {
      // Skip overlapping matches (pills take priority over other things)
      if (matchData.index < lastProcessedEnd) {
        continue;
      }

      processedMatches.push(matchData);
      lastProcessedEnd = matchData.index + matchData.length;
    }

    // Render content with processed matches
    currentPos = 0;
    lastIndex = 0;

    for (const matchData of processedMatches) {
      const { type, index, length, content } = matchData;

      // Add text before this match with potential cursor
      const beforeMatch = value.slice(lastIndex, index);
      if (beforeMatch) {
        let textWithCursor = [];
        for (let i = 0; i < beforeMatch.length; i++) {
          if (isFocused && cursorPosition === currentPos) {
            textWithCursor.push(
              <span
                key={`cursor-${keyCounter++}`}
                className="border-l border-text-default inline-block"
                style={{
                  animation: 'blink 1s step-end infinite',
                  height: '1.3em',
                  width: '1px',
                  marginLeft: '0px',
                  transform: 'translateY(0.1em)',
                  position: 'relative',
                }}
              />
            );
          }
          textWithCursor.push(beforeMatch[i]);
          currentPos++;
        }

        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {textWithCursor}
          </span>
        );
      }

      // Add cursor before match if needed
      if (isFocused && cursorPosition === currentPos) {
        parts.push(
          <span
            key={`cursor-${keyCounter++}`}
            className="border-l border-text-default inline-block"
            style={{
              animation: 'blink 1s step-end infinite',
              height: '1.3em',
              width: '1px',
              marginLeft: '0px',
              transform: 'translateY(0.1em)',
              position: 'relative',
            }}
          />
        );
      }

      if (type === 'action') {
        const commandLabel = content;
        const matchedCommand = commands.find((cmd) => cmd.name === commandLabel);

        if (matchedCommand) {
          parts.push(
            <CommandPill
              key={`action-${keyCounter++}`}
              commandId={matchedCommand.id}
              label={matchedCommand.name}
              icon={<Zap size={12} />}
              variant="default"
              size="sm"
              onRemove={() => handleRemoveCommand(commandLabel)}
            />
          );
        } else {
          // If no matching action, render as text
          parts.push(
            <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
              {value.slice(index, index + length)}
            </span>
          );
        }
      } else if (type === 'mention') {
        // Handle mention pills
        const fileName = content;
        const filePath = `@${fileName}`;

        console.log('📁 Creating mention pill:', { fileName, filePath });

        parts.push(
          <MentionPill
            key={`mention-${keyCounter++}`}
            fileName={fileName}
            filePath={filePath}
            variant="default"
            size="sm"
            onRemove={() => handleRemoveMention(fileName)}
          />
        );
      }

      currentPos += length;
      lastIndex = index + length;
    }

    // Add remaining text with potential cursor
    const remainingText = value.slice(lastIndex);
    if (remainingText || lastIndex < value.length) {
      let textWithCursor = [];
      for (let i = 0; i < remainingText.length; i++) {
        if (isFocused && cursorPosition === currentPos) {
          textWithCursor.push(
            <span
              key={`cursor-${keyCounter++}`}
              className="border-l border-text-default inline-block"
              style={{
                animation: 'blink 1s step-end infinite',
                height: '1.3em',
                width: '1px',
                marginLeft: '0px',
                transform: 'translateY(0.1em)',
                position: 'relative',
              }}
            />
          );
        }
        textWithCursor.push(remainingText[i]);
        currentPos++;
      }

      parts.push(
        <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
          {textWithCursor}
        </span>
      );
    }

    // Always check for cursor at the end, including after trailing newlines
    if (isFocused && cursorPosition === currentPos) {
      parts.push(
        <span
          key={`cursor-${keyCounter++}`}
          className="border-l border-text-default inline-block"
          style={{
            animation: 'blink 1s step-end infinite',
            height: '1.3em',
            width: '1px',
            marginLeft: '0px',
            transform: 'translateY(0.1em)',
            position: 'relative',
          }}
        />
      );
    }

    // Ensure we have content even if it's just newlines
    // This handles cases like "text\n\n\n" where trailing newlines need to be visible
    if (parts.length === 0 && value.length > 0) {
      // We have content but no rendered parts, likely just whitespace/newlines
      parts.push(
        <span key={`whitespace-${keyCounter++}`} className="inline whitespace-pre-wrap">
          {value}
          {isFocused && cursorPosition === value.length && (
            <span
              className="border-l border-text-default inline-block"
              style={{
                animation: 'blink 1s step-end infinite',
                height: '1.3em',
                width: '1px',
                marginLeft: '0px',
                transform: 'translateY(0.1em)',
                position: 'relative',
              }}
            />
          )}
        </span>
      );
    }

    return (
      <div className="whitespace-pre-wrap">
        {parts.length > 0
          ? parts
          : isFocused && (
              <span
                className="border-l border-text-default inline-block"
                style={{
                  animation: 'blink 1s step-end infinite',
                  height: '1.3em',
                  width: '1px',
                  marginLeft: '0px',
                  transform: 'translateY(0.1em)',
                  position: 'relative',
                }}
              />
            )}
      </div>
    );
  }, [
    value,
    isFocused,
    handleRemoveCommand,
    handleRemoveMention,
    placeholder,
    cursorPosition,
    commands,
  ]);

  const handleTextareaChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      const newValue = e.target.value;
      const newCursorPos = e.target.selectionStart;

      // Track typing activity to prevent hover tooltips while actively typing
      if (hiddenTextareaRef.current) {
        hiddenTextareaRef.current.dataset.lastActivity = Date.now().toString();
      }
      onChange(newValue, newCursorPos);
      setCursorPosition(newCursorPos);

      // Sync display height immediately for better responsiveness
      // Use both immediate sync and deferred sync for reliability
      syncDisplayHeight();
      ensureStyleConsistency();
      requestAnimationFrame(() => syncDisplayHeight());
    },
    [onChange, syncDisplayHeight, ensureStyleConsistency]
  );

  const handleTextareaKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      setTimeout(updateCursorPosition, 0);

      if (e.key === 'Backspace') {
        const cursorPos = e.currentTarget.selectionStart;
        const beforeCursor = value.slice(0, cursorPos);

        const commandMatch = beforeCursor.match(/\[([^\]]+)\]$/);
        if (commandMatch) {
          e.preventDefault();
          handleRemoveCommand(commandMatch[1]);
          return;
        }

        const mentionMatch = beforeCursor.match(/@([^\s]+)$/);
        if (mentionMatch) {
          e.preventDefault();
          handleRemoveMention(mentionMatch[1]);
          return;
        }
      }

      const syntheticEvent = {
        ...e,
        key: e.key,
        code: e.code,
        keyCode: e.keyCode,
        which: e.which,
        shiftKey: e.shiftKey,
        altKey: e.altKey,
        ctrlKey: e.ctrlKey,
        metaKey: e.metaKey,
        repeat: e.repeat,
        preventDefault: e.preventDefault.bind(e),
        stopPropagation: e.stopPropagation.bind(e),
        currentTarget: displayRef.current,
        target: displayRef.current,
      } as React.KeyboardEvent<HTMLDivElement>;

      onKeyDown?.(syntheticEvent);
    },
    [value, handleRemoveCommand, onKeyDown, handleRemoveMention, updateCursorPosition]
  );

  const handleTextareaPaste = useCallback(
    (e: React.ClipboardEvent<HTMLTextAreaElement>) => {
      // Update cursor position after paste
      setTimeout(updateCursorPosition, 0);

      // Create proper synthetic event
      const syntheticEvent = {
        ...e,
        preventDefault: () => e.preventDefault(),
        stopPropagation: () => e.stopPropagation(),
        clipboardData: e.clipboardData,
        currentTarget: displayRef.current,
        target: displayRef.current,
      } as React.ClipboardEvent<HTMLDivElement>;

      onPaste?.(syntheticEvent);
    },
    [onPaste, updateCursorPosition]
  );

  const handleTextareaFocus = useCallback(() => {
    setIsFocused(true);
    updateCursorPosition();
    onFocus?.();
  }, [onFocus, updateCursorPosition]);

  const handleTextareaBlur = useCallback(() => {
    setIsFocused(false);
    onBlur?.();
  }, [onBlur]);

  // Handle selection changes (cursor movement)
  const handleSelectionChange = useCallback(() => {
    if (document.activeElement === hiddenTextareaRef.current) {
      updateCursorPosition();
    }
  }, [updateCursorPosition]);

  // Auto-focus effect
  useEffect(() => {
    if (autoFocus && hiddenTextareaRef.current) {
      hiddenTextareaRef.current.focus();
    }
  }, [autoFocus]);

  // Listen for selection changes to update cursor position
  useEffect(() => {
    document.addEventListener('selectionchange', handleSelectionChange);
    return () => {
      document.removeEventListener('selectionchange', handleSelectionChange);
    };
  }, [handleSelectionChange]);

  // Start monitoring textarea changes for height synchronization
  useEffect(() => {
    const cleanup = monitorTextareaChanges();
    return cleanup;
  }, [monitorTextareaChanges]);

  // Explicitly sync height whenever value changes (including when text is deleted)
  useEffect(() => {
    syncDisplayHeight();
    ensureStyleConsistency();
  }, [value, syncDisplayHeight, ensureStyleConsistency]);

  return (
    <div
      ref={containerRef}
      className="relative rich-text-input"
      style={{ height: containerHeight > 0 ? `${containerHeight}px` : 'auto' }}
    >
      <textarea
        ref={hiddenTextareaRef}
        value={value}
        onChange={handleTextareaChange}
        onKeyDown={handleTextareaKeyDown}
        onPaste={handleTextareaPaste}
        onFocus={handleTextareaFocus}
        onBlur={handleTextareaBlur}
        onCompositionStart={onCompositionStart}
        onCompositionEnd={onCompositionEnd}
        disabled={disabled}
        data-testid={testId}
        className="absolute inset-0 w-full resize-none overflow-y-auto"
        onScroll={handleTextareaScroll}
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          // Remove height: '100%' to let it be controlled by syncDisplayHeight
          opacity: 1, // Fully visible for selection highlighting
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'transparent', // Use CSS transparent instead of rgba
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
          fontFamily: 'Cash Sans, sans-serif', // Match exact font
          fontSize: '0.875rem', // Match text-sm (14px)
          lineHeight: '1.5', // Match leading-relaxed
          padding: '12px 80px 12px 12px', // Match top and bottom padding: 12px each
          margin: '0',
          boxSizing: 'border-box', // Match textarea box model
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
          WebkitTextFillColor: 'transparent', // Webkit-specific transparent text
        }}
        rows={rows}
      />

      {/* Visual display with action pills, mention pills, and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text absolute inset-0 overflow-y-auto rich-text-display`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          maxHeight: style?.maxHeight || 'none', // Respect parent max height constraints
          zIndex: 3,
          pointerEvents: 'none', // Don't interfere with text selection by default
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '12px 80px 12px 12px', // Match textarea padding: 12px top and bottom
          margin: '0',
          boxSizing: 'border-box', // Match textarea box model
          whiteSpace: 'pre-wrap', // Match textarea
          wordWrap: 'break-word',
          // Hide scrollbars but keep scrolling functionality
          scrollbarWidth: 'none', // Firefox
          msOverflowStyle: 'none', // IE/Edge
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>

      {/* CSS to hide webkit scrollbars and enable text selection visibility */}
      <style
        dangerouslySetInnerHTML={{
          __html: `
          .rich-text-display::-webkit-scrollbar {
            display: none;
          }
          
          /* Make text selection visible on the hidden textarea */
          .rich-text-input textarea {
            /* Ensure selection is visible */
            -webkit-user-select: text;
            -moz-user-select: text;
            -ms-user-select: text;
            user-select: text;
          }
          
          .rich-text-input textarea::selection {
            background-color: #BBD6FB !important; /* Custom light blue selection */
            color: transparent !important; /* Keep text transparent even when selected */
          }
          
          .rich-text-input textarea::-moz-selection {
            background-color: #BBD6FB !important; /* Custom light blue selection */
            color: transparent !important; /* Keep text transparent even when selected */
          }
        `,
        }}
      />
    </div>
  );
});

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
  totalTokens?: number;
  accumulatedInputTokens?: number;
  accumulatedOutputTokens?: number;
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
  totalTokens,
  accumulatedInputTokens,
  accumulatedOutputTokens,
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
  // Draft functionality - get chat context and global draft context
  // We need to handle the case where ChatInput is used without ChatProvider (e.g., in Hub)
  const chatContext = useChatContext(); // This should always be available now
  const agentIsReady = chatContext === null || chatContext.agentWaitingMessage === null;
  const draftLoadedRef = useRef(false);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);

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
  const [commandPopover, setCommandPopover] = useState<{
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
  const commandPopoverRef = useRef<{
    getDisplayCommands: () => Command[];
    selectCommand: (index: number) => void;
  }>(null);
  const mentionPopoverRef = useRef<{
    getDisplayFiles: () => FileItemWithMatch[];
    selectFile: (index: number) => void;
  }>(null);
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
  const textAreaRef = useRef<RichTextAreaRef>(null);
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

  // Handle tool count alerts and token usage
  useEffect(() => {
    clearAlerts();
    // Show alert when either there is registered token usage, or we know the limit
    if ((totalTokens && totalTokens > 0) || (isTokenLimitLoaded && tokenLimit)) {
      addAlert({
        type: AlertType.Info,
        message: 'Context window',
        progress: {
          current: totalTokens || 0,
          total: tokenLimit,
        },
        showCompactButton: true,
        compactButtonDisabled: !totalTokens,
        onCompact: () => {
          window.dispatchEvent(new CustomEvent('hide-alert-popover'));
          const customEvent = new CustomEvent('submit', {
            detail: { value: MANUAL_COMPACT_TRIGGER },
          }) as unknown as React.FormEvent;
          handleSubmit(customEvent);
        },
        compactIcon: <ScrollText size={12} />,
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
  }, [totalTokens, toolCount, tokenLimit, isTokenLimitLoaded, addAlert, clearAlerts]);
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
      const element = textAreaRef.current.contentRef?.current;
      if (element) {
        debouncedAutosize(element);
      }
    }
  }, [debouncedAutosize, displayValue]);
  // Reset textarea height when displayValue is empty
  useEffect(() => {
    if (textAreaRef.current && displayValue === '') {
      textAreaRef.current.resetHeight();
    }
  }, [displayValue]);

  const checkForMention = (text: string, cursorPosition: number, textArea: HTMLTextAreaElement) => {
    const beforeCursor = text.slice(0, cursorPosition);
    const lastAtIndex = beforeCursor.lastIndexOf('@');
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    const isSlashTrigger = lastSlashIndex > lastAtIndex;
    const triggerIndex = isSlashTrigger ? lastSlashIndex : lastAtIndex;
    if (triggerIndex === -1) {
      // No trigger symbol found, close both popovers
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setCommandPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }
    // Check if there's a space between trigger symbol and cursor (which would end the trigger)
    const afterTrigger = beforeCursor.slice(triggerIndex + 1);
    if (afterTrigger.includes(' ') || afterTrigger.includes('\n')) {
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setCommandPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }
    // Calculate position for the popover - position it above the chat input
    const textAreaRect = textArea.getBoundingClientRect();
    if (isSlashTrigger) {
      // Open action popover for / trigger
      setMentionPopover((prev) => ({ ...prev, isOpen: false }));
      setCommandPopover({
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
      setCommandPopover((prev) => ({ ...prev, isOpen: false }));
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
      displayValue,
      droppedFiles.length,
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
    if (commandPopover.isOpen && commandPopoverRef.current) {
      if (evt.key === 'ArrowDown') {
        evt.preventDefault();
        const displayCommands = commandPopoverRef.current.getDisplayCommands();
        const maxIndex = Math.max(0, displayCommands.length - 1);
        setCommandPopover((prev) => ({
          ...prev,
          selectedIndex: Math.min(prev.selectedIndex + 1, maxIndex),
        }));
        return;
      }
      if (evt.key === 'ArrowUp') {
        evt.preventDefault();
        setCommandPopover((prev) => ({
          ...prev,
          selectedIndex: Math.max(prev.selectedIndex - 1, 0),
        }));
        return;
      }
      if (evt.key === 'Enter') {
        evt.preventDefault();
        commandPopoverRef.current.selectCommand(commandPopover.selectedIndex);
        return;
      }
      if (evt.key === 'Escape') {
        evt.preventDefault();
        setCommandPopover((prev) => ({ ...prev, isOpen: false }));
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
    const fileName = filePath.split('/').pop() || filePath;
    const mentionText = `@${fileName}`;
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
  };
  const handleCommandButtonClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    const buttonRect = event.currentTarget.getBoundingClientRect();
    // Get the current cursor position from the RichTextArea
    const currentCursorPosition = textAreaRef.current?.getBoundingClientRect
      ? displayValue.length
      : 0;
    setCommandPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
      cursorPosition: currentCursorPosition,
    });
  };
  const handleCommandSelect = (commandId: string) => {
    const displayCommands = commandPopoverRef.current?.getDisplayCommands() || [];
    const selectedCommand = displayCommands.find((command) => command.id === commandId);
    if (!selectedCommand) {
      console.error('selected command not found:', commandId);
      setCommandPopover((prev) => ({ ...prev, isOpen: false }));
      return;
    }
    // Get current cursor position from the RichTextArea
    const currentValue = displayValue;
    const cursorPosition = commandPopover.cursorPosition || 0;
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const afterCursor = currentValue.slice(cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      // Check if we're still in the same "word" after the slash
      if (!afterSlash.includes(' ') && !afterSlash.includes('\n')) {
        // Replace the /query with [Command name] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        const commandText = `[${selectedCommand.name}]`;
        const newValue = beforeSlash + commandText + ' ' + afterCursor;
        setDisplayValue(newValue);
        setValue(newValue);
        // Set cursor position after the action text and space
        const newCursorPosition = lastSlashIndex + commandText.length + 1;
        setTimeout(() => {
          if (textAreaRef.current) {
            textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
            textAreaRef.current.focus();
          }
        }, 0);
      }
    }
    setCommandPopover((prev) => ({ ...prev, isOpen: false }));
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
        <div className="relative flex-1" style={{ maxHeight: `${maxHeight}px` }}>
          <RichTextArea
            data-testid="chat-input"
            autoFocus
            placeholder={isRecording ? '' : '⌘↑/⌘↓ to navigate messages'}
            value={displayValue}
            onChange={(newValue, cursorPos) => {
              setDisplayValue(newValue);
              updateValue(newValue);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              // Check for @ mention and / action triggers
              if (cursorPos !== undefined) {
                const syntheticTarget = {
                  getBoundingClientRect: () =>
                    // eslint-disable-next-line no-undef
                    textAreaRef.current?.getBoundingClientRect?.() || new DOMRect(),
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
                        className="bg-slate-600 text-white cursor-not-allowed opacity-50 border-slate-600 rounded-full px-3 py-2"
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
                    ) : VOICE_DICTATION_ELEVENLABS_ENABLED &&
                      dictationSettings.provider === 'elevenlabs' ? (
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
                  className={`rounded-full px-3 py-2 ${
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
              className="bg-slate-600 text-white hover:bg-slate-700 border-slate-600 rounded-full px-3 py-2"
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
                    className={`rounded-full px-6 py-2 flex items-center justify-center ${
                      isSubmitButtonDisabled
                        ? 'bg-slate-600 text-white cursor-not-allowed opacity-50 border-slate-600'
                        : 'bg-slate-600 text-white hover:bg-slate-700 border-slate-600 hover:cursor-pointer'
                    }`}
                  >
                    <Send className="w-4 h-4" />
                  </Button>
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <p>
                  {isExtensionsLoading
                    ? 'Loading extensions...'
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
                  className={`flex items-center justify-center ${estimatedSize > 20 ? 'text-orange-500' : 'text-textSubtle'}`}
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
                <div className="flex items-center justify-center px-3 py-2 bg-bgSubtle border border-borderStandard rounded-lg min-w-[120px] max-w-[200px]">
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
        <DirSwitcher />
        <div className="w-px h-4 bg-border-default mx-2" />
        {/* Commands */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              onClick={handleCommandButtonClick}
              variant="ghost"
              size="sm"
              className="flex items-center justify-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
            >
              <CommandIcon className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Commands</TooltipContent>
        </Tooltip>
        <div className="w-px h-4 bg-border-default mx-2" />
        {/* Attach button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              onClick={handleFileSelect}
              variant="ghost"
              size="sm"
              className="flex items-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors !px-0"
            >
              <Attach className="w-4 h-4 min-[1050px]:mr-1" />
              <span className="text-xs hidden min-[1050px]:inline">Attach</span>
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
              <CostTracker
                inputTokens={accumulatedInputTokens}
                outputTokens={accumulatedOutputTokens}
                sessionCosts={sessionCosts}
              />
            </>
          )}
          <Tooltip>
            <div>
              <ModelsBottomBar
                sessionId={sessionId}
                dropdownRef={dropdownRef}
                setView={setView}
                alerts={alerts}
                recipe={recipe}
                recipeId={recipeId}
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
          {sessionId && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  onClick={() => setDiagnosticsOpen(true)}
                  variant="ghost"
                  size="sm"
                  className="flex items-center justify-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors"
                >
                  <Bug className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Generate diagnostics bundle</TooltipContent>
            </Tooltip>
          )}
        </div>
        {sessionId && diagnosticsOpen && (
          <DiagnosticsModal
            isOpen={diagnosticsOpen}
            onClose={() => setDiagnosticsOpen(false)}
            sessionId={sessionId}
          />
        )}
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
        <CommandPopover
          ref={commandPopoverRef}
          isOpen={commandPopover.isOpen}
          onClose={() => setCommandPopover((prev) => ({ ...prev, isOpen: false }))}
          onSelect={handleCommandSelect}
          position={commandPopover.position}
          selectedIndex={commandPopover.selectedIndex}
          onSelectedIndexChange={(index) =>
            setCommandPopover((prev) => ({ ...prev, selectedIndex: index }))
          }
          query={commandPopover.query}
        />
      </div>
    </div>
  );
}
