import React, { useRef, useEffect, useState, useCallback, forwardRef, useImperativeHandle } from 'react';
import SpellCheckTooltip from './SpellCheckTooltip';
// Remove unused import - using Electron spell checking instead
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

// Use Electron's system spell checking
const checkSpelling = async (text: string): Promise<{ word: string; start: number; end: number; suggestions: string[] }[]> => {
  console.log('🔍 ELECTRON SPELL CHECK: Starting system spell check for text:', text);
  const misspelledWords: { word: string; start: number; end: number; suggestions: string[] }[] = [];
  
  // Check if Electron API is available
  if (!window.electron?.spellCheck || !window.electron?.spellSuggestions) {
    console.warn('🔍 ELECTRON SPELL CHECK: Electron spell check API not available, falling back to no spell checking');
    return misspelledWords;
  }
  
  // Split text into words while preserving positions
  const wordRegex = /\b[a-zA-Z]+\b/g;
  let match;
  
  const wordChecks: Array<{word: string; start: number; end: number}> = [];
  
  // First, collect all words and their positions
  while ((match = wordRegex.exec(text)) !== null) {
    const word = match[0];
    const start = match.index;
    const end = start + word.length;
    
    // Skip very short words (less than 3 characters)
    if (word.length < 3) {
      continue;
    }
    
    wordChecks.push({ word, start, end });
  }
  
  console.log('🔍 ELECTRON SPELL CHECK: Found words to check:', wordChecks.map(w => w.word));
  
  // Check each word and collect results
  for (const { word, start, end } of wordChecks) {
    try {
      const isCorrect = await window.electron.spellCheck(word);
      console.log('🔍 ELECTRON SPELL CHECK: Word:', word, 'isCorrect:', isCorrect);
      
      if (!isCorrect) {
        // Get suggestions from Electron
        const suggestions = await window.electron.spellSuggestions(word);
        console.log('🔍 ELECTRON SPELL CHECK: Suggestions for', word, ':', suggestions);
        
        misspelledWords.push({
          word: word,
          start: start,
          end: end,
          suggestions: suggestions || []
        });
      }
    } catch (error) {
      console.error('🔍 ELECTRON SPELL CHECK: Error checking word', word, ':', error);
    }
  }
  
  // Sort misspelled words by position
  misspelledWords.sort((a, b) => a.start - b.start);
  
  console.log('🔍 ELECTRON SPELL CHECK: Final result:', misspelledWords);
  return misspelledWords;
};

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
  const [misspelledWords, setMisspelledWords] = useState<{ word: string; start: number; end: number; suggestions: string[] }[]>([]);
  
  // Spell check tooltip state
  const [tooltip, setTooltip] = useState<{
    isVisible: boolean;
    position: { x: number; y: number };
    misspelledWord: string;
    suggestions: string[];
    wordStart: number;
    wordEnd: number;
    isHoveringTooltip: boolean;
  }>({
    isVisible: false,
    position: { x: 0, y: 0 },
    misspelledWord: '',
    suggestions: [],
    wordStart: 0,
    wordEnd: 0,
    isHoveringTooltip: false,
  });

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
  }, []);

  // Spell check the content using Electron's system spell checker
  const performSpellCheck = useCallback(async (text: string) => {
    console.log('🔍 ELECTRON SPELL CHECK: Starting system spell check for text:', text);
    
    // Use the Electron spell checking function
    try {
      const misspelledWords = await checkSpelling(text);
      console.log('🔍 ELECTRON SPELL CHECK: System spell check result:', misspelledWords);
      setMisspelledWords(misspelledWords);
    } catch (error) {
      console.error('🔍 ELECTRON SPELL CHECK: Error performing spell check:', error);
      // Fallback to no spell checking on error
      setMisspelledWords([]);
    }
  }, []);

  // Debounced spell check
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      if (value.trim()) {
        performSpellCheck(value);
      } else {
        setMisspelledWords([]);
      }
    }, 500); // Debounce spell check by 500ms

    return () => clearTimeout(timeoutId);
  }, [value, performSpellCheck]);

  // Parse and render content with action pills, mention pills, spell checking, and cursor
  const renderContent = useCallback(() => {
    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative min-h-[1.5em] flex items-center">
          <span className="text-text-muted pointer-events-none select-none">
            {placeholder}
          </span>
          {isFocused && (
            <span 
              className="border-l-2 border-text-default inline-block align-baseline absolute left-0" 
              style={{ 
                animation: "blink 1s step-end infinite", 
                height: "1em", 
                verticalAlign: "baseline",
                top: "50%",
                transform: "translateY(-50%)"
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
    console.log('📝 Misspelled words:', misspelledWords);
    
    // Find all actions, mentions, and misspelled words, then sort by position
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

    // Add misspelled words
    misspelledWords.forEach(misspelled => {
      allMatches.push({
        type: 'misspelled',
        match: null,
        index: misspelled.start,
        length: misspelled.end - misspelled.start,
        content: misspelled.word
      });
    });
    
    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 All matches found:', allMatches);
    
    // Process all matches in order, handling overlaps
    const processedMatches = [];
    let lastProcessedEnd = 0;
    
    for (const matchData of allMatches) {
      // Skip overlapping matches (pills take priority over spell check)
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
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
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
          <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
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
      } else if (type === 'misspelled') {
        // Handle misspelled words with red highlighting and hover tooltip
        const misspelledData = misspelledWords.find(m => m.word === content);
        console.log('🎨 RENDERING MISSPELLED: word:', content, 'data:', misspelledData);
        console.log('🎨 RENDERING MISSPELLED: all misspelled words:', misspelledWords);
        
        parts.push(
          <span 
            key={`misspelled-${keyCounter++}`} 
            data-misspelled="true"
            className="inline whitespace-pre-wrap cursor-pointer bg-red-50 dark:bg-red-950/30 text-red-600 dark:text-red-400 font-medium px-1 py-0.5 rounded-sm border border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/40 hover:border-red-300 dark:hover:border-red-700 hover:scale-105 transition-all duration-150 relative z-10"
            style={{
              pointerEvents: 'auto', // Override parent's pointer-events: none
              userSelect: 'text', // Allow text selection for normal text editing
            }}
            title={`Click or hover for suggestions: ${content}`}
            onClick={(e) => {
              console.log('🖱️ CLICK: Clicked on misspelled word:', content);
              e.preventDefault();
              e.stopPropagation();
              
              if (misspelledData) {
                const rect = e.currentTarget.getBoundingClientRect();
                console.log('🖱️ CLICK: Element rect:', rect);
                
                // Show tooltip on click - positioned at center of word
                const tooltipData = {
                  isVisible: true,
                  position: { 
                    x: rect.left + rect.width / 2, 
                    y: rect.top 
                  },
                  misspelledWord: misspelledData.word,
                  suggestions: misspelledData.suggestions || [],
                  wordStart: misspelledData.start,
                  wordEnd: misspelledData.end,
                  isHoveringTooltip: false,
                };
                console.log('🖱️ CLICK: Setting tooltip data:', tooltipData);
                setTooltip(tooltipData);
              }
            }}
            onMouseEnter={(e) => {
              console.log('🖱️ MOUSEENTER: Mouse entered misspelled word:', content);
              
              if (misspelledData) {
                const rect = e.currentTarget.getBoundingClientRect();
                console.log('🖱️ MOUSEENTER: Element rect:', rect);
                const tooltipData = {
                  isVisible: true,
                  position: { 
                    x: rect.left + rect.width / 2, 
                    y: rect.top 
                  },
                  misspelledWord: misspelledData.word,
                  suggestions: misspelledData.suggestions || [],
                  wordStart: misspelledData.start,
                  wordEnd: misspelledData.end,
                  isHoveringTooltip: false,
                };
                console.log('🖱️ MOUSEENTER: Setting tooltip data:', tooltipData);
                setTooltip(tooltipData);
              }
            }}
            onMouseLeave={(e) => {
              console.log('🖱️ MOUSELEAVE: Mouse left misspelled word:', content);
              
              // Add a small delay before hiding to allow moving to tooltip
              setTimeout(() => {
                setTooltip(prev => {
                  // Only hide if not hovering over the tooltip
                  if (!prev.isHoveringTooltip) {
                    return { ...prev, isVisible: false };
                  }
                  return prev;
                });
              }, 150);
            }}
          >
            {content}
          </span>
        );
      }
      
      currentPos += length;
      lastIndex = index + length;
    }
    
    // Add remaining text with potential cursor
    const remainingText = value.slice(lastIndex);
    if (remainingText) {
      let textWithCursor = [];
      for (let i = 0; i < remainingText.length; i++) {
        if (isFocused && cursorPosition === currentPos) {
          textWithCursor.push(
            <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
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
    
    // Add cursor at the end if needed
    if (isFocused && cursorPosition === currentPos) {
      parts.push(
        <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
      );
    }
    
    return (
      <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
          )
        )}
      </div>
    );
  }, [value, isFocused, placeholder, cursorPosition, misspelledWords]);

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
  }, [onChange]);

  const handleTextareaKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Hide tooltip on any key press
    setTooltip(prev => ({ ...prev, isVisible: false }));
    
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
    // Hide tooltip when input loses focus
    setTooltip(prev => ({ ...prev, isVisible: false }));
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

  // Tooltip handlers
  const handleSuggestionSelect = useCallback((suggestion: string) => {
    const newValue = value.slice(0, tooltip.wordStart) + 
                     suggestion + 
                     value.slice(tooltip.wordEnd);
    onChange(newValue);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [value, onChange, tooltip.wordStart, tooltip.wordEnd]);

  const handleAddToDictionary = useCallback(() => {
    // TODO: Implement add to dictionary functionality
    console.log('Add to dictionary:', tooltip.misspelledWord);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [tooltip.misspelledWord]);

  const handleIgnore = useCallback(() => {
    // TODO: Implement ignore functionality
    console.log('Ignore word:', tooltip.misspelledWord);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [tooltip.misspelledWord]);

  // Container mouse leave handler
  const handleContainerMouseLeave = useCallback(() => {
    console.log('🖱️ CONTAINER MOUSE LEAVE: Hiding tooltip');
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, []);

  // Hide tooltip when clicking outside or when component loses focus
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      
      // Don't hide if clicking on the tooltip itself or its children
      const tooltipElement = document.querySelector('[data-spell-tooltip="true"]');
      if (tooltipElement && tooltipElement.contains(target)) {
        return;
      }
      
      // Don't hide if clicking on a misspelled word
      const misspelledElement = target as Element;
      if (misspelledElement?.closest?.('[data-misspelled="true"]')) {
        return;
      }
      
      // Hide tooltip if clicking outside the input area
      if (displayRef.current && !displayRef.current.contains(target)) {
        setTooltip(prev => ({ ...prev, isVisible: false }));
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Tooltip hover handlers
  const handleTooltipEnter = useCallback(() => {
    console.log('🖱️ TOOLTIP ENTER: Setting isHoveringTooltip to true');
    setTooltip(prev => ({
      ...prev,
      isHoveringTooltip: true,
    }));
  }, []);

  const handleTooltipLeave = useCallback(() => {
    console.log('🖱️ TOOLTIP LEAVE: Setting isHoveringTooltip to false and hiding tooltip');
    setTooltip(prev => ({
      ...prev,
      isHoveringTooltip: false,
      isVisible: false,
    }));
  }, []);

  return (
    <div 
      className="relative rich-text-input"
      onMouseLeave={handleContainerMouseLeave}
    >
      {/* Hidden textarea for actual input handling with spell check enabled */}
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
        spellCheck={false} // Disable browser spell check - we handle it ourselves
        className="absolute inset-0 w-full h-full resize-none selection:bg-blue-500 selection:text-white"
        onMouseMove={(e) => {
          // Improved hover detection with better coordinate mapping
          const textarea = e.currentTarget;
          const rect = textarea.getBoundingClientRect();
          const textContent = textarea.value;
          
          // Calculate relative mouse position
          const relativeX = e.clientX - rect.left;
          const relativeY = e.clientY - rect.top;
          
          console.log('🖱️ TEXTAREA HOVER: Mouse at relative position:', { x: relativeX, y: relativeY });
          
          // Try multiple methods to get character position
          let charIndex = -1;
          
          // Method 1: Use caretRangeFromPoint
          try {
            const range = document.caretRangeFromPoint(e.clientX, e.clientY);
            if (range && range.startContainer) {
              // If we're in a text node, get the offset
              if (range.startContainer.nodeType === Node.TEXT_NODE) {
                charIndex = range.startOffset;
              }
            }
          } catch (error) {
            console.log('🖱️ TEXTAREA HOVER: caretRangeFromPoint failed:', error);
          }
          
          // Method 2: Estimate based on mouse position (fallback)
          if (charIndex === -1) {
            const charWidth = 8; // Approximate character width
            const lineHeight = 24; // Approximate line height
            const paddingLeft = 12; // Match textarea padding
            const paddingTop = 12;
            
            const adjustedX = Math.max(0, relativeX - paddingLeft);
            const adjustedY = Math.max(0, relativeY - paddingTop);
            
            const estimatedCharInLine = Math.floor(adjustedX / charWidth);
            const estimatedLine = Math.floor(adjustedY / lineHeight);
            
            // Simple estimation for single line
            charIndex = Math.min(estimatedCharInLine, textContent.length - 1);
            console.log('🖱️ TEXTAREA HOVER: Estimated char index:', charIndex);
          }
          
          if (charIndex >= 0 && charIndex < textContent.length) {
            console.log('🖱️ TEXTAREA HOVER: Checking char at index:', charIndex, 'char:', textContent[charIndex]);
            
            // Check if this character position is within any misspelled word
            const misspelledWord = misspelledWords.find(word => 
              charIndex >= word.start && charIndex < word.end
            );
            
            if (misspelledWord) {
              console.log('🖱️ TEXTAREA HOVER: ✅ Found misspelled word:', misspelledWord);
              
              // Calculate static position above the misspelled word
              const charWidth = 8; // Approximate character width
              const paddingLeft = 12; // Match textarea padding
              
              // Calculate the X position of the word start
              const wordStartX = rect.left + paddingLeft + (misspelledWord.start * charWidth);
              
              // Position tooltip above the word with minimal gap
              setTooltip({
                isVisible: true,
                position: { 
                  x: wordStartX, 
                  y: rect.top - 2 // Just 2px above the input
                },
                misspelledWord: misspelledWord.word,
                suggestions: misspelledWord.suggestions || [],
                wordStart: misspelledWord.start,
                wordEnd: misspelledWord.end,
              });
            } else {
              console.log('🖱️ TEXTAREA HOVER: ❌ No misspelled word at position');
              setTooltip(prev => ({ ...prev, isVisible: false }));
            }
          }
        }}
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.15, // Slightly more visible for selection
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'rgba(59, 130, 246, 0.2)', // Slightly more visible blue text
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
          fontFamily: 'Cash Sans, sans-serif', // Match exact font
          fontSize: '0.875rem', // Match text-sm (14px)
          lineHeight: '1.5', // Match leading-relaxed
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          boxSizing: 'border-box',
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
        }}
        rows={rows}
      />
      
      {/* Visual display with action pills, mention pills, spell check, and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 1, // Lower z-index, behind textarea
          pointerEvents: 'none', // Don't interfere with text selection
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          whiteSpace: 'pre-wrap', // Match textarea
          wordWrap: 'break-word',
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>
      
      {/* Spell Check Hover Tooltip */}
      {console.log('🖱️ TOOLTIP RENDER: tooltip state:', tooltip)}
      <SpellCheckTooltip
        isVisible={tooltip.isVisible}
        position={tooltip.position}
        suggestions={tooltip.suggestions}
        misspelledWord={tooltip.misspelledWord}
        onSuggestionSelect={handleSuggestionSelect}
        onAddToDictionary={handleAddToDictionary}
        onIgnore={handleIgnore}
        onMouseEnter={handleTooltipEnter}
        onMouseLeave={handleTooltipLeave}
      />
    </div>
  );
});

RichChatInput.displayName = 'RichChatInput';

export default RichChatInput;
