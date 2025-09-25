import React, { useRef, useEffect, useState, useCallback, forwardRef, useImperativeHandle } from 'react';
import { ActionPill } from './ActionPill';
import MentionPill from './MentionPill';
import { Zap, Code, FileText, Search, Play, Settings } from 'lucide-react';

interface RichChatInputProps {
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

// Action mapping for pill display
const ACTION_MAP = {
  'quick-task': { label: 'Quick Task', icon: <Zap size={12} /> },
  'generate-code': { label: 'Generate Code', icon: <Code size={12} /> },
  'create-document': { label: 'Create Document', icon: <FileText size={12} /> },
  'search-files': { label: 'Search Files', icon: <Search size={12} /> },
  'run-command': { label: 'Run Command', icon: <Play size={12} /> },
  'settings': { label: 'Settings', icon: <Settings size={12} /> },
};

export interface RichChatInputRef {
  focus: () => void;
  blur: () => void;
  setSelectionRange: (start: number, end: number) => void;
  getBoundingClientRect: () => DOMRect;
}

export const RichChatInput = forwardRef<RichChatInputRef, RichChatInputProps>(({
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
}, ref) => {
  const hiddenTextareaRef = useRef<HTMLTextAreaElement>(null);
  const displayRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [cursorPosition, setCursorPosition] = useState(0);
  
  // Scroll synchronization - ensure both layers stay perfectly in sync
  const handleTextareaScroll = useCallback(() => {
    if (hiddenTextareaRef.current && displayRef.current) {
      const textarea = hiddenTextareaRef.current;
      const display = displayRef.current;
      
      // Force immediate synchronization
      requestAnimationFrame(() => {
        display.scrollTop = textarea.scrollTop;
        display.scrollLeft = textarea.scrollLeft;
      });
    }
  }, []);

  // Comprehensive height and scroll synchronization
  const syncDisplayHeight = useCallback(() => {
    if (hiddenTextareaRef.current && displayRef.current) {
      const textarea = hiddenTextareaRef.current;
      const display = displayRef.current;
      
      // Calculate line height more accurately based on actual font metrics
      const lineHeight = 21; // 14px * 1.5 line-height = 21px
      const minHeight = rows * lineHeight;
      const maxHeight = 300;
      
      // Store current styles to avoid unnecessary resets if they're already correct
      const currentTextareaHeight = parseInt(textarea.style.height) || 0;
      const currentDisplayHeight = parseInt(display.style.height) || 0;
      
      // Only reset height temporarily to measure if needed
      const originalHeight = textarea.style.height;
      textarea.style.height = 'auto';
      const textareaScrollHeight = textarea.scrollHeight;
      
      // Calculate desired height based on content, but cap at maxHeight
      const desiredHeight = Math.min(textareaScrollHeight, maxHeight);
      const finalHeight = Math.max(desiredHeight, minHeight);
      
      // Only update if the height actually needs to change (prevents unnecessary layout shifts)
      if (Math.abs(currentTextareaHeight - finalHeight) > 1 || Math.abs(currentDisplayHeight - finalHeight) > 1) {
        // Update both textarea and display layer heights to match exactly
        textarea.style.height = `${finalHeight}px`;
        textarea.style.minHeight = `${finalHeight}px`;
        textarea.style.maxHeight = `${finalHeight}px`;
        
        display.style.height = `${finalHeight}px`;
        display.style.minHeight = `${finalHeight}px`;
        display.style.maxHeight = `${finalHeight}px`;
        
        // Handle display content height for scrolling
        const displayContent = display.firstElementChild as HTMLElement;
        if (displayContent) {
          if (textareaScrollHeight > finalHeight) {
            displayContent.style.minHeight = `${textareaScrollHeight}px`;
          } else {
            displayContent.style.minHeight = 'auto';
          }
        }
      } else {
        // Restore original height if no change needed
        textarea.style.height = originalHeight;
      }
      
      // Sync scroll positions
      display.scrollTop = textarea.scrollTop;
      display.scrollLeft = textarea.scrollLeft;
      
      console.log('🔄 SYNC HEIGHT:', {
        value: textarea.value,
        valueLength: textarea.value.length,
        textareaScrollHeight,
        desiredHeight,
        finalHeight,
        maxHeight,
        minHeight,
        heightChanged: Math.abs(currentTextareaHeight - finalHeight) > 1,
        scrollTop: textarea.scrollTop,
        textareaHeight: textarea.style.height,
        displayHeight: display.style.height
      });
    }
  }, [rows]);

  // Monitor textarea for any changes that might affect height
  const monitorTextareaChanges = useCallback(() => {
    if (hiddenTextareaRef.current) {
      const textarea = hiddenTextareaRef.current;
      
      // Use ResizeObserver to detect when textarea dimensions change
      const resizeObserver = new ResizeObserver(() => {
        syncDisplayHeight();
      });
      
      resizeObserver.observe(textarea);
      
      // Also monitor scroll height changes
      let lastScrollHeight = textarea.scrollHeight;
      const checkScrollHeight = () => {
        if (textarea.scrollHeight !== lastScrollHeight) {
          lastScrollHeight = textarea.scrollHeight;
          syncDisplayHeight();
        }
        requestAnimationFrame(checkScrollHeight);
      };
      
      checkScrollHeight();
      
      return () => {
        resizeObserver.disconnect();
      };
    }
  }, [monitorTextareaChanges]);
  
  // Expose methods to parent component
  useImperativeHandle(ref, () => ({
    focus: () => hiddenTextareaRef.current?.focus(),
    blur: () => hiddenTextareaRef.current?.blur(),
    setSelectionRange: (start: number, end: number) => {
      hiddenTextareaRef.current?.setSelectionRange(start, end);
      setCursorPosition(start);
    },
    getBoundingClientRect: () => {
      return displayRef.current?.getBoundingClientRect() || new DOMRect();
    },
  }), []);

  // Update cursor position when selection changes
  const updateCursorPosition = useCallback(() => {
    if (hiddenTextareaRef.current) {
      setCursorPosition(hiddenTextareaRef.current.selectionStart);
    }
  }, [updateCursorPosition]);

  // Parse and render content with action pills, mention pills, and cursor
  const renderContent = useCallback(() => {
    // Show placeholder when there's no text content (but account for whitespace-only content with newlines)
    if (!value || (value.trim() === '' && !value.includes('\n'))) {
      return (
        <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed relative">
          {/* Placeholder text positioned absolutely to prevent movement */}
          <span className="text-text-muted pointer-events-none select-none absolute inset-0">
            {placeholder}
          </span>
          {/* Cursor positioned absolutely at the start */}
          {isFocused && cursorPosition === 0 && (
            <span 
              className="border-l border-text-default inline-block absolute" 
              style={{ 
                animation: "blink 1s step-end infinite", 
                height: "1.3em", // Taller than text to extend below baseline
                width: "1px",
                left: "0px",
                top: "0.1em", // Start slightly below text top
                position: "absolute"
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

    console.log('🎨 RichChatInput renderContent called with value:', value);
    console.log('🔍 Looking for action and mention patterns with regex:', { actionRegex, mentionRegex });
    
    // Find all actions and mentions, then sort by position
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
        content: actionMatch[1]
      });
    }
    
    // Find all mention matches
    let mentionMatch;
    mentionRegex.lastIndex = 0;
    console.log('🔍 Searching for mentions in value:', value);
    while ((mentionMatch = mentionRegex.exec(value)) !== null) {
      console.log('📁 Found mention match:', mentionMatch);
      allMatches.push({
        type: 'mention',
        match: mentionMatch,
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1]
      });
    }

    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 All matches found:', allMatches);
    
    // Process all matches in order, handling overlaps
    const processedMatches = [];
    let lastProcessedEnd = 0;
    
    for (const matchData of allMatches) {
      // Skip overlapping matches
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
              <span key={`cursor-${keyCounter++}`} className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
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
          <span key={`cursor-${keyCounter++}`} className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
        );
      }
      
      console.log('🎨 PROCESSING MATCH: type:', type, 'content:', content, 'index:', index);
      if (type === 'action') {
        // Handle action pills
        const actionLabel = content;
        const actionEntry = Object.entries(ACTION_MAP).find(
          ([_, config]) => config.label === actionLabel
        );
        
        console.log('🏷️ Creating action pill:', { actionLabel, actionEntry });
        
        if (actionEntry) {
          const [actionId, config] = actionEntry;
          parts.push(
            <ActionPill
              key={`action-${keyCounter++}`}
              actionId={actionId}
              label={config.label}
              icon={config.icon}
              variant="default"
              size="sm"
              onRemove={() => handleRemoveAction(actionLabel)}
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
            <span key={`cursor-${keyCounter++}`} className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
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
        <span key={`cursor-${keyCounter++}`} className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
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
            <span className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
          )}
        </span>
      );
    }
    
    return (
      <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
          )
        )}
      </div>
    );
  }, [value, isFocused, placeholder, cursorPosition]);

  const handleRemoveAction = useCallback((actionLabel: string) => {
    const actionText = `[${actionLabel}]`;
    const newValue = value.replace(actionText, '');
    onChange(newValue);
  }, [value, onChange]);

  const handleRemoveMention = useCallback((fileName: string) => {
    const mentionText = `@${fileName}`;
    const newValue = value.replace(mentionText, '');
    onChange(newValue);
  }, [value, onChange]);

  const handleTextareaChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const newCursorPos = e.target.selectionStart;
    
    console.log('🔄 RichChatInput: onChange', { newValue, newCursorPos });
    onChange(newValue, newCursorPos);
    setCursorPosition(newCursorPos);
    
    // Sync display height immediately for better responsiveness
    // Use both immediate sync and deferred sync for reliability
    syncDisplayHeight();
    requestAnimationFrame(() => syncDisplayHeight());
  }, [onChange, syncDisplayHeight]);

  const handleTextareaKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Update cursor position on key events
    setTimeout(updateCursorPosition, 0);
    
    // Handle backspace on action and mention pills
    if (e.key === 'Backspace') {
      const cursorPos = e.currentTarget.selectionStart;
      const beforeCursor = value.slice(0, cursorPos);
      
      console.log('🔙 Backspace pressed, cursor at:', cursorPos);
      console.log('🔙 Text before cursor:', beforeCursor);
      
      // Check if cursor is right after an action pill [Action]
      const actionMatch = beforeCursor.match(/\[([^\]]+)\]$/);
      if (actionMatch) {
        console.log('🔙 Found action pill to remove:', actionMatch[1]);
        e.preventDefault();
        handleRemoveAction(actionMatch[1]);
        return;
      }
      
      // Check if cursor is right after a mention pill @filename
      const mentionMatch = beforeCursor.match(/@([^\s]+)$/);
      if (mentionMatch) {
        console.log('🔙 Found mention pill to remove:', mentionMatch[1]);
        e.preventDefault();
        handleRemoveMention(mentionMatch[1]);
        return;
      }
    }
    
    // Create a proper synthetic event that maintains all the original event properties
    const syntheticEvent = {
      ...e,
      key: e.key,
      shiftKey: e.shiftKey,
      altKey: e.altKey,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
      preventDefault: () => e.preventDefault(),
      stopPropagation: () => e.stopPropagation(),
      currentTarget: {
        ...e.currentTarget,
        value: e.currentTarget.value,
        selectionStart: e.currentTarget.selectionStart,
        selectionEnd: e.currentTarget.selectionEnd,
        getBoundingClientRect: () => displayRef.current?.getBoundingClientRect() || new DOMRect(),
      },
      target: {
        ...e.currentTarget,
        value: e.currentTarget.value,
        selectionStart: e.currentTarget.selectionStart,
        selectionEnd: e.currentTarget.selectionEnd,
        getBoundingClientRect: () => displayRef.current?.getBoundingClientRect() || new DOMRect(),
      },
    } as any;
    
    onKeyDown?.(syntheticEvent);
  }, [value, handleRemoveAction, onKeyDown, updateCursorPosition]);

  const handleTextareaPaste = useCallback((e: React.ClipboardEvent<HTMLTextAreaElement>) => {
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
    } as any;
    
    onPaste?.(syntheticEvent);
  }, [onPaste, updateCursorPosition]);

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

  return (
    <div className="relative rich-text-input">
      {/* Hidden textarea for actual input handling with native spell check enabled */}
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
        spellCheck={true} // Enable native OS spell checking
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
          boxSizing: 'border-box',
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
          WebkitTextFillColor: 'transparent', // Webkit-specific transparent text
        }}
        rows={rows}
      />
      
      {/* Visual display with action pills, mention pills, and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text relative overflow-y-auto rich-text-display`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          maxHeight: style?.maxHeight || 'none', // Respect parent max height constraints
          zIndex: 3, // Higher z-index, above textarea for misspelled word interactions
          pointerEvents: 'none', // Don't interfere with text selection by default
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '12px 80px 12px 12px', // Match textarea padding: 12px top and bottom
          margin: '0',
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
      <style dangerouslySetInnerHTML={{
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
        `
      }} />
    </div>
  );
});

RichChatInput.displayName = 'RichChatInput';

export default RichChatInput;
