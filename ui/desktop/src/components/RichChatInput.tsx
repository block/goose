import React, { useRef, useEffect, useState, useCallback, forwardRef, useImperativeHandle, lazy, Suspense } from 'react';
import SpellCheckTooltip from './SpellCheckTooltip';
// Custom spell checking with hover tooltips and highlighting
import { ActionPill } from './ActionPill';
import MentionPill from './MentionPill';
import { Zap, Code, FileText, Search, Play, Settings } from 'lucide-react';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';

// Lazy load Monaco for better performance
const MonacoCodeInput = lazy(() => import('./MonacoCodeInput').then(m => ({ default: m.MonacoCodeInput })));

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

// Icon mapping for custom commands
const getCustomCommandIcon = (iconName?: string) => {
  const iconMap: Record<string, React.ReactNode> = {
    'Zap': <Zap size={12} />,
    'Code': <Code size={12} />,
    'FileText': <FileText size={12} />,
    'Search': <Search size={12} />,
    'Play': <Play size={12} />,
    'Settings': <Settings size={12} />,
  };
  return iconMap[iconName || 'Zap'] || <Zap size={12} />;
};

// Import built-in commands
import { BUILT_IN_COMMANDS } from '../types/customCommands';

// Dynamic action mapping that loads both built-in and user commands
const getActionMap = () => {
  const actionMap: Record<string, { label: string; icon: React.ReactNode }> = {};
  
  try {
    // Add built-in commands
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
    
    builtInCommands.forEach((cmd: any) => {
      actionMap[cmd.id] = {
        label: cmd.label,
        icon: getCustomCommandIcon(cmd.icon),
      };
    });

    // Add user commands
    const stored = localStorage.getItem('goose-custom-commands');
    if (stored) {
      const commands = JSON.parse(stored);
      commands
        .filter((cmd: any) => !cmd.isBuiltIn) // Only user commands
        .forEach((cmd: any) => {
          actionMap[cmd.id] = {
            label: cmd.label,
            icon: getCustomCommandIcon(cmd.icon),
          };
        });
    }
    
    return actionMap;
  } catch (error) {
    console.error('Error loading commands for action map:', error);
    return {};
  }
};

export interface RichChatInputRef {
  focus: () => void;
  blur: () => void;
  setSelectionRange: (start: number, end: number) => void;
  getBoundingClientRect: () => DOMRect;
  contentRef: React.RefObject<HTMLTextAreaElement>;
  resetHeight: () => void;
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
  const containerRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [misspelledWords, setMisspelledWords] = useState<{ word: string; start: number; end: number; suggestions: string[] }[]>([]);
  const [containerHeight, setContainerHeight] = useState<number>(0);
  const [codeMode, setCodeMode] = useState<{ active: boolean; language: string; startPos: number } | null>(null);
  
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
      // Calculate actual line height from the textarea
      const computedStyle = window.getComputedStyle(textarea);
      const fontSize = parseFloat(computedStyle.fontSize);
      const lineHeightValue = computedStyle.lineHeight;
      const lineHeight = Math.round(lineHeightValue === "normal" ? fontSize * 1.2 : parseFloat(lineHeightValue));
      const minHeight = rows * lineHeight;
      // Respect parent's maxHeight constraint from style prop, or use default
      const maxHeight = style?.maxHeight ? parseInt(style.maxHeight.toString()) : 300;
      
      // Force a complete reset to ensure accurate measurement
      textarea.style.height = '0px';
      // Force a reflow
      textarea.offsetHeight;
      textarea.style.height = 'auto';
      
      const textareaScrollHeight = textarea.scrollHeight;
      
      // Calculate desired height based on content, but cap at maxHeight
      const desiredHeight = Math.min(textareaScrollHeight, maxHeight);
      const finalHeight = Math.max(desiredHeight, minHeight);
      
      // Always update heights to ensure consistency
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
        const linesBeforeCursor = textBeforeCursor.split("\n").length;
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
        displayHeight: display.style.height
      });
    }
  }, [rows, style, value]);

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
      display.style.setProperty("line-height", exactLineHeight, "important");
      
      // Also set minHeight using the computed line height for consistency
      const computedLineHeight = parseFloat(exactLineHeight);
      const calculatedMinHeight = rows * computedLineHeight;
      display.style.setProperty("min-height", `${calculatedMinHeight}px`, "important");
      
      // Ensure all child elements inherit the exact line height
      const allChildren = display.querySelectorAll("*");
      allChildren.forEach(child => {
        (child as HTMLElement).style.setProperty("line-height", "inherit", "important");
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
      display.style.webkitFontSmoothing = computedStyle.webkitFontSmoothing;
      display.style.mozOsxFontSmoothing = computedStyle.mozOsxFontSmoothing;
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
    contentRef: hiddenTextareaRef,
    resetHeight: () => {
      console.log('🔄 RESET HEIGHT CALLED');
      if (hiddenTextareaRef.current && displayRef.current) {
        const textarea = hiddenTextareaRef.current;
        const display = displayRef.current;
        // Calculate actual line height from the textarea
        const computedStyle = window.getComputedStyle(textarea);
        const fontSize = parseFloat(computedStyle.fontSize);
        const lineHeightValue = computedStyle.lineHeight;
        const lineHeight = Math.round(lineHeightValue === "normal" ? fontSize * 1.2 : parseFloat(lineHeightValue));
        const minHeight = rows * lineHeight;
        
        // Reset to auto first, then set to minHeight - DON'T call syncDisplayHeight after!
        textarea.style.height = "auto";
        textarea.style.height = `${minHeight}px`;
        display.style.height = `${minHeight}px`;
        setContainerHeight(minHeight);
        
        console.log('🔄 RESET HEIGHT: Set to minHeight', minHeight);
      }
    },
  }), [rows, syncDisplayHeight, ensureStyleConsistency]);

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

  // Detect code mode triggers (#language) - can be anywhere in the text
  useEffect(() => {
    // Match #language at word boundary (not followed by more letters)
    const languageTriggerRegex = /#(javascript|typescript|python|java|cpp|c|go|rust|ruby|php|swift|kotlin|scala|html|css|json|yaml|sql|bash|shell|powershell|r|matlab|lua|perl|haskell|elixir|clojure|dart|jsx|tsx)(?![a-z])/i;
    const match = value.match(languageTriggerRegex);
    
    if (match && !codeMode) {
      // Enter code mode
      const language = match[1].toLowerCase();
      const triggerStart = match.index || 0;
      const triggerLength = match[0].length;
      console.log('💻 CODE MODE ACTIVATED:', language, 'at position', triggerStart);
      
      // Always insert a newline after the trigger to move cursor into code block
      const beforeTrigger = value.slice(0, triggerStart + triggerLength);
      const afterTrigger = value.slice(triggerStart + triggerLength);
      
      // Check if there's already a newline
      const hasNewline = afterTrigger.startsWith('\n');
      
      if (!hasNewline) {
        const newValue = beforeTrigger + '\n' + afterTrigger;
        const newCursorPos = triggerStart + triggerLength + 1; // Position after the newline
        
        console.log('💻 CODE MODE: Inserting newline after trigger');
        onChange(newValue, newCursorPos);
        
        // Set cursor position in textarea
        if (hiddenTextareaRef.current) {
          setTimeout(() => {
            hiddenTextareaRef.current?.setSelectionRange(newCursorPos, newCursorPos);
            setCursorPosition(newCursorPos);
          }, 0);
        }
        
        setCodeMode({
          active: true,
          language: language,
          startPos: triggerStart + triggerLength + 1 // Account for the newline
        });
      } else {
        setCodeMode({
          active: true,
          language: language,
          startPos: triggerStart + triggerLength + 1 // Skip the existing newline
        });
      }
    } else if (!match && codeMode) {
      // Exit code mode if trigger is removed
      console.log('💻 CODE MODE DEACTIVATED');
      setCodeMode(null);
    } else if (match && codeMode) {
      // Check for double newline to exit code mode
      const codeContent = value.slice(codeMode.startPos);
      if (codeContent.endsWith('\n\n')) {
        console.log('💻 CODE MODE: Double newline detected, exiting code mode and converting to code block');
        // Exit code mode by converting to triple-backtick code block
        const triggerStart = match.index || 0;
        const triggerEnd = triggerStart + match[0].length + 1; // +1 for the newline after trigger
        const textBefore = value.slice(0, triggerStart);
        const codeAndAfter = value.slice(triggerEnd);
        
        // Remove the double newline at the end
        const cleanedCode = codeAndAfter.replace(/\n\n$/, '');
        
        // Convert to triple-backtick format: ```language\ncode\n```
        const language = codeMode.language;
        const newValue = textBefore + '```' + language + '\n' + cleanedCode + '\n```\n';
        const newCursorPos = newValue.length;
        
        onChange(newValue, newCursorPos);
        setCodeMode(null);
        
        // Set cursor position
        if (hiddenTextareaRef.current) {
          setTimeout(() => {
            hiddenTextareaRef.current?.setSelectionRange(newCursorPos, newCursorPos);
            setCursorPosition(newCursorPos);
          }, 0);
        }
      }
    }
  }, [value, codeMode, onChange]);

  // Parse and render content with action pills, mention pills, and cursor
  const renderContent = useCallback(() => {
    // Show placeholder when there's no text content (but account for whitespace-only content with newlines)
    if (!value || (value.trim() === '' && !value.includes('\n'))) {
      return (
        <div className="whitespace-pre-wrap relative">
          {/* Placeholder text positioned absolutely to prevent movement */}
          <span className="text-text-muted pointer-events-none select-none absolute inset-0">
            {placeholder || (codeMode ? `Type your ${codeMode.language} code here... (Cmd+Enter to send)` : placeholder)}
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
    
    // Check if we're in code mode
    if (codeMode && codeMode.active) {
      console.log('💻 RENDERING IN CODE MODE:', codeMode);
      
      // Find the trigger position (use same regex as detection)
      const languageTriggerRegex = /#(javascript|typescript|python|java|cpp|c|go|rust|ruby|php|swift|kotlin|scala|html|css|json|yaml|sql|bash|shell|powershell|r|matlab|lua|perl|haskell|elixir|clojure|dart|jsx|tsx)(?![a-z])/i;
      const match = value.match(languageTriggerRegex);
      
      if (!match || match.index === undefined) {
        // Shouldn't happen, but fallback to normal rendering
        return null;
      }
      
      const triggerStart = match.index;
      const textBefore = value.slice(0, triggerStart);
      const codeContent = value.slice(codeMode.startPos);
      
      // Calculate cursor position within the code content
      const cursorInCode = cursorPosition >= codeMode.startPos;
      const cursorOffsetInCode = cursorInCode ? cursorPosition - codeMode.startPos : -1;
      
      // Split code content at cursor position for rendering
      let codeBeforeCursor = '';
      let codeAfterCursor = '';
      if (cursorInCode && cursorOffsetInCode >= 0) {
        codeBeforeCursor = codeContent.slice(0, cursorOffsetInCode);
        codeAfterCursor = codeContent.slice(cursorOffsetInCode);
      }
      
      return (
        <div className="whitespace-pre-wrap relative w-full">
          {/* Text before the trigger (if any) */}
          {textBefore && (
            <span className="inline whitespace-pre-wrap">{textBefore}</span>
          )}
          
          {/* Language badge */}
          <div className="inline-flex items-center gap-1 px-2 py-0.5 mb-1 text-xs font-mono text-gray-300 bg-gray-800 rounded border border-gray-700">
            <Code size={12} />
            <span>{codeMode.language}</span>
          </div>
          
          {/* Code content with syntax highlighting - constrained to container width */}
          <div 
            className="block font-mono text-sm bg-[#1E1E1E]/30 rounded p-2 border border-gray-700/50 mt-1 w-full overflow-x-auto relative"
            style={{ 
              fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, source-code-pro, monospace',
              maxWidth: '100%',
              boxSizing: 'border-box',
            }}
          >
            {cursorInCode ? (
              // Render with cursor at the correct position
              <div className="relative">
                <SyntaxHighlighter
                  language={codeMode.language}
                  style={vscDarkPlus}
                  customStyle={{
                    margin: 0,
                    padding: 0,
                    background: 'transparent',
                    fontSize: '0.875rem',
                    lineHeight: '1.5',
                    maxWidth: '100%',
                    overflowX: 'auto',
                  }}
                  PreTag="span"
                  CodeTag="span"
                  wrapLines={true}
                  showLineNumbers={false}
                >
                  {codeBeforeCursor || ' '}
                </SyntaxHighlighter>
                {/* Cursor at current position */}
                {isFocused && (
                  <span 
                    className="border-l border-text-default inline-block" 
                    style={{ 
                      animation: "blink 1s step-end infinite", 
                      height: "1.3em",
                      width: "1px",
                      position: "relative",
                      verticalAlign: "text-bottom"
                    }} 
                  />
                )}
                <SyntaxHighlighter
                  language={codeMode.language}
                  style={vscDarkPlus}
                  customStyle={{
                    margin: 0,
                    padding: 0,
                    background: 'transparent',
                    fontSize: '0.875rem',
                    lineHeight: '1.5',
                    maxWidth: '100%',
                    overflowX: 'auto',
                  }}
                  PreTag="span"
                  CodeTag="span"
                  wrapLines={true}
                  showLineNumbers={false}
                >
                  {codeAfterCursor}
                </SyntaxHighlighter>
              </div>
            ) : (
              // No cursor in code area, just render the code
              <SyntaxHighlighter
                language={codeMode.language}
                style={vscDarkPlus}
                customStyle={{
                  margin: 0,
                  padding: 0,
                  background: 'transparent',
                  fontSize: '0.875rem',
                  lineHeight: '1.5',
                  maxWidth: '100%',
                  overflowX: 'auto',
                }}
                PreTag="div"
                CodeTag="code"
                wrapLines={true}
                showLineNumbers={false}
              >
                {codeContent || ' '}
              </SyntaxHighlighter>
            )}
          </div>
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
    
    // Find all code block matches
    const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;
    let codeBlockMatch;
    codeBlockRegex.lastIndex = 0;
    console.log('💻 Searching for code blocks in value:', value);
    while ((codeBlockMatch = codeBlockRegex.exec(value)) !== null) {
      const language = codeBlockMatch[1] || 'text';
      const code = codeBlockMatch[2];
      console.log('💻 Found code block match:', { language, code, index: codeBlockMatch.index });
      allMatches.push({
        type: 'codeblock',
        match: codeBlockMatch,
        index: codeBlockMatch.index,
        length: codeBlockMatch[0].length,
        content: code,
        language: language
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
        const actionMap = getActionMap();
        const actionEntry = Object.entries(actionMap).find(
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
      } else if (type === 'codeblock') {
        // Handle code blocks with syntax highlighting
        const language = (matchData as any).language || 'text';
        const code = content;
        
        console.log('💻 Creating code block:', { language, code });
        
        parts.push(
          <div
            key={`codeblock-${keyCounter++}`}
            className="block my-2"
            style={{ pointerEvents: 'auto' }}
          >
            <div className="relative rounded-lg overflow-hidden bg-[#1E1E1E] border border-gray-700">
              {/* Language badge */}
              {language && language !== 'text' && (
                <div className="absolute top-2 right-2 px-2 py-1 text-xs font-mono text-gray-300 bg-gray-800/80 rounded">
                  {language}
                </div>
              )}
              {/* Syntax highlighted code */}
              <SyntaxHighlighter
                language={language}
                style={vscDarkPlus}
                customStyle={{
                  margin: 0,
                  padding: '16px',
                  background: 'transparent',
                  fontSize: '0.875rem',
                  lineHeight: '1.5',
                }}
                codeTagProps={{
                  style: {
                    fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, source-code-pro, monospace',
                  }
                }}
              >
                {code}
              </SyntaxHighlighter>
            </div>
          </div>
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
      <div className="whitespace-pre-wrap">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l border-text-default inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.3em", width: "1px", marginLeft: "0px", transform: "translateY(0.1em)", position: "relative" }} />
          )
        )}
      </div>
    );
  }, [value, isFocused, placeholder, cursorPosition, misspelledWords, codeMode]);

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
      ensureStyleConsistency();
    requestAnimationFrame(() => syncDisplayHeight());
  }, [onChange, syncDisplayHeight]);

  const handleTextareaKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Hide tooltip on any key press
    setTooltip(prev => ({ ...prev, isVisible: false }));
    
    // Update cursor position on key events
    setTimeout(updateCursorPosition, 0);
    
    // In code mode, handle Enter differently
    if (codeMode && codeMode.active) {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        // Cmd+Enter or Ctrl+Enter: Exit code mode and send message
        console.log('💻 CODE MODE: Cmd/Ctrl+Enter pressed, passing to parent');
        // Don't prevent default - let parent handle sending
      } else if (e.key === 'Enter') {
        // Plain Enter in code mode: Insert newline, don't send
        console.log('💻 CODE MODE: Enter pressed, inserting newline');
        e.preventDefault();
        const cursorPos = e.currentTarget.selectionStart;
        const newValue = value.slice(0, cursorPos) + '\n' + value.slice(cursorPos);
        onChange(newValue, cursorPos + 1);
        return;
      }
    }
    
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
    
    // Pass the original event directly to maintain all properties and behavior
    // The parent ChatInput expects a KeyboardEvent<HTMLDivElement>, so we need to cast it properly
    const syntheticEvent = {
      ...e,
      // Preserve all keyboard event properties
      key: e.key,
      code: e.code,
      keyCode: e.keyCode,
      which: e.which,
      shiftKey: e.shiftKey,
      altKey: e.altKey,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
      repeat: e.repeat,
      // Preserve event methods
      preventDefault: e.preventDefault.bind(e),
      stopPropagation: e.stopPropagation.bind(e),
      stopImmediatePropagation: e.stopImmediatePropagation?.bind(e),
      // Update target and currentTarget to reference the display div for positioning
      currentTarget: displayRef.current as any,
      target: displayRef.current as any,
      // Preserve other event properties
      bubbles: e.bubbles,
      cancelable: e.cancelable,
      defaultPrevented: e.defaultPrevented,
      eventPhase: e.eventPhase,
      isTrusted: e.isTrusted,
      timeStamp: e.timeStamp,
      type: e.type,
    } as React.KeyboardEvent<HTMLDivElement>;
    
    onKeyDown?.(syntheticEvent);
  }, [value, handleRemoveAction, onKeyDown, updateCursorPosition, codeMode, onChange]);

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

  // Explicitly sync height whenever value changes (including when text is deleted)
  useEffect(() => {
    syncDisplayHeight();
    ensureStyleConsistency();
  }, [value, syncDisplayHeight, ensureStyleConsistency]);
  
  // Sync height when code mode changes or when there are code blocks (for initial render and updates)
  useEffect(() => {
    // Check if we need display-based height measurement
    const hasCodeBlocks = /```\w*\n[\s\S]*?```/.test(value);
    const needsDisplayMeasurement = codeMode || hasCodeBlocks;
    
    if (needsDisplayMeasurement) {
      // When code mode is active or code blocks are present, we need to measure the actual display height
      // because the textarea doesn't know about the styled code block
      console.log('💻 CODE BLOCKS: Triggering height sync for code blocks', { codeMode: !!codeMode, hasCodeBlocks });
      
      // Use a small delay to ensure the SyntaxHighlighter has rendered
      const timer = setTimeout(() => {
        if (displayRef.current && hiddenTextareaRef.current) {
          const display = displayRef.current;
          const textarea = hiddenTextareaRef.current;
          
          // Get the actual rendered height of the display content
          const displayScrollHeight = display.scrollHeight;
          
          console.log('💻 CODE BLOCKS: Display scrollHeight:', displayScrollHeight);
          
          // Calculate line height
          const computedStyle = window.getComputedStyle(textarea);
          const fontSize = parseFloat(computedStyle.fontSize);
          const lineHeightValue = computedStyle.lineHeight;
          const lineHeight = Math.round(lineHeightValue === "normal" ? fontSize * 1.2 : parseFloat(lineHeightValue));
          const minHeight = rows * lineHeight;
          const maxHeight = style?.maxHeight ? parseInt(style.maxHeight.toString()) : 300;
          
          // Use the display's scroll height instead of textarea's
          const desiredHeight = Math.min(displayScrollHeight, maxHeight);
          const finalHeight = Math.max(desiredHeight, minHeight);
          
          console.log('💻 CODE BLOCKS: Setting height to', finalHeight);
          
          // Update both layers
          textarea.style.height = `${finalHeight}px`;
          display.style.height = `${finalHeight}px`;
          setContainerHeight(finalHeight);
        }
      }, 50); // Small delay to let SyntaxHighlighter render
      
      return () => clearTimeout(timer);
    }
  }, [codeMode, value, rows, style]);
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
      ref={containerRef}
      className="relative rich-text-input"
      style={{ height: containerHeight > 0 ? `${containerHeight}px` : 'auto' }}
      onMouseLeave={handleContainerMouseLeave}
    >
      {/* Hidden textarea for actual input handling with spell check disabled (we handle it ourselves) */}
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
          boxSizing: "border-box", // Match textarea box model
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
        className={`${className} cursor-text absolute inset-0 overflow-y-auto rich-text-display`}
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
          boxSizing: "border-box", // Match textarea box model
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
