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
      
      // Critical: Completely reset height and clear any constraints to get accurate measurement
      textarea.style.height = 'auto';
      textarea.style.minHeight = 'auto';
      textarea.style.maxHeight = 'none';
      
      // Also reset display constraints
      display.style.height = 'auto';
      display.style.minHeight = 'auto';
      display.style.maxHeight = 'none';
      
      // Reset display content constraints too
      const displayContent = display.firstElementChild as HTMLElement;
      if (displayContent) {
        displayContent.style.minHeight = 'auto';
        displayContent.style.height = 'auto';
      }
      
      // Force a reflow to ensure the browser recalculates dimensions
      textarea.offsetHeight;
      display.offsetHeight;
      
      const textareaScrollHeight = textarea.scrollHeight;
      
      // Allow expansion up to 300px, then use scrolling
      const maxHeight = 300;
      const minHeight = rows * 24; // Approximate line height
      
      // Calculate desired height based on content, but cap at maxHeight
      const desiredHeight = Math.min(textareaScrollHeight, maxHeight);
      const finalHeight = Math.max(desiredHeight, minHeight);
      
      // Update both textarea and display layer heights to match exactly
      textarea.style.height = `${finalHeight}px`;
      textarea.style.minHeight = `${finalHeight}px`;
      textarea.style.maxHeight = `${finalHeight}px`;
      
      display.style.height = `${finalHeight}px`;
      display.style.minHeight = `${finalHeight}px`;
      display.style.maxHeight = `${finalHeight}px`;
      
      // Critical: Ensure both have the same dimensions and overflow behavior
      display.style.width = textarea.style.width;
      
      // Force the display layer to have the same scrollable content area
      // This ensures that when the textarea scrolls, the display can show the same content
      if (displayContent) {
        // Only set minHeight if content is taller than the container, otherwise let it be natural
        if (textareaScrollHeight > finalHeight) {
          displayContent.style.minHeight = `${textareaScrollHeight}px`;
        } else {
          displayContent.style.minHeight = 'auto';
        }
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
        scrollTop: textarea.scrollTop,
        textareaHeight: textarea.style.height,
        displayHeight: display.style.height,
        displayContentHeight: displayContent?.style.minHeight
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
  }, [syncDisplayHeight]);
  
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

  // Track the last spell checked text to avoid unnecessary re-checks
  const lastSpellCheckedTextRef = useRef<string>('');
  const spellCheckTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Spell check the content using Electron's system spell checker
  const performSpellCheck = useCallback(async (text: string, isIncremental = false) => {
    console.log('🔍 ELECTRON SPELL CHECK: Starting system spell check for text:', text, 'incremental:', isIncremental);
    
    // Skip if we've already checked this exact text
    if (text === lastSpellCheckedTextRef.current) {
      console.log('🔍 ELECTRON SPELL CHECK: Skipping - text unchanged');
      return;
    }

    // Use the Electron spell checking function
    try {
      const misspelledWords = await checkSpelling(text);
      console.log('🔍 ELECTRON SPELL CHECK: System spell check result:', misspelledWords);
      
      // Critical: Check current value at time of update, not captured value
      const currentValue = hiddenTextareaRef.current?.value || '';
      
      // Only update if the text hasn't changed since we started checking
      if (text === currentValue) {
        // Use functional update to avoid dependency on value in useCallback
        setMisspelledWords(misspelledWords);
        lastSpellCheckedTextRef.current = text;
      } else {
        console.log('🔍 ELECTRON SPELL CHECK: Discarding stale result - text changed during check');
      }
    } catch (error) {
      console.error('🔍 ELECTRON SPELL CHECK: Error performing spell check:', error);
      // Fallback to no spell checking on error
      const currentValue = hiddenTextareaRef.current?.value || '';
      if (text === currentValue) {
        setMisspelledWords([]);
      }
    }
  }, []); // Remove value dependency to prevent recreation

  // Smart spell check timing - check after word completion and with shorter delays
  useEffect(() => {
    // Clear any existing timeout
    if (spellCheckTimeoutRef.current) {
      clearTimeout(spellCheckTimeoutRef.current);
    }

    if (!value.trim()) {
      setMisspelledWords([]);
      lastSpellCheckedTextRef.current = '';
      return;
    }

    // Detect if user just completed a word (typed space, punctuation, or newline)
    const lastChar = value[value.length - 1];
    const isWordBoundary = /[\s\n\.,!?;:]/.test(lastChar);
    
    // Use different delays based on context
    let delay: number;
    if (isWordBoundary) {
      // Just completed a word - check quickly
      delay = 150;
    } else {
      // Still typing within a word - wait longer to avoid interrupting
      delay = 300;
    }

    console.log('🔍 SPELL CHECK TIMING: Setting timeout with delay:', delay, 'isWordBoundary:', isWordBoundary, 'lastChar:', lastChar);

    spellCheckTimeoutRef.current = setTimeout(() => {
      performSpellCheck(value, !isWordBoundary);
    }, delay);

    return () => {
      if (spellCheckTimeoutRef.current) {
        clearTimeout(spellCheckTimeoutRef.current);
      }
    };
  }, [value, performSpellCheck]);

  // Parse and render content with action pills, mention pills, spell checking, and cursor
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
              className="border-l-2 border-text-default inline-block align-baseline absolute" 
              style={{ 
                animation: "blink 1s step-end infinite", 
                height: "1em", 
                width: "1px",
                left: "0px",
                top: "0.125em", // Align with text baseline
                verticalAlign: "baseline"
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
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
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
          <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
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
            className="inline whitespace-pre-wrap cursor-pointer bg-red-50 dark:bg-red-950/30 text-red-600 dark:text-red-400 font-medium px-1 py-0.5 rounded-sm border border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/40 hover:border-red-300 dark:hover:border-red-700 hover:scale-105 transition-all duration-150 relative z-50"
            style={{
              pointerEvents: 'auto', // Override parent's pointer-events: none
              userSelect: 'text', // Allow text selection for normal text editing
            }}
            title={`Click or hover for suggestions: ${content}`}
            onClick={(e) => {
              console.log('🖱️ CLICK: Clicked on misspelled word:', content);
              e.preventDefault();
              e.stopPropagation();
              
              // Ensure textarea maintains focus after click
              setTimeout(() => {
                if (hiddenTextareaRef.current) {
                  hiddenTextareaRef.current.focus();
                }
              }, 0);
              
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
              
              // Only show tooltip on hover if we're not actively typing
              // Check if the textarea has been recently focused/active
              const now = Date.now();
              const lastActivity = hiddenTextareaRef.current?.dataset.lastActivity;
              const timeSinceActivity = lastActivity ? now - parseInt(lastActivity) : Infinity;
              
              // Only show hover tooltip if it's been more than 500ms since last typing activity
              if (timeSinceActivity > 500 && misspelledData) {
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
    if (remainingText || lastIndex < value.length) {
      let textWithCursor = [];
      for (let i = 0; i < remainingText.length; i++) {
        if (isFocused && cursorPosition === currentPos) {
          textWithCursor.push(
            <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
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
        <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
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
            <span className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
          )}
        </span>
      );
    }
    
    return (
      <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", width: "1px", marginLeft: "0px", verticalAlign: "baseline" }} />
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
    
    // Track typing activity to prevent hover tooltips while actively typing
    if (hiddenTextareaRef.current) {
      hiddenTextareaRef.current.dataset.lastActivity = Date.now().toString();
    }
    
    console.log('🔄 RichChatInput: onChange', { newValue, newCursorPos });
    onChange(newValue, newCursorPos);
    setCursorPosition(newCursorPos);
    
    // Sync display height immediately for better responsiveness
    // Use both immediate sync and deferred sync for reliability
    syncDisplayHeight();
    requestAnimationFrame(() => syncDisplayHeight());
  }, [onChange, syncDisplayHeight]);

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

  // Start monitoring textarea changes for height synchronization
  useEffect(() => {
    const cleanup = monitorTextareaChanges();
    return cleanup;
  }, [monitorTextareaChanges]);

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
      
      {/* Visual display with action pills, mention pills, spell check, and cursor */}
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
